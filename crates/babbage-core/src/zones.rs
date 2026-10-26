//! Time zones: which zone a city, country or abbreviation means, and how a wall-clock time maps to an instant.
//!
//! Rules come from the IANA database bundled with `chrono-tz`, so results do not depend on the machine, or, when
//! the settings ask for it, from the machine's own zoneinfo files (kept current by the operating system), with
//! the bundled copy as the fallback for any zone the machine does not have.
//! Seconds are whole numbers throughout; no floating point is used.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use chrono::{DateTime, Offset, TimeZone};
use chrono_tz::{TZ_VARIANTS, Tz};

/// The IANA release the bundled database carries.
pub const BUNDLED_RELEASE: &str = "2025b";

/// Where zone rules come from.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ZoneSource {
    /// The database inside the program: the same answers on every machine.
    #[default]
    Bundled,
    /// The zoneinfo files in this folder (usually `/usr/share/zoneinfo`), falling back to the bundled
    /// database for a zone the folder does not have.
    System { dir: PathBuf },
}

/// A zone read from a TZif file.
#[derive(Debug)]
pub struct Loaded {
    name: String,
    rules: tz::TimeZone,
}

impl PartialEq for Loaded {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Loaded {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    /// A place with daylight saving rules, from the bundled database.
    Named(Tz),
    /// A place with daylight saving rules, from the machine's zoneinfo files.
    Loaded(&'static Loaded),
    /// A fixed offset in seconds east of UTC: `GMT+8`, `PST`.
    Fixed(i64),
}

impl Zone {
    /// Seconds east of UTC at this instant.
    pub fn offset_at(self, utc: i64) -> i64 {
        match self {
            Zone::Fixed(seconds) => seconds,
            Zone::Loaded(loaded) => loaded
                .rules
                .find_local_time_type(utc)
                .map_or(0, |t| i64::from(t.ut_offset())),
            Zone::Named(tz) => DateTime::from_timestamp(utc, 0).map_or(0, |instant| {
                i64::from(
                    tz.offset_from_utc_datetime(&instant.naive_utc())
                        .fix()
                        .local_minus_utc(),
                )
            }),
        }
    }

    /// The instant a wall-clock reading falls on, in seconds since 1970-01-01 UTC. When the clocks repeat that
    /// reading the first occurrence is used; when they skip it, the offset from before the change is used.
    pub fn local_to_utc(self, local: i64) -> i64 {
        let before = self.offset_at(local - 86_400);
        let after = self.offset_at(local + 86_400);
        let fits = |utc: i64| utc + self.offset_at(utc) == local;
        let (first, second) = (local - before, local - after);
        match (fits(first), fits(second)) {
            (true, true) => first.min(second),
            (true, false) => first,
            (false, true) => second,
            (false, false) => first,
        }
    }

    /// The zone with this IANA name, such as `America/New_York`, from the source the settings choose.
    pub fn named(name: &str) -> Option<Zone> {
        if let ZoneSource::System { dir } = crate::settings::zone_source()
            && let Some(loaded) = load(&dir, name)
        {
            return Some(Zone::Loaded(loaded));
        }
        name.parse::<Tz>().ok().map(Zone::Named)
    }

    /// A text form that can travel inside a value: `local`, `fixed:-28800` or `tz:Asia/Tokyo`.
    pub fn to_spec(self) -> String {
        match self {
            Zone::Fixed(seconds) => format!("fixed:{seconds}"),
            Zone::Named(tz) => format!("tz:{}", tz.name()),
            Zone::Loaded(loaded) => format!("tz:{}", loaded.name),
        }
    }

