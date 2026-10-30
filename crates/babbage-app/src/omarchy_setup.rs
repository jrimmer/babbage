//! `babbage setup omarchy`: wires Babbage into an Omarchy desktop, or takes it out again.
//!
//! Everything added to Hyprland lives in its own file, `~/.config/hypr/babbage.lua` (a quick-calculation key and a
//! window rule), loaded by one marked line in `hyprland.lua`. The launcher gets a marked block in
//! `~/.config/omarchy/extensions/omarchy-menu.jsonc`. Each file is backed up before it changes, the result is
//! checked with `hyprctl configerrors`, and if the check finds an error Babbage did not already have to live with,
//! every file is put back. A key that is already bound is refused, never taken. `--remove` undoes all of it.
//!
//! The editing is plain text work on strings, tested without a desktop; the commands that ask the desktop
//! (`omarchy`, `hyprctl`) sit behind `Desktop`, so the whole run is tested against a fake home and a fake desktop.

use std::path::{Path, PathBuf};
use std::process::Command;

pub const DEFAULT_KEY: &str = "SUPER + ALT + C";

pub const USAGE: &str = "usage: babbage setup omarchy [--remove] [--key \"SUPER + ALT + C\"]";

/// The first line of the Hyprland file Babbage writes. A file that has lost it is the user's now and is left alone.
const HEADER: &str = "-- Babbage, added by `babbage setup omarchy`.";
const REQUIRE: &str = "require(\"hypr.babbage\")";
const REQUIRE_LINE: &str = "require(\"hypr.babbage\") -- babbage: added by `babbage setup omarchy`";
const BEGIN: &str = "// babbage: begin";
const END: &str = "// babbage: end";

#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    pub remove: bool,
    pub key: String,
    /// How the key and the menu start Babbage: `babbage` when that name is on the PATH, else the full path of this
    /// program (see `program_to_run`).
    pub program: String,
}

/// The arguments after `setup`.
pub fn parse(args: &[String]) -> Result<Args, String> {
    let mut rest = args.iter();
    if rest.next().map(String::as_str) != Some("omarchy") {
        return Err(USAGE.into());
    }
    let mut parsed = Args {
        remove: false,
        key: DEFAULT_KEY.into(),
        program: "babbage".into(),
    };
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--remove" => parsed.remove = true,
            "--key" => {
                parsed.key = rest
                    .next()
                    .cloned()
                    .ok_or_else(|| format!("--key needs a key\n{USAGE}"))?;
            }
            "-h" | "--help" => return Err(USAGE.into()),
            other => return Err(format!("unknown option {other}\n{USAGE}")),
        }
    }
    Ok(parsed)
}

/// A key as Hyprland binds it: modifier bits and the key's name.
#[derive(Debug, PartialEq, Eq)]
pub struct Key {
    mask: u32,
    name: String,
}

const SHIFT: u32 = 1;
const CTRL: u32 = 4;
const ALT: u32 = 8;
const SUPER: u32 = 64;

/// `SUPER + ALT + C` as a mask and a key. The text goes into a Lua string, so only letters, digits, spaces, plus signs
/// and underscores are accepted.
pub fn parse_key(spec: &str) -> Result<Key, String> {
    if spec.is_empty()
        || !spec
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '+' || c == '_')
    {
        return Err(format!(
            "{spec:?} is not a key: write it like \"SUPER + ALT + C\""
        ));
    }
    let parts: Vec<String> = spec
        .split('+')
        .map(|p| p.trim().to_ascii_uppercase())
        .collect();
    let (name, mods) = parts.split_last().ok_or("no key")?;
    if name.is_empty() || mods.iter().any(String::is_empty) || name.contains(' ') {
        return Err(format!(
            "{spec:?} is not a key: write it like \"SUPER + ALT + C\""
        ));
    }
    let mut mask = 0;
    for m in mods {
        mask |= match m.as_str() {
            "SUPER" => SUPER,
            "SHIFT" => SHIFT,
            "CTRL" | "CONTROL" => CTRL,
            "ALT" => ALT,
            other => {
                return Err(format!(
                    "{other:?} is not a modifier: use SUPER, SHIFT, CTRL or ALT"
                ));
            }
        };
    }
    if mask == 0 {
        return Err(format!(
            "{spec:?} has no modifier: a bare key would stop you typing it"
        ));
    }
    Ok(Key {
        mask,
        name: name.clone(),
    })
}

impl Key {
    /// The words of the key in the order Omarchy writes them: modifiers, then the key.
    fn display_words(&self) -> Vec<String> {
        let mut w: Vec<String> = [
            (SUPER, "SUPER"),
            (SHIFT, "SHIFT"),
            (CTRL, "CTRL"),
            (ALT, "ALT"),
        ]
        .iter()
        .filter(|(bit, _)| self.mask & bit != 0)
        .map(|(_, n)| (*n).to_owned())
        .collect();
        w.push(self.name.clone());
        w
    }

    /// The same words sorted, so the two listings can be compared whatever order they spell them in.
    fn words(&self) -> Vec<String> {
        let mut w = self.display_words();
        w.sort();
        w
    }
}

