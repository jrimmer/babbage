//! Named functions: `round(x)`, `max(a, b, c)`, `gcd of 20 and 30` and friends, on exact values.

use std::cmp::Ordering;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive};

use crate::approx::{self, Trig};
use crate::error::Error;
use crate::finance;
use crate::number::Number;
use crate::parser::BinOp;
use crate::units;
use crate::value::{Kind, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Func {
    Abs,
    Fact,
    Round,
    Floor,
    Ceil,
    Min,
    Max,
    Sum,
    Average,
    Median,
    Count,
    Gcd,
    Lcm,
    Clamp,
    Permutations,
    Combinations,
    Half,
    Midpoint,
    Random,
    /// `3 permutations of 10`: the same as `Permutations` with its arguments swapped.
    PermutationsOf,
    CombinationsOf,
    AsNumber,
    AsDms,
    AsPitch,
    ScreenPpi,
    InvestmentRequired,
    Hex,
    Bin,
    Oct,
    /// `... as base 8`: the value, then the base.
    ToBase,
    Sqrt,
    Cbrt,
    /// `root 5 of 100`: the degree, then the value.
    Root,
    IsPrime,
    Ln,
    Log2,
    Log10,
    /// `log 20 base 4`: the value, then the base.
    LogBase,
    Exp,
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
    /// The same functions with the angle in degrees (`sind`, `asind`).
    SinD,
    CosD,
    TanD,
    AsinD,
    AcosD,
    AtanD,
    /// Cotangent and cosecant, with their hyperbolic, inverse and degree variants.
    Cot,
    Csc,
    Coth,
    Csch,
    Acot,
    Acsc,
    CotD,
    CscD,
    AcotD,
    AcscD,
    /// Sample standard deviation.
    StdDev,
    /// (principal, time, rate, compounding periods per year)
    Compound,
    InterestOn,
    InterestEach,
    PresentValue,
    /// (invested, returned)
    ReturnMultiple,
    /// (invested, returned, time)
    AnnualReturn,
    /// (principal, time, rate, period code: 1 daily, 2 monthly, 3 annual, 4 total)
    Repayment,
    LoanInterest,
    /// (start, time, rate, period)
    GrowPerPeriod,
    /// (from, to, step, every)
    TimeBySteps,
    /// (from, to, rate, every)
    TimeByGrowth,
    /// (from, to, period, time)
    GrowthRate,
    /// `30 hours at $30/hour` and `$500 at $20/hour`: multiplies or divides, by the units.
    AtRate,
    /// `20% is 500, what is 750`: (a, b, c) gives a x c / b, in the kind of a.
    Proportion,
    /// `5% is 1 in what`: (share, top) shows as `top/(top / share)`.
    InWhat,
    /// `1 cm in px at 326 ppi`: (value, pixels per inch, 1 of the target unit).
    AtDensity,
    Date(crate::datefns::DateFn),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arity {
    Exactly(usize),
    AtLeast(usize),
}

impl Arity {
    fn accepts(self, n: usize) -> bool {
        match self {
            Arity::Exactly(k) => n == k,
            Arity::AtLeast(k) => n >= k,
        }
    }
}

impl Func {
    pub fn arity(self) -> Arity {
        match self {
            Func::Abs
            | Func::Fact
            | Func::Round
            | Func::Floor
            | Func::Ceil
            | Func::Half
            | Func::IsPrime
            | Func::AsNumber
            | Func::AsDms
            | Func::AsPitch => Arity::Exactly(1),
            Func::Min
            | Func::Max
            | Func::Sum
            | Func::Average
            | Func::Median
            | Func::Count
            | Func::Gcd
            | Func::Lcm => Arity::AtLeast(1),
            Func::Permutations
            | Func::Combinations
            | Func::PermutationsOf
            | Func::CombinationsOf
            | Func::Midpoint
            | Func::Random => Arity::Exactly(2),
            Func::Clamp => Arity::Exactly(3),
            Func::Root
            | Func::LogBase
            | Func::ToBase
            | Func::ReturnMultiple
            | Func::InvestmentRequired
            | Func::AtRate => Arity::Exactly(2),
            Func::AnnualReturn | Func::Proportion | Func::AtDensity | Func::ScreenPpi => {
                Arity::Exactly(3)
            }
            Func::InWhat => Arity::Exactly(2),
            Func::Date(f) => Arity::Exactly(f.arity()),
            Func::Compound
            | Func::InterestOn
            | Func::PresentValue
            | Func::Repayment
            | Func::LoanInterest
            | Func::GrowPerPeriod
            | Func::TimeBySteps
            | Func::TimeByGrowth
            | Func::GrowthRate => Arity::Exactly(4),
            Func::InterestEach => Arity::Exactly(5),
            Func::StdDev => Arity::AtLeast(2),
            _ => Arity::Exactly(1),
        }
    }
}

/// Spellings of each function as written with parentheses: `name(...)`.
const NAMES: &[(&str, Func)] = &[
    ("abs", Func::Abs),
    ("fact", Func::Fact),
    ("round", Func::Round),
    ("floor", Func::Floor),
    ("ceil", Func::Ceil),
    ("ceiling", Func::Ceil),
    ("min", Func::Min),
    ("max", Func::Max),
    ("sum", Func::Sum),
    ("total", Func::Sum),
    ("average", Func::Average),
    ("avg", Func::Average),
    ("mean", Func::Average),
    ("median", Func::Median),
    ("count", Func::Count),
    ("gcd", Func::Gcd),
    ("lcm", Func::Lcm),
    ("clamp", Func::Clamp),
    ("midpoint", Func::Midpoint),
    ("hex", Func::Hex),
    ("bin", Func::Bin),
    ("oct", Func::Oct),
    ("int", Func::AsNumber),
    ("sqrt", Func::Sqrt),
    ("cbrt", Func::Cbrt),
    ("ln", Func::Ln),
    ("log2", Func::Log2),
    ("log10", Func::Log10),
    ("log", Func::Log10),
    ("exp", Func::Exp),
    ("sin", Func::Sin),
    ("cos", Func::Cos),
    ("tan", Func::Tan),
    ("asin", Func::Asin),
    ("acos", Func::Acos),
    ("atan", Func::Atan),
    ("sinh", Func::Sinh),
    ("cosh", Func::Cosh),
    ("tanh", Func::Tanh),
    ("asinh", Func::Asinh),
    ("acosh", Func::Acosh),
    ("atanh", Func::Atanh),
    ("sind", Func::SinD),
    ("cosd", Func::CosD),
    ("tand", Func::TanD),
    ("asind", Func::AsinD),
    ("acosd", Func::AcosD),
    ("atand", Func::AtanD),
    ("cot", Func::Cot),
    ("csc", Func::Csc),
    ("coth", Func::Coth),
    ("csch", Func::Csch),
    ("acot", Func::Acot),
    ("acsc", Func::Acsc),
    ("cotd", Func::CotD),
    ("cscd", Func::CscD),
    ("acotd", Func::AcotD),
    ("acscd", Func::AcscD),
    ("stddev", Func::StdDev),
];

pub fn lookup(name: &str) -> Option<Func> {
    NAMES
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, f)| *f)
}

