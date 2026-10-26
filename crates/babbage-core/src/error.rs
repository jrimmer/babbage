use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Empty,
    InputTooLong,
    UnexpectedChar { ch: char, at: usize },
    InvalidNumber { text: String, at: usize },
    UnexpectedToken { found: String, at: usize },
    UnexpectedEnd,
    UnexpectedWord { word: String, at: usize },
    UnsupportedOperation(&'static str),
    NotAPlainNumber,
    IncompatibleUnits,
    CurrencyRateNeeded,
    BadReference { line: usize },
    OutOfDomain,
    DivisionByZero,
    ZeroToTheZero,
    NonIntegerExponent,
    Overflow,
    InvalidPrecision,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Empty => write!(f, "empty expression"),
            Error::InputTooLong => write!(f, "input too long"),
            Error::UnexpectedChar { ch, at } => {
                write!(f, "unexpected character {ch:?} at byte {at}")
            }
            Error::InvalidNumber { text, at } => write!(f, "invalid number {text:?} at byte {at}"),
            Error::UnexpectedToken { found, at } => write!(f, "unexpected {found} at byte {at}"),
            Error::UnexpectedEnd => write!(f, "unexpected end of expression"),
            Error::UnexpectedWord { word, at } => {
                write!(f, "unexpected word {word:?} at byte {at}")
            }
            Error::UnsupportedOperation(what) => write!(f, "unsupported: {what}"),
            Error::NotAPlainNumber => write!(f, "result is not a plain number"),
            Error::IncompatibleUnits => write!(f, "these units measure different things"),
            Error::CurrencyRateNeeded => write!(f, "different currencies need an exchange rate"),
            Error::OutOfDomain => write!(f, "that value is outside what this function accepts"),
            Error::BadReference { line } => {
                write!(f, "line {line} has no result above this line to use")
            }
            Error::DivisionByZero => write!(f, "division by zero"),
            Error::ZeroToTheZero => write!(f, "0 to the power 0 is undefined"),
            Error::NonIntegerExponent => write!(f, "exponent must be a whole number"),
            Error::Overflow => write!(f, "number too large to represent"),
            Error::InvalidPrecision => write!(f, "decimal places out of range"),
        }
    }
}

impl std::error::Error for Error {}
