//! Compound growth, present value, returns, loan repayments and growth rates.
//!
//! Everything is exact rational arithmetic. An answer that needs a fractional power or a logarithm (`time to
//! reach a target`, `growth rate`) comes back approximate, flagged like any other irrational result.

use crate::approx;
use crate::error::Error;
use crate::number::Number;
use crate::parser::BinOp;
use crate::units;
use crate::value::{Kind, Value};

fn int(n: i64) -> Number {
    Number::from_i64(n)
}

/// 365.2425 days, the reference's year, in seconds.
fn year_in_seconds() -> Number {
    Number::from_decimal_str("31556952").expect("a plain integer")
}

/// A length of time in years. A plain number counts as years.
pub fn years(v: &Value) -> Result<Number, Error> {
    if v.kind() != Kind::Plain {
        return Err(Error::UnsupportedOperation("a time span is needed here"));
    }
    if v.unit().is_empty() {
        return Ok(v.number().clone());
    }
    let seconds = units::lookup("s").expect("seconds are defined");
    if v.unit().dim() != seconds.dim() {
        return Err(Error::IncompatibleUnits);
    }
    v.number().mul(&v.unit().factor()?)?.div(&year_in_seconds())
}

/// A rate as a fraction: `7%` or `0.07`.
pub fn rate(v: &Value) -> Result<Number, Error> {
    match v.kind() {
        Kind::Percent | Kind::Plain if v.unit().is_empty() => Ok(v.number().clone()),
        _ => Err(Error::UnsupportedOperation(
            "an interest rate is needed here",
        )),
    }
}

fn scale(principal: &Value, factor: approx::Outcome) -> Result<Value, Error> {
    let f = Value::plain(factor.number);
    let f = if factor.exact { f } else { f.approximate() };
    Value::binary(BinOp::Mul, principal, &f)
}

/// What `principal` becomes after `time` at `rate`, compounded `per_year` times a year.
pub fn compound(
    principal: &Value,
    time: &Value,
    rate_value: &Value,
    per_year: i64,
) -> Result<Value, Error> {
    let (t, r) = (years(time)?, rate(rate_value)?);
    let m = int(per_year);
    let base = int(1).add(&r.div(&m)?)?;
    scale(principal, approx::pow_rational(&base, &m.mul(&t)?)?)
}

pub fn interest_on(
    principal: &Value,
    time: &Value,
    rate_value: &Value,
    per_year: i64,
) -> Result<Value, Error> {
    Value::binary(
        BinOp::Sub,
        &compound(principal, time, rate_value, per_year)?,
        principal,
    )
}

pub fn present_value(
    future: &Value,
    time: &Value,
    rate_value: &Value,
    per_year: i64,
) -> Result<Value, Error> {
    let (t, r) = (years(time)?, rate(rate_value)?);
    let m = int(per_year);
    let base = int(1).add(&r.div(&m)?)?;
    scale(future, approx::pow_rational(&base, &m.mul(&t)?.neg())?)
}

/// Returned over invested, as a multiple.
pub fn return_multiple(invested: &Value, returned: &Value) -> Result<Value, Error> {
    let ratio = Value::binary(BinOp::Div, returned, invested)?;
    let n = ratio
        .clone()
        .into_plain()
        .map_err(|_| Error::IncompatibleUnits)?;
    let m = Value::multiplier(n);
    Ok(if ratio.is_approximate() {
        m.approximate()
    } else {
        m
    })
}