/// `156.742° as DMS` is `156° 44′ 31.2″`: whole degrees, whole minutes, and the rest in seconds, to ten places at
/// most. A plain number is read as degrees.
fn as_dms(angle: &Value) -> Result<Value, Error> {
    let degrees = match angle.kind() {
        Kind::Plain if angle.unit().is_empty() => angle.number().clone(),
        Kind::Plain
            if angle.unit().dim()
                == units::lookup("deg").ok_or(Error::IncompatibleUnits)?.dim() =>
        {
            angle
                .clone()
                .convert(&units::lookup("deg").ok_or(Error::IncompatibleUnits)?)?
                .number()
                .clone()
        }
        _ => return Err(Error::IncompatibleUnits),
    };
    let ten = Number::from_i64(10).pow(&Number::from_i64(10))?;
    let sixty = Number::from_i64(60);
    let negative = degrees.is_negative();
    let total = degrees.abs();
    let whole = |n: &Number| Number::from_ratio(n.floor(), BigInt::one());
    let d = whole(&total)?;
    let rest = total.sub(&d)?.mul(&sixty)?;
    let m = whole(&rest)?;
    // Seconds to ten decimal places, rounded half up, so a result such as 59.99999999999 carries over.
    let scaled = rest
        .sub(&m)?
        .mul(&sixty)?
        .mul(&ten)?
        .add(&Number::from_ratio(BigInt::one(), BigInt::from(2))?)?;
    let mut s = whole(&scaled)?.div(&ten)?;
    let (mut d, mut m) = (d, m);
    if s >= sixty {
        s = s.sub(&sixty)?;
        m = m.add(&Number::from_i64(1))?;
    }
    if m >= sixty {
        m = m.sub(&sixty)?;
        d = d.add(&Number::from_i64(1))?;
    }
    let seconds = s.to_exact_decimal().unwrap_or_else(|| "0".to_string());
    let plain = |n: &Number| n.to_exact_decimal().unwrap_or_default();
    Ok(Value::text(&format!(
        "{}{}° {}′ {}″",
        if negative { "-" } else { "" },
        plain(&d),
        plain(&m),
        seconds
    )))
}

