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
    /// Show a part of a pound in ounces, `13.5 lb` as `13 lb 8 oz`, as the app does.
    pub compound_units: bool,
    /// Write money with its currency code (`5.00 EUR`) rather than its symbol (`€5.00`).
    pub currency_codes: bool,
    /// Hide the trailing zeros of money (`$5` for `$5.00`, `$5.5` for `$5.50`).
    pub money_trim_zeros: bool,
    pub number_format: NumberFormat,
    /// Write a number that never ends as its repeating part in brackets, `0.(3)`, when the period fits in
    /// `MAX_REPEAT_PERIOD` digits; a longer one is written as usual.
    pub repeating: bool,
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
            compound_units: false,
            currency_codes: false,
            money_trim_zeros: false,
            number_format: NumberFormat::NorthAmerica,
            repeating: false,
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
            compound_units: false,
            currency_codes: false,
            money_trim_zeros: false,
            number_format: NumberFormat::NorthAmerica,
            repeating: false,
        }
    }

    /// The same options with repeating decimals on or off.
    pub fn with_repeating(&self, repeating: bool) -> FormatOptions {
        FormatOptions {
            repeating,
            ..self.clone()
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
            compound_units: true,
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

/// Symbols for large amounts, smallest first: `k` thousand, `M` million, `G` billion, `T` trillion.
const LARGE: [(&str, u32, u32); 4] = [("k", 3, 5), ("M", 6, 6), ("G", 9, 9), ("T", 12, 12)];

/// An amount below a millionth is written in scientific form, `1e-12`, as expected for
/// `1 ng in kg`, `0.0000001` (`1e-7`) and `0.00000000004` (`4e-11`); a millionth itself is `0.000001`.
pub fn format_amount(n: &Number, opts: &FormatOptions) -> Formatted {
    if !n.is_zero()
        && n.abs()
            .mul(&power_of_ten(6))
            .is_ok_and(|scaled| scaled < Number::from_i64(1))
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

/// A number in scientific form, `1.05e4`, as `in sci` shows it; zero stays 0.
pub fn format_scientific(n: &Number, opts: &FormatOptions) -> Formatted {
    if n.is_zero() {
        return Formatted {
            text: "0".to_string(),
            exact: true,
        };
    }
    scientific(n, opts)
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

/// The longest repeating part, in digits, that is written in full.
pub const MAX_REPEAT_PERIOD: usize = 50;
/// Digits tried before the repeating part must have started and ended: a fraction that has not repeated by then
/// is written as usual.
const MAX_REPEAT_SEARCH: usize = 150;

/// `0.1(6)` for 1/6: the digits of a fraction whose decimal expansion never ends, with the part that repeats in
/// brackets. None when the expansion ends (the ordinary path writes it) or the repeating part is longer than
/// `MAX_REPEAT_PERIOD`.
fn format_repeating(n: &Number, opts: &FormatOptions) -> Option<Formatted> {
    let denom = n.denom();
    let (whole, mut rem) = n.numer().abs().div_rem(denom);
    let mut digits: Vec<u8> = Vec::new();
    let mut seen: std::collections::HashMap<BigInt, usize> = std::collections::HashMap::new();
    let start = loop {
        if rem.is_zero() || digits.len() >= MAX_REPEAT_SEARCH {
            return None;
        }
        if let Some(&at) = seen.get(&rem) {
            break at;
        }
        seen.insert(rem.clone(), digits.len());
        let ten_times: BigInt = &rem * 10;
        let (digit, next) = ten_times.div_rem(denom);
        digits.push(u8::try_from(digit).expect("a decimal digit"));
        rem = next;
    };
    if digits.len() - start > MAX_REPEAT_PERIOD {
        return None;
    }
    let digit_text = |part: &[u8]| {
        part.iter()
            .map(|d| char::from(b'0' + d))
            .collect::<String>()
    };
    let mut text = String::new();
    if n.is_negative() {
        text.push('-');
    }
    if opts.group_thousands {
        push_grouped(
            &mut text,
            &whole.to_string(),
            opts.number_format.group_mark(),
        );
    } else {
        text.push_str(&whole.to_string());
    }
    text.push(opts.number_format.decimal_mark());
    text.push_str(&digit_text(&digits[..start]));
    text.push('(');
    text.push_str(&digit_text(&digits[start..]));
    text.push(')');
    Some(Formatted { text, exact: true })
}

fn format_plain(n: &Number, opts: &FormatOptions) -> Formatted {
    if opts.repeating
        && let Some(f) = format_repeating(n, opts)
    {
        return f;
    }
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
