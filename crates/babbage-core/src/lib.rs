pub mod approx;
pub mod calendar;
pub mod datefns;
pub mod dateforms;
pub mod dates;
pub mod error;
pub mod eval;
pub mod finance;
pub mod format;
pub mod functions;
pub mod lexer;
pub mod number;
pub mod parser;
pub mod phrase;
pub mod settings;
pub mod sheet;
pub mod units;
pub mod value;
pub mod words;
pub mod zones;

pub use error::Error;
pub use format::{FormatOptions, Formatted, NumberFormat, RoundingMode, format_decimal};
pub use number::Number;
pub use sheet::{LineKind, LineResult, SheetOptions, evaluate_sheet};
pub use value::{Kind, Shown, Value, format_value};

pub fn parse_expression(line: &str) -> Result<parser::Expr, Error> {
    phrase::parse_line(&lexer::lex(line)?)
}

/// Evaluates one line with the given settings (a tax rate, for instance) in force.
pub fn evaluate_with(line: &str, settings: &settings::Settings) -> Result<Value, Error> {
    settings::scoped(settings, || evaluate(line))
}

pub fn evaluate(line: &str) -> Result<Value, Error> {
    eval::eval(&parse_expression(line)?)
}

/// Evaluates a line that must produce a plain number (no percentage).
pub fn evaluate_plain(line: &str) -> Result<Number, Error> {
    evaluate(line)?.into_plain()
}
