//! The sheetbook: a folder of `*.bbg` files.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::atomic::{sweep, write_atomic};
use crate::format::{count_lines, decode, encode, title_of};
use crate::{Error, IoContext, Result};

const EXT: &str = "bbg";
/// The first name of the file, before the owner chose `.bbg`: sheets saved so are renamed when the book opens.
const LEGACY_EXT: &str = "babbage";
const TRASH: &str = ".trash";
/// Names the book gives sheets itself start with this; any other name was chosen by the user.
const AUTO_PREFIX: &str = "sheet-";

/// A sheet's name: its file name without `.bbg`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SheetId(String);

impl SheetId {
    /// Checks that `name` is safe to use as a file name inside the book.
    pub fn new(name: &str) -> Result<SheetId> {
        let name = name.trim();
        let bad = name.is_empty()
            || name.starts_with('.')
            || name.chars().count() > 100
            || name
                .chars()
                .any(|c| c.is_control() || c == '/' || c == '\\');
        if bad {
            return Err(Error::BadName(name.to_owned()));
        }
        Ok(SheetId(name.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SheetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// What the sidebar shows of a sheet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetInfo {
    pub id: SheetId,
    /// The first line with something on it, if any.
    pub title: Option<String>,
    /// Whether the user named the sheet (as opposed to the book's own `sheet-N`).
    pub named: bool,
    pub modified: SystemTime,
    /// Lines up to the last one with something on it.
    pub lines: usize,
}

impl SheetInfo {
    /// The text the sidebar prints as the sheet's name: the user's name if it has one, else the title,
    /// else "Empty".
    pub fn display_name(&self) -> String {
        if self.named {
            self.id.0.clone()
        } else {
            self.title.clone().unwrap_or_else(|| "Empty".to_owned())
        }
    }
}

/// Where the sheets live: `BABBAGE_DATA_DIR`, else `$XDG_DATA_HOME/babbage/sheets`, else
/// `~/.local/share/babbage/sheets`. Tests set the variable.
pub fn data_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("BABBAGE_DATA_DIR").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(d));
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|d| !d.is_empty())
                .map(|h| PathBuf::from(h).join(".local/share"))
        })?;
    Some(base.join("babbage").join("sheets"))
}

/// Renames `name.babbage` to `name.bbg` where no `name.bbg` exists, so sheets saved under the first extension
/// stay in the book. A pair with both is left alone: neither is overwritten.
fn adopt_legacy_names(dir: &Path) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let old = entry.path();
        if old.extension().is_none_or(|e| e != LEGACY_EXT)
            || !entry.file_type().is_ok_and(|t| t.is_file())
        {
            continue;
        }
        let new = old.with_extension(EXT);
        if SheetBook::id_of(&new).is_some() && !new.exists() {
            let _ = fs::rename(&old, &new);
        }
    }
}

#[derive(Debug)]
pub struct SheetBook {
    dir: PathBuf,
}

impl SheetBook {
    /// Opens (creating if needed) the book at `dir`, and clears temporary files a crash left.
    pub fn open(dir: impl Into<PathBuf>) -> Result<SheetBook> {
        let dir = dir.into();
        fs::create_dir_all(&dir).ctx(|| format!("create {}", dir.display()))?;
        sweep(&dir);
        adopt_legacy_names(&dir);
        Ok(SheetBook { dir })
    }

    /// Opens the book at `data_dir()`.
    pub fn open_default() -> Result<SheetBook> {
        let dir = data_dir().ok_or_else(|| Error::Io {
            context: "find the data folder".into(),
            source: std::io::Error::other(
                "neither BABBAGE_DATA_DIR, XDG_DATA_HOME nor HOME is set",
            ),
        })?;
        SheetBook::open(dir)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn trash_dir(&self) -> PathBuf {
        self.dir.join(TRASH)
    }

    pub fn path_of(&self, id: &SheetId) -> PathBuf {
        self.dir.join(format!("{}.{EXT}", id.0))
    }

    fn id_of(path: &Path) -> Option<SheetId> {
        if path.extension()? != EXT {
            return None;
        }
        let stem = path.file_stem()?.to_str()?;
        SheetId::new(stem).ok().filter(|id| id.0 == stem)
    }

    /// Every sheet, most recently modified first (ties by name). Files that are not `*.bbg`, hidden
    /// files and the trash are skipped; a sheet that cannot be read as text is skipped too.
    pub fn list(&self) -> Result<Vec<SheetInfo>> {
        let rd = fs::read_dir(&self.dir).ctx(|| format!("list {}", self.dir.display()))?;
        let mut out = Vec::new();
        for entry in rd.flatten() {
            let path = entry.path();
            let Some(id) = Self::id_of(&path) else {
                continue;
            };
            if !entry.file_type().is_ok_and(|t| t.is_file()) {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            let lines = decode(&text);
            out.push(SheetInfo {
                named: !id.0.starts_with(AUTO_PREFIX),
                title: title_of(&lines),
                modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                lines: count_lines(&lines),
                id,
            });
        }
        out.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.id.cmp(&b.id)));
        Ok(out)
    }

