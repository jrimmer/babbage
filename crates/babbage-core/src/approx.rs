//! Irrational results: roots, logarithms, trigonometry and constants.
//!
//! A result that is exactly rational is returned exactly (`sqrt(16)` is 4, `sin(30°)` is 1/2, `log10(1000)` is 3).
//! Anything else is computed to about 115 significant digits with `astro-float`, rounded correctly, and returned
//! as a close rational that the caller must mark approximate.

use std::cell::RefCell;

use astro_float::{BigFloat, Consts, Radix, RoundingMode};
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

use crate::error::Error;
use crate::number::Number;

const PRECISION_BITS: usize = 384;
const ROUNDING: RoundingMode = RoundingMode::ToEven;
/// Past this size an exponential or power is certain to exceed the engine's number size limit.
const EXPONENT_LIMIT: i64 = 400_000;

/// A result and whether it is exactly the true value.
pub struct Outcome {
    pub number: Number,
    pub exact: bool,
}

fn exact(number: Number) -> Result<Outcome, Error> {
    Ok(Outcome {
        number,
        exact: true,
    })
}

fn int(n: i64) -> Number {
    Number::from_i64(n)
}

fn ratio(n: i64, d: i64) -> Number {
    Number::from_ratio(BigInt::from(n), BigInt::from(d)).expect("nonzero denominator")
}

thread_local! {
    static CONSTANTS: RefCell<Consts> = RefCell::new(Consts::new().expect("constants cache"));
}

fn with<T>(f: impl FnOnce(&mut Consts) -> T) -> T {
    CONSTANTS.with(|c| f(&mut c.borrow_mut()))
}

fn to_float(n: &Number, cc: &mut Consts) -> BigFloat {
    let numer = BigFloat::parse(
        &n.numer().to_string(),
        Radix::Dec,
        PRECISION_BITS,
        ROUNDING,
        cc,
    );
    if n.denom().is_one() {
        return numer;
    }
    let denom = BigFloat::parse(
        &n.denom().to_string(),
        Radix::Dec,
        PRECISION_BITS,
        ROUNDING,
        cc,
    );
    numer.div(&denom, PRECISION_BITS, ROUNDING)
}

fn from_float(x: &BigFloat, cc: &mut Consts) -> Result<Number, Error> {
    if x.is_nan() || x.is_inf() {
        return Err(Error::OutOfDomain);
    }
    if x.is_zero() {
        return Ok(int(0));
    }
    let text = x
        .format(Radix::Dec, ROUNDING, cc)
        .map_err(|_| Error::Overflow)?;
    Number::from_decimal_str(&text)
}

fn approximate(f: impl FnOnce(&mut Consts) -> BigFloat) -> Result<Outcome, Error> {
    with(|cc| {
        let x = f(cc);
        Ok(Outcome {
            number: from_float(&x, cc)?,
            exact: false,
        })
    })
}

pub fn pi() -> Number {
    approximate(|cc| cc.pi(PRECISION_BITS, ROUNDING))
        .expect("pi converts")
        .number
}

pub fn tau() -> Number {
    pi().mul(&int(2)).expect("small")
}

pub fn phi() -> Number {
    approximate(|cc| {
        let five = BigFloat::from_word(5, PRECISION_BITS);
        let root = five.sqrt(PRECISION_BITS, ROUNDING);
        let one = BigFloat::from_word(1, PRECISION_BITS);
        let two = BigFloat::from_word(2, PRECISION_BITS);
        let _ = cc;
        root.add(&one, PRECISION_BITS, ROUNDING)
            .div(&two, PRECISION_BITS, ROUNDING)
    })
    .expect("phi converts")
    .number
}

/// Degrees per radian, 180 / pi.
pub fn degrees_per_radian() -> Number {
    Number::from_i64(180).div(&pi()).expect("pi is not zero")
}

fn root_of_integer(n: &BigInt, k: u32) -> Option<BigInt> {
    let r = n.nth_root(k);
    (r.pow(k) == *n).then_some(r)
}

/// The k-th root of a rational when it is rational. Odd roots of negatives are allowed.
fn exact_root(x: &Number, k: u32) -> Option<Number> {
    if x.is_negative() && k.is_multiple_of(2) {
        return None;
    }
    let sign = if x.is_negative() {
        BigInt::from(-1)
    } else {
        BigInt::one()
    };
    let n = root_of_integer(&x.numer().abs(), k)?;
    let d = root_of_integer(x.denom(), k)?;
    Number::from_ratio(sign * n, d).ok()
}

