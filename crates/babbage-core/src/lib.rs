//! An exact-arithmetic calculation engine for plain-language lines, the engine behind the Babbage notepad calculator.
//!
//! Hand it a line like `20% of 85` or `150 lb in kg` and it hands back a [`Value`], exact until it cannot be, and
//! [`format_value`] writes that value the way the app shows it, saying whether the text had to be rounded.
//!
//! ```
//! use babbage_core::{FormatOptions, evaluate, format_value};
//!
//! # fn main() -> Result<(), babbage_core::Error> {
//! let value = evaluate("20% of 85")?;
//! let shown = format_value(&value, &FormatOptions::default());
//! assert_eq!(shown.text, "17");
//! assert!(shown.exact);
//!
//! let third = format_value(&evaluate("1/3")?, &FormatOptions::default());
//! assert_eq!(third.text, "0.3333333333");
//! assert!(!third.exact);
//! # Ok(())
//! # }
//! ```
//!
//! A sheet is a list of lines in which a line can use the names and answers of the lines above it:
//!
//! ```
//! use babbage_core::{FormatOptions, LineKind, SheetOptions, evaluate_sheet, format_value};
//!
//! let results = evaluate_sheet(&["rent = $900", "rent * 12"], &SheetOptions::default());
//! let LineKind::Value(yearly) = &results[1].kind else {
//!     panic!("a value");
//! };
//! assert_eq!(format_value(yearly, &FormatOptions::default()).text, "$10,800.00");
//! ```
//!
//! [`FormatOptions`] chooses the decimal places, thousands separators, the number format of a region and
//! repeating decimals (`1/3` as `0.(3)`); [`settings::Settings`] holds the clock, the time zone, the region and
//! the other things a line can depend on. The modules that are not documented here are the engine's internals and
//! can change at any time.

#[doc(hidden)]
pub mod approx;
#[doc(hidden)]
pub mod calendar;
mod chinese_new_year;
#[doc(hidden)]
pub mod datefns;
#[doc(hidden)]
pub mod dateforms;
#[doc(hidden)]
pub mod dates;
pub mod error;
#[doc(hidden)]
pub mod eval;
#[doc(hidden)]
pub mod finance;
pub mod format;
#[doc(hidden)]
pub mod functions;
#[doc(hidden)]
pub mod lexer;
#[doc(hidden)]
pub mod lunar;
pub mod number;
#[doc(hidden)]
pub mod parser;
#[doc(hidden)]
pub mod phrase;
pub mod settings;
pub mod sheet;
#[doc(hidden)]
pub mod units;
pub mod value;
#[doc(hidden)]
pub mod words;
pub mod zones;

pub use error::Error;
pub use format::{
    FormatOptions, Formatted, MAX_REPEAT_PERIOD, NumberFormat, RoundingMode, format_decimal,
};
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