/// What already uses `key`, from `omarchy menu keybindings --print` (`SUPER SHIFT + RETURN → Browser`) and from
/// `hyprctl binds -j`, whichever of them could be asked. Entries that are Babbage's own do not count.
pub fn taken(key: &Key, listing: Option<&str>, binds_json: Option<&str>) -> Option<String> {
    let wanted = key.words();
    for line in listing.unwrap_or("").lines() {
        let Some((left, what)) = line.split_once('→') else {
            continue;
        };
        let mut words: Vec<String> = left
            .split(|c: char| c == '+' || c.is_whitespace())
            .filter(|w| !w.is_empty())
            .map(|w| match w.to_ascii_uppercase().as_str() {
                "CONTROL" => "CTRL".to_owned(),
                other => other.to_owned(),
            })
            .collect();
        words.sort();
        if words == wanted && !what.to_ascii_lowercase().contains("babbage") {
            return Some(format!("{} → {}", left.trim(), what.trim()));
        }
    }
    let binds: serde_json::Value = serde_json::from_str(binds_json.unwrap_or("[]")).ok()?;
    for bind in binds.as_array()? {
        // Caps lock and the numeric-keypad modifiers do not change which key it is.
        let mask = bind["modmask"].as_u64().unwrap_or(0) as u32 & !(2 | 16 | 32);
        let name = bind["key"].as_str().unwrap_or("");
        let what = bind["description"].as_str().unwrap_or("");
        if mask == key.mask
            && name.eq_ignore_ascii_case(&key.name)
            && bind["submap"].as_str().unwrap_or("").is_empty()
            && !what.to_ascii_lowercase().contains("babbage")
        {
            let what = if what.is_empty() {
                bind["dispatcher"].as_str().unwrap_or("a command")
            } else {
                what
            };
            return Some(format!("{} → {what}", key.display_words().join(" + ")));
        }
    }
    None
}

