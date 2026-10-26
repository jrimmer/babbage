use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{Signed, Zero};

use crate::error::Error;
use crate::number::Number;

pub const MAX_DECIMAL_PLACES: u32 = 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundingMode {
    HalfAwayFromZero,
    HalfEven,
    TowardZero,
    AwayFromZero,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatOptions {
    decimal_places: u32,
    pub rounding: RoundingMode,
    pub group_thousands: bool,
    pub trim_trailing_zeros: bool,
}

impl FormatOptions {
    pub fn new(decimal_places: u32) -> Result<Self, Error> {
        if decimal_places > MAX_DECIMAL_PLACES {
            return Err(Error::InvalidPrecision);
        }
        Ok(FormatOptions {
            decimal_places,
            rounding: RoundingMode::HalfAwayFromZero,
            group_thousands: true,
            trim_trailing_zeros: true,
        })
    }

    pub(crate) fn exact(places: u32) -> Self {
        FormatOptions {
            decimal_places: places,
            rounding: RoundingMode::TowardZero,
            group_thousands: false,
            trim_trailing_zeros: false,
        }
    }

    pub fn decimal_places(&self) -> u32 {
        self.decimal_places
    }
}

impl Default for FormatOptions {
    fn default() -> Self {
        FormatOptions::new(10).expect("10 is within range")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Formatted {
    pub text: String,
    /// False when `text` differs from the true value because of rounding.
    pub exact: bool,
}

pub fn format_decimal(n: &Number, opts: &FormatOptions) -> Formatted {
    let negative = n.is_negative();
    let numer = n.numer().abs();
    let denom = n.denom();
    let scaled = numer * BigInt::from(10).pow(opts.decimal_places);
    let (mut quot, rem) = scaled.div_rem(denom);
    let exact = rem.is_zero();
    if !exact {
        let twice: BigInt = &rem * 2;
        let half = twice.cmp(denom);
        let round_up = match opts.rounding {
            RoundingMode::TowardZero => false,
            RoundingMode::AwayFromZero => true,
            RoundingMode::HalfAwayFromZero => half.is_ge(),
            RoundingMode::HalfEven => match half {
                std::cmp::Ordering::Greater => true,
                std::cmp::Ordering::Equal => quot.is_odd(),
                std::cmp::Ordering::Less => false,
            },
        };
        if round_up {
            quot += 1;
        }
    }

    let places = opts.decimal_places as usize;
    let mut digits = quot.to_string();
    if digits.len() <= places {
        digits = format!("{}{}", "0".repeat(places + 1 - digits.len()), digits);
    }
    let (int_part, frac_part) = digits.split_at(digits.len() - places);
    let frac_part = if opts.trim_trailing_zeros {
        frac_part.trim_end_matches('0')
    } else {
        frac_part
    };

    let mut text = String::new();
    if negative && !quot.is_zero() {
        text.push('-');
    }
    if opts.group_thousands {
        push_grouped(&mut text, int_part);
    } else {
        text.push_str(int_part);
    }
    if !frac_part.is_empty() {
        text.push('.');
        text.push_str(frac_part);
    }
    Formatted { text, exact }
}

fn push_grouped(out: &mut String, int_part: &str) {
    let len = int_part.len();
    for (i, ch) in int_part.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
}
