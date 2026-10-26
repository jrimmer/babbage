pub mod error;
pub mod eval;
pub mod format;
pub mod lexer;
pub mod number;
pub mod parser;
pub mod phrase;
pub mod units;
pub mod value;

pub use error::Error;
pub use format::{FormatOptions, Formatted, RoundingMode, format_decimal};
pub use number::Number;
pub use value::{Kind, Value, format_value};

pub fn parse_expression(line: &str) -> Result<parser::Expr, Error> {
    phrase::parse_line(&lexer::lex(line)?)
}

pub fn evaluate(line: &str) -> Result<Value, Error> {
    eval::eval(&parse_expression(line)?)
}

/// Evaluates a line that must produce a plain number (no percentage).
pub fn evaluate_plain(line: &str) -> Result<Number, Error> {
    evaluate(line)?.into_plain()
}
