//! Update checks and installs, through praeco. Babbage is an offline calculator, so the check is infrequent
//! (weekly unless the settings say otherwise), can be turned off, and says what it does: it asks the release
//! source for the list of releases (one request, nothing about the sheets or the machine but the address a
//! request always carries) and keeps the answer so a pill can stay up between checks.
//!
//! This module holds the parts that need no window: the settings, the rule for when a check is due, the kept
//! answer, and the notes of a release as plain lines.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use praeco::{CheckOptions, CheckResult, Event, InstallOptions, Source};
use serde_json::{Value, json};

use crate::prefs::raw;

/// Where releases are looked up unless the settings name another place.
pub const DEFAULT_SOURCE: &str = "github jrimmer/babbage";
/// The running version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// How often a check is made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interval {
    Off,
    Daily,
    Weekly,
    Monthly,
}

impl Interval {
    pub const DEFAULT: Interval = Interval::Weekly;

    pub fn parse(text: &str) -> Option<Interval> {
        match text.trim().to_ascii_lowercase().as_str() {
            "off" | "never" | "no" => Some(Interval::Off),
            "daily" => Some(Interval::Daily),
            "weekly" => Some(Interval::Weekly),
            "monthly" => Some(Interval::Monthly),
            _ => None,
        }
    }

    pub fn every(self) -> Option<Duration> {
        let day = 24 * 60 * 60;
        match self {
            Interval::Off => None,
            Interval::Daily => Some(Duration::from_secs(day)),
            Interval::Weekly => Some(Duration::from_secs(7 * day)),
            Interval::Monthly => Some(Duration::from_secs(30 * day)),
        }
    }
}

/// How often to check: the `update_check` setting, and never when `BABBAGE_UPDATES=off` is in the environment
/// (for an install that must not go online whatever the settings say).
pub fn interval() -> Interval {
    if std::env::var("BABBAGE_UPDATES").is_ok_and(|v| v.eq_ignore_ascii_case("off")) {
        return Interval::Off;
    }
    raw("update_check")
        .and_then(|v| Interval::parse(&v))
        .unwrap_or(Interval::DEFAULT)
}

/// The release source as written in the settings: `github owner/repo` or `forgejo https://host owner/repo`.
pub fn source_text() -> String {
    raw("update_source")
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_SOURCE.to_string())
}

/// The newest version the user chose to skip.
pub fn skipped() -> Option<String> {
    raw("update_skip").filter(|v| !v.trim().is_empty())
}

pub fn parse_source(text: &str) -> Result<Source, String> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    let made = match parts.as_slice() {
        [slug] | ["github", slug] => Source::github(slug),
        ["forgejo", base, slug] => Source::forgejo(base, slug),
        _ => {
            return Err(
                "Write the source as `github owner/repo` or `forgejo https://host owner/repo`"
                    .to_string(),
            );
        }
    };
    made.map_err(|e| e.message)
}

/// One release newer than the running version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub date: String,
    pub notes: String,
}

/// What a check found, as it is kept between launches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    /// When the check was made, in seconds since 1970.
    pub checked: u64,
    /// The version that was running then.
    pub running: String,
    /// The source text that was asked.
    pub source: String,
    /// Newer releases, newest first; empty when up to date.
    pub releases: Vec<Release>,
}

impl Found {
    pub fn from_result(result: &CheckResult, source: &str, now: u64) -> Found {
        Found {
            checked: now,
            running: VERSION.to_string(),
            source: source.to_string(),
            releases: result
                .releases
                .iter()
                .map(|r| Release {
                    version: r.version.clone(),
                    date: r
                        .published_at
                        .as_deref()
                        .map(|d| d.chars().take(10).collect())
                        .unwrap_or_default(),
                    notes: r.notes.clone(),
                })
                .collect(),
        }
    }

    /// The newest release, if any is newer than the running version.
    pub fn latest(&self) -> Option<&Release> {
        self.releases.first()
    }

    pub fn to_json(&self) -> Value {
        json!({
            "checked": self.checked,
            "running": self.running,
            "source": self.source,
            "releases": self.releases.iter().map(|r| json!({
                "version": r.version, "date": r.date, "notes": r.notes,
            })).collect::<Vec<_>>(),
        })
    }

