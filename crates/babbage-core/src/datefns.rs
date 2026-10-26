//! Questions about dates: which weekday, which week, how many days, how many workdays.

use crate::calendar::{self, Civil, WEEKDAYS};
use crate::dates;
use crate::error::Error;
use crate::number::Number;
use crate::settings;
use crate::units;
use crate::value::{Kind, Value};
use crate::zones::Zone;

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
    /// (time or date, zone it is written in, zone to show it in): zones travel as text.
    ZoneConvert,
    /// The current time of day in a zone.
    TimeIn,
    /// The current date in a zone.
    DateIn,
    /// (zone, zone): how far the first is ahead of the second right now.
    ZoneDifference,
    /// How much of the current year, month, week (from Monday) or day has gone, as a percentage.
    ElapsedYear,
    ElapsedMonth,
    ElapsedWeek,
    ElapsedDay,
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
            | DateFn::AsFrames
            | DateFn::TimeIn
            | DateFn::DateIn => 1,
            DateFn::CurrentTimestamp
            | DateFn::ElapsedYear
            | DateFn::ElapsedMonth
            | DateFn::ElapsedWeek
            | DateFn::ElapsedDay => 0,
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
            | DateFn::AsFramesAt
            | DateFn::ZoneDifference => 2,
            DateFn::ZoneConvert => 3,
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
            // A date with no time is midday, as the reference app does (`April 1, 2019` is 12:00 local).
            let rest = if args[0].is_bare_date() {
                Number::from_ratio(num_bigint::BigInt::from(1), num_bigint::BigInt::from(2))?
            } else {
                rest
            };
            let local = Number::from_i64(day)
                .add(&rest)?
                .mul(&Number::from_i64(86_400))?;
            Ok(digits(settings::wall_to_utc(&local)?))
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
        DateFn::ZoneConvert => zone_convert(&args[0], &args[1], &args[2]),
        DateFn::TimeIn => {
            let zone = zone_arg(&args[0])?;
            let now = settings::now_utc_seconds();
            let wall = now + zone.offset_at(now);
            // On the day before or after the local day the answer names that day: `Tomorrow at 12:30 AM`.
            if wall.div_euclid(86_400) != settings::local_now().0 {
                let days = Number::from_ratio(
                    num_bigint::BigInt::from(wall),
                    num_bigint::BigInt::from(86_400),
                )?;
                return Ok(Value::date_at(days, true));
            }
            Ok(Value::clock(Number::from_i64(wall.rem_euclid(86_400))))
        }
        DateFn::DateIn => {
            let zone = zone_arg(&args[0])?;
            let now = settings::now_utc_seconds();
            Ok(Value::date(Number::from_i64(
                (now + zone.offset_at(now)).div_euclid(86_400),
            )))
        }
        DateFn::ZoneDifference => {
            let (a, b) = (zone_arg(&args[0])?, zone_arg(&args[1])?);
            let now = settings::now_utc_seconds();
            // How far ahead the second place is of the first, as the reference app prints it.
            Ok(Value::quantity(
                Number::from_i64(b.offset_at(now) - a.offset_at(now)),
                unit("s"),
            )?
            .as_timespan())
        }
        DateFn::ElapsedYear | DateFn::ElapsedMonth | DateFn::ElapsedWeek | DateFn::ElapsedDay => {
            elapsed(f)
        }
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

/// How much of the current year, month, week or day has gone, as a percentage of its whole length.
fn elapsed(f: DateFn) -> Result<Value, Error> {
    let (today, seconds) = settings::local_now();
    let civil = calendar::civil_from_days(today);
    let (start, end) = match f {
        DateFn::ElapsedYear => (
            calendar::days_from_civil(calendar::Civil {
                year: civil.year,
                month: 1,
                day: 1,
            }),
            calendar::days_from_civil(calendar::Civil {
                year: civil.year + 1,
                month: 1,
                day: 1,
            }),
        ),
        DateFn::ElapsedMonth => {
            let first = calendar::days_from_civil(calendar::Civil {
                year: civil.year,
                month: civil.month,
                day: 1,
            });
            (
                first,
                first + i64::from(calendar::days_in_month(civil.year, civil.month)),
            )
        }
        DateFn::ElapsedWeek => {
            let monday = today - i64::from(calendar::weekday(today)).rem_euclid(7);
            (monday, monday + 7)
        }
        _ => (today, today + 1),
    };
    let gone = Number::from_i64((today - start) * 86_400 + seconds);
    let whole = Number::from_i64((end - start) * 86_400);
    Ok(Value::percent(gone.div(&whole)?))
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

/// The zone a text value names: `local`, or what `Zone::to_spec` wrote.
fn zone_arg(v: &Value) -> Result<Zone, Error> {
    match v.words() {
        Some("local") => Ok(settings::local_zone()),
        Some(spec) => {
            Zone::from_spec(spec).ok_or(Error::UnsupportedOperation("this time zone is not known"))
        }
        None => Err(Error::UnsupportedOperation("a time zone is needed")),
    }
}

/// A time of day or a date written in one zone, shown in another. A time of day with no date means that
/// time on today's date where it was written; it stays a time of day if it lands on the same date there.
fn zone_convert(v: &Value, from: &Value, to: &Value) -> Result<Value, Error> {
    let (from, to) = (zone_arg(from)?, zone_arg(to)?);
    let now = settings::now_utc_seconds();
    let day_seconds = Number::from_i64(86_400);
    let (written_day, local) = match v.kind() {
        Kind::Clock => {
            let day = (now + from.offset_at(now)).div_euclid(86_400);
            (
                Some(day),
                Number::from_i64(day).mul(&day_seconds)?.add(v.number())?,
            )
        }
        Kind::Date => (None, v.number().mul(&day_seconds)?),
        _ => {
            return Err(Error::UnsupportedOperation(
                "a time zone applies to a time of day or a date",
            ));
        }
    };
    let whole = local.floor();
    let fraction = local.sub(&Number::from_ratio(
        whole.clone(),
        num_bigint::BigInt::from(1),
    )?)?;
    let whole = i64::try_from(&whole).map_err(|_| Error::Overflow)?;
    let utc = from.local_to_utc(whole);
    let there = Number::from_i64(utc + to.offset_at(utc)).add(&fraction)?;
    let day = i64::try_from(&there.div(&day_seconds)?.floor()).map_err(|_| Error::Overflow)?;
    if written_day == Some(day) {
        let into_day = there.sub(&Number::from_i64(day).mul(&day_seconds)?)?;
        return Ok(Value::clock(into_day));
    }
    Ok(Value::date_at(there.div(&day_seconds)?, true))
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
    if v.unit().dim() != unit("s").dim() {
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
    if length.unit().dim() != unit("s").dim() {
        return Err(Error::UnsupportedOperation("this needs a length of time"));
    }
    let seconds = length.number().mul(&length.unit().factor()?)?;
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
            let local = settings::utc_to_wall(&seconds)?;
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
    let wall = Number::from_i64(day)
        .mul(&seconds_per_day())?
        .add(&seconds_in_day)?;
    let utc_at =
        i64::try_from(&settings::wall_to_utc(&wall)?.floor()).map_err(|_| Error::Overflow)?;
    let offset = settings::local_offset_at(utc_at);
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
