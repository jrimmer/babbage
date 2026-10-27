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

/// A day off work: one date (`2026-12-25`), or the same day every year (`12-25`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Holiday {
    On(i64),
    Yearly { month: u32, day: u32 },
}

impl Holiday {
    /// `2026-12-25` or `12-25`; anything else is not a holiday.
    pub fn parse(text: &str) -> Option<Holiday> {
        use crate::calendar::{Civil, days_from_civil, is_valid};
        let parts: Vec<&str> = text.trim().split('-').collect();
        let num = |s: &str| s.trim().parse::<u32>().ok();
        match parts.as_slice() {
            [y, m, d] => {
                let civil = Civil {
                    year: y.trim().parse::<i64>().ok()?,
                    month: num(m)?,
                    day: num(d)?,
                };
                is_valid(civil).then(|| Holiday::On(days_from_civil(civil)))
            }
            [m, d] => {
                let (month, day) = (num(m)?, num(d)?);
                // A day that exists in some year (`02-29` does).
                is_valid(Civil {
                    year: 2024,
                    month,
                    day,
                })
                .then_some(Holiday::Yearly { month, day })
            }
            _ => None,
        }
    }
}

/// The holidays written in a settings line: dates separated by `;` or `,`.
pub fn parse_holidays(text: &str) -> Vec<Holiday> {
    text.split([';', ',']).filter_map(Holiday::parse).collect()
}

/// Names for places that are zones: `HQ = Europe/Berlin`, written in a settings line separated by `;`.
pub fn parse_custom_zones(text: &str) -> Vec<(String, String)> {
    text.split(';')
        .filter_map(|part| {
            let (name, zone) = part.split_once('=')?;
            let (name, zone) = (name.trim().to_lowercase(), zone.trim().to_string());
            (!name.is_empty() && crate::zones::Zone::named(&zone).is_some()).then_some((name, zone))
        })
        .collect()
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
    /// Where zone rules come from: the bundled database (the default) or the machine's zoneinfo files.
    pub zone_source: crate::zones::ZoneSource,
    /// How numbers are typed and shown: `1,234.56`, `1.234,56` or `1 234,56`.
    pub number_format: crate::format::NumberFormat,
    /// `12/02/1988` and `12-02-1988` mean 12 February (day first, as in Australia) when set, and December 2
    /// (month first, as in the United States) when not.
    pub day_first: bool,
    /// How many decimal places answers show at most; None is ten.
    pub decimal_places: Option<u32>,
    /// Whether large numbers read `300k` and `16.7M`; None is yes.
    pub notation: Option<bool>,
    /// Whether answers have thousands separators; None is yes.
    pub group_thousands: Option<bool>,
    /// Money with its code (`5.00 EUR`) and not its symbol (`€5.00`).
    pub currency_codes: bool,
    /// Money without trailing zeros (`$5`).
    pub money_trim_zeros: bool,
    /// Days off, which workday arithmetic skips.
    pub holidays: Vec<Holiday>,
    /// The user's own names for places: lower-case name and the zone it stands for.
    pub custom_zones: Vec<(String, String)>,
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
        let mut o = crate::format::FormatOptions::default().in_format(self.number_format);
        if let Some(places) = self.decimal_places
            && let Ok(fixed) = o.with_places(places.min(10))
        {
            o = fixed;
        }
        o.abbreviate_large = self.notation.unwrap_or(true);
        o.group_thousands = self.group_thousands.unwrap_or(true);
        o.currency_codes = self.currency_codes;
        o.money_trim_zeros = self.money_trim_zeros;
        o
    }
}

/// The holidays in force that fall on days `from` up to but not including `to` (days since 1970-01-01).
pub fn holidays_in(from: i64, to: i64) -> Vec<i64> {
    use crate::calendar::{Civil, civil_from_days, days_from_civil, is_valid};
    current(|s| {
        let mut out = Vec::new();
        for h in &s.holidays {
            match h {
                Holiday::On(d) if *d >= from && *d < to => out.push(*d),
                Holiday::On(_) => {}
                Holiday::Yearly { month, day } => {
                    // Years far past any calendar the rules matter for are left out.
                    let (a, b) = (
                        civil_from_days(from).year,
                        civil_from_days(to.saturating_sub(1).max(from)).year,
                    );
                    for year in a.max(-9999)..=b.min(a.saturating_add(400)).min(9999) {
                        let civil = Civil {
                            year,
                            month: *month,
                            day: *day,
                        };
                        if is_valid(civil) {
                            let d = days_from_civil(civil);
                            if d >= from && d < to {
                                out.push(d);
                            }
                        }
                    }
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    })
}

/// The zone a user's own name for a place stands for.
pub fn custom_zone(name: &str) -> Option<crate::zones::Zone> {
    current(|s| {
        s.custom_zones
            .iter()
            .find(|(n, _)| n == name)
            .and_then(|(_, zone)| crate::zones::Zone::named(zone))
    })
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

/// How numbers are typed and shown right now.
pub fn number_format() -> crate::format::NumberFormat {
    current(|s| s.number_format)
}

/// Whether `12/02/1988` is read day first.
pub fn day_first() -> bool {
    current(|s| s.day_first)
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

/// The local zone's short name at this instant (`PST`).
pub fn local_abbreviation_at(utc: i64) -> String {
    local_zone().abbreviation_at(utc)
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