/// `ppi of 15" screen at 1024x768`: the pixels along the diagonal over the diagonal in inches, as text.
fn screen_ppi(diagonal: &Value, width: &Value, height: &Value) -> Result<Value, Error> {
    let inches = match diagonal.unit().is_empty() {
        true => diagonal.number().clone(),
        false => diagonal
            .clone()
            .convert(&units::lookup("inch").ok_or(Error::IncompatibleUnits)?)?
            .number()
            .clone(),
    };
    let (w, h) = (plain_number(width)?, plain_number(height)?);
    if !inches.is_positive() || !w.is_positive() || !h.is_positive() {
        return Err(Error::OutOfDomain);
    }
    let squares = w.mul(w)?.add(&h.mul(h)?)?;
    let diagonal_pixels = call(Func::Sqrt, &[Value::plain(squares)])?;
    let density = Value::binary(BinOp::Div, &diagonal_pixels, &Value::plain(inches))?;
    let text =
        crate::format::format_decimal(density.number(), &crate::format::FormatOptions::default())
            .text;
    Ok(Value::text(&format!("{text} PPI")))
}

const NOTES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// `440 hz as pitch` is `A4`: the note nearest the frequency, named with sharps, middle C being C4 (MIDI 60) and A4
/// 440 Hz (MIDI 69).
fn as_pitch(frequency: &Value) -> Result<Value, Error> {
    let hertz = units::lookup("Hz").ok_or(Error::IncompatibleUnits)?;
    if frequency.kind() != Kind::Plain || frequency.unit().dim() != hertz.dim() {
        return Err(Error::IncompatibleUnits);
    }
    let in_hertz = frequency.clone().convert(&hertz)?;
    let f = in_hertz
        .number()
        .numer()
        .to_f64()
        .zip(in_hertz.number().denom().to_f64())
        .map(|(n, d)| n / d)
        .filter(|f| f.is_finite() && *f > 0.0)
        .ok_or(Error::OutOfDomain)?;
    let midi = (69.0 + 12.0 * (f / 440.0).log2()).round();
    if !(0.0..=127.0).contains(&midi) {
        return Err(Error::OutOfDomain);
    }
    let midi = midi as i64;
    Ok(Value::text(&format!(
        "{}{}",
        NOTES[midi.rem_euclid(12) as usize],
        midi.div_euclid(12) - 1
    )))
}

/// Trial division, which is exact and quick up to the limit (a million steps at most).
fn is_prime(n: &BigInt) -> Result<bool, Error> {
    const LIMIT: u64 = 1_000_000_000_000;
    if n < &BigInt::from(2) {
        return Ok(false);
    }
    let n = n
        .to_u64()
        .filter(|n| *n <= LIMIT)
        .ok_or(Error::UnsupportedOperation(
            "a number this large is not tested for being prime",
        ))?;
    let mut d = 2;
    while d * d <= n {
        if n % d == 0 {
            return Ok(false);
        }
        d += if d == 2 { 1 } else { 2 };
    }
    Ok(true)
}

fn plain_integer(v: &Value) -> Result<BigInt, Error> {
    if v.kind() != Kind::Plain || !v.unit().is_empty() || !v.number().is_integer() {
        return Err(Error::UnsupportedOperation(
            "this needs a whole number without a unit",
        ));
    }
    Ok(v.number().numer().clone())
}

fn compare(a: &Value, b: &Value) -> Result<Ordering, Error> {
    Value::compare(a, b)
}

