//! The settings the engine needs from the person and the machine: their time zone, number format and working day.
//!
//! The zone comes from the system (`TZ`, then `/etc/localtime`) so "now" and bare times mean the user's own
//! clock, daylight saving included. A small `key = value` file can override the rest.

use std::sync::RwLock;

use babbage_core::Number;
use babbage_core::NumberFormat;
use babbage_core::settings::{Settings, Tax};
use babbage_core::zones::{Zone, ZoneSource};

static SETTINGS: RwLock<Option<&'static Settings>> = RwLock::new(None);

fn load() -> Settings {
    let file = config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default();
    from_parts(
        &file,
        std::env::var("TZ").ok().as_deref(),
        std::fs::read_link("/etc/localtime").ok().as_deref(),
        std::env::var("BABBAGE_NOW").ok().as_deref(),
        std::env::var("BABBAGE_TZDIR").ok().as_deref(),
    )
}

/// The settings now: read from the file on first use, and again after `set` changes one. Each reading is kept for
/// the life of the process (a settings change is rare, so the few kept copies cost nothing).
pub fn current() -> &'static Settings {
    if let Some(s) = *SETTINGS.read().expect("settings lock") {
        return s;
    }
    let mut slot = SETTINGS.write().expect("settings lock");
    if slot.is_none() {
        *slot = Some(Box::leak(Box::new(load())));
    }
    let settings: Option<&'static Settings> = *slot;
    settings.expect("set just above")
}

/// `text` with the line `key = value` set: the first line for the key is replaced, else one is added.
pub fn with_key(text: &str, key: &str, value: &str) -> String {
    let mut out = Vec::new();
    let mut done = false;
    for line in text.lines() {
        let named = line
            .split('#')
            .next()
            .and_then(|l| l.split_once('='))
            .is_some_and(|(k, _)| k.trim() == key);
        if named && !done {
            out.push(format!("{key} = {value}"));
            done = true;
        } else if !named {
            out.push(line.to_string());
        }
    }
    if !done {
        out.push(format!("{key} = {value}"));
    }
    out.join("\n") + "\n"
}

/// Writes `key = value` into the settings file and applies it at once.
pub fn set(key: &str, value: &str) -> std::io::Result<()> {
    let path = config_path().ok_or_else(|| {
        std::io::Error::other("no settings file: neither XDG_CONFIG_HOME nor HOME is set")
    })?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, with_key(&text, key, value))?;
    std::fs::rename(&tmp, &path)?;
    *SETTINGS.write().expect("settings lock") = Some(Box::leak(Box::new(load())));
    Ok(())
}

/// Where the settings file is (shown in the settings panel).
pub fn settings_path() -> Option<std::path::PathBuf> {
    config_path()
}

fn config_path() -> Option<std::path::PathBuf> {
    if let Some(p) = std::env::var_os("BABBAGE_CONFIG_FILE") {
        return Some(p.into());
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".config")))?;
    Some(base.join("babbage").join("settings"))
}

/// Where the machine keeps its zone rules.
const SYSTEM_ZONEINFO: &str = "/usr/share/zoneinfo";

