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
            Error::DivisionByZero => write!(f, "division by zero"),
            Error::ZeroToTheZero => write!(f, "0 to the power 0 is undefined"),
            Error::NonIntegerExponent => write!(f, "exponent must be a whole number"),
            Error::Overflow => write!(f, "number too large to represent"),
            Error::InvalidPrecision => write!(f, "decimal places out of range"),
        }
    }
}

impl std::error::Error for Error {}