pub fn call(func: Func, args: &[Value]) -> Result<Value, Error> {
    if !func.arity().accepts(args.len()) {
        return Err(Error::UnsupportedOperation(
            "this function was given the wrong number of values",
        ));
    }
    match func {
        Func::Abs => Ok(if args[0].number().is_negative() {
            args[0].neg()
        } else {
            args[0].clone()
        }),
        Func::Fact => {
            let n = plain_integer(&args[0])?;
            if n.is_negative() {
                return Err(Error::OutOfDomain);
            }
            let n = n.to_u64().ok_or(Error::Overflow)?;
            Ok(Value::plain(Number::from_ratio(
                factorial(n)?,
                BigInt::one(),
            )?))
        }
        Func::Round | Func::Floor | Func::Ceil => {
            let mode = match func {
                Func::Round => Rounding::Nearest,
                Func::Floor => Rounding::Down,
                _ => Rounding::Up,
            };
            args[0].map_number(|n| round_to_multiple(n, &Number::from_i64(1), mode))
        }
        Func::Min | Func::Max => {
            let mut best = args[0].clone();
            for v in &args[1..] {
                let order = compare(v, &best)?;
                if (func == Func::Max && order == Ordering::Greater)
                    || (func == Func::Min && order == Ordering::Less)
                {
                    best = v.clone();
                }
            }
            Ok(best)
        }
        Func::Sum => add_all(args),
        Func::Count => Ok(Value::plain(Number::from_i64(args.len() as i64))),
        Func::Average => {
            let sum = add_all(args)?;
            Value::binary(
                BinOp::Div,
                &sum,
                &Value::plain(Number::from_i64(args.len() as i64)),
            )
        }
        Func::Median => {
            let mut sorted: Vec<Value> = args.to_vec();
            let mut failure = None;
            sorted.sort_by(|a, b| {
                compare(a, b).unwrap_or_else(|e| {
                    failure.get_or_insert(e);
                    Ordering::Equal
                })
            });
            if let Some(e) = failure {
                return Err(e);
            }
            let mid = sorted.len() / 2;
            if sorted.len() % 2 == 1 {
                Ok(sorted[mid].clone())
            } else {
                call(Func::Average, &sorted[mid - 1..=mid])
            }
        }
        Func::IsPrime => is_prime(&plain_integer(&args[0])?).map(Value::boolean),
        Func::Gcd | Func::Lcm => {
            let mut acc = plain_integer(&args[0])?.abs();
            for v in &args[1..] {
                let x = plain_integer(v)?.abs();
                acc = if func == Func::Gcd {
                    acc.gcd(&x)
                } else {
                    acc.lcm(&x)
                };
            }
            Ok(Value::plain(Number::from_ratio(acc, BigInt::one())?))
        }
        Func::Clamp => {
            let (x, lo, hi) = (&args[0], &args[1], &args[2]);
            if compare(lo, hi)? == Ordering::Greater {
                return Err(Error::UnsupportedOperation(
                    "the lower limit is above the upper limit",
                ));
            }
            if compare(x, lo)? == Ordering::Less {
                Ok(lo.clone())
            } else if compare(x, hi)? == Ordering::Greater {
                Ok(hi.clone())
            } else {
                Ok(x.clone())
            }
        }
        Func::Permutations | Func::Combinations | Func::PermutationsOf | Func::CombinationsOf => {
            let swap = matches!(func, Func::PermutationsOf | Func::CombinationsOf);
            let (n, k) = if swap {
                (&args[1], &args[0])
            } else {
                (&args[0], &args[1])
            };
            let (n, k) = (plain_integer(n)?, plain_integer(k)?);
            if n.is_negative() || k.is_negative() || k > n {
                return Err(Error::OutOfDomain);
            }
            let combinations = matches!(func, Func::Combinations | Func::CombinationsOf);
            let k = if combinations {
                k.clone().min(&n - &k)
            } else {
                k
            };
            let (n, k) = (
                n.to_u64().ok_or(Error::Overflow)?,
                k.to_u64().ok_or(Error::Overflow)?,
            );
            let mut acc = BigInt::one();
            for i in 0..k {
                acc *= BigInt::from(n - i);
                if combinations {
                    acc /= BigInt::from(i + 1);
                }
                if acc.bits() > crate::number::MAX_BITS {
                    return Err(Error::Overflow);
                }
            }
            Ok(Value::plain(Number::from_ratio(acc, BigInt::one())?))
        }
        Func::Half => Value::binary(BinOp::Div, &args[0], &Value::plain(Number::from_i64(2))),
        Func::Midpoint if args[0].kind() == Kind::Date => {
            crate::datefns::midpoint(&args[0], &args[1])
        }
        Func::Midpoint => {
            let sum = Value::binary(BinOp::Add, &args[0], &args[1])?;
            Value::binary(BinOp::Div, &sum, &Value::plain(Number::from_i64(2)))
        }
        Func::Random => random_between(&args[0], &args[1]),
        // A time of day is a count of seconds inside; as a decimal it is hours: `10:15 to decimal` is 10.25.
        Func::AsNumber if args[0].kind() == Kind::Clock => {
            Ok(Value::plain(args[0].number().div(&Number::from_i64(3600))?))
        }
        Func::AsNumber => Ok(Value::plain(args[0].number().clone())),
        Func::AsDms => as_dms(&args[0]),
        Func::AsPitch => as_pitch(&args[0]),
        Func::ScreenPpi => screen_ppi(&args[0], &args[1], &args[2]),
        Func::Hex => args[0].clone().in_radix(16),
        Func::Bin => args[0].clone().in_radix(2),
        Func::Oct => args[0].clone().in_radix(8),
        Func::ToBase => {
            let base = plain_integer(&args[1])?.to_u8().ok_or(Error::OutOfDomain)?;
            args[0].clone().in_radix(base)
        }
        Func::StdDev => standard_deviation(args),
        Func::Compound => finance::compound(&args[0], &args[1], &args[2], whole(&args[3])?),
        Func::InterestOn => finance::interest_on(&args[0], &args[1], &args[2], whole(&args[3])?),
        Func::InterestEach => finance::interest_each(
            &args[0],
            &args[1],
            &args[2],
            whole(&args[3])?,
            whole(&args[4])?,
        ),
        Func::PresentValue => {
            finance::present_value(&args[0], &args[1], &args[2], whole(&args[3])?)
        }
        Func::ReturnMultiple => finance::return_multiple(&args[0], &args[1]),
        Func::InvestmentRequired => finance::investment_required(&args[0], &args[1]),
        Func::AnnualReturn => finance::annual_return(&args[0], &args[1], &args[2]),
        Func::Repayment | Func::LoanInterest => {
            let period = finance::Period::from_code(whole(&args[3])?).ok_or(Error::OutOfDomain)?;
            if func == Func::Repayment {
                finance::repayment(&args[0], &args[1], &args[2], period)
            } else {
                finance::loan_interest(&args[0], &args[1], &args[2], period)
            }
        }
        Func::GrowPerPeriod => finance::grow_per_period(&args[0], &args[1], &args[2], &args[3]),
        Func::TimeBySteps => finance::time_by_steps(&args[0], &args[1], &args[2], &args[3]),
        Func::TimeByGrowth => finance::time_by_growth(&args[0], &args[1], &args[2], &args[3]),
        Func::GrowthRate => finance::growth_rate(&args[0], &args[1], &args[2], &args[3]),
        Func::AtRate => at_rate(&args[0], &args[1]),
        Func::Proportion => proportion(&args[0], &args[1], &args[2]),
        Func::InWhat => in_what(&args[0], &args[1]),
        Func::AtDensity => at_density(&args[0], &args[1], &args[2]),
        Func::Date(f) => crate::datefns::call(f, args),
        Func::Root => {
            let k = plain_integer(&args[0])?;
            let k = k
                .to_u32()
                .filter(|k| *k >= 1 && *k <= 1000)
                .ok_or(Error::OutOfDomain)?;
            from_outcome(approx::root(plain_number(&args[1])?, k)?, &args[1])
        }
        Func::Sqrt => from_outcome(approx::sqrt(plain_number(&args[0])?)?, &args[0]),
        Func::Cbrt => from_outcome(approx::root(plain_number(&args[0])?, 3)?, &args[0]),
        Func::Ln => from_outcome(approx::ln(plain_number(&args[0])?)?, &args[0]),
        Func::Log2 => from_outcome(
            approx::log_base(plain_number(&args[0])?, &Number::from_i64(2))?,
            &args[0],
        ),
        Func::Log10 => from_outcome(
            approx::log_base(plain_number(&args[0])?, &Number::from_i64(10))?,
            &args[0],
        ),
        Func::LogBase => from_outcome(
            approx::log_base(plain_number(&args[0])?, plain_number(&args[1])?)?,
            &args[0],
        ),
        Func::Exp => from_outcome(approx::exp(plain_number(&args[0])?)?, &args[0]),
        Func::Sinh | Func::Cosh | Func::Tanh | Func::Asinh | Func::Acosh | Func::Atanh => {
            let f = trig_of(func);
            from_outcome(approx::trig_radians(f, plain_number(&args[0])?)?, &args[0])
        }
        Func::Sin | Func::Cos | Func::Tan | Func::SinD | Func::CosD | Func::TanD => {
            forward_trig(func, &args[0])
        }
        Func::Asin | Func::Acos | Func::Atan | Func::AsinD | Func::AcosD | Func::AtanD => {
            inverse_trig(func, &args[0])
        }
        // The reciprocal functions are the plain ones inverted: `cot` is 1/tan and `acot(x)` is atan(1/x).
        Func::Cot => reciprocal(&call(Func::Tan, args)?),
        Func::Csc => reciprocal(&call(Func::Sin, args)?),
        Func::Coth => reciprocal(&call(Func::Tanh, args)?),
        Func::Csch => reciprocal(&call(Func::Sinh, args)?),
        Func::CotD => reciprocal(&call(Func::TanD, args)?),
        Func::CscD => reciprocal(&call(Func::SinD, args)?),
        Func::Acot => call(Func::Atan, &[reciprocal(&args[0])?]),
        Func::Acsc => call(Func::Asin, &[reciprocal(&args[0])?]),
        Func::AcotD => call(Func::AtanD, &[reciprocal(&args[0])?]),
        Func::AcscD => call(Func::AsinD, &[reciprocal(&args[0])?]),
    }
}