/// The yearly rate that turns `invested` into `returned` in `time`.
pub fn annual_return(invested: &Value, returned: &Value, time: &Value) -> Result<Value, Error> {
    let multiple = return_multiple(invested, returned)?;
    let t = years(time)?;
    if t.is_zero() || t.is_negative() {
        return Err(Error::OutOfDomain);
    }
    let o = approx::pow_rational(multiple.number(), &int(1).div(&t)?)?;
    let v = Value::percent(o.number.sub(&int(1))?);
    Ok(if o.exact && !multiple.is_approximate() {
        v
    } else {
        v.approximate()
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Period {
    Daily,
    Monthly,
    Annual,
    Total,
}

impl Period {
    pub fn from_code(code: i64) -> Option<Period> {
        [
            Period::Daily,
            Period::Monthly,
            Period::Annual,
            Period::Total,
        ]
        .get(usize::try_from(code).ok()?.checked_sub(1)?)
        .copied()
    }
}

/// The level monthly payment that repays `principal` over `time` at `rate`, before rounding to cents.
fn monthly_payment(
    principal: &Value,
    time: &Value,
    rate_value: &Value,
) -> Result<(Value, Number), Error> {
    let (t, annual) = (years(time)?, rate(rate_value)?);
    let n = int(12).mul(&t)?;
    if n.is_zero() || n.is_negative() {
        return Err(Error::OutOfDomain);
    }
    let r = annual.div(&int(12))?;
    let payment = if r.is_zero() {
        Value::binary(BinOp::Div, principal, &Value::plain(n.clone()))?
    } else {
        let discount = approx::pow_rational(&int(1).add(&r)?, &n.neg())?;
        let denominator = int(1).sub(&discount.number)?;
        let factor = Value::plain(r.div(&denominator)?);
        let factor = if discount.exact {
            factor
        } else {
            factor.approximate()
        };
        Value::binary(BinOp::Mul, principal, &factor)?
    };
    Ok((payment, n))
}

fn per_period(total: &Value, time: &Value, period: Period) -> Result<Value, Error> {
    let t = years(time)?;
    let divisor = match period {
        Period::Total => return Ok(total.clone()),
        Period::Annual => t,
        Period::Monthly => int(12).mul(&t)?,
        Period::Daily => int(365).mul(&t)?,
    };
    Value::binary(BinOp::Div, total, &Value::plain(divisor))
}

/// `monthly repayment on $10,000 over 6 years at 6%` and the daily, annual and total forms.
pub fn repayment(
    principal: &Value,
    time: &Value,
    rate_value: &Value,
    period: Period,
) -> Result<Value, Error> {
    let (payment, n) = monthly_payment(principal, time, rate_value)?;
    let total = Value::binary(BinOp::Mul, &payment, &Value::plain(n))?;
    per_period(&total, time, period)
}

pub fn loan_interest(
    principal: &Value,
    time: &Value,
    rate_value: &Value,
    period: Period,
) -> Result<Value, Error> {
    let (payment, n) = monthly_payment(principal, time, rate_value)?;
    let total = Value::binary(BinOp::Mul, &payment, &Value::plain(n))?;
    let interest = Value::binary(BinOp::Sub, &total, principal)?;
    per_period(&interest, time, period)
}

/// `20k after 6 months at 10% per month`: growth by `rate` each `period`, over `time`.
pub fn grow_per_period(
    start: &Value,
    time: &Value,
    rate_value: &Value,
    period: &Value,
) -> Result<Value, Error> {
    let n = periods_in(time, period)?;
    let base = int(1).add(&rate(rate_value)?)?;
    scale(start, approx::pow_rational(&base, &n)?)
}

/// How many of `period` fit in `time`.
fn periods_in(time: &Value, period: &Value) -> Result<Number, Error> {
    let ratio = Value::binary(BinOp::Div, time, period)?;
    ratio.into_plain().map_err(|_| Error::IncompatibleUnits)
}

/// `time from $20k to $40k at $2k every month`: the number of steps, times the length of one step.
pub fn time_by_steps(
    from: &Value,
    to: &Value,
    step: &Value,
    every: &Value,
) -> Result<Value, Error> {
    let gap = Value::binary(BinOp::Sub, to, from)?;
    let steps = Value::binary(BinOp::Div, &gap, step)?;
    let n = steps
        .clone()
        .into_plain()
        .map_err(|_| Error::IncompatibleUnits)?;
    if n.is_negative() {
        return Err(Error::OutOfDomain);
    }
    Value::binary(BinOp::Mul, &Value::plain(n), every)
}

/// `time from 20k to 100k at 10% growth per month`: periods needed, times the length of one period.
pub fn time_by_growth(
    from: &Value,
    to: &Value,
    rate_value: &Value,
    every: &Value,
) -> Result<Value, Error> {
    let ratio = Value::binary(BinOp::Div, to, from)?
        .into_plain()
        .map_err(|_| Error::IncompatibleUnits)?;
    let base = int(1).add(&rate(rate_value)?)?;
    let o = approx::log_base(&ratio, &base)?;
    let n = if o.exact {
        Value::plain(o.number)
    } else {
        Value::plain(o.number).approximate()
    };
    Value::binary(BinOp::Mul, &n, every)
}

/// `growth per day from 10k to 15k over 1 week`: the steady rate per `period` that gets from one to the other.
pub fn growth_rate(from: &Value, to: &Value, period: &Value, time: &Value) -> Result<Value, Error> {
    let ratio = Value::binary(BinOp::Div, to, from)?
        .into_plain()
        .map_err(|_| Error::IncompatibleUnits)?;
    let n = periods_in(time, period)?;
    if n.is_zero() || n.is_negative() {
        return Err(Error::OutOfDomain);
    }
    let o = approx::pow_rational(&ratio, &int(1).div(&n)?)?;
    let v = Value::percent(o.number.sub(&int(1))?);
    Ok(if o.exact { v } else { v.approximate() })
}
