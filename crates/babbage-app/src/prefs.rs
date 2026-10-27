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

static FILE_TEXT: RwLock<Option<String>> = RwLock::new(None);

fn read_file() -> String {
    config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default()
}

/// The value the settings file holds for `key`, if it has a line for it.
pub fn raw(key: &str) -> Option<String> {
    if FILE_TEXT.read().expect("settings lock").is_none() {
        *FILE_TEXT.write().expect("settings lock") = Some(read_file());
    }
    let text = FILE_TEXT.read().expect("settings lock");
    text.as_deref()?.lines().find_map(|line| {
        let (k, v) = without_comment(line).split_once('=')?;
        (k.trim() == key).then(|| v.trim().to_string())
    })
}

/// A settings line without its comment. A comment starts at a `#` that begins the line or follows a space and is
/// followed by one (or ends the line), so a color such as `#ff7a90` is a value and not a comment.
pub fn without_comment(line: &str) -> &str {
    if line.trim_start().starts_with('#') {
        return "";
    }
    let bytes = line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        let before = i == 0 || bytes[i - 1].is_ascii_whitespace();
        let after = bytes.get(i + 1).is_none_or(|c| c.is_ascii_whitespace());
        if b == b'#' && before && after {
            return &line[..i];
        }
    }
    line
}

fn load() -> Settings {
    let file = read_file();
    *FILE_TEXT.write().expect("settings lock") = Some(file.clone());
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
        let named = without_comment(line)
            .split_once('=')
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
        let line = without_comment(line).trim();
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
            "decimal_places" => {
                settings.decimal_places = value.parse::<u32>().ok().filter(|p| *p <= 10);
            }
            "notation" => settings.notation = on_off(value),
            "group_thousands" => settings.group_thousands = on_off(value),
            "currency_codes" => settings.currency_codes = on_off(value).unwrap_or(false),
            "money_trim_zeros" => settings.money_trim_zeros = on_off(value).unwrap_or(false),
            "day_first" => settings.day_first = on_off(value).unwrap_or(false),
            "holidays" => settings.holidays = babbage_core::settings::parse_holidays(value),
            "custom_zones" => {
                settings.custom_zones = babbage_core::settings::parse_custom_zones(value)
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

/// `on` or `off` (also `yes`, `no`, `true`, `false`); anything else is not a choice.
fn on_off(value: &str) -> Option<bool> {
    match value {
        "on" | "yes" | "true" => Some(true),
        "off" | "no" | "false" => Some(false),
        _ => None,
    }
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

    #[test]
    fn a_colour_is_a_value_and_a_spaced_hash_a_comment() {
        let w = without_comment;
        assert_eq!(w("color_number = #ff7a90"), "color_number = #ff7a90");
        assert_eq!(w("tax = VAT 15% # the rate"), "tax = VAT 15% ");
        assert_eq!(w("# a note"), "");
        assert_eq!(w("#note"), "");
        assert_eq!(w("work_hours = 8 #"), "work_hours = 8 ");
    }

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
    fn the_file_sets_how_answers_are_written() {
        let s = parts(
            "decimal_places = 4\nnotation = off\ngroup_thousands = off\ncurrency_codes = on\nmoney_trim_zeros = yes\nday_first = on\n",
            None,
            None,
            None,
        );
        assert_eq!(s.decimal_places, Some(4));
        assert_eq!(s.notation, Some(false));
        assert_eq!(s.group_thousands, Some(false));
        assert!(s.currency_codes && s.money_trim_zeros && s.day_first);
        let o = s.format_options();
        assert_eq!(o.decimal_places(), 4);
        assert!(
            !o.abbreviate_large && !o.group_thousands && o.currency_codes && o.money_trim_zeros
        );
        // Nothing set is what the app showed before.
        let d = parts("", None, None, None).format_options();
        assert_eq!(d.decimal_places(), 10);
        assert!(
            d.abbreviate_large && d.group_thousands && !d.currency_codes && !d.money_trim_zeros
        );
        // A bad value is no choice.
        let bad = parts("decimal_places = 11\nnotation = maybe\n", None, None, None);
        assert_eq!((bad.decimal_places, bad.notation), (None, None));
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