    pub fn from_spec(spec: &str) -> Option<Zone> {
        if let Some(seconds) = spec.strip_prefix("fixed:") {
            return seconds.parse().ok().map(Zone::Fixed);
        }
        Zone::named(spec.strip_prefix("tz:")?)
    }
}

/// US and a few widely used abbreviations. Each is a fixed offset: `PST` is eight hours behind UTC all year.
const ABBREVIATIONS: &[(&str, i64)] = &[
    ("utc", 0),
    ("gmt", 0),
    ("ut", 0),
    ("z", 0),
    ("est", -5 * 3600),
    ("edt", -4 * 3600),
    ("cst", -6 * 3600),
    ("cdt", -5 * 3600),
    ("mst", -7 * 3600),
    ("mdt", -6 * 3600),
    ("pst", -8 * 3600),
    ("pdt", -7 * 3600),
    ("akst", -9 * 3600),
    ("akdt", -8 * 3600),
    ("hst", -10 * 3600),
    ("wet", 0),
    ("west", 3600),
    ("cet", 3600),
    ("cest", 2 * 3600),
    ("eet", 2 * 3600),
    ("eest", 3 * 3600),
    ("msk", 3 * 3600),
    ("jst", 9 * 3600),
    ("kst", 9 * 3600),
    ("awst", 8 * 3600),
    ("aest", 10 * 3600),
    ("aedt", 11 * 3600),
    ("nzst", 12 * 3600),
    ("nzdt", 13 * 3600),
];

/// Zones named by region, with daylight saving.
const REGIONS: &[(&str, &str)] = &[
    ("eastern time", "America/New_York"),
    ("central time", "America/Chicago"),
    ("mountain time", "America/Denver"),
    ("pacific time", "America/Los_Angeles"),
    ("alaska time", "America/Anchorage"),
    ("hawaii time", "Pacific/Honolulu"),
    ("atlantic time", "America/Halifax"),
];

/// Cities and short names that are not the last part of an IANA name.
const ALIASES: &[(&str, &str)] = &[
    ("nyc", "America/New_York"),
    ("new york city", "America/New_York"),
    ("washington", "America/New_York"),
    ("washington dc", "America/New_York"),
    ("boston", "America/New_York"),
    ("miami", "America/New_York"),
    ("atlanta", "America/New_York"),
    ("philadelphia", "America/New_York"),
    ("orlando", "America/New_York"),
    ("san francisco", "America/Los_Angeles"),
    ("sf", "America/Los_Angeles"),
    ("san diego", "America/Los_Angeles"),
    ("san jose", "America/Los_Angeles"),
    ("oakland", "America/Los_Angeles"),
    ("seattle", "America/Los_Angeles"),
    ("portland", "America/Los_Angeles"),
    ("las vegas", "America/Los_Angeles"),
    ("dallas", "America/Chicago"),
    ("houston", "America/Chicago"),
    ("austin", "America/Chicago"),
    ("minneapolis", "America/Chicago"),
    ("new orleans", "America/Chicago"),
    ("montreal", "America/Toronto"),
    ("ottawa", "America/Toronto"),
    ("calgary", "America/Edmonton"),
    ("rio de janeiro", "America/Sao_Paulo"),
    ("brasilia", "America/Sao_Paulo"),
    ("canberra", "Australia/Sydney"),
    ("wellington", "Pacific/Auckland"),
    ("beijing", "Asia/Shanghai"),
    ("shenzhen", "Asia/Shanghai"),
    ("guangzhou", "Asia/Shanghai"),
    ("mumbai", "Asia/Kolkata"),
    ("bombay", "Asia/Kolkata"),
    ("delhi", "Asia/Kolkata"),
    ("new delhi", "Asia/Kolkata"),
    ("bangalore", "Asia/Kolkata"),
    ("osaka", "Asia/Tokyo"),
    ("kiev", "Europe/Kyiv"),
    ("munich", "Europe/Berlin"),
    ("frankfurt", "Europe/Berlin"),
    ("hamburg", "Europe/Berlin"),
    ("milan", "Europe/Rome"),
    ("naples", "Europe/Rome"),
    ("barcelona", "Europe/Madrid"),
    ("lyon", "Europe/Paris"),
    ("marseille", "Europe/Paris"),
    ("manchester", "Europe/London"),
    ("edinburgh", "Europe/London"),
    ("glasgow", "Europe/London"),
    ("st petersburg", "Europe/Moscow"),
    ("saint petersburg", "Europe/Moscow"),
    ("tel aviv", "Asia/Jerusalem"),
    ("cape town", "Africa/Johannesburg"),
    ("abu dhabi", "Asia/Dubai"),
    ("hanoi", "Asia/Ho_Chi_Minh"),
    ("saigon", "Asia/Ho_Chi_Minh"),
];

/// A country means its capital's zone.
const COUNTRIES: &[(&str, &str)] = &[
    ("united states", "America/New_York"),
    ("united states of america", "America/New_York"),
    ("usa", "America/New_York"),
    ("us", "America/New_York"),
    ("america", "America/New_York"),
    ("canada", "America/Toronto"),
    ("mexico", "America/Mexico_City"),
    ("brazil", "America/Sao_Paulo"),
    ("argentina", "America/Argentina/Buenos_Aires"),
    ("chile", "America/Santiago"),
    ("colombia", "America/Bogota"),
    ("peru", "America/Lima"),
    ("venezuela", "America/Caracas"),
    ("ecuador", "America/Guayaquil"),
    ("bolivia", "America/La_Paz"),
    ("uruguay", "America/Montevideo"),
    ("paraguay", "America/Asuncion"),
    ("cuba", "America/Havana"),
    ("jamaica", "America/Jamaica"),
    ("costa rica", "America/Costa_Rica"),
    ("panama", "America/Panama"),
    ("guatemala", "America/Guatemala"),
    ("dominican republic", "America/Santo_Domingo"),
    ("puerto rico", "America/Puerto_Rico"),
    ("united kingdom", "Europe/London"),
    ("uk", "Europe/London"),
    ("great britain", "Europe/London"),
    ("britain", "Europe/London"),
    ("england", "Europe/London"),
    ("scotland", "Europe/London"),
    ("wales", "Europe/London"),
    ("ireland", "Europe/Dublin"),
    ("france", "Europe/Paris"),
    ("germany", "Europe/Berlin"),
    ("italy", "Europe/Rome"),
    ("spain", "Europe/Madrid"),
    ("portugal", "Europe/Lisbon"),
    ("netherlands", "Europe/Amsterdam"),
    ("holland", "Europe/Amsterdam"),
    ("belgium", "Europe/Brussels"),
    ("switzerland", "Europe/Zurich"),
    ("austria", "Europe/Vienna"),
    ("sweden", "Europe/Stockholm"),
    ("norway", "Europe/Oslo"),
    ("denmark", "Europe/Copenhagen"),
    ("finland", "Europe/Helsinki"),
    ("iceland", "Atlantic/Reykjavik"),
    ("poland", "Europe/Warsaw"),
    ("czech republic", "Europe/Prague"),
    ("czechia", "Europe/Prague"),
    ("hungary", "Europe/Budapest"),
    ("romania", "Europe/Bucharest"),
    ("bulgaria", "Europe/Sofia"),
    ("greece", "Europe/Athens"),
    ("turkey", "Europe/Istanbul"),
    ("ukraine", "Europe/Kyiv"),
    ("russia", "Europe/Moscow"),
    ("belarus", "Europe/Minsk"),
    ("serbia", "Europe/Belgrade"),
    ("croatia", "Europe/Zagreb"),
    ("slovakia", "Europe/Bratislava"),
    ("slovenia", "Europe/Ljubljana"),
    ("estonia", "Europe/Tallinn"),
    ("latvia", "Europe/Riga"),
    ("lithuania", "Europe/Vilnius"),
    ("luxembourg", "Europe/Luxembourg"),
    ("malta", "Europe/Malta"),
    ("cyprus", "Asia/Nicosia"),
    ("israel", "Asia/Jerusalem"),
    ("saudi arabia", "Asia/Riyadh"),
    ("united arab emirates", "Asia/Dubai"),
    ("uae", "Asia/Dubai"),
    ("qatar", "Asia/Qatar"),
    ("kuwait", "Asia/Kuwait"),
    ("iran", "Asia/Tehran"),
    ("iraq", "Asia/Baghdad"),
    ("jordan", "Asia/Amman"),
    ("lebanon", "Asia/Beirut"),
    ("egypt", "Africa/Cairo"),
    ("pakistan", "Asia/Karachi"),
    ("india", "Asia/Kolkata"),
    ("bangladesh", "Asia/Dhaka"),
    ("sri lanka", "Asia/Colombo"),
    ("nepal", "Asia/Kathmandu"),
    ("afghanistan", "Asia/Kabul"),
    ("kazakhstan", "Asia/Almaty"),
    ("china", "Asia/Shanghai"),
    ("japan", "Asia/Tokyo"),
    ("south korea", "Asia/Seoul"),
    ("korea", "Asia/Seoul"),
    ("north korea", "Asia/Pyongyang"),
    ("taiwan", "Asia/Taipei"),
    ("mongolia", "Asia/Ulaanbaatar"),
    ("thailand", "Asia/Bangkok"),
    ("vietnam", "Asia/Ho_Chi_Minh"),
    ("cambodia", "Asia/Phnom_Penh"),
    ("laos", "Asia/Vientiane"),
    ("malaysia", "Asia/Kuala_Lumpur"),
    ("indonesia", "Asia/Jakarta"),
    ("philippines", "Asia/Manila"),
    ("myanmar", "Asia/Yangon"),
    ("australia", "Australia/Sydney"),
    ("new zealand", "Pacific/Auckland"),
    ("fiji", "Pacific/Fiji"),
    ("papua new guinea", "Pacific/Port_Moresby"),
    ("south africa", "Africa/Johannesburg"),
    ("nigeria", "Africa/Lagos"),
    ("kenya", "Africa/Nairobi"),
    ("ethiopia", "Africa/Addis_Ababa"),
    ("ghana", "Africa/Accra"),
    ("morocco", "Africa/Casablanca"),
    ("algeria", "Africa/Algiers"),
    ("tunisia", "Africa/Tunis"),
    ("libya", "Africa/Tripoli"),
    ("tanzania", "Africa/Dar_es_Salaam"),
    ("uganda", "Africa/Kampala"),
    ("zimbabwe", "Africa/Harare"),
    ("zambia", "Africa/Lusaka"),
    ("senegal", "Africa/Dakar"),
    ("ivory coast", "Africa/Abidjan"),
    ("angola", "Africa/Luanda"),
];

/// IANA names under these prefixes are old aliases of other zones and are not offered as city names.
const LEGACY_PREFIXES: &[&str] = &[
    "Etc/", "SystemV/", "US/", "Canada/", "Brazil/", "Chile/", "Mexico/",
];

/// Reads a zone from `dir`, once. A name that could leave the folder is refused.
fn load(dir: &Path, name: &str) -> Option<&'static Loaded> {
    type Cache = Mutex<HashMap<(PathBuf, String), Option<&'static Loaded>>>;
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let plain = !name.is_empty()
        && name
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '+'));
    if !plain {
        return None;
    }
    let mut cache = CACHE.get_or_init(Default::default).lock().ok()?;
    let key = (dir.to_path_buf(), name.to_string());
    if let Some(found) = cache.get(&key) {
        return *found;
    }
    let loaded = std::fs::read(dir.join(name))
        .ok()
        .and_then(|bytes| tz::TimeZone::from_tz_data(&bytes).ok())
        .map(|rules| {
            &*Box::leak(Box::new(Loaded {
                name: name.to_string(),
                rules,
            }))
        });
    cache.insert(key, loaded);
    loaded
}