/// The Hyprland file Babbage writes.
pub fn module_text(key: &str, program: &str) -> String {
    format!(
        "{HEADER} Remove with
-- `babbage setup omarchy --remove`, or edit freely.

-- Quick calculation from anywhere: press it again to bring the window back.
o.bind(\"{key}\", \"Babbage quick calculation\", \"{program} --quick\")

-- The quick window floats, centered, and keeps its size.
o.window({{ class = \"^babbage$\", title = \"^Babbage Quick$\" }}, {{ float = true, center = true, size = {{ 760, 480 }} }})
"
    )
}

/// `hyprland.lua` with the line that loads Babbage's file added at the end, unless it is there already.
pub fn with_require(main: &str) -> String {
    if main.contains(REQUIRE) {
        return main.to_owned();
    }
    let mut out = main.to_owned();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(REQUIRE_LINE);
    out.push('\n');
    out
}

/// `hyprland.lua` without the line that loads Babbage's file.
pub fn without_require(main: &str) -> String {
    main.split_inclusive('\n')
        .filter(|l| !l.contains(REQUIRE))
        .collect()
}

/// What the block's first line says when `add_menu` also put a comma after the entry before it, so `remove_menu` takes
/// exactly that comma back out and never one the file had of its own.
const COMMA_NOTE: &str = "with a comma before it";

fn menu_block(program: &str) -> String {
    format!(
        "  // babbage: begin (added by `babbage setup omarchy`)
  \"babbage\": {{\"icon\":\"\u{f00ec}\",\"label\":\"Babbage\",\"description\":\"Notepad calculator: quick calculation, open\"}},
  \"babbage.quick\": {{\"icon\":\"\u{f0415}\",\"label\":\"Quick Calculation\",\"action\":\"{program} --quick\",\"description\":\"A small floating window\"}},
  \"babbage.open\": {{\"icon\":\"\u{f00ec}\",\"label\":\"Open Babbage\",\"action\":\"{program}\"}}
  // babbage: end
"
    )
}

/// The characters of JSONC outside strings and comments, with their byte positions.
fn code_chars(text: &str) -> Vec<(usize, char)> {
    let mut out = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => {
                while let Some((_, s)) = chars.next() {
                    match s {
                        '\\' => {
                            chars.next();
                        }
                        '"' => break,
                        _ => {}
                    }
                }
            }
            '/' if chars.peek().is_some_and(|(_, n)| *n == '/') => {
                while chars.peek().is_some_and(|(_, n)| *n != '\n') {
                    chars.next();
                }
            }
            '/' if chars.peek().is_some_and(|(_, n)| *n == '*') => {
                chars.next();
                let mut last = ' ';
                for (_, s) in chars.by_ref() {
                    if last == '*' && s == '/' {
                        break;
                    }
                    last = s;
                }
            }
            c if c.is_whitespace() => {}
            c => out.push((i, c)),
        }
    }
    out
}

/// The menu file without Babbage's block. When the block was the last entry, the comma that was added before it goes
/// too, so the file is as it was.
pub fn remove_menu(text: &str) -> Result<String, String> {
    let Some(begin) = text.find(BEGIN) else {
        return Ok(text.to_owned());
    };
    let start = text[..begin].rfind('\n').map_or(0, |n| n + 1);
    let end_at = text[begin..].find(END).ok_or_else(|| {
        format!("the menu file has a `{BEGIN}` line with no `{END}` line: remove it by hand")
    })? + begin;
    let end = text[end_at..]
        .find('\n')
        .map_or(text.len(), |n| end_at + n + 1);
    let added_comma =
        text[start..begin + text[begin..].find('\n').unwrap_or(0)].contains(COMMA_NOTE);
    let mut out = format!("{}{}", &text[..start], &text[end..]);
    if added_comma && let Some((at, ',')) = code_chars(&out).into_iter().rfind(|(i, _)| *i < start)
    {
        out.remove(at);
    }
    Ok(out)
}

/// The menu file with Babbage's block added before the closing brace, with a comma after the entry before it when
/// there is one.
pub fn add_menu(text: &str, program: &str) -> Result<String, String> {
    let mut text = remove_menu(text)?;
    let code = code_chars(&text);
    let close = code
        .iter()
        .rposition(|(_, c)| *c == '}')
        .ok_or("the menu file has no closing brace")?;
    let close_at = code[close].0;
    let needs_comma = close > 0 && !matches!(code[close - 1].1, '{' | ',');
    if needs_comma {
        text.insert(code[close - 1].0 + 1, ',');
    }
    let close_at = close_at + usize::from(needs_comma);
    let head = text[..close_at].trim_end();
    let block = if needs_comma {
        menu_block(program).replacen(
            "`babbage setup omarchy`)",
            &format!("`babbage setup omarchy`, {COMMA_NOTE})"),
            1,
        )
    } else {
        menu_block(program)
    };
    Ok(format!("{head}\n{block}}}\n"))
}

/// What the run asks of the desktop. `Real` runs `omarchy` and `hyprctl`; tests use a fake.
pub trait Desktop {
    /// `omarchy menu keybindings --print`.
    fn keybindings(&self) -> Option<String>;
    /// `hyprctl binds -j`.
    fn binds_json(&self) -> Option<String>;
    /// The lines `hyprctl configerrors` reports after a reload, or None when Hyprland cannot be asked.
    fn config_errors(&self) -> Option<Vec<String>>;
}

pub struct Real;

fn output_of(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn in_hyprland() -> bool {
    std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some_and(|v| !v.is_empty())
}

impl Desktop for Real {
    fn keybindings(&self) -> Option<String> {
        output_of("omarchy", &["menu", "keybindings", "--print"])
    }

    fn binds_json(&self) -> Option<String> {
        in_hyprland().then(|| output_of("hyprctl", &["binds", "-j"]))?
    }

    fn config_errors(&self) -> Option<Vec<String>> {
        if !in_hyprland() {
            return None;
        }
        output_of("hyprctl", &["reload"])?;
        let text = output_of("hyprctl", &["configerrors"])?;
        Some(
            text.lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.to_ascii_lowercase().contains("no errors"))
                .map(str::to_owned)
                .collect(),
        )
    }
}

/// The files the run changes, with what each held before (None: it did not exist), so a failed check can put them back.
struct Changes {
    originals: Vec<(PathBuf, Option<String>)>,
    stamp: u64,
}

impl Changes {
    /// Writes `text` to `path` through a temporary file, after keeping a `.bak.<time>` copy of what was there.
    fn write(&mut self, path: &Path, text: &str) -> Result<(), String> {
        let before = std::fs::read_to_string(path).ok();
        if let Some(old) = &before {
            let backup = path.with_file_name(format!(
                "{}.bak.{}",
                path.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
                self.stamp
            ));
            std::fs::write(&backup, old).map_err(|e| format!("{}: {e}", backup.display()))?;
        }
        if before.as_deref() == Some(text) {
            return Ok(());
        }
        write_file(path, text)?;
        self.originals.push((path.to_owned(), before));
        Ok(())
    }

    fn delete(&mut self, path: &Path) -> Result<(), String> {
        let before = std::fs::read_to_string(path).ok();
        if let Some(old) = &before {
            let backup = path.with_file_name(format!(
                "{}.bak.{}",
                path.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
                self.stamp
            ));
            std::fs::write(&backup, old).map_err(|e| format!("{}: {e}", backup.display()))?;
            std::fs::remove_file(path).map_err(|e| format!("{}: {e}", path.display()))?;
            self.originals.push((path.to_owned(), before));
        }
        Ok(())
    }

    fn undo(&self) {
        for (path, before) in self.originals.iter().rev() {
            let _ = match before {
                Some(text) => write_file(path, text),
                None => std::fs::remove_file(path).map_err(|e| e.to_string()),
            };
        }
    }
}

fn write_file(path: &Path, text: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let tmp = path.with_extension("babbage-tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Does the work against `home`; returns what to tell the user, or why it stopped.
pub fn run(
    args: &Args,
    home: &Path,
    desktop: &dyn Desktop,
    stamp: u64,
) -> Result<Vec<String>, String> {
    let hypr = home.join(".config/hypr");
    let main = hypr.join("hyprland.lua");
    let module = hypr.join("babbage.lua");
    let menu = home.join(".config/omarchy/extensions/omarchy-menu.jsonc");
    let main_text = std::fs::read_to_string(&main).map_err(|_| {
        format!(
            "This doesn't look like Omarchy: {} is missing.",
            main.display()
        )
    })?;
    if !args.remove {
        let key = parse_key(&args.key)?;
        if let Some(what) = taken(
            &key,
            desktop.keybindings().as_deref(),
            desktop.binds_json().as_deref(),
        ) {
            return Err(format!(
                "{} is already bound: {what}\nPick another with: babbage setup omarchy --key \"SUPER + ALT + B\"",
                args.key
            ));
        }
    }
    // What Hyprland already complains about is not ours to fix, or to be stopped by.
    let before = desktop.config_errors();
    let mut changes = Changes {
        originals: Vec::new(),
        stamp,
    };
    let mut said = Vec::new();
    let done = (|| -> Result<(), String> {
        if args.remove {
            let ours = std::fs::read_to_string(&module).is_ok_and(|t| t.starts_with(HEADER));
            if ours {
                changes.delete(&module)?;
                said.push(format!("Removed {}", module.display()));
            } else if module.exists() {
                said.push(format!(
                    "Left {} alone: it is no longer the file Babbage wrote",
                    module.display()
                ));
            }
            if main_text.contains(REQUIRE) {
                changes.write(&main, &without_require(&main_text))?;
                said.push(format!("Removed Babbage from {}", main.display()));
            }
            if let Ok(text) = std::fs::read_to_string(&menu)
                && text.contains(BEGIN)
            {
                changes.write(&menu, &remove_menu(&text)?)?;
                said.push("Removed Babbage from the launcher menu".into());
            }
        } else {
            changes.write(&module, &module_text(&args.key, &args.program))?;
            said.push(format!("Wrote {}", module.display()));
            if !main_text.contains(REQUIRE) {
                changes.write(&main, &with_require(&main_text))?;
                said.push(format!("Loaded it from {}", main.display()));
            }
            let menu_text = std::fs::read_to_string(&menu).unwrap_or_else(|_| "{\n}\n".into());
            changes.write(&menu, &add_menu(&menu_text, &args.program)?)?;
            said.push(format!(
                "Added a Babbage section to the Omarchy menu ({})",
                menu.display()
            ));
        }
        Ok(())
    })();
    if let Err(e) = done {
        changes.undo();
        return Err(format!("{e}\nNothing was changed."));
    }
    if let (Some(before), Some(after)) = (before, desktop.config_errors()) {
        let new: Vec<&String> = after.iter().filter(|l| !before.contains(l)).collect();
        if !new.is_empty() {
            changes.undo();
            let _ = desktop.config_errors();
            let lines: Vec<&str> = new.iter().map(|l| l.as_str()).collect();
            return Err(format!(
                "Hyprland reports config errors after the change:\n{}\nEvery file was put back as it was.",
                lines.join("\n")
            ));
        }
    }
    if args.remove {
        said.push(
            "Babbage is no longer wired into Omarchy (the app itself stays installed).".into(),
        );
    } else {
        said.push(String::new());
        said.push("Babbage is wired into Omarchy:".into());
        said.push(format!(
            "  {:<18} quick calculation from anywhere",
            args.key
        ));
        said.push(
            "  Super+Space        Omarchy menu → Babbage → Quick Calculation / Open Babbage".into(),
        );
        said.push("  Undo it all with:  babbage setup omarchy --remove".into());
    }
    Ok(said)
}

/// Whether a program path can sit inside a Lua string, a JSON string and a shell command unquoted.
fn plain_path(path: &str) -> bool {
    !path.is_empty()
        && path
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | '+'))
}

/// What the key and the menu should run: the plain name `babbage` when the PATH finds a program of that name (for
/// the usual install in `~/.local/bin` the key and menu see the same PATH), otherwise the full path of this program:
/// the AppImage file when it is one (its path inside the mount is gone when it quits), else the running executable.
pub fn program_to_run(
    path_var: &str,
    appimage: Option<&str>,
    exe: Option<&str>,
) -> Result<String, String> {
    let on_path = path_var
        .split(':')
        .filter(|d| !d.is_empty())
        .any(|d| std::path::Path::new(d).join("babbage").is_file());
    if on_path {
        return Ok("babbage".into());
    }
    let full = appimage.filter(|a| !a.is_empty()).or(exe).ok_or(
        "cannot tell where this program is: put `babbage` on your PATH and run setup again",
    )?;
    if !plain_path(full) {
        return Err(format!(
            "{full} has characters that cannot go into the key and menu files safely: put `babbage` on your PATH (a link in ~/.local/bin) and run setup again"
        ));
    }
    Ok(full.to_owned())
}

/// `babbage setup ARGS`: the exit status.
pub fn main(args: &[String]) -> i32 {
    let parsed = match parse(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("babbage setup: {e}");
            return 2;
        }
    };
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        eprintln!("babbage setup: HOME is not set");
        return 1;
    };
    let mut parsed = parsed;
    if !parsed.remove {
        let exe = std::env::current_exe().ok();
        match program_to_run(
            &std::env::var("PATH").unwrap_or_default(),
            std::env::var("APPIMAGE").ok().as_deref(),
            exe.as_deref().and_then(|e| e.to_str()),
        ) {
            Ok(p) => parsed.program = p,
            Err(e) => {
                eprintln!("babbage setup: {e}");
                return 1;
            }
        }
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    match run(&parsed, &home, &Real, stamp) {
        Ok(lines) => {
            for l in lines {
                println!("{l}");
            }
            0
        }
        Err(e) => {
            eprintln!("babbage setup: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    const MAIN: &str = "-- Omarchy\ndofile(\"bootstrap.lua\")\nrequire(\"hypr.bindings\")\n";

    /// A desktop that answers from what the test gives it. `errors` are handed out one per question.
    struct Fake {
        keys: Option<String>,
        binds: Option<String>,
        errors: RefCell<Vec<Option<Vec<String>>>>,
    }

    impl Fake {
        fn quiet() -> Fake {
            Fake {
                keys: Some(String::new()),
                binds: Some("[]".into()),
                errors: RefCell::new(vec![Some(vec![]); 8]),
            }
        }
    }

    impl Desktop for Fake {
        fn keybindings(&self) -> Option<String> {
            self.keys.clone()
        }
        fn binds_json(&self) -> Option<String> {
            self.binds.clone()
        }
        fn config_errors(&self) -> Option<Vec<String>> {
            let mut q = self.errors.borrow_mut();
            if q.is_empty() {
                Some(vec![])
            } else {
                q.remove(0)
            }
        }
    }

    /// A home that looks like Omarchy's: a hyprland.lua and, when `menu` is given, a menu file.
    fn omarchy_home(menu: Option<&str>) -> tempfile::TempDir {
        let home = tempfile::tempdir().unwrap();
        let hypr = home.path().join(".config/hypr");
        std::fs::create_dir_all(&hypr).unwrap();
        std::fs::write(hypr.join("hyprland.lua"), MAIN).unwrap();
        if let Some(m) = menu {
            let dir = home.path().join(".config/omarchy/extensions");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("omarchy-menu.jsonc"), m).unwrap();
        }
        home
    }

    fn read(home: &tempfile::TempDir, rel: &str) -> String {
        std::fs::read_to_string(home.path().join(rel)).unwrap()
    }

    fn args(remove: bool, key: &str) -> Args {
        Args {
            remove,
            key: key.into(),
            program: "babbage".into(),
        }
    }

    /// The menu as data: comments out (outside strings), then read as JSON.
    fn menu_value(text: &str) -> serde_json::Value {
        let mut bare = String::new();
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '"' => {
                    bare.push(c);
                    while let Some(s) = chars.next() {
                        bare.push(s);
                        if s == '\\' {
                            bare.extend(chars.next());
                        } else if s == '"' {
                            break;
                        }
                    }
                }
                '/' if chars.peek() == Some(&'/') => {
                    while chars.peek().is_some_and(|n| *n != '\n') {
                        chars.next();
                    }
                }
                '/' if chars.peek() == Some(&'*') => {
                    chars.next();
                    let mut last = ' ';
                    for s in chars.by_ref() {
                        if last == '*' && s == '/' {
                            break;
                        }
                        last = s;
                    }
                }
                _ => bare.push(c),
            }
        }
        serde_json::from_str(&bare).unwrap_or_else(|e| panic!("{e}: {bare}"))
    }

    #[test]
    fn the_arguments_are_setup_omarchy_remove_and_a_key() {
        let a = |ws: &[&str]| parse(&ws.iter().map(|w| w.to_string()).collect::<Vec<_>>());
        assert_eq!(a(&["omarchy"]), Ok(args(false, DEFAULT_KEY)));
        assert_eq!(a(&["omarchy", "--remove"]), Ok(args(true, DEFAULT_KEY)));
        assert_eq!(
            a(&["omarchy", "--key", "SUPER + B"]),
            Ok(args(false, "SUPER + B"))
        );
        assert!(a(&[]).is_err());
        assert!(a(&["gnome"]).is_err());
        assert!(
            a(&["omarchy", "--key"])
                .unwrap_err()
                .contains("needs a key")
        );
        assert!(
            a(&["omarchy", "--nope"])
                .unwrap_err()
                .contains("unknown option --nope")
        );
        assert!(a(&["omarchy", "--help"]).unwrap_err().starts_with("usage:"));
    }

    #[test]
    fn a_key_is_modifiers_and_a_name_and_nothing_that_could_leave_the_lua_string() {
        let k = parse_key("super + alt + c").unwrap();
        assert_eq!((k.mask, k.name.as_str()), (SUPER | ALT, "C"));
        assert_eq!(
            parse_key("CONTROL + SHIFT + F5").unwrap().mask,
            CTRL | SHIFT
        );
        for bad in [
            "",
            "C",
            "SUPER",
            "SUPER +",
            "+ C",
            "SUPER + + C",
            "HYPER + C",
            "SUPER + A B",
            "SUPER + \"); os.execute(\"x\") --",
            "SUPER + C\n",
        ] {
            assert!(parse_key(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_taken_key_is_found_in_either_listing_in_any_modifier_spelling() {
        let key = parse_key("SUPER + SHIFT + RETURN").unwrap();
        let listing = "SUPER + K → Keybindings\nSUPER SHIFT + RETURN → Browser\n";
        assert_eq!(
            taken(&key, Some(listing), None).as_deref(),
            Some("SUPER SHIFT + RETURN → Browser")
        );
        assert!(taken(&parse_key("SUPER + ALT + C").unwrap(), Some(listing), None).is_none());
        // The same key written the other way round and in lower case is the same key.
        let listing = "ALT SUPER + c → Something\n";
        assert!(taken(&parse_key("SUPER + ALT + C").unwrap(), Some(listing), None).is_some());
        assert!(
            taken(
                &parse_key("CTRL + SUPER + C").unwrap(),
                Some("SUPER CONTROL + C → X"),
                None
            )
            .is_some()
        );
        // Babbage's own entry is not a conflict.
        assert!(
            taken(
                &key,
                Some("SUPER SHIFT + RETURN → Babbage quick calculation"),
                None
            )
            .is_none()
        );
        let binds = r#"[{"modmask":72,"key":"c","description":"Mine","submap":""},
            {"modmask":64,"key":"C","description":"Close","submap":""},
            {"modmask":72,"key":"D","description":"","dispatcher":"exec","submap":""},
            {"modmask":72,"key":"E","description":"Other layer","submap":"resize"}]"#;
        let alt_c = parse_key("SUPER + ALT + C").unwrap();
        assert_eq!(
            taken(&alt_c, None, Some(binds)).as_deref(),
            Some("SUPER + ALT + C → Mine")
        );
        assert!(
            taken(&parse_key("SUPER + C").unwrap(), None, Some(binds))
                .unwrap()
                .contains("Close")
        );
        assert!(
            taken(&parse_key("SUPER + ALT + D").unwrap(), None, Some(binds))
                .unwrap()
                .contains("exec")
        );
        assert!(taken(&parse_key("SUPER + ALT + E").unwrap(), None, Some(binds)).is_none());
        // Caps lock (2) and numlock (16) do not change the key.
        let locked = r#"[{"modmask":90,"key":"X","description":"Locked","submap":""}]"#;
        assert!(taken(&parse_key("SUPER + ALT + X").unwrap(), None, Some(locked)).is_some());
        assert!(taken(&alt_c, None, Some("not json")).is_none());
        assert!(taken(&alt_c, None, None).is_none());
    }

    #[test]
    fn the_key_and_menu_run_the_name_on_the_path_or_else_the_full_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("babbage"), "").unwrap();
        let on = format!("/nope:{}", dir.path().display());
        assert_eq!(
            program_to_run(&on, None, Some("/x/y/babbage")).unwrap(),
            "babbage"
        );
        assert_eq!(
            program_to_run("/nope::", None, Some("/home/me/b/target/release/babbage")).unwrap(),
            "/home/me/b/target/release/babbage"
        );
        // An AppImage is run by its own file, not by the path inside its mount.
        assert_eq!(
            program_to_run(
                "/nope",
                Some("/home/me/Apps/babbage.AppImage"),
                Some("/tmp/.mount_x/usr/bin/babbage")
            )
            .unwrap(),
            "/home/me/Apps/babbage.AppImage"
        );
        assert_eq!(
            program_to_run("/nope", Some(""), Some("/a/babbage")).unwrap(),
            "/a/babbage"
        );
        assert!(
            program_to_run("/nope", None, None)
                .unwrap_err()
                .contains("PATH")
        );
        for bad in ["/a b/babbage", "/a\"b/babbage", "/a$b/babbage", "rel path"] {
            assert!(program_to_run("/nope", None, Some(bad)).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_full_path_goes_into_the_key_and_both_menu_actions() {
        let t = module_text("SUPER + ALT + C", "/home/me/b/babbage");
        assert!(t.contains("\"/home/me/b/babbage --quick\")"));
        let menu = add_menu("{\n}\n", "/home/me/b/babbage").unwrap();
        let v = menu_value(&menu);
        assert_eq!(v["babbage.quick"]["action"], "/home/me/b/babbage --quick");
        assert_eq!(v["babbage.open"]["action"], "/home/me/b/babbage");
    }

    #[test]
    fn the_hyprland_file_binds_the_key_floats_the_quick_window_and_starts_with_the_header() {
        let t = module_text("SUPER + ALT + C", "babbage");
        assert!(t.starts_with(HEADER));
        assert!(t.contains(
            "o.bind(\"SUPER + ALT + C\", \"Babbage quick calculation\", \"babbage --quick\")"
        ));
        assert!(t.contains("class = \"^babbage$\", title = \"^Babbage Quick$\""));
        assert!(t.contains("float = true, center = true, size = { 760, 480 }"));
        assert!(
            !t.contains("launch_on_start"),
            "Babbage does not run in the background"
        );
    }

    #[test]
    fn one_marked_line_loads_the_file_and_it_comes_out_again() {
        let once = with_require(MAIN);
        assert_eq!(once.matches(REQUIRE).count(), 1);
        assert!(once.ends_with("added by `babbage setup omarchy`\n"));
        assert_eq!(with_require(&once), once);
        assert_eq!(without_require(&once), MAIN);
        assert_eq!(without_require(MAIN), MAIN);
        assert_eq!(with_require("a").as_str(), format!("a\n{REQUIRE_LINE}\n"));
        assert_eq!(with_require("").as_str(), format!("{REQUIRE_LINE}\n"));
    }

    const MENUS: [&str; 7] = [
        // A neighbour's block that ends in its own trailing comma, as another app's setup leaves it.
        "{\n  // other: begin\n  \"o\": {\"label\":\"O\"},\n  \"o.s\": {\"label\":\"S\"},\n  // other: end\n}\n",
        "{\n}\n",
        "{\n  // comments only\n}\n",
        "{\n  \"a\": {\"label\":\"A\"}\n}\n",
        "{\n  \"a\": {\"label\":\"A\"}, // note\n  \"b\": {\"label\":\"B\", \"action\":\"x https://example.com\"}\n  // end\n}\n",
        "{\n  \"a\": {\"label\":\"A\"},\n}\n",
        "// header\n{\n  /* block } */ \"a\": {\"label\":\"A\"} /* tail */\n}\n",
    ];

    #[test]
    fn the_menu_block_is_added_once_stays_valid_and_leaves_the_rest_alone() {
        for text in MENUS {
            let added = add_menu(text, "babbage").unwrap();
            let v = menu_value(&added);
            assert_eq!(v["babbage.quick"]["action"], "babbage --quick", "{added}");
            assert_eq!(v["babbage.open"]["action"], "babbage");
            assert_eq!(v["babbage"]["label"], "Babbage");
            assert_eq!(added.matches(BEGIN).count(), 1);
            assert_eq!(
                add_menu(&added, "babbage").unwrap(),
                added,
                "adding twice changed the file\n{text}"
            );
            // Entries that were there stay.
            if text.contains("\"a\"") {
                assert_eq!(v["a"]["label"], "A");
            }
            if text.contains("\"b\"") {
                assert_eq!(v["b"]["action"], "x https://example.com");
            }
        }
    }

    #[test]
    fn taking_the_block_out_gives_back_the_file_that_was_there() {
        for text in MENUS {
            let back = remove_menu(&add_menu(text, "babbage").unwrap()).unwrap();
            assert_eq!(back, text, "round trip of {text:?}");
        }
        assert_eq!(remove_menu("{\n}\n").unwrap(), "{\n}\n");
    }

    #[test]
    fn awkward_menu_files_are_read_as_code_not_as_text() {
        let samples = [
            // A brace in a comment after the real one.
            "{\n  \"a\": {\"label\":\"A\"}\n}\n/* } */\n",
            // A star and a slash inside a block comment that holds a brace.
            "{\n  \"a\": {\"label\":\"A\"}\n}\n/* x * y / z } */\n",
            // An escaped quote just before the closing brace.
            "{\n  \"a\": {\"label\":\"say \\\"hi\\\"\"}\n}\n",
            // One escaped quote in the last string: if the escape were not skipped, the next quote would open a string.
            "{\n  \"a\": {\"label\":\"A \\\" b\"}\n}\n",
            // The closing brace right after the last entry.
            "{\"a\":{\"label\":\"A\"}}\n",
        ];
        for text in samples {
            let added = add_menu(text, "babbage").unwrap_or_else(|e| panic!("{e}: {text}"));
            let v = menu_value(&added);
            assert_eq!(v["babbage.quick"]["action"], "babbage --quick", "{added}");
            assert!(
                v["a"]["label"].as_str().unwrap().starts_with('A')
                    || v["a"]["label"].as_str().unwrap().starts_with("say")
            );
            assert_eq!(add_menu(&added, "babbage").unwrap(), added, "{text}");
            assert_eq!(menu_value(&remove_menu(&added).unwrap())["a"], v["a"]);
            assert!(
                menu_value(&remove_menu(&added).unwrap())
                    .get("babbage")
                    .is_none()
            );
        }
        // A block with nothing before it keeps the brace after it, and a lone brace takes the block.
        assert_eq!(
            remove_menu("// babbage: begin\n// babbage: end\n}\n").unwrap(),
            "}\n"
        );
        let lone = add_menu("}\n", "babbage").unwrap();
        assert!(lone.contains("babbage.quick") && lone.ends_with("}\n"));
    }

    #[test]
    fn a_half_marked_or_braceless_menu_is_refused_not_mangled() {
        let half = format!("{{\n  {BEGIN}\n  \"x\": 1\n}}\n");
        assert!(
            remove_menu(&half)
                .unwrap_err()
                .contains("remove it by hand")
        );
        assert!(add_menu(&half, "babbage").is_err());
        assert!(
            add_menu("no braces here", "babbage")
                .unwrap_err()
                .contains("no closing brace")
        );
        assert!(add_menu("", "babbage").is_err());
    }

    #[test]
    fn setup_writes_the_module_the_require_line_and_the_menu_with_backups() {
        let home = omarchy_home(Some("{\n  \"a\": {\"label\":\"A\"}\n}\n"));
        let said = run(
            &args(false, "SUPER + ALT + C"),
            home.path(),
            &Fake::quiet(),
            77,
        )
        .unwrap();
        assert!(
            said.iter()
                .any(|l| l.contains("SUPER + ALT + C") && l.contains("quick calculation"))
        );
        assert!(read(&home, ".config/hypr/babbage.lua").starts_with(HEADER));
        let main = read(&home, ".config/hypr/hyprland.lua");
        assert_eq!(main, with_require(MAIN));
        assert_eq!(read(&home, ".config/hypr/hyprland.lua.bak.77"), MAIN);
        let menu = read(&home, ".config/omarchy/extensions/omarchy-menu.jsonc");
        assert_eq!(
            menu_value(&menu)["babbage.quick"]["label"],
            "Quick Calculation"
        );
        assert!(
            read(
                &home,
                ".config/omarchy/extensions/omarchy-menu.jsonc.bak.77"
            )
            .contains("\"a\"")
        );
    }

    #[test]
    fn running_setup_again_changes_nothing_and_adds_no_second_line() {
        let home = omarchy_home(None);
        run(&args(false, DEFAULT_KEY), home.path(), &Fake::quiet(), 1).unwrap();
        let (main, menu, module) = (
            read(&home, ".config/hypr/hyprland.lua"),
            read(&home, ".config/omarchy/extensions/omarchy-menu.jsonc"),
            read(&home, ".config/hypr/babbage.lua"),
        );
        run(&args(false, DEFAULT_KEY), home.path(), &Fake::quiet(), 2).unwrap();
        assert_eq!(read(&home, ".config/hypr/hyprland.lua"), main);
        assert_eq!(
            read(&home, ".config/omarchy/extensions/omarchy-menu.jsonc"),
            menu
        );
        assert_eq!(read(&home, ".config/hypr/babbage.lua"), module);
        assert_eq!(main.matches(REQUIRE).count(), 1);
    }

    #[test]
    fn a_menu_file_that_is_not_there_is_made() {
        let home = omarchy_home(None);
        run(&args(false, DEFAULT_KEY), home.path(), &Fake::quiet(), 1).unwrap();
        let menu = read(&home, ".config/omarchy/extensions/omarchy-menu.jsonc");
        assert_eq!(menu_value(&menu)["babbage"]["label"], "Babbage");
    }

    #[test]
    fn a_key_that_is_taken_is_refused_and_nothing_is_written() {
        let home = omarchy_home(Some("{\n}\n"));
        let desktop = Fake {
            keys: Some("SUPER ALT + C → Something of yours\n".into()),
            ..Fake::quiet()
        };
        let e = run(&args(false, "SUPER + ALT + C"), home.path(), &desktop, 1).unwrap_err();
        assert!(
            e.contains("already bound") && e.contains("Something of yours") && e.contains("--key"),
            "{e}"
        );
        assert!(!home.path().join(".config/hypr/babbage.lua").exists());
        assert_eq!(read(&home, ".config/hypr/hyprland.lua"), MAIN);
        assert_eq!(
            read(&home, ".config/omarchy/extensions/omarchy-menu.jsonc"),
            "{\n}\n"
        );
        // Another key is fine, and a taken key does not stop the removal.
        assert!(run(&args(false, "SUPER + ALT + B"), home.path(), &desktop, 1).is_ok());
        assert!(run(&args(true, "SUPER + ALT + C"), home.path(), &desktop, 2).is_ok());
    }

    #[test]
    fn something_that_is_not_omarchy_is_refused() {
        let home = tempfile::tempdir().unwrap();
        let e = run(&args(false, DEFAULT_KEY), home.path(), &Fake::quiet(), 1).unwrap_err();
        assert!(
            e.contains("doesn't look like Omarchy") && e.contains("hyprland.lua"),
            "{e}"
        );
        assert!(!home.path().join(".config/hypr/babbage.lua").exists());
    }

    #[test]
    fn an_error_the_change_causes_puts_every_file_back() {
        let home = omarchy_home(Some("{\n  \"a\": 1\n}\n"));
        let desktop = Fake {
            errors: RefCell::new(vec![
                Some(vec![]),
                Some(vec!["babbage.lua:3: bad key".into()]),
            ]),
            ..Fake::quiet()
        };
        let e = run(&args(false, DEFAULT_KEY), home.path(), &desktop, 5).unwrap_err();
        assert!(e.contains("bad key") && e.contains("put back"), "{e}");
        assert!(!home.path().join(".config/hypr/babbage.lua").exists());
        assert_eq!(read(&home, ".config/hypr/hyprland.lua"), MAIN);
        assert_eq!(
            read(&home, ".config/omarchy/extensions/omarchy-menu.jsonc"),
            "{\n  \"a\": 1\n}\n"
        );
    }

    #[test]
    fn an_error_hyprland_already_had_is_not_blamed_on_the_change() {
        let home = omarchy_home(None);
        let old = vec!["tidyshot.lua:9: old problem".to_string()];
        let desktop = Fake {
            errors: RefCell::new(vec![Some(old.clone()), Some(old)]),
            ..Fake::quiet()
        };
        assert!(run(&args(false, DEFAULT_KEY), home.path(), &desktop, 1).is_ok());
        assert!(home.path().join(".config/hypr/babbage.lua").exists());
        // Without Hyprland running there is nothing to check, and that is not a failure.
        let silent = Fake {
            errors: RefCell::new(vec![None, None]),
            ..Fake::quiet()
        };
        assert!(run(&args(false, "SUPER + ALT + B"), home.path(), &silent, 2).is_ok());
    }

    #[test]
    fn remove_undoes_it_all_and_leaves_what_was_there() {
        let menu = "{\n  \"a\": {\"label\":\"A\"}\n}\n";
        let home = omarchy_home(Some(menu));
        run(&args(false, DEFAULT_KEY), home.path(), &Fake::quiet(), 1).unwrap();
        let said = run(&args(true, DEFAULT_KEY), home.path(), &Fake::quiet(), 2).unwrap();
        assert!(said.iter().any(|l| l.contains("no longer wired")));
        assert!(!home.path().join(".config/hypr/babbage.lua").exists());
        assert_eq!(read(&home, ".config/hypr/hyprland.lua"), MAIN);
        assert_eq!(
            read(&home, ".config/omarchy/extensions/omarchy-menu.jsonc"),
            menu
        );
        // Removing again, with nothing to remove, is fine and changes nothing.
        run(&args(true, DEFAULT_KEY), home.path(), &Fake::quiet(), 3).unwrap();
        assert_eq!(read(&home, ".config/hypr/hyprland.lua"), MAIN);
    }

    #[test]
    fn remove_leaves_a_file_that_is_no_longer_the_one_babbage_wrote() {
        let home = omarchy_home(None);
        run(&args(false, DEFAULT_KEY), home.path(), &Fake::quiet(), 1).unwrap();
        let module = home.path().join(".config/hypr/babbage.lua");
        std::fs::write(
            &module,
            "-- mine now\no.bind(\"SUPER + B\", \"x\", \"y\")\n",
        )
        .unwrap();
        let said = run(&args(true, DEFAULT_KEY), home.path(), &Fake::quiet(), 2).unwrap();
        assert!(
            said.iter()
                .any(|l| l.starts_with("Left ") && l.contains("babbage.lua"))
        );
        assert!(read(&home, ".config/hypr/babbage.lua").starts_with("-- mine now"));
        assert_eq!(read(&home, ".config/hypr/hyprland.lua"), MAIN);
    }

    #[test]
    fn an_edited_module_that_keeps_its_header_is_removed() {
        let home = omarchy_home(None);
        run(&args(false, DEFAULT_KEY), home.path(), &Fake::quiet(), 1).unwrap();
        let module = home.path().join(".config/hypr/babbage.lua");
        let edited = format!(
            "{}\n-- my own line\n",
            read(&home, ".config/hypr/babbage.lua")
        );
        std::fs::write(&module, edited).unwrap();
        run(&args(true, DEFAULT_KEY), home.path(), &Fake::quiet(), 2).unwrap();
        assert!(!module.exists());
        assert!(read(&home, ".config/hypr/babbage.lua.bak.2").contains("my own line"));
    }
}