pub fn root(x: &Number, k: u32) -> Result<Outcome, Error> {
    if k == 0 {
        return Err(Error::OutOfDomain);
    }
    if x.is_negative() && k.is_multiple_of(2) {
        return Err(Error::OutOfDomain);
    }
    if let Some(r) = exact_root(x, k) {
        return exact(r);
    }
    approximate(|cc| {
        let magnitude = to_float(&x.abs(), cc);
        let inverse = BigFloat::from_word(1, PRECISION_BITS).div(
            &BigFloat::from_word(u64::from(k), PRECISION_BITS),
            PRECISION_BITS,
            ROUNDING,
        );
        let r = magnitude.pow(&inverse, PRECISION_BITS, ROUNDING, cc);
        if x.is_negative() { r.neg() } else { r }
    })
}

/// `base` to the power of a rational `exp`: exact when the answer is rational, otherwise approximate.
pub fn pow_rational(base: &Number, exp: &Number) -> Result<Outcome, Error> {
    if exp.is_integer() {
        return exact(base.pow(exp)?);
    }
    if base.is_negative() {
        return Err(Error::OutOfDomain);
    }
    if base.is_zero() {
        return if exp.is_negative() {
            Err(Error::DivisionByZero)
        } else {
            exact(int(0))
        };
    }
    if let Some(q) = u32::try_from(exp.denom().clone())
        .ok()
        .filter(|q| *q <= 1000)
        && let Some(r) = exact_root(base, q)
    {
        return exact(r.pow(&Number::from_ratio(exp.numer().clone(), BigInt::one())?)?);
    }
    if exp.abs() > int(EXPONENT_LIMIT) {
        return Err(Error::Overflow);
    }
    approximate(|cc| {
        let b = to_float(base, cc);
        let e = to_float(exp, cc);
        b.pow(&e, PRECISION_BITS, ROUNDING, cc)
    })
}

pub fn sqrt(x: &Number) -> Result<Outcome, Error> {
    if x.is_negative() {
        return Err(Error::OutOfDomain);
    }
    if let Some(r) = exact_root(x, 2) {
        return exact(r);
    }
    approximate(|cc| to_float(x, cc).sqrt(PRECISION_BITS, ROUNDING))
}

fn require_positive(x: &Number) -> Result<(), Error> {
    if x.is_negative() || x.is_zero() {
        Err(Error::OutOfDomain)
    } else {
        Ok(())
    }
}

pub fn ln(x: &Number) -> Result<Outcome, Error> {
    require_positive(x)?;
    if *x == int(1) {
        return exact(int(0));
    }
    approximate(|cc| to_float(x, cc).ln(PRECISION_BITS, ROUNDING, cc))
}

/// The integer k with base^k equal to x, for a whole-number base of at least 2.
fn exact_log(x: &Number, base: &BigInt) -> Option<i64> {
    let (mut n, mut d) = (x.numer().clone(), x.denom().clone());
    let mut k = 0i64;
    if d.is_one() {
        while (&n % base).is_zero() && !n.is_one() {
            n /= base;
            k += 1;
        }
        n.is_one().then_some(k)
    } else if n.is_one() {
        while (&d % base).is_zero() && !d.is_one() {
            d /= base;
            k -= 1;
        }
        d.is_one().then_some(k)
    } else {
        None
    }
}

pub fn log_base(x: &Number, base: &Number) -> Result<Outcome, Error> {
    require_positive(x)?;
    require_positive(base)?;
    if *base == int(1) {
        return Err(Error::OutOfDomain);
    }
    if base.is_integer()
        && let Some(k) = exact_log(x, base.numer())
    {
        return exact(int(k));
    }
    approximate(|cc| to_float(x, cc).log(&to_float(base, cc), PRECISION_BITS, ROUNDING, cc))
}