/// The IANA release of a zoneinfo folder, from the first line of its `tzdata.zi` (`# version 2026e`).
pub fn release(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("tzdata.zi")).ok()?;
    let first = text.lines().next()?;
    first
        .strip_prefix("# version ")
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// The release of the rules the current settings use: the folder's when it has one, else the bundled one.
pub fn current_release() -> String {
    match crate::settings::zone_source() {
        ZoneSource::System { dir } => release(&dir).unwrap_or_else(|| BUNDLED_RELEASE.to_string()),
        ZoneSource::Bundled => BUNDLED_RELEASE.to_string(),
    }
}

fn city_table() -> &'static HashMap<String, &'static str> {
    static TABLE: OnceLock<HashMap<String, &'static str>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table: HashMap<String, &'static str> = HashMap::new();
        let mut zones: Vec<&Tz> = TZ_VARIANTS.iter().collect();
        zones.sort_by_key(|tz| tz.name());
        for tz in zones {
            let name = tz.name();
            if LEGACY_PREFIXES.iter().any(|p| name.starts_with(p)) {
                continue;
            }
            let Some((_, city)) = name.rsplit_once('/') else {
                continue;
            };
            table
                .entry(city.replace('_', " ").to_lowercase())
                .or_insert(name);
        }
        for (alias, iana) in ALIASES.iter().chain(COUNTRIES).chain(REGIONS) {
            table.insert((*alias).to_string(), *iana);
        }
        table
    })
}

