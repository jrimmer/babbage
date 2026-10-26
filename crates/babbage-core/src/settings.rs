//! Per-evaluation settings that are not part of the text of a line: the tax rate, the clock and the home time zone.
//!
//! They are held in a thread-local for the duration of an evaluation, so the lexer can turn `VAT` or `today` into
//! a value without every function in the engine carrying a context argument.

use std::cell::RefCell;

use crate::error::Error;
use crate::number::Number;

/// A sales tax named in settings: `VAT` at 15%.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tax {
    pub name: String,
    pub rate: Number,
}

/// Which gallons, quarts, pints and fluid ounces are meant. The cup, tablespoon and teaspoon stay US customary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum VolumeSystem {
    #[default]
    UsCustomary,
    Imperial,
}

/// Dollar currencies a region can make `$` mean.
pub const REGION_DOLLARS: &[&str] = &["USD", "AUD", "CAD", "NZD", "SGD", "HKD"];

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Settings {
    pub tax: Option<Tax>,
    /// The moment `now` and `today` mean, in seconds since 1970-01-01 UTC. None reads the system clock.
    pub now_utc_seconds: Option<i64>,
    /// Seconds east of UTC for local time when `local_zone` is not set: one fixed offset all year.
    pub local_utc_offset_seconds: i64,
    /// The user's own time zone, with its daylight saving rules. When set it replaces the fixed offset.
    pub local_zone: Option<crate::zones::Zone>,
    /// Hours in a working day, for `work hours`.
    pub work_hours_per_day: Option<u32>,
    /// The currency a bare `$` means: `AUD` in Australia. None is US dollars.
    pub dollar: Option<&'static str>,
    pub volume_system: VolumeSystem,
    /// Where zone rules come from: the bundled database (the default) or the machine's zoneinfo files.
    pub zone_source: crate::zones::ZoneSource,
    /// How numbers are typed and shown: `1,234.56`, `1.234,56` or `1 234,56`.
    pub number_format: crate::format::NumberFormat,
}

impl Settings {
    /// A clock fixed at `utc_seconds`, for repeatable tests.
    pub fn at(utc_seconds: i64) -> Settings {
        Settings {
            now_utc_seconds: Some(utc_seconds),
            ..Settings::default()
        }
    }
}

impl Settings {
    /// The options to show answers with under these settings.
    pub fn format_options(&self) -> crate::format::FormatOptions {
        crate::format::FormatOptions::default().in_format(self.number_format)
    }
}

/// The currency code a bare `$` means right now.
pub fn dollar_code() -> &'static str {
    current(|s| {
        s.dollar
            .filter(|c| REGION_DOLLARS.contains(c))
            .unwrap_or("USD")
    })
}

pub fn zone_source() -> crate::zones::ZoneSource {
    current(|s| s.zone_source.clone())
}

pub fn volume_system() -> VolumeSystem {
    current(|s| s.volume_system)
}

/// How numbers are typed and shown right now.
pub fn number_format() -> crate::format::NumberFormat {
    current(|s| s.number_format)
}

/// The current moment in UTC seconds, from the settings or the system clock.
pub fn now_utc_seconds() -> i64 {
    current(|s| s.now_utc_seconds).unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
    })
}

/// The zone local time is read in: the user's own, or the fixed offset from the settings.
pub fn local_zone() -> crate::zones::Zone {
    current(|s| {
        s.local_zone
            .unwrap_or(crate::zones::Zone::Fixed(s.local_utc_offset_seconds))
    })
}

/// Seconds east of UTC for local time at this instant.
pub fn local_offset_at(utc: i64) -> i64 {
    local_zone().offset_at(utc)
}

/// Seconds east of UTC for local time right now.
pub fn local_offset_seconds() -> i64 {
    local_offset_at(now_utc_seconds())
}

/// A local wall-clock reading in seconds (since 1970-01-01 00:00 local) as the UTC instant it falls on.
pub fn wall_to_utc(wall: &Number) -> Result<Number, Error> {
    let whole = wall.floor();
    let fraction = wall.sub(&Number::from_ratio(
        whole.clone(),
        num_bigint::BigInt::from(1),
    )?)?;
    let whole = i64::try_from(&whole).map_err(|_| Error::Overflow)?;
    Number::from_i64(local_zone().local_to_utc(whole)).add(&fraction)
}

/// A UTC instant in seconds as the local wall-clock reading it shows.
pub fn utc_to_wall(utc: &Number) -> Result<Number, Error> {
    let whole = i64::try_from(&utc.floor()).map_err(|_| Error::Overflow)?;
    utc.add(&Number::from_i64(local_offset_at(whole)))
}

/// Local time as (days since 1970-01-01, seconds into that day).
pub fn local_now() -> (i64, i64) {
    let local = now_utc_seconds() + local_offset_seconds();
    (local.div_euclid(86_400), local.rem_euclid(86_400))
}

pub fn work_hours_per_day() -> u32 {
    current(|s| s.work_hours_per_day).unwrap_or(8)
}

thread_local! {
    static CURRENT: RefCell<Settings> = RefCell::new(Settings::default());
}

/// Runs `f` with `settings` in force, then restores whatever was there.
pub fn scoped<T>(settings: &Settings, f: impl FnOnce() -> T) -> T {
    let previous = CURRENT.with(|c| c.replace(settings.clone()));
    let result = f();
    CURRENT.with(|c| *c.borrow_mut() = previous);
    result
}

pub fn current<T>(f: impl FnOnce(&Settings) -> T) -> T {
    CURRENT.with(|c| f(&c.borrow()))
}

/// The tax rate when `name` is the configured tax's name, ignoring case.
pub fn tax_rate_named(name: &str) -> Option<Number> {
    current(|s| {
        s.tax
            .as_ref()
            .filter(|t| t.name.eq_ignore_ascii_case(name))
            .map(|t| t.rate.clone())
    })
}