    pub fn exists(&self, id: &SheetId) -> bool {
        self.path_of(id).is_file()
    }

    /// Reads a sheet's lines.
    pub fn read(&self, id: &SheetId) -> Result<Vec<String>> {
        let path = self.path_of(id);
        let bytes = match fs::read(&path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(Error::NoSuchSheet(id.0.clone()));
            }
            Err(e) => return Err(e).ctx(|| format!("read {}", path.display())),
        };
        let text =
            String::from_utf8(bytes).map_err(|_| Error::NotUtf8(path.display().to_string()))?;
        Ok(decode(&text))
    }

    /// Writes a sheet's lines atomically, creating the sheet if it is new.
    pub fn save<S: AsRef<str>>(&self, id: &SheetId, lines: &[S]) -> Result<()> {
        write_atomic(&self.path_of(id), encode(lines).as_bytes())
    }

    /// Makes a new sheet under the book's own next free name (`sheet-1`, `sheet-2`, ...).
    pub fn create<S: AsRef<str>>(&self, lines: &[S]) -> Result<SheetId> {
        for n in 1.. {
            let id = SheetId(format!("{AUTO_PREFIX}{n}"));
            if !self.path_of(&id).exists() {
                self.save(&id, lines)?;
                return Ok(id);
            }
        }
        unreachable!("the loop above ends when a free name is found")
    }

    /// Makes a new sheet called `name`; refuses a name already taken.
    pub fn create_named<S: AsRef<str>>(&self, name: &str, lines: &[S]) -> Result<SheetId> {
        let id = SheetId::new(name)?;
        if self.exists(&id) {
            return Err(Error::Exists(id.0));
        }
        self.save(&id, lines)?;
        Ok(id)
    }

    /// Gives a sheet a new name (renaming its file). Refuses a name already taken. Renaming to the
    /// same name does nothing.
    pub fn rename(&self, id: &SheetId, new_name: &str) -> Result<SheetId> {
        let to = SheetId::new(new_name)?;
        if to == *id {
            return Ok(to);
        }
        if !self.exists(id) {
            return Err(Error::NoSuchSheet(id.0.clone()));
        }
        if self.exists(&to) {
            return Err(Error::Exists(to.0));
        }
        let (a, b) = (self.path_of(id), self.path_of(&to));
        fs::rename(&a, &b).ctx(|| format!("rename {} to {}", a.display(), b.display()))?;
        Ok(to)
    }

    /// Moves a sheet to the trash folder, under a name that cannot collide. Returns where it went.
    pub fn delete(&self, id: &SheetId) -> Result<PathBuf> {
        let from = self.path_of(id);
        if !from.is_file() {
            return Err(Error::NoSuchSheet(id.0.clone()));
        }
        let trash = self.trash_dir();
        fs::create_dir_all(&trash).ctx(|| format!("create {}", trash.display()))?;
        let stamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        for n in 0.. {
            let suffix = if n == 0 {
                String::new()
            } else {
                format!("-{n}")
            };
            let to = trash.join(format!("{}.{stamp}{suffix}.{EXT}", id.0));
            if to.exists() {
                continue;
            }
            fs::rename(&from, &to).ctx(|| format!("move {} to the trash", from.display()))?;
            return Ok(to);
        }
        unreachable!("the loop above ends when a free trash name is found")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::time::Duration;

    fn book() -> (tempfile::TempDir, SheetBook) {
        let d = tempfile::tempdir().unwrap();
        let b = SheetBook::open(d.path().join("sheets")).unwrap();
        (d, b)
    }

    fn set_age(b: &SheetBook, id: &SheetId, secs_ago: u64) {
        let t = SystemTime::now() - Duration::from_secs(secs_ago);
        let f = fs::File::options().write(true).open(b.path_of(id)).unwrap();
        f.set_modified(t).unwrap();
    }

    #[test]
    fn a_new_book_is_empty_and_creates_its_folder() {
        let (_d, b) = book();
        assert!(b.dir().is_dir());
        assert_eq!(b.list().unwrap(), vec![]);
    }

    #[test]
    fn create_picks_the_next_free_automatic_name() {
        let (_d, b) = book();
        let a = b.create(&["1 + 1"]).unwrap();
        let c = b.create::<&str>(&[]).unwrap();
        assert_eq!((a.as_str(), c.as_str()), ("sheet-1", "sheet-2"));
        b.delete(&a).unwrap();
        assert_eq!(b.create::<&str>(&[]).unwrap().as_str(), "sheet-1");
    }

    #[test]
    fn a_saved_sheet_reads_back_and_is_plain_text_on_disk() {
        let (_d, b) = book();
        let id = b.create(&["12 + 12", "", "// note"]).unwrap();
        assert_eq!(b.read(&id).unwrap(), vec!["12 + 12", "", "// note"]);
        assert_eq!(
            fs::read_to_string(b.path_of(&id)).unwrap(),
            "12 + 12\n\n// note\n"
        );
    }

    #[test]
    fn the_listing_shows_title_line_count_and_time_newest_first() {
        let (_d, b) = book();
        let old = b.create(&["", "10 x $7.49", "a", "b", "", ""]).unwrap();
        let new = b.create::<&str>(&[]).unwrap();
        set_age(&b, &old, 3600);
        let l = b.list().unwrap();
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].id, new);
        assert_eq!((l[0].title.clone(), l[0].lines), (None, 0));
        assert_eq!(l[0].display_name(), "Empty");
        assert_eq!(l[1].id, old);
        assert_eq!(l[1].title.as_deref(), Some("10 x $7.49"));
        assert_eq!(l[1].lines, 4, "trailing blank lines do not count");
        assert!(l[0].modified > l[1].modified);
    }

    #[test]
    fn a_user_name_replaces_the_title_in_the_display_name() {
        let (_d, b) = book();
        let id = b.create(&["1 + 1"]).unwrap();
        assert_eq!(b.list().unwrap()[0].display_name(), "1 + 1");
        let id = b.rename(&id, "Budget").unwrap();
        let l = b.list().unwrap();
        assert!(l[0].named);
        assert_eq!(l[0].display_name(), "Budget");
        assert_eq!(l[0].title.as_deref(), Some("1 + 1"));
        assert_eq!(b.read(&id).unwrap(), vec!["1 + 1"]);
    }

    #[test]
    fn rename_refuses_a_taken_name_and_keeps_both_sheets() {
        let (_d, b) = book();
        let a = b.create_named("A", &["a"]).unwrap();
        let c = b.create_named("C", &["c"]).unwrap();
        assert!(matches!(b.rename(&a, "C"), Err(Error::Exists(_))));
        assert_eq!(b.read(&a).unwrap(), vec!["a"]);
        assert_eq!(b.read(&c).unwrap(), vec!["c"]);
        assert_eq!(
            b.rename(&a, "A").unwrap(),
            a,
            "renaming to itself is a no-op"
        );
    }

    #[test]
    fn rename_of_a_missing_sheet_is_an_error() {
        let (_d, b) = book();
        let ghost = SheetId::new("ghost").unwrap();
        assert!(matches!(b.rename(&ghost, "x"), Err(Error::NoSuchSheet(_))));
    }

    #[test]
    fn unsafe_names_are_refused() {
        for bad in [
            "",
            "  ",
            ".hidden",
            "a/b",
            "..",
            "a\\b",
            "tab\there",
            &"x".repeat(101),
        ] {
            assert!(SheetId::new(bad).is_err(), "{bad:?}");
        }
        assert!(SheetId::new(" fine name ").is_ok());
    }

    #[test]
    fn delete_moves_the_file_to_the_trash_and_never_removes_it() {
        let (_d, b) = book();
        let id = b.create(&["keep me"]).unwrap();
        let gone = b.delete(&id).unwrap();
        assert!(!b.exists(&id));
        assert!(b.list().unwrap().is_empty());
        assert!(gone.starts_with(b.trash_dir()));
        assert_eq!(fs::read_to_string(&gone).unwrap(), "keep me\n");
    }

    #[test]
    fn deleting_two_sheets_of_one_name_keeps_both_in_the_trash() {
        let (_d, b) = book();
        let id = b.create_named("same", &["one"]).unwrap();
        let p1 = b.delete(&id).unwrap();
        b.create_named("same", &["two"]).unwrap();
        let p2 = b.delete(&id).unwrap();
        assert_ne!(p1, p2);
        assert_eq!(fs::read_to_string(p1).unwrap(), "one\n");
        assert_eq!(fs::read_to_string(p2).unwrap(), "two\n");
    }

    #[test]
    fn delete_of_a_missing_sheet_is_an_error() {
        let (_d, b) = book();
        assert!(matches!(
            b.delete(&SheetId::new("x").unwrap()),
            Err(Error::NoSuchSheet(_))
        ));
    }

    #[test]
    fn the_listing_ignores_other_files_hidden_files_the_trash_and_directories() {
        let (_d, b) = book();
        b.create(&["real"]).unwrap();
        fs::write(b.dir().join("notes.txt"), "x").unwrap();
        fs::write(b.dir().join(".hidden.bbg"), "x").unwrap();
        fs::create_dir(b.dir().join("dir.bbg")).unwrap();
        let t = b.create(&["trashed"]).unwrap();
        b.delete(&t).unwrap();
        let l = b.list().unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].title.as_deref(), Some("real"));
    }

    #[test]
    fn a_sheet_saved_under_the_first_extension_is_renamed_when_the_book_opens() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("Budget.babbage"), "rent = $900\n").unwrap();
        fs::write(d.path().join("Both.babbage"), "old\n").unwrap();
        fs::write(d.path().join("Both.bbg"), "new\n").unwrap();
        fs::write(d.path().join(".hidden.babbage"), "x").unwrap();
        let b = SheetBook::open(d.path()).unwrap();
        assert!(!d.path().join("Budget.babbage").exists());
        assert_eq!(
            b.read(&SheetId::new("Budget").unwrap()).unwrap(),
            vec!["rent = $900".to_string()]
        );
        // A pair is left alone: neither file is overwritten.
        assert_eq!(
            fs::read_to_string(d.path().join("Both.babbage")).unwrap(),
            "old\n"
        );
        assert_eq!(
            fs::read_to_string(d.path().join("Both.bbg")).unwrap(),
            "new\n"
        );
        assert!(d.path().join(".hidden.babbage").exists());
        // Opening again changes nothing.
        SheetBook::open(d.path()).unwrap();
        assert!(d.path().join("Budget.bbg").is_file());
    }

    #[test]
    fn a_file_that_is_not_utf8_is_skipped_by_the_listing_and_refused_by_read() {
        let (_d, b) = book();
        fs::write(b.dir().join("bad.bbg"), [0xff, 0xfe, 0x41]).unwrap();
        assert!(b.list().unwrap().is_empty());
        assert!(matches!(
            b.read(&SheetId::new("bad").unwrap()),
            Err(Error::NotUtf8(_))
        ));
    }

    #[test]
    fn opening_the_book_again_sweeps_a_crashed_write_and_keeps_the_sheet() {
        let (d, b) = book();
        let id = b.create(&["old"]).unwrap();
        crate::write_atomic_until(&b.path_of(&id), b"new\n", Some(crate::Stage::BeforeRename))
            .unwrap();
        let leftovers = || {
            fs::read_dir(b.dir())
                .unwrap()
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().starts_with(".tmp-"))
                .count()
        };
        assert_eq!(leftovers(), 1);
        let again = SheetBook::open(d.path().join("sheets")).unwrap();
        assert_eq!(leftovers(), 0);
        assert_eq!(again.read(&id).unwrap(), vec!["old"]);
        assert_eq!(
            again.list().unwrap().len(),
            1,
            "the temporary never showed as a sheet"
        );
    }

    #[test]
    fn reading_a_missing_sheet_is_an_error() {
        let (_d, b) = book();
        assert!(matches!(
            b.read(&SheetId::new("nope").unwrap()),
            Err(Error::NoSuchSheet(_))
        ));
    }

    #[test]
    fn data_dir_follows_the_environment() {
        // One test mutates the process environment, and it restores it, so the others never see it.
        // SAFETY: no other test in this crate reads these variables.
        unsafe {
            std::env::set_var("BABBAGE_DATA_DIR", "/somewhere/else");
        }
        assert_eq!(data_dir(), Some(PathBuf::from("/somewhere/else")));
        unsafe {
            std::env::remove_var("BABBAGE_DATA_DIR");
            std::env::set_var("XDG_DATA_HOME", "/xdg");
        }
        assert_eq!(data_dir(), Some(PathBuf::from("/xdg/babbage/sheets")));
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(48))]
        #[test]
        fn any_lines_survive_save_and_read(lines in proptest::collection::vec("[^\n\r]{0,60}", 0..15)) {
            let (_d, b) = book();
            let id = b.create(&lines).unwrap();
            prop_assert_eq!(b.read(&id).unwrap(), lines.clone());
            b.save(&id, &lines).unwrap();
            prop_assert_eq!(b.read(&id).unwrap(), lines);
        }
    }
}