pub fn exp(x: &Number) -> Result<Outcome, Error> {
    if x.is_zero() {
        return exact(int(1));
    }
    if x.abs() > int(EXPONENT_LIMIT) {
        return Err(Error::Overflow);
    }
    approximate(|cc| to_float(x, cc).exp(PRECISION_BITS, ROUNDING, cc))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trig {
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    Asinh,
    Acosh,
    Atanh,
}

/// Sine, cosine or tangent of an angle given in degrees, exact at the multiples of 30 and 45 that are rational.
pub fn trig_degrees(f: Trig, degrees: &Number) -> Result<Outcome, Error> {
    if degrees.is_integer() {
        let m = degrees.numer().mod_floor(&BigInt::from(360));
        let m = i64::try_from(m).expect("below 360");
        let hit = match f {
            Trig::Sin => match m {
                0 | 180 => Some(int(0)),
                30 | 150 => Some(ratio(1, 2)),
                90 => Some(int(1)),
                210 | 330 => Some(ratio(-1, 2)),
                270 => Some(int(-1)),
                _ => None,
            },
            Trig::Cos => match m {
                0 => Some(int(1)),
                60 | 300 => Some(ratio(1, 2)),
                90 | 270 => Some(int(0)),
                120 | 240 => Some(ratio(-1, 2)),
                180 => Some(int(-1)),
                _ => None,
            },
            Trig::Tan => match m {
                0 | 180 => Some(int(0)),
                45 | 225 => Some(int(1)),
                135 | 315 => Some(int(-1)),
                90 | 270 => return Err(Error::OutOfDomain),
                _ => None,
            },
            _ => None,
        };
        if let Some(n) = hit {
            return exact(n);
        }
    }
    let radians = degrees.mul(&pi())?.div(&int(180))?;
    trig_radians(f, &radians)
}

/// The angle, in degrees, whose sine, cosine or tangent is `x`. Exact where the angle is a whole number of degrees.
pub fn inverse_trig_degrees(f: Trig, x: &Number) -> Result<Outcome, Error> {
    let half = ratio(1, 2);
    let hit = match f {
        Trig::Asin => {
            if x.abs() > int(1) {
                return Err(Error::OutOfDomain);
            }
            match () {
                _ if x.is_zero() => Some(int(0)),
                _ if *x == half => Some(int(30)),
                _ if *x == half.neg() => Some(int(-30)),
                _ if *x == int(1) => Some(int(90)),
                _ if *x == int(-1) => Some(int(-90)),
                _ => None,
            }
        }
        Trig::Acos => {
            if x.abs() > int(1) {
                return Err(Error::OutOfDomain);
            }
            match () {
                _ if *x == int(1) => Some(int(0)),
                _ if *x == half => Some(int(60)),
                _ if x.is_zero() => Some(int(90)),
                _ if *x == half.neg() => Some(int(120)),
                _ if *x == int(-1) => Some(int(180)),
                _ => None,
            }
        }
        Trig::Atan => match () {
            _ if x.is_zero() => Some(int(0)),
            _ if *x == int(1) => Some(int(45)),
            _ if *x == int(-1) => Some(int(-45)),
            _ => None,
        },
        _ => return Err(Error::OutOfDomain),
    };
    if let Some(n) = hit {
        return exact(n);
    }
    let radians = trig_radians(f, x)?;
    Ok(Outcome {
        number: radians.number.mul(&degrees_per_radian())?,
        exact: false,
    })
}

/// A trigonometric or hyperbolic function of a plain number, which means radians.
pub fn trig_radians(f: Trig, x: &Number) -> Result<Outcome, Error> {
    let zero_gives = |at_zero: i64| {
        if x.is_zero() {
            Some(int(at_zero))
        } else {
            None
        }
    };
    let known = match f {
        Trig::Sin
        | Trig::Tan
        | Trig::Asin
        | Trig::Atan
        | Trig::Sinh
        | Trig::Tanh
        | Trig::Asinh
        | Trig::Atanh => zero_gives(0),
        Trig::Cos | Trig::Cosh => zero_gives(1),
        Trig::Acos | Trig::Acosh => (*x == int(1)).then(|| int(0)),
    };
    let in_unit = x.abs() <= int(1);
    match f {
        Trig::Asin | Trig::Acos | Trig::Atanh if !in_unit => return Err(Error::OutOfDomain),
        Trig::Atanh if x.abs() == int(1) => return Err(Error::OutOfDomain),
        Trig::Acosh if *x < int(1) => return Err(Error::OutOfDomain),
        Trig::Sinh | Trig::Cosh | Trig::Tanh if x.abs() > int(EXPONENT_LIMIT) => {
            return Err(Error::Overflow);
        }
        _ => {}
    }
    if let Some(n) = known {
        return exact(n);
    }
    approximate(|cc| {
        let v = to_float(x, cc);
        match f {
            Trig::Sin => v.sin(PRECISION_BITS, ROUNDING, cc),
            Trig::Cos => v.cos(PRECISION_BITS, ROUNDING, cc),
            Trig::Tan => v.tan(PRECISION_BITS, ROUNDING, cc),
            Trig::Asin => v.asin(PRECISION_BITS, ROUNDING, cc),
            Trig::Acos => v.acos(PRECISION_BITS, ROUNDING, cc),
            Trig::Atan => v.atan(PRECISION_BITS, ROUNDING, cc),
            Trig::Sinh => v.sinh(PRECISION_BITS, ROUNDING, cc),
            Trig::Cosh => v.cosh(PRECISION_BITS, ROUNDING, cc),
            Trig::Tanh => v.tanh(PRECISION_BITS, ROUNDING, cc),
            Trig::Asinh => v.asinh(PRECISION_BITS, ROUNDING, cc),
            Trig::Acosh => v.acosh(PRECISION_BITS, ROUNDING, cc),
            Trig::Atanh => v.atanh(PRECISION_BITS, ROUNDING, cc),
        }
    })
}
