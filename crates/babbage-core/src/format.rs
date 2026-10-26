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

/// How numbers are written: which mark groups thousands and which marks the decimal point.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NumberFormat {
    /// `1,234.56`
    #[default]
    NorthAmerica,
    /// `1.234,56`
    WesternEurope,
    /// `1 234,56`, where either `,` or `.` is read as the decimal point.
    EasternEurope,
}

impl NumberFormat {
    /// The mark written between groups of three digits.
    pub fn group_mark(self) -> char {
        match self {
            NumberFormat::NorthAmerica => ',',
            NumberFormat::WesternEurope => '.',
            NumberFormat::EasternEurope => '\u{a0}',
        }
    }

    /// The mark written before the fraction.
    pub fn decimal_mark(self) -> char {
        match self {
            NumberFormat::NorthAmerica => '.',
            NumberFormat::WesternEurope | NumberFormat::EasternEurope => ',',
        }
    }

    /// True for a character that separates groups of three digits when typed.
    pub fn reads_as_group(self, c: char) -> bool {
        match self {
            NumberFormat::NorthAmerica => c == ',',
            NumberFormat::WesternEurope => c == '.',
            NumberFormat::EasternEurope => matches!(c, ' ' | '\u{a0}' | '\u{202f}'),
        }
    }

    /// True for a character that starts the fraction when typed.
    pub fn reads_as_decimal(self, c: char) -> bool {
        match self {
            NumberFormat::NorthAmerica => c == '.',
            NumberFormat::WesternEurope => c == ',',
            NumberFormat::EasternEurope => c == ',' || c == '.',
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatOptions {
    decimal_places: u32,
    pub rounding: RoundingMode,
    pub group_thousands: bool,
    pub trim_trailing_zeros: bool,
    /// Show 300,000 as `300k` and 16,777,216 as `16.777216M`. Only the decimal point moves, so no digit is lost.
    pub abbreviate_large: bool,
    /// Show money with its currency's fixed digits (`$5.00`). Off shows every digit, as for a declared variable.
    pub fixed_money_digits: bool,
    pub number_format: NumberFormat,
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
            abbreviate_large: false,
            fixed_money_digits: true,
            number_format: NumberFormat::NorthAmerica,
        })
    }

    pub(crate) fn exact(places: u32) -> Self {
        FormatOptions {
            decimal_places: places,
            rounding: RoundingMode::TowardZero,
            group_thousands: false,
            trim_trailing_zeros: false,
            abbreviate_large: false,
            fixed_money_digits: true,
            number_format: NumberFormat::NorthAmerica,
        }
    }

    /// The same options written in another number format.
    pub fn in_format(&self, number_format: NumberFormat) -> FormatOptions {
        FormatOptions {
            number_format,
            ..self.clone()
        }
    }

    /// The same options without large-number abbreviation.
    pub fn unabbreviated(&self) -> FormatOptions {
        FormatOptions {
            abbreviate_large: false,
            ..self.clone()
        }
    }

    /// The same options with no thousands separators and no abbreviation: every digit, as in a timestamp.
    pub fn every_digit(&self) -> FormatOptions {
        FormatOptions {
            group_thousands: false,
            abbreviate_large: false,
            ..self.clone()
        }
    }

    /// The same options with a different number of places, which also stops abbreviation.
    pub fn with_places(&self, places: u32) -> Result<FormatOptions, Error> {
        Ok(FormatOptions {
            decimal_places: FormatOptions::new(places)?.decimal_places,
            ..self.unabbreviated()
        })
    }

    pub fn decimal_places(&self) -> u32 {
        self.decimal_places
    }
}