/// One over a value, which is an error when the value is zero.
fn reciprocal(v: &Value) -> Result<Value, Error> {
    Value::binary(BinOp::Div, &Value::plain(Number::from_i64(1)), v)
}

fn whole(v: &Value) -> Result<i64, Error> {
    plain_integer(v)?.to_i64().ok_or(Error::Overflow)
}

/// `a is b, what is c`: the number that stands to c as a stands to b.
fn proportion(a: &Value, b: &Value, c: &Value) -> Result<Value, Error> {
    let bare = |v: &Value| matches!(v.kind(), Kind::Plain | Kind::Percent) && v.unit().is_empty();
    if bare(a) && bare(b) && bare(c) {
        let n = a.number().mul(c.number())?.div(b.number())?;
        let v = if a.kind() == Kind::Percent {
            Value::percent(n)
        } else {
            Value::plain(n)
        };
        return Ok(
            if a.is_approximate() || b.is_approximate() || c.is_approximate() {
                v.approximate()
            } else {
                v
            },
        );
    }
    let times = Value::binary(BinOp::Mul, a, c)?;
    Value::binary(BinOp::Div, &times, b)
}

/// `5% is 1 in what`: one in twenty.
fn in_what(share: &Value, top: &Value) -> Result<Value, Error> {
    let bare = |v: &Value| matches!(v.kind(), Kind::Plain | Kind::Percent) && v.unit().is_empty();
    if !bare(share) || !bare(top) {
        return Err(Error::UnsupportedOperation(
            "this needs plain numbers or a percentage",
        ));
    }
    if share.number().is_zero() {
        return Err(Error::DivisionByZero);
    }
    if top.number().is_zero() {
        return Err(Error::OutOfDomain);
    }
    Ok(Value::plain(share.number().clone()).as_ratio_of(top.number().clone()))
}

