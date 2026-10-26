//! The settings the engine needs from the person and the machine: their time zone, number format and working day.
//!
//! The zone comes from the system (`TZ`, then `/etc/localtime`) so "now" and bare times mean the user's own
//! clock, daylight saving included. A small `key = value` file can override the rest.

use std::sync::OnceLock;

use babbage_core::NumberFormat;
use babbage_core::settings::{Settings, VolumeSystem};
use babbage_core::zones::Zone;

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

/// The settings from a config file's text, the `TZ` variable, the `/etc/localtime` link and the test clock.
pub fn from_parts(
    file: &str,
    tz_env: Option<&str>,
    localtime_link: Option<&std::path::Path>,
    now_override: Option<&str>,
) -> Settings {
    let mut settings = Settings::default();
    let mut zone_from_file = None;
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
            "timezone" if value != "system" => zone_from_file = Zone::named(value),
            "dollar" => {
                settings.dollar = babbage_core::settings::REGION_DOLLARS
                    .iter()
                    .find(|c| c.eq_ignore_ascii_case(value))
                    .copied();
            }
            "volume_system" => {
                settings.volume_system = match value {
                    "imperial" => VolumeSystem::Imperial,
                    "us-customary" => VolumeSystem::UsCustomary,
                    _ => settings.volume_system,
                }
            }
            _ => {}
        }
    }
    settings.local_zone = zone_from_file.or_else(|| system_zone(tz_env, localtime_link));
    settings.now_utc_seconds = now_override.and_then(|n| n.trim().parse().ok());
    settings
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
    fn the_tz_variable_wins_over_the_localtime_link() {
        let s = from_parts(
            "",
            Some("Asia/Tokyo"),
            Some(Path::new("/usr/share/zoneinfo/Europe/Paris")),
            None,
        );
        assert_eq!(s.local_zone, Zone::named("Asia/Tokyo"));
    }

    #[test]
    fn the_localtime_link_names_the_zone_when_tz_is_unset() {
        let s = from_parts(
            "",
            None,
            Some(Path::new("/usr/share/zoneinfo/America/Los_Angeles")),
            None,
        );
        assert_eq!(s.local_zone, Zone::named("America/Los_Angeles"));
        let s = from_parts("", Some(":America/Chicago"), None, None);
        assert_eq!(s.local_zone, Zone::named("America/Chicago"));
    }

    #[test]
    fn nothing_known_means_utc_not_a_guess() {
        assert_eq!(from_parts("", None, None, None).local_zone, None);
        assert_eq!(
            from_parts(
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
    fn the_file_sets_the_format_work_hours_and_zone() {
        let s = from_parts(
            "number_format = western-europe  # decimal comma\nwork_hours = 7\ntimezone = Pacific/Auckland\nbogus line\n",
            Some("Asia/Tokyo"),
            None,
            None,
        );
        assert_eq!(s.number_format, NumberFormat::WesternEurope);
        assert_eq!(s.work_hours_per_day, Some(7));
        assert_eq!(s.local_zone, Zone::named("Pacific/Auckland"));
    }

    #[test]
    fn the_file_sets_the_dollar_and_the_volume_system() {
        let s = from_parts("dollar = aud\nvolume_system = imperial\n", None, None, None);
        assert_eq!(s.dollar, Some("AUD"));
        assert_eq!(s.volume_system, VolumeSystem::Imperial);
        let s = from_parts("dollar = EUR\nvolume_system = metric\n", None, None, None);
        assert_eq!(s.dollar, None);
        assert_eq!(s.volume_system, VolumeSystem::UsCustomary);
    }

    #[test]
    fn bad_values_are_ignored_and_the_test_clock_is_read() {
        let s = from_parts(
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
