//! Atomic file replacement: write a temporary file beside the target, fsync it, rename it over the
//! target, fsync the folder. A crash at any point leaves either the old file or the new one whole.

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{IoContext, Result};

/// Where `write_atomic_until` stops, to stand in for a process that died there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// The temporary file exists and is partly written.
    HalfWritten,
    /// The temporary file is complete and synced, the rename has not happened.
    BeforeRename,
    /// The rename happened, the folder is not synced yet.
    AfterRename,
}

/// The temporary files start with this, so a listing can skip them and `sweep` can remove them.
pub(crate) const TEMP_PREFIX: &str = ".tmp-";

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_path(path: &Path) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let name = path
        .file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    path.with_file_name(format!("{TEMP_PREFIX}{}-{n}-{name}", std::process::id()))
}

/// Replaces `path` with `bytes` atomically.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    write_atomic_until(path, bytes, None)
}

/// As `write_atomic`, but stops dead at `stage` (returning `Ok` and leaving the temporary file where a
/// killed process would), so tests can look at what a crash leaves behind.
pub fn write_atomic_until(path: &Path, bytes: &[u8], stop: Option<Stage>) -> Result<()> {
    let tmp = temp_path(path);
    let what = |verb: &str, p: &Path| format!("{verb} {}", p.display());
    let mut f = File::create(&tmp).ctx(|| what("create", &tmp))?;
    if stop == Some(Stage::HalfWritten) {
        f.write_all(&bytes[..bytes.len() / 2])
            .ctx(|| what("write", &tmp))?;
        return Ok(());
    }
    f.write_all(bytes).ctx(|| what("write", &tmp))?;
    f.sync_all().ctx(|| what("sync", &tmp))?;
    drop(f);
    if stop == Some(Stage::BeforeRename) {
        return Ok(());
    }
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(e).ctx(|| what("rename onto", path));
    }
    if stop == Some(Stage::AfterRename) {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        // Opening a folder to sync it works on Linux; a failure here cannot lose the data already renamed.
        if let Ok(d) = File::open(dir) {
            let _ = d.sync_all();
        }
    }
    Ok(())
}

/// Removes temporary files an earlier crash left in `dir`. Returns how many it removed.
pub(crate) fn sweep(dir: &Path) -> usize {
    let Ok(rd) = fs::read_dir(dir) else { return 0 };
    rd.flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(TEMP_PREFIX))
        .filter(|e| fs::remove_file(e.path()).is_ok())
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(p: &Path) -> String {
        fs::read_to_string(p).unwrap()
    }

    #[test]
    fn a_write_replaces_the_file_and_leaves_no_temporary() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.babbage");
        write_atomic(&p, b"one\n").unwrap();
        write_atomic(&p, b"two\n").unwrap();
        assert_eq!(read(&p), "two\n");
        let names: Vec<_> = fs::read_dir(d.path()).unwrap().flatten().collect();
        assert_eq!(names.len(), 1);
    }

    #[test]
    fn a_crash_before_the_rename_keeps_the_old_file() {
        for stage in [Stage::HalfWritten, Stage::BeforeRename] {
            let d = tempfile::tempdir().unwrap();
            let p = d.path().join("a.babbage");
            write_atomic(&p, b"old\n").unwrap();
            write_atomic_until(&p, b"new and longer\n", Some(stage)).unwrap();
            assert_eq!(read(&p), "old\n", "{stage:?}");
            assert_eq!(sweep(d.path()), 1, "the leftover temporary is swept");
            assert_eq!(read(&p), "old\n");
        }
    }

    #[test]
    fn a_crash_after_the_rename_has_the_new_file() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.babbage");
        write_atomic(&p, b"old\n").unwrap();
        write_atomic_until(&p, b"new\n", Some(Stage::AfterRename)).unwrap();
        assert_eq!(read(&p), "new\n");
    }

    #[test]
    fn a_process_killed_between_write_and_rename_leaves_the_old_file() {
        // The child re-runs this test binary, writes up to the rename and aborts, as a SIGKILL would.
        if let Ok(path) = std::env::var("BABBAGE_STORE_CRASH_CHILD") {
            let _ = write_atomic_until(Path::new(&path), b"new\n", Some(Stage::BeforeRename));
            std::process::abort();
        }
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.babbage");
        write_atomic(&p, b"old\n").unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "atomic::tests::a_process_killed_between_write_and_rename_leaves_the_old_file",
                "--nocapture",
            ])
            .env("BABBAGE_STORE_CRASH_CHILD", &p)
            .stderr(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(!status.success(), "the child aborted");
        assert_eq!(read(&p), "old\n");
    }

    #[test]
    fn writing_into_a_missing_folder_is_an_error() {
        let d = tempfile::tempdir().unwrap();
        assert!(write_atomic(&d.path().join("no/such/a.babbage"), b"x").is_err());
    }
}