    pub fn from_json(v: &Value) -> Option<Found> {
        let text = |x: &Value| x.as_str().map(str::to_string);
        Some(Found {
            checked: v["checked"].as_u64()?,
            running: text(&v["running"])?,
            source: text(&v["source"])?,
            releases: v["releases"]
                .as_array()?
                .iter()
                .map(|r| {
                    Some(Release {
                        version: text(&r["version"])?,
                        date: text(&r["date"]).unwrap_or_default(),
                        notes: text(&r["notes"]).unwrap_or_default(),
                    })
                })
                .collect::<Option<Vec<_>>>()?,
        })
    }
}

/// Where the last answer is kept: `$XDG_STATE_HOME/babbage/updates.json`, else under `~/.local/state`.
pub fn state_file() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .filter(|v| !v.is_empty())
                .map(|h| PathBuf::from(h).join(".local/state"))
        })?;
    Some(base.join("babbage/updates.json"))
}

pub fn load() -> Option<Found> {
    let text = std::fs::read_to_string(state_file()?).ok()?;
    Found::from_json(&serde_json::from_str(&text).ok()?)
}

pub fn save(found: &Found) -> std::io::Result<()> {
    let path = state_file().ok_or_else(|| std::io::Error::other("no state folder"))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, found.to_json().to_string())?;
    std::fs::rename(tmp, path)
}

/// Is a check due? When there is no answer yet, when it came from another source or another version, or when it
/// is older than the interval.
pub fn due(found: Option<&Found>, source: &str, now: u64, every: Duration) -> bool {
    match found {
        None => true,
        Some(f) => {
            f.source != source
                || f.running != VERSION
                || f.checked.saturating_add(every.as_secs()) <= now
        }
    }
}

/// The release to offer: the newest one, unless it is not newer than the running version (the app was updated
/// since) or the user skipped it.
pub fn pending<'a>(found: Option<&'a Found>, skipped: Option<&str>) -> Option<&'a Release> {
    let found = found.filter(|f| f.running == VERSION)?;
    let latest = found.latest()?;
    let newer = |a: &str, b: &str| match (praeco::version::parse(a), praeco::version::parse(b)) {
        (Some(a), Some(b)) => a > b,
        _ => a != b,
    };
    if !newer(&latest.version, VERSION) {
        return None;
    }
    if skipped.is_some_and(|s| !newer(&latest.version, s)) {
        return None;
    }
    Some(latest)
}

/// Seconds since 1970.
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Asks the release source what is new. Blocks: run it off the window's thread.
pub fn run_check(source_text: &str) -> Result<Found, String> {
    let source = parse_source(source_text)?;
    let result = praeco::check(&CheckOptions::new(source, VERSION)).map_err(|e| e.message)?;
    Ok(Found::from_result(&result, source_text, now()))
}

/// Downloads, verifies and installs `version`, reporting what happens. Blocks: run it off the window's thread.
pub fn install(
    source_text: &str,
    version: &str,
    emit: &mut dyn FnMut(&Event),
) -> Result<(), String> {
    let source = parse_source(source_text)?;
    let exe = std::env::current_exe()
        .map_err(|e| format!("Cannot tell which program is running: {e}"))?;
    praeco::install(&InstallOptions::new(source, version, exe), emit)
        .map(|_| ())
        .map_err(|e| e.message)
}

/// The command that starts the new version: the AppImage when this is one, else the running program, with the
/// arguments it was started with.
pub fn restart_command() -> Vec<OsString> {
    let program = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .or_else(|| std::env::current_exe().ok())
        .unwrap_or_else(|| PathBuf::from("babbage"));
    let mut argv = vec![program.into_os_string()];
    argv.extend(std::env::args_os().skip(1));
    argv
}

/// Starts the new version on its own, so it outlives this process.
pub fn launch(argv: &[OsString]) -> Result<(), String> {
    praeco::restart::launch_detached(argv).map_err(|e| e.message)
}

/// One line of a release's notes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Note {
    Heading(String),
    Bullet(String),
    Text(String),
}