/// Pixels depend on how many fit in an inch: `1 cm in px at 326 ppi` and `326 px in cm at 326 ppi`.
fn at_density(value: &Value, density: &Value, target: &Value) -> Result<Value, Error> {
    let (px, inch) = (
        units::lookup("px").expect("px is defined"),
        units::lookup("inch").expect("inch is defined"),
    );
    if density.kind() != Kind::Plain
        || !density.unit().is_empty()
        || !density.number().is_positive()
    {
        return Err(Error::UnsupportedOperation(
            "pixels per inch must be a number above zero",
        ));
    }
    let to = target.unit();
    if value.kind() != Kind::Plain || value.unit().dim() != px.dim() {
        return Err(Error::UnsupportedOperation(
            "this needs a length or a number of pixels",
        ));
    }
    if to.dim() != px.dim() {
        return Err(Error::UnsupportedOperation(
            "the answer must be a length or pixels",
        ));
    }
    if value.unit() == &px && to != &px {
        let inches = value.number().div(density.number())?;
        return Value::quantity(inches, inch)?.convert(to);
    }
    if to == &px && value.unit() != &px {
        let inches = value.clone().convert(&inch)?;
        return Value::quantity(inches.number().mul(density.number())?, px);
    }
    Err(Error::UnsupportedOperation(
        "this needs pixels on one side and a length on the other",
    ))
}

