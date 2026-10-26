pub mod error;
pub mod eval;
pub mod format;
pub mod lexer;
pub mod number;
pub mod parser;

pub use error::Error;
pub use format::{FormatOptions, Formatted, RoundingMode, format_decimal};
pub use number::Number;

pub fn parse_expression(line: &str) -> Result<parser::Expr, Error> {
    parser::parse(&lexer::lex(line)?)
}

pub fn evaluate(line: &str) -> Result<Number, Error> {
    eval::eval(&parse_expression(line)?)
}