/// The settings from a config file's text, the `TZ` variable, the `/etc/localtime` link, the test clock and
/// the zoneinfo folder to read (None means the machine's own).
pub fn from_parts(
    file: &str,
    tz_env: Option<&str>,
    localtime_link: Option<&std::path::Path>,
    now_override: Option<&str>,
    tzdir_override: Option<&str>,
) -> Settings {
    let mut settings = Settings {
        zone_source: ZoneSource::System {
            dir: tzdir_override.unwrap_or(SYSTEM_ZONEINFO).into(),
        },
        ..Settings::default()
    };
    let mut zone_from_file: Option<String> = None;
    for line in file.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        match key {
            "number_format" => {
                settings.number_format = match value {
                    "north-america" => NumberFormat::NorthAmerica,
                    "western-europe" => NumberFormat::WesternEurope,
                    "eastern-europe" => NumberFormat::EasternEurope,
                    _ => settings.number_format,
                }
            }
            "work_hours" => {
                if let Ok(h) = value.parse::<u32>()
                    && (1..=24).contains(&h)
                {
                    settings.work_hours_per_day = Some(h);
                }
            }
            "timezone" if value != "system" => zone_from_file = Some(value.to_string()),
            "tax" => settings.tax = parse_tax(value),
            "tz_source" => {
                if value == "bundled" {
                    settings.zone_source = ZoneSource::Bundled;
                }
            }
            "dollar" => {
                settings.dollar = babbage_core::settings::REGION_DOLLARS
                    .iter()
                    .find(|c| c.eq_ignore_ascii_case(value))
                    .copied();
            }
            _ => {}
        }
    }
    // The zone is looked up under the chosen rule source, so it must be resolved inside that scope.
    let source_only = settings.clone();
    settings.local_zone = babbage_core::settings::scoped(&source_only, || {
        zone_from_file
            .map(|name: String| Zone::named(&name))
            .unwrap_or_else(|| system_zone(tz_env, localtime_link))
    });
    settings.now_utc_seconds = now_override.and_then(|n| n.trim().parse().ok());
    settings
}

/// `VAT 15%` or `GST 10`: the name a line can use and its rate. A name that is a number, a unit or has no rate
/// is refused, so a typo cannot make `5` or `km` mean a tax.
fn parse_tax(value: &str) -> Option<Tax> {
    let (name, rate) = value.rsplit_once(char::is_whitespace)?;
    let (name, rate) = (name.trim(), rate.trim().trim_end_matches('%').trim());
    let letters = !name.is_empty() && name.chars().all(char::is_alphabetic);
    let percent = Number::from_decimal_str(rate).ok()?;
    let in_range = !percent.is_negative() && percent <= Number::from_i64(100);
    if !(letters && in_range) {
        return None;
    }
    Some(Tax {
        name: name.to_string(),
        rate: percent.div(&Number::from_i64(100)).ok()?,
    })
}