/// `30 hours at $30/hour` multiplies, `$500 at $20/hour` divides: which one the units of the rate decide.
fn at_rate(amount: &Value, rate: &Value) -> Result<Value, Error> {
    let (per, of) = (rate.unit().denominator(), rate.unit().numerator());
    if per.is_empty() || of.is_empty() {
        return Err(Error::UnsupportedOperation(
            "a rate such as $30/hour is needed after `at`",
        ));
    }
    if amount.unit().dim() == per.dim() {
        Value::binary(BinOp::Mul, amount, rate)
    } else if amount.unit().dim() == of.dim() {
        Value::binary(BinOp::Div, amount, rate)
    } else {
        Err(Error::IncompatibleUnits)
    }
}

fn trig_of(func: Func) -> Trig {
    match func {
        Func::Sin | Func::SinD => Trig::Sin,
        Func::Cos | Func::CosD => Trig::Cos,
        Func::Tan | Func::TanD => Trig::Tan,
        Func::Asin | Func::AsinD => Trig::Asin,
        Func::Acos | Func::AcosD => Trig::Acos,
        Func::Atan | Func::AtanD => Trig::Atan,
        Func::Sinh => Trig::Sinh,
        Func::Cosh => Trig::Cosh,
        Func::Tanh => Trig::Tanh,
        Func::Asinh => Trig::Asinh,
        Func::Acosh => Trig::Acosh,
        _ => Trig::Atanh,
    }
}

fn plain_number(v: &Value) -> Result<&Number, Error> {
    if v.kind() != Kind::Plain || !v.unit().is_empty() {
        return Err(Error::UnsupportedOperation(
            "this function needs a plain number",
        ));
    }
    Ok(v.number())
}

/// The answer as a plain value, marked approximate when it is not exactly the true result.
fn from_outcome(o: approx::Outcome, from: &Value) -> Result<Value, Error> {
    let v = Value::plain(o.number);
    Ok(if o.exact && !from.is_approximate() {
        v
    } else {
        v.approximate()
    })
}

fn degrees_unit() -> crate::units::UnitExpr {
    crate::units::lookup("deg").expect("degrees are defined")
}

/// `sin(x)` takes radians for a plain number and degrees for an angle unit; `sind(x)` takes degrees for a plain number.
fn forward_trig(func: Func, arg: &Value) -> Result<Value, Error> {
    let f = trig_of(func);
    let in_degrees_by_default = matches!(func, Func::SinD | Func::CosD | Func::TanD);
    if arg.kind() != Kind::Plain {
        return Err(Error::UnsupportedOperation(
            "this function needs a plain number or an angle",
        ));
    }
    if arg.unit().is_empty() {
        let o = if in_degrees_by_default {
            approx::trig_degrees(f, arg.number())?
        } else {
            approx::trig_radians(f, arg.number())?
        };
        return from_outcome(o, arg);
    }
    if arg.unit().dim() != degrees_unit().dim() {
        return Err(Error::IncompatibleUnits);
    }
    let degrees = arg.number().mul(&arg.unit().factor()?)?;
    let o = approx::trig_degrees(f, &degrees)?;
    let tainted = Value::plain(o.number);
    Ok(
        if o.exact && !arg.is_approximate() && !arg.unit().is_approximate() {
            tainted
        } else {
            tainted.approximate()
        },
    )
}

/// `asin(x)` is in radians (a plain number); `asind(x)`, or an argument written in degrees, gives degrees.
fn inverse_trig(func: Func, arg: &Value) -> Result<Value, Error> {
    let f = trig_of(func);
    if arg.kind() != Kind::Plain {
        return Err(Error::UnsupportedOperation(
            "this function needs a plain number",
        ));
    }
    let wants_degrees =
        matches!(func, Func::AsinD | Func::AcosD | Func::AtanD) || *arg.unit() == degrees_unit();
    if !arg.unit().is_empty() && *arg.unit() != degrees_unit() {
        return Err(Error::IncompatibleUnits);
    }
    if wants_degrees {
        let o = approx::inverse_trig_degrees(f, arg.number())?;
        let v = Value::quantity(o.number, degrees_unit())?;
        return Ok(if o.exact && !arg.is_approximate() {
            v
        } else {
            v.approximate()
        });
    }
    from_outcome(approx::trig_radians(f, arg.number())?, arg)
}