/// The zone a name stands for: a city, a country, a region, or an abbreviation. Case does not matter.
pub fn place(words: &[&str]) -> Option<Zone> {
    if words.len() > 4 {
        return None;
    }
    let name = words.join(" ").to_lowercase();
    if let Some((_, seconds)) = ABBREVIATIONS.iter().find(|(abbr, _)| *abbr == name) {
        return Some(Zone::Fixed(*seconds));
    }
    city_table().get(&name).and_then(|iana| Zone::named(iana))
}

/// `GMT+8` or `UTC-5:30` as a fixed zone, from the hours and minutes written after the sign.
pub fn offset_zone(negative: bool, hours: i64, minutes: i64) -> Option<Zone> {
    (hours <= 18 && minutes < 60).then(|| {
        let seconds = hours * 3600 + minutes * 60;
        Zone::Fixed(if negative { -seconds } else { seconds })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_table_entry_names_a_zone_in_the_bundled_database() {
        for (alias, iana) in ALIASES.iter().chain(COUNTRIES).chain(REGIONS) {
            assert!(iana.parse::<Tz>().is_ok(), "{alias} -> {iana}");
        }
    }

    #[test]
    fn no_alias_or_country_is_listed_twice() {
        let mut seen = std::collections::HashSet::new();
        for (name, _) in ALIASES.iter().chain(COUNTRIES).chain(REGIONS) {
            assert!(seen.insert(*name), "{name} is listed twice");
        }
    }

    #[test]
    fn a_city_name_shared_by_two_database_names_means_the_same_rules() {
        let mut seen: HashMap<String, Tz> = HashMap::new();
        let instants = (0..600).map(|k| 631_152_000 + k * 30 * 86_400);
        for tz in TZ_VARIANTS.iter() {
            let name = tz.name();
            if LEGACY_PREFIXES.iter().any(|p| name.starts_with(p)) {
                continue;
            }
            if let Some((_, city)) = name.rsplit_once('/') {
                let key = city.replace('_', " ").to_lowercase();
                if let Some(other) = seen.insert(key.clone(), *tz) {
                    for t in instants.clone() {
                        assert_eq!(
                            Zone::Named(other).offset_at(t),
                            Zone::Named(*tz).offset_at(t),
                            "{key}: {} and {name} differ at {t}",
                            other.name()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_repeated_hour_takes_the_first_reading_and_a_skipped_hour_the_offset_before() {
        let new_york = Zone::named("America/New_York").expect("bundled");
        // 2024-11-03 01:30 happens twice; the first is EDT (UTC-4).
        let local = 1_730_597_400;
        assert_eq!(new_york.local_to_utc(local), local + 4 * 3600);
        // 2024-03-10 02:30 does not exist; the offset before the change (UTC-5) is used.
        let skipped = 1_710_037_800;
        assert_eq!(new_york.local_to_utc(skipped), skipped + 5 * 3600);
    }
}