/// What the app shows: 10 places, grouped, trimmed, large numbers abbreviated.
impl Default for FormatOptions {
    fn default() -> Self {
        FormatOptions {
            abbreviate_large: true,
            ..FormatOptions::new(10).expect("10 is within range")
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Formatted {
    pub text: String,
    /// False when `text` differs from the true value because of rounding.
    pub exact: bool,
}

/// Symbols from the reference, smallest first: `k` thousand, `M` million, `G` billion, `T` trillion.
const LARGE: [(&str, u32, u32); 4] = [("k", 3, 5), ("M", 6, 6), ("G", 9, 9), ("T", 12, 12)];

/// An amount with a unit: one too small for ten decimal places would print as 0, so it is written in scientific
/// form instead, `1e-12`, as the reference app does for `1 ng in kg`.
pub fn format_amount(n: &Number, opts: &FormatOptions) -> Formatted {
    let half = Number::from_ratio(BigInt::from(1), BigInt::from(2)).expect("a half");
    if !n.is_zero()
        && n.abs()
            .mul(&power_of_ten(10))
            .is_ok_and(|scaled| scaled < half)
    {
        return scientific(n, opts);
    }
    format_decimal(n, opts)
}

pub fn format_decimal(n: &Number, opts: &FormatOptions) -> Formatted {
    if !opts.abbreviate_large {
        return format_plain(n, opts);
    }
    let magnitude = n.abs();
    let Some(mut step) = LARGE
        .iter()
        .rposition(|(_, _, floor)| magnitude >= power_of_ten(*floor))
    else {
        return format_plain(n, opts);
    };
    let plain = FormatOptions {
        abbreviate_large: false,
        ..opts.clone()
    };
    loop {
        let (symbol, shift, _) = LARGE[step];
        let scaled = n
            .div(&power_of_ten(shift))
            .expect("ten to a power is never zero");
        let mut f = format_plain(&scaled, &plain);
        let integer_digits = f
            .text
            .trim_start_matches('-')
            .split(opts.number_format.decimal_mark())
            .next()
            .unwrap_or("")
            .replace(opts.number_format.group_mark(), "")
            .len();
        if integer_digits > 3 && step + 1 < LARGE.len() {
            step += 1;
            continue;
        }
        f.text.push_str(symbol);
        return f;
    }
}

/// `1e-12`, `-2.5e-11`: ten significant digits at most, the mantissa between 1 and 10.
fn scientific(n: &Number, opts: &FormatOptions) -> Formatted {
    let ten = |e: i64| {
        let p = BigInt::from(10).pow(e.unsigned_abs() as u32);
        if e >= 0 {
            Number::from_ratio(p, BigInt::from(1))
        } else {
            Number::from_ratio(BigInt::from(1), p)
        }
        .expect("a power of ten")
    };
    let x = n.abs();
    let mut k = x.numer().to_string().len() as i64 - x.denom().to_string().len() as i64;
    let mantissa = loop {
        let m = x.div(&ten(k)).expect("ten to a power is never zero");
        if m < ten(0) {
            k -= 1;
        } else if m >= ten(1) {
            k += 1;
        } else {
            break m;
        }
    };
    let digits = 10;
    let mut scaled = mantissa
        .mul(&ten(digits))
        .and_then(|m| m.add(&Number::from_ratio(BigInt::from(1), BigInt::from(2)).expect("a half")))
        .map(|m| m.floor())
        .expect("small");
    if scaled >= BigInt::from(10).pow(digits as u32 + 1) {
        scaled /= BigInt::from(10);
        k += 1;
    }
    let text = scaled.to_string();
    let rest = text[1..].trim_end_matches('0');
    let mantissa_text = if rest.is_empty() {
        text[..1].to_string()
    } else {
        format!("{}{}{rest}", &text[..1], opts.number_format.decimal_mark())
    };
    Formatted {
        text: format!(
            "{}{mantissa_text}e{k}",
            if n.is_negative() { "-" } else { "" }
        ),
        exact: false,
    }
}

fn power_of_ten(exp: u32) -> Number {
    Number::new(num_rational::BigRational::from_integer(
        BigInt::from(10).pow(exp),
    ))
    .expect("small power of ten")
}

fn format_plain(n: &Number, opts: &FormatOptions) -> Formatted {
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
        push_grouped(&mut text, int_part, opts.number_format.group_mark());
    } else {
        text.push_str(int_part);
    }
    if !frac_part.is_empty() {
        text.push(opts.number_format.decimal_mark());
        text.push_str(frac_part);
    }
    Formatted { text, exact }
}

fn push_grouped(out: &mut String, int_part: &str, mark: char) {
    let len = int_part.len();
    for (i, ch) in int_part.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(mark);
        }
        out.push(ch);
    }
}