fn standard_deviation(args: &[Value]) -> Result<Value, Error> {
    let unit = args[0].unit().clone();
    let mut numbers = Vec::with_capacity(args.len());
    for v in args {
        if v.kind() != Kind::Plain || v.unit().dim() != unit.dim() {
            return Err(Error::IncompatibleUnits);
        }
        let in_first = v.clone().convert_to_unit_of(&args[0])?;
        numbers.push(in_first);
    }
    let n = Number::from_i64(numbers.len() as i64);
    let mut sum = Number::from_i64(0);
    for x in &numbers {
        sum = sum.add(x)?;
    }
    let mean = sum.div(&n)?;
    let mut squares = Number::from_i64(0);
    for x in &numbers {
        let d = x.sub(&mean)?;
        squares = squares.add(&d.mul(&d)?)?;
    }
    let variance = squares.div(&n.sub(&Number::from_i64(1))?)?;
    let o = approx::sqrt(&variance)?;
    let tainted = args.iter().any(Value::is_approximate) || !o.exact;
    let v = Value::quantity(o.number, unit)?;
    Ok(if tainted { v.approximate() } else { v })
}

/// n!, refusing as soon as the result would not fit the engine's size limit.
fn factorial(n: u64) -> Result<BigInt, Error> {
    let mut acc = BigInt::one();
    for k in 2..=n {
        acc *= BigInt::from(k);
        if acc.bits() > crate::number::MAX_BITS {
            return Err(Error::Overflow);
        }
    }
    Ok(acc)
}

fn add_all(args: &[Value]) -> Result<Value, Error> {
    let mut total = args[0].clone();
    for v in &args[1..] {
        total = Value::binary(BinOp::Add, &total, v)?;
    }
    Ok(total)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rounding {
    Nearest,
    Up,
    Down,
}

/// Rounds `n` to a multiple of `step` (a positive number). Ties go away from zero.
pub fn round_to_multiple(n: &Number, step: &Number, mode: Rounding) -> Result<Number, Error> {
    if step.is_zero() || step.is_negative() {
        return Err(Error::OutOfDomain);
    }
    let q = n.div(step)?;
    let floor = BigInt::div_floor(q.numer(), q.denom());
    let whole = Number::from_ratio(floor.clone(), BigInt::one())?;
    let has_fraction = !q.is_integer();
    let rounded = match mode {
        Rounding::Down => whole,
        Rounding::Up if has_fraction => whole.add(&Number::from_i64(1))?,
        Rounding::Up => whole,
        Rounding::Nearest => {
            let frac = q.sub(&whole)?;
            let twice = frac.mul(&Number::from_i64(2))?;
            let one = Number::from_i64(1);
            let up = match twice.cmp(&one) {
                Ordering::Greater => true,
                Ordering::Less => false,
                Ordering::Equal => !n.is_negative(),
            };
            if up { whole.add(&one)? } else { whole }
        }
    };
    rounded.mul(step)
}

thread_local! {
    static SEED: std::cell::Cell<u64> = const { std::cell::Cell::new(0x9E37_79B9_7F4A_7C15) };
}

/// Seeds the generator behind `random number between`, so tests can be repeatable.
pub fn seed_random(seed: u64) {
    SEED.with(|s| s.set(seed | 1));
}

fn next_random() -> u64 {
    SEED.with(|s| {
        let mut x = s.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        s.set(x);
        x
    })
}

/// An integer in the range when both ends are whole numbers, otherwise ten decimal places.
fn random_between(a: &Value, b: &Value) -> Result<Value, Error> {
    if a.kind() != Kind::Plain
        || b.kind() != Kind::Plain
        || !a.unit().is_empty()
        || !b.unit().is_empty()
    {
        return Err(Error::UnsupportedOperation(
            "a random number needs plain limits",
        ));
    }
    let (lo, hi) = if a.number() <= b.number() {
        (a.number(), b.number())
    } else {
        (b.number(), a.number())
    };
    // `between 0 and 1` is the documented way to ask for a fraction, so it is not the integer range 0 to 1.
    let unit_interval = lo.is_zero() && hi.cmp(&Number::from_i64(1)) == std::cmp::Ordering::Equal;
    if lo.is_integer() && hi.is_integer() && !unit_interval {
        let span = hi.sub(lo)?.numer().clone() + 1;
        let pick = BigInt::from(next_random()) % span;
        return Ok(Value::plain(
            lo.add(&Number::from_ratio(pick, BigInt::one())?)?,
        ));
    }
    let scale = BigInt::from(10).pow(10);
    let steps = hi
        .sub(lo)?
        .mul(&Number::from_ratio(scale.clone(), BigInt::one())?)?;
    let top = BigInt::div_floor(steps.numer(), steps.denom()) + 1;
    let pick = BigInt::from(next_random()) % top;
    Ok(Value::plain(lo.add(&Number::from_ratio(pick, scale)?)?))
}
