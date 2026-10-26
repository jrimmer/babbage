//! Per-evaluation settings that are not part of the text of a line: the tax rate, the clock and the home time zone.
//!
//! They are held in a thread-local for the duration of an evaluation, so the lexer can turn `VAT` or `today` into
//! a value without every function in the engine carrying a context argument.

use std::cell::RefCell;

use crate::number::Number;

/// A sales tax named in settings: `VAT` at 15%.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tax {
    pub name: String,
    pub rate: Number,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Settings {
    pub tax: Option<Tax>,
    /// The moment `now` and `today` mean, in seconds since 1970-01-01 UTC. None reads the system clock.
    pub now_utc_seconds: Option<i64>,
    /// Seconds east of UTC for local time (a fixed offset; named zones are looked up where they are written).
    pub local_utc_offset_seconds: i64,
    /// Hours in a working day, for `work hours`.
    pub work_hours_per_day: Option<u32>,
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

/// The current moment in UTC seconds, from the settings or the system clock.
pub fn now_utc_seconds() -> i64 {
    current(|s| s.now_utc_seconds).unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
    })
}

pub fn local_offset_seconds() -> i64 {
    current(|s| s.local_utc_offset_seconds)
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
