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
            | DateFn::WorkdaysIn => 1,
            _ => 2,
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
