//! The settings the engine needs from the person and the machine: their time zone, number format and working day.
//!
//! The zone comes from the system (`TZ`, then `/etc/localtime`) so "now" and bare times mean the user's own
//! clock, daylight saving included. A small `key = value` file can override the rest.

use std::sync::OnceLock;

use babbage_core::Number;
use babbage_core::NumberFormat;
use babbage_core::settings::{Settings, Tax};
use babbage_core::zones::{Zone, ZoneSource};

/// The settings for this run, read once.
pub fn current() -> &'static Settings {
    static SETTINGS: OnceLock<Settings> = OnceLock::new();
    SETTINGS.get_or_init(|| {
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
    })
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