/// A release's notes (Markdown) as plain lines: headings, bullets and text, with the emphasis marks taken out.
pub fn note_lines(markdown: &str) -> Vec<Note> {
    let plain = |s: &str| s.replace("**", "").replace("__", "").replace('`', "");
    let mut out = Vec::new();
    for line in markdown.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('#') {
            out.push(Note::Heading(plain(t.trim_start_matches('#').trim())));
        } else if let Some(rest) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
            out.push(Note::Bullet(plain(rest)));
        } else {
            out.push(Note::Text(plain(t)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(v: &str) -> Release {
        Release {
            version: v.to_string(),
            date: "2026-10-26".to_string(),
            notes: "- one".to_string(),
        }
    }

    fn found(running: &str, versions: &[&str]) -> Found {
        Found {
            checked: 1_000,
            running: running.to_string(),
            source: DEFAULT_SOURCE.to_string(),
            releases: versions.iter().map(|v| release(v)).collect(),
        }
    }

    #[test]
    fn the_interval_defaults_to_weekly_and_reads_every_word() {
        assert_eq!(Interval::DEFAULT, Interval::Weekly);
        assert_eq!(Interval::parse("Daily"), Some(Interval::Daily));
        assert_eq!(Interval::parse(" monthly "), Some(Interval::Monthly));
        assert_eq!(Interval::parse("off"), Some(Interval::Off));
        assert_eq!(Interval::parse("hourly"), None);
        assert_eq!(Interval::Off.every(), None);
        assert_eq!(
            Interval::Weekly.every(),
            Some(Duration::from_secs(7 * 86_400))
        );
    }

    #[test]
    fn a_source_is_github_by_default_or_named() {
        assert!(parse_source("jrimmer/babbage").is_ok());
        assert!(parse_source("github jrimmer/babbage").is_ok());
        assert!(parse_source("forgejo https://git.example.com jrimmer/babbage").is_ok());
        assert!(parse_source("forgejo jrimmer/babbage").is_err());
        assert!(parse_source("github not-a-repo").is_err());
        assert!(parse_source("").is_err());
        assert!(parse_source(DEFAULT_SOURCE).is_ok());
    }

    #[test]
    fn a_check_is_due_when_there_is_none_or_it_is_old_or_it_asked_somewhere_else() {
        let week = Duration::from_secs(7 * 86_400);
        let f = found(VERSION, &[]);
        assert!(due(None, DEFAULT_SOURCE, 1_000, week));
        assert!(!due(Some(&f), DEFAULT_SOURCE, 1_000 + 86_400, week));
        assert!(due(Some(&f), DEFAULT_SOURCE, 1_000 + 7 * 86_400, week));
        assert!(due(Some(&f), "github other/repo", 1_001, week));
        assert!(due(Some(&found("0.0.0", &[])), DEFAULT_SOURCE, 1_001, week));
    }

    #[test]
    fn a_release_is_offered_until_it_is_skipped_or_installed() {
        let f = found(VERSION, &["99.1.0", "99.0.0"]);
        assert_eq!(
            pending(Some(&f), None).map(|r| r.version.as_str()),
            Some("99.1.0")
        );
        // Skipping a version hides it and anything not newer; a newer one shows again.
        assert!(pending(Some(&f), Some("99.1.0")).is_none());
        assert!(pending(Some(&f), Some("99.5.0")).is_none());
        assert!(pending(Some(&f), Some("99.0.0")).is_some());
        // Up to date, nothing kept, or kept by another version of the app: nothing is offered.
        assert!(pending(Some(&found(VERSION, &[])), None).is_none());
        assert!(pending(None, None).is_none());
        assert!(pending(Some(&found("0.0.0", &["99.1.0"])), None).is_none());
        // A "newer" release that is not newer than the running version is not offered.
        assert!(pending(Some(&found(VERSION, &["0.0.1"])), None).is_none());
    }

    #[test]
    fn the_kept_answer_round_trips() {
        let f = found(VERSION, &["99.1.0", "99.0.0"]);
        assert_eq!(Found::from_json(&f.to_json()), Some(f));
        assert_eq!(Found::from_json(&json!({"checked": 1})), None);
    }

    #[test]
    fn notes_become_plain_lines() {
        assert_eq!(
            note_lines("## What's new\n\n- **Faster** totals\n* `x` works\nA line.\n"),
            [
                Note::Heading("What's new".into()),
                Note::Bullet("Faster totals".into()),
                Note::Bullet("x works".into()),
                Note::Text("A line.".into()),
            ]
        );
        assert!(note_lines("").is_empty());
    }
}