/// The zone named by `TZ`, or the one `/etc/localtime` points into, or None (UTC).
fn system_zone(tz_env: Option<&str>, link: Option<&std::path::Path>) -> Option<Zone> {
    if let Some(tz) = tz_env {
        let name = tz.trim_start_matches(':');
        if let Some(zone) = Zone::named(name) {
            return Some(zone);
        }
    }
    let target = link?.to_string_lossy().into_owned();
    let (_, name) = target.rsplit_once("zoneinfo/")?;
    Zone::named(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Real zoneinfo files that ship with the engine's tests, so these tests do not depend on the machine.
    fn fixtures() -> String {
        format!(
            "{}/../babbage-core/tests/fixtures/zoneinfo",
            env!("CARGO_MANIFEST_DIR")
        )
    }

    fn parts(file: &str, tz: Option<&str>, link: Option<&Path>, now: Option<&str>) -> Settings {
        from_parts(file, tz, link, now, Some(&fixtures()))
    }

    fn zone(s: &Settings) -> Option<String> {
        s.local_zone.map(Zone::to_spec)
    }

    #[test]
    fn the_tz_variable_wins_over_the_localtime_link() {
        let s = parts(
            "",
            Some("Asia/Tokyo"),
            Some(Path::new("/usr/share/zoneinfo/Europe/Paris")),
            None,
        );
        assert_eq!(zone(&s).as_deref(), Some("tz:Asia/Tokyo"));
    }

    #[test]
    fn the_localtime_link_names_the_zone_when_tz_is_unset() {
        let s = parts(
            "",
            None,
            Some(Path::new("/usr/share/zoneinfo/America/Los_Angeles")),
            None,
        );
        assert_eq!(zone(&s).as_deref(), Some("tz:America/Los_Angeles"));
        let s = parts("", Some(":America/Chicago"), None, None);
        assert_eq!(zone(&s).as_deref(), Some("tz:America/Chicago"));
    }

    #[test]
    fn nothing_known_means_utc_not_a_guess() {
        assert_eq!(parts("", None, None, None).local_zone, None);
        assert_eq!(
            parts(
                "",
                Some("Not/AZone"),
                Some(Path::new("/etc/elsewhere")),
                None
            )
            .local_zone,
            None
        );
    }

    #[test]
    fn zone_rules_come_from_the_machine_unless_the_file_says_bundled() {
        let s = parts("", None, None, None);
        assert_eq!(
            s.zone_source,
            ZoneSource::System {
                dir: fixtures().into()
            }
        );
        let s = parts("tz_source = bundled\n", None, None, None);
        assert_eq!(s.zone_source, ZoneSource::Bundled);
        let s = from_parts("", None, None, None, None);
        assert_eq!(
            s.zone_source,
            ZoneSource::System {
                dir: "/usr/share/zoneinfo".into()
            }
        );
    }

    #[test]
    fn a_zone_the_machine_has_is_read_from_its_files() {
        let s = parts("", Some("America/Vancouver"), None, None);
        assert!(matches!(s.local_zone, Some(Zone::Loaded(_))));
        let s = parts(
            "tz_source = bundled\n",
            Some("America/Vancouver"),
            None,
            None,
        );
        assert!(matches!(s.local_zone, Some(Zone::Named(_))));
    }

    #[test]
    fn the_file_sets_the_format_work_hours_and_zone() {
        let s = parts(
            "number_format = western-europe  # decimal comma\nwork_hours = 7\ntimezone = Pacific/Auckland\nbogus line\n",
            Some("Asia/Tokyo"),
            None,
            None,
        );
        assert_eq!(s.number_format, NumberFormat::WesternEurope);
        assert_eq!(s.work_hours_per_day, Some(7));
        assert_eq!(zone(&s).as_deref(), Some("tz:Pacific/Auckland"));
    }

    #[test]
    fn a_key_is_replaced_in_place_or_added() {
        assert_eq!(with_key("", "work_hours", "7"), "work_hours = 7\n");
        assert_eq!(
            with_key(
                "number_format = north-america # note\nwork_hours = 8\n",
                "work_hours",
                "6"
            ),
            "number_format = north-america # note\nwork_hours = 6\n"
        );
        assert_eq!(
            with_key("a = 1\nwork_hours = 8\nwork_hours = 9\n", "work_hours", "5"),
            "a = 1\nwork_hours = 5\n"
        );
        assert_eq!(
            with_key("# work_hours = 8\n", "work_hours", "4"),
            "# work_hours = 8\nwork_hours = 4\n"
        );
    }

    #[test]
    fn the_file_sets_the_dollar() {
        let s = parts("dollar = aud\n", None, None, None);
        assert_eq!(s.dollar, Some("AUD"));
        let s = parts("dollar = EUR\n", None, None, None);
        assert_eq!(s.dollar, None);
    }

    #[test]
    fn a_tax_is_a_name_and_a_rate() {
        let tax = |text: &str| parts(text, None, None, None).tax.map(|t| (t.name, t.rate));
        let rate = |d: &str| Number::from_decimal_str(d).unwrap();
        assert_eq!(tax("tax = VAT 15%\n"), Some(("VAT".into(), rate("0.15"))));
        assert_eq!(tax("tax = GST 10\n"), Some(("GST".into(), rate("0.1"))));
        assert_eq!(
            tax("tax = Sales 8.25%\n"),
            Some(("Sales".into(), rate("0.0825")))
        );
        assert_eq!(tax("tax = VAT 0%\n"), Some(("VAT".into(), rate("0"))));
        for bad in [
            "tax = VAT\n",
            "tax = 15%\n",
            "tax = VAT fifteen\n",
            "tax = VAT 150%\n",
            "tax = VAT -5%\n",
            "tax = V4T 15%\n",
            "tax =\n",
            "tax = VAT 15% 20%\n",
            "tax = Sales Tax 8%\n",
        ] {
            assert_eq!(tax(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn bad_values_are_ignored_and_the_test_clock_is_read() {
        let s = parts(
            "number_format = mars\nwork_hours = 99\ntimezone = system\n",
            None,
            None,
            Some(" 1718452800 "),
        );
        assert_eq!(s.number_format, NumberFormat::NorthAmerica);
        assert_eq!(s.work_hours_per_day, None);
        assert_eq!(s.now_utc_seconds, Some(1_718_452_800));
    }
}
