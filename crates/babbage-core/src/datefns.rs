//! Questions about dates: which weekday, which week, how many days, how many workdays.

use crate::calendar::{self, Civil, WEEKDAYS};
use crate::dates;
use crate::error::Error;
use crate::number::Number;
use crate::settings;
use crate::units;
use crate::value::{Kind, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateFn {
    Weekday,
    DayOfMonth,
    DayNumber,
    WeekNumber,
    /// The days in the month a date falls in.
    DaysInMonthOf,
    /// (quarter 1 to 4, year)
    DaysInQuarter,
    DaysInYear,
    DaysBetween,
    /// A date minus today, and today minus a date.
    Until,
    Since,
    /// Both ends counted: `April 1 through April 30`.
    Inclusive,
    WorkdaysBetween,
    /// Workdays from today over a length of time.
    WorkdaysIn,
    WorkHoursBetween,
    /// (date, pattern text)
    Format,
    /// (date, time of day)
    DateAt,
    /// `7:30am to 8:45pm` and `March 1 to March 5`: from the first to the second, a clock rolling over midnight.
    Interval,
    /// (length of time, speed): `1 hour 30 minutes at 1.5x`.
    Playback,
    /// (length of time, speed): `time saved 5 min at 1.5x`.
    TimeSaved,
    /// A Unix timestamp in seconds or milliseconds, or a date, as a date.
    ToDate,
    ToTimestamp,
    AsIso,
    CurrentTimestamp,
    /// A time or a frame count as `01:02:03:04`, at 24 frames per second unless it already is a timecode.
    AsTimecode,
    /// (value, frames per second)
    AsTimecodeAt,
    /// A timecode as a count of frames.
    AsFrames,
    /// (length of time, frames per second)
    AsFramesAt,
}

impl DateFn {
    pub fn arity(self) -> usize {
        match self {
            DateFn::Weekday
            | DateFn::DayOfMonth
            | DateFn::DayNumber
            | DateFn::WeekNumber
            | DateFn::DaysInMonthOf
            | DateFn::DaysInYear
            | DateFn::Until
            | DateFn::Since
            | DateFn::WorkdaysIn
            | DateFn::ToDate
            | DateFn::ToTimestamp
            | DateFn::AsIso
            | DateFn::AsTimecode
            | DateFn::AsFrames => 1,
            DateFn::CurrentTimestamp => 0,
            DateFn::DaysInQuarter
            | DateFn::DaysBetween
            | DateFn::Inclusive
            | DateFn::WorkdaysBetween
            | DateFn::WorkHoursBetween
            | DateFn::Format
            | DateFn::DateAt
            | DateFn::Interval
            | DateFn::Playback
            | DateFn::TimeSaved
            | DateFn::AsTimecodeAt
            | DateFn::AsFramesAt => 2,
        }
    }
}

fn unit(name: &str) -> units::UnitExpr {
    units::lookup(name).expect("unit is defined")
}

fn date_arg(v: &Value) -> Result<(i64, Number), Error> {
    if v.kind() != Kind::Date {
        return Err(Error::UnsupportedOperation("this needs a date"));
    }
    let day = v.number().floor();
    let rest = v.number().sub(&Number::from_ratio(
        day.clone(),
        num_bigint::BigInt::from(1),
    )?)?;
    Ok((i64::try_from(&day).map_err(|_| Error::Overflow)?, rest))
}

fn whole(v: &Value) -> Result<i64, Error> {
    if v.kind() != Kind::Plain || !v.unit().is_empty() || !v.number().is_integer() {
        return Err(Error::UnsupportedOperation("this needs a whole number"));
    }
    i64::try_from(v.number().numer()).map_err(|_| Error::Overflow)
}

fn days(n: i64) -> Result<Value, Error> {
    Value::quantity(Number::from_i64(n), unit("day"))
}

pub fn call(f: DateFn, args: &[Value]) -> Result<Value, Error> {
    if args.len() != f.arity() {
        return Err(Error::UnsupportedOperation(
            "this was given the wrong number of values",
        ));
    }
    let (today, _) = settings::local_now();
    match f {
        DateFn::Weekday => Ok(Value::text(
            WEEKDAYS[calendar::weekday(date_arg(&args[0])?.0) as usize],
        )),
        DateFn::DayOfMonth => Ok(Value::plain(Number::from_i64(i64::from(
            calendar::civil_from_days(date_arg(&args[0])?.0).day,
        )))),
        DateFn::DayNumber => Ok(Value::plain(Number::from_i64(i64::from(
            calendar::day_of_year(calendar::civil_from_days(date_arg(&args[0])?.0)),
        )))),
        DateFn::WeekNumber => Ok(Value::plain(Number::from_i64(i64::from(
            calendar::iso_week(calendar::civil_from_days(date_arg(&args[0])?.0)).1,
        )))),
        DateFn::DaysInMonthOf => {
            let c = calendar::civil_from_days(date_arg(&args[0])?.0);
            days(i64::from(calendar::days_in_month(c.year, c.month)))
        }
        DateFn::DaysInQuarter => {
            let (q, year) = (whole(&args[0])?, whole(&args[1])?);
            if !(1..=4).contains(&q) {
                return Err(Error::OutOfDomain);
            }
            let first = (q as u32 - 1) * 3 + 1;
            days(
                (first..first + 3)
                    .map(|m| i64::from(calendar::days_in_month(year, m)))
                    .sum(),
            )
        }
        DateFn::DaysInYear => days(if calendar::is_leap(whole(&args[0])?) {
            366
        } else {
            365
        }),
        DateFn::DaysBetween => days((date_arg(&args[0])?.0 - date_arg(&args[1])?.0).abs()),
        DateFn::Until => days(date_arg(&args[0])?.0 - today),
        DateFn::Since => days(today - date_arg(&args[0])?.0),
        DateFn::Inclusive => days(date_arg(&args[1])?.0 - date_arg(&args[0])?.0 + 1),
        DateFn::WorkdaysBetween => Value::quantity(
            Number::from_i64(calendar::workdays_between(
                date_arg(&args[0])?.0,
                date_arg(&args[1])?.0,
            )),
            unit("workday"),
        ),
        DateFn::WorkdaysIn => {
            let length = &args[0];
            let seconds = length.number().mul(&length.unit().factor()?)?;
            let end = today
                + i64::try_from(&seconds.div(&Number::from_i64(86_400))?.floor())
                    .map_err(|_| Error::Overflow)?;
            Value::quantity(
                Number::from_i64(calendar::workdays_between(today, end)),
                unit("workday"),
            )
        }
        DateFn::WorkHoursBetween => {
            let n = calendar::workdays_between(date_arg(&args[0])?.0, date_arg(&args[1])?.0);
            Value::quantity(
                Number::from_i64(n * i64::from(settings::work_hours_per_day())),
                unit("h"),
            )
        }
        DateFn::Format => {
            let pattern = args[1]
                .words()
                .ok_or(Error::UnsupportedOperation("a date format is needed"))?;
            dates::format_pattern(args[0].number(), pattern).map(|s| Value::text(&s))
        }
        DateFn::Interval => interval(&args[0], &args[1]),
        DateFn::Playback => playback(&args[0], &args[1], false),
        DateFn::TimeSaved => playback(&args[0], &args[1], true),
        DateFn::ToDate => to_date(&args[0]),
        DateFn::ToTimestamp => {
            let (day, rest) = date_arg(&args[0])?;
            let local = Number::from_i64(day)
                .add(&rest)?
                .mul(&Number::from_i64(86_400))?;
            Ok(digits(local.sub(&Number::from_i64(
                settings::local_offset_seconds(),
            ))?))
        }
        DateFn::AsIso => as_iso(&args[0]),
        DateFn::AsTimecode => {
            let rate = args[0]
                .timecode_rate()
                .cloned()
                .unwrap_or_else(|| Number::from_i64(DEFAULT_FRAME_RATE));
            as_timecode(&args[0], rate)
        }
        DateFn::AsTimecodeAt => as_timecode(&args[0], plain_rate(&args[1])?),
        DateFn::AsFrames => as_frames(&args[0], args[0].timecode_rate().cloned()),
        DateFn::AsFramesAt => as_frames(&args[0], Some(plain_rate(&args[1])?)),
        DateFn::CurrentTimestamp => Ok(digits(Number::from_i64(settings::now_utc_seconds()))),
        DateFn::DateAt => {
            let (day, _) = date_arg(&args[0])?;
            if args[1].kind() != Kind::Clock {
                return Err(Error::UnsupportedOperation(
                    "a time of day is needed after `at`",
                ));
            }
            let fraction = args[1].number().div(&Number::from_i64(86_400))?;
            Ok(Value::date(Number::from_i64(day).add(&fraction)?))
        }
    }
}

/// The date halfway between two dates.
pub fn midpoint(a: &Value, b: &Value) -> Result<Value, Error> {
    if a.kind() != Kind::Date || b.kind() != Kind::Date {
        return Err(Error::IncompatibleUnits);
    }
    Ok(Value::date(
        a.number().add(b.number())?.div(&Number::from_i64(2))?,
    ))
}

/// The first day of `month` in `year`, as a date value.
pub fn first_of(year: i64, month: u32) -> Option<Value> {
    let c = Civil {
        year,
        month,
        day: 1,
    };
    calendar::is_valid(c).then(|| Value::date(Number::from_i64(calendar::days_from_civil(c))))
}

/// Frames per second when a timecode does not say.
const DEFAULT_FRAME_RATE: i64 = 24;

fn plain_rate(v: &Value) -> Result<Number, Error> {
    if v.kind() != Kind::Plain
        || !v.unit().is_empty()
        || !v.number().is_integer()
        || !v.number().is_positive()
    {
        return Err(Error::UnsupportedOperation(
            "frames per second must be a whole number above zero",
        ));
    }
    Ok(v.number().clone())
}

fn as_timecode(v: &Value, rate: Number) -> Result<Value, Error> {
    let frame = units::lookup("frame").expect("frames are defined");
    let seconds = unit("s");
    if v.kind() != Kind::Plain || v.unit().is_empty() {
        return Err(Error::UnsupportedOperation(
            "this needs a length of time or a number of frames",
        ));
    }
    let length = if v.unit().dim() == frame.dim() {
        v.number().mul(&v.unit().factor()?)?.div(&rate)?
    } else if v.unit().dim() == seconds.dim() {
        v.number().mul(&v.unit().factor()?)?
    } else {
        return Err(Error::UnsupportedOperation(
            "this needs a length of time or a number of frames",
        ));
    };
    Ok(Value::quantity(length, seconds)?.as_timecode(rate))
}

fn as_frames(v: &Value, rate: Option<Number>) -> Result<Value, Error> {
    let frame = unit("frame");
    if v.kind() == Kind::Plain && !v.unit().is_empty() && v.unit().dim() == frame.dim() {
        return v.clone().convert(&frame);
    }
    if v.kind() != Kind::Plain || v.unit().is_empty() || v.unit().dim() != unit("s").dim() {
        return Err(Error::UnsupportedOperation("this needs a length of time"));
    }
    let rate = rate.ok_or(Error::UnsupportedOperation(
        "a frame count needs a timecode or `at <rate> fps`",
    ))?;
    let frames = v.number().mul(&v.unit().factor()?)?.mul(&rate)?;
    Value::quantity(frames, unit("frame"))
}

/// Past this many seconds (the year 5138) a timestamp is read as milliseconds.
const MILLISECOND_THRESHOLD: i64 = 100_000_000_000;

fn digits(n: Number) -> Value {
    Value::plain(n).written_in_full()
}

fn seconds_per_day() -> Number {
    Number::from_i64(86_400)
}

fn interval(a: &Value, b: &Value) -> Result<Value, Error> {
    let seconds = || units::lookup("s").expect("seconds are defined");
    match (a.kind(), b.kind()) {
        (Kind::Clock, Kind::Clock) => {
            let mut apart = b.number().sub(a.number())?;
            if apart.is_negative() {
                apart = apart.add(&seconds_per_day())?;
            }
            Ok(Value::quantity(apart, seconds())?.as_timespan())
        }
        (Kind::Date, Kind::Date) => {
            let apart = b.number().sub(a.number())?.mul(&seconds_per_day())?;
            Ok(Value::quantity(apart, seconds())?.as_timespan())
        }
        _ => Err(Error::UnsupportedOperation(
            "`to` between times needs two times of day or two dates",
        )),
    }
}

/// A length of time at a playback speed takes that much less; the time saved is the difference.
fn playback(length: &Value, speed: &Value, saved: bool) -> Result<Value, Error> {
    if speed.kind() != Kind::Multiplier {
        return Err(Error::UnsupportedOperation("a speed like 1.5x is needed"));
    }
    let seconds = if length.kind() == Kind::Plain && !length.unit().is_empty() {
        length.number().mul(&length.unit().factor()?)?
    } else {
        return Err(Error::UnsupportedOperation("this needs a length of time"));
    };
    if length.unit().dim() != units::lookup("s").expect("seconds are defined").dim() {
        return Err(Error::UnsupportedOperation("this needs a length of time"));
    }
    let at_speed = seconds.div(speed.number())?;
    let result = if saved {
        seconds.sub(&at_speed)?
    } else {
        at_speed
    };
    Ok(Value::quantity(result, unit("s"))?.as_timespan())
}

fn to_date(v: &Value) -> Result<Value, Error> {
    match v.kind() {
        Kind::Date => Ok(v.clone()),
        Kind::Plain if v.unit().is_empty() => {
            let thousand = Number::from_i64(1000);
            let mut seconds = v.number().clone();
            if seconds.abs().cmp(&Number::from_i64(MILLISECOND_THRESHOLD))
                != std::cmp::Ordering::Less
            {
                seconds = seconds.div(&thousand)?;
            }
            let local = seconds.add(&Number::from_i64(settings::local_offset_seconds()))?;
            Ok(Value::date(local.div(&seconds_per_day())?))
        }
        _ => Err(Error::UnsupportedOperation(
            "`to date` needs a timestamp, a date or a date string",
        )),
    }
}

fn as_iso(v: &Value) -> Result<Value, Error> {
    if v.kind() != Kind::Date {
        return Err(Error::UnsupportedOperation("this needs a date"));
    }
    let (day, rest) = date_arg(v)?;
    let civil = calendar::civil_from_days(day);
    let seconds_in_day = rest.mul(&seconds_per_day())?;
    let (_, whole, fraction) = dates::split_seconds(&seconds_in_day);
    let (secs, exact) = dates::seconds_text(whole % 60, &fraction);
    if !exact {
        return Err(Error::UnsupportedOperation(
            "this time has a fraction of a second that does not end",
        ));
    }
    let secs = if secs.split('.').next().is_some_and(|w| w.len() < 2) {
        format!("0{secs}")
    } else {
        secs
    };
    let offset = settings::local_offset_seconds();
    let zone = if offset == 0 {
        "Z".to_string()
    } else {
        let a = offset.abs();
        format!(
            "{}{:02}:{:02}",
            if offset < 0 { '-' } else { '+' },
            a / 3600,
            a % 3600 / 60
        )
    };
    Ok(Value::text(&format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{secs}{zone}",
        civil.year,
        civil.month,
        civil.day,
        whole / 3600,
        whole % 3600 / 60
    )))
}
