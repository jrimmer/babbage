use num_bigint::BigInt;
use num_integer::Integer;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

use crate::error::Error;

pub const MAX_BITS: u64 = 1 << 20;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Number(BigRational);

/// A base needing `base_bits` bits has |numer| >= 2^(bits-1), so its power
/// needs at least (bits-1)*magnitude bits. Refusing early avoids computing
/// a result that `Number::new` would reject anyway.
fn pow_exceeds_cap(base_bits: u64, magnitude: u32) -> bool {
    (base_bits - 1) * u64::from(magnitude) > MAX_BITS
}

impl Number {
    pub fn new(r: BigRational) -> Result<Self, Error> {
        if r.numer().bits() + r.denom().bits() > MAX_BITS {
            return Err(Error::Overflow);
        }
        Ok(Number(r))
    }

    pub fn from_i64(n: i64) -> Self {
        Number(BigRational::from_integer(BigInt::from(n)))
    }

    pub fn from_ratio(numer: BigInt, denom: BigInt) -> Result<Self, Error> {
        if denom.is_zero() {
            return Err(Error::DivisionByZero);
        }
        Self::new(BigRational::new(numer, denom))
    }

    pub fn numer(&self) -> &BigInt {
        self.0.numer()
    }

    pub fn denom(&self) -> &BigInt {
        self.0.denom()
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    pub fn is_negative(&self) -> bool {
        self.0.is_negative()
    }

    pub fn is_integer(&self) -> bool {
        self.0.is_integer()
    }

    pub fn neg(&self) -> Number {
        Number(-&self.0)
    }

    pub fn abs(&self) -> Number {
        Number(self.0.abs())
    }

    pub fn add(&self, rhs: &Number) -> Result<Number, Error> {
        Number::new(&self.0 + &rhs.0)
    }

    pub fn sub(&self, rhs: &Number) -> Result<Number, Error> {
        Number::new(&self.0 - &rhs.0)
    }

    pub fn mul(&self, rhs: &Number) -> Result<Number, Error> {
        Number::new(&self.0 * &rhs.0)
    }

    pub fn div(&self, rhs: &Number) -> Result<Number, Error> {
        if rhs.is_zero() {
            return Err(Error::DivisionByZero);
        }
        Number::new(&self.0 / &rhs.0)
    }

    pub fn pow(&self, exp: &Number) -> Result<Number, Error> {
        if !exp.is_integer() {
            return Err(Error::NonIntegerExponent);
        }
        let e = exp.0.numer();
        let one = BigInt::one();
        if self.is_zero() {
            return match e.sign() {
                num_bigint::Sign::NoSign => Err(Error::ZeroToTheZero),
                num_bigint::Sign::Minus => Err(Error::DivisionByZero),
                num_bigint::Sign::Plus => Ok(Number(BigRational::zero())),
            };
        }
        if *self.denom() == one && self.numer().abs() == one {
            let odd = e.is_odd();
            return Ok(if self.is_negative() && odd { self.clone() } else { Number(BigRational::one()) });
        }
        let magnitude = e.abs().to_u32().filter(|m| *m <= i32::MAX as u32).ok_or(Error::Overflow)?;
        let base_bits = self.numer().bits().max(self.denom().bits());
        if pow_exceeds_cap(base_bits, magnitude) {
            return Err(Error::Overflow);
        }
        let positive = Number::new(self.0.pow(magnitude as i32))?;
        if e.is_negative() {
            Number::new(positive.0.recip())
        } else {
            Ok(positive)
        }
    }

    /// True when the decimal expansion terminates (denominator is 2^a * 5^b).
    pub fn is_terminating(&self) -> bool {
        self.terminating_places().is_some()
    }

    /// Number of decimal places in the exact expansion, if it terminates.
    pub fn terminating_places(&self) -> Option<u32> {
        let mut d = self.0.denom().clone();
        let two = BigInt::from(2);
        let five = BigInt::from(5);
        let (mut twos, mut fives) = (0u32, 0u32);
        while d.is_even() {
            d /= &two;
            twos += 1;
        }
        loop {
            let (q, r) = d.div_rem(&five);
            if !r.is_zero() {
                break;
            }
            d = q;
            fives += 1;
        }
        d.is_one().then_some(twos.max(fives))
    }

    /// Exact decimal text, e.g. "-0.125". None when the expansion repeats.
    pub fn to_exact_decimal(&self) -> Option<String> {
        let places = self.terminating_places()?;
        let opts = crate::format::FormatOptions::exact(places);
        Some(crate::format::format_decimal(self, &opts).text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pow_cap_boundary_is_exact() {
        let cap = MAX_BITS as u32;
        assert!(!pow_exceeds_cap(2, cap));
        assert!(pow_exceeds_cap(2, cap + 1));
        assert!(!pow_exceeds_cap(3, cap / 2));
        assert!(pow_exceeds_cap(3, cap / 2 + 1));
        assert!(!pow_exceeds_cap(1025, cap / 1024));
        assert!(pow_exceeds_cap(1025, cap / 1024 + 1));
        assert!(!pow_exceeds_cap(2, 0));
        assert!(!pow_exceeds_cap(1, u32::MAX));
        assert!(pow_exceeds_cap(1 << 20, 2));
    }
}
