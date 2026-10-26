//! Per-evaluation settings that are not part of the text of a line: the tax rate, the clock and the home time zone.
//!
//! They are held in a thread-local for the duration of an evaluation, so the lexer can turn `VAT` or `today` into
//! a value without every function in the engine carrying a context argument.

use std::cell::RefCell;

use crate::number::Number;

/// A sales tax named in settings: `VAT` at 15%.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tax {
    pub name: String,
    pub rate: Number,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Settings {
    pub tax: Option<Tax>,
}

thread_local! {
    static CURRENT: RefCell<Settings> = RefCell::new(Settings::default());
}

/// Runs `f` with `settings` in force, then restores whatever was there.
pub fn scoped<T>(settings: &Settings, f: impl FnOnce() -> T) -> T {
    let previous = CURRENT.with(|c| c.replace(settings.clone()));
    let result = f();
    CURRENT.with(|c| *c.borrow_mut() = previous);
    result
}

pub fn current<T>(f: impl FnOnce(&Settings) -> T) -> T {
    CURRENT.with(|c| f(&c.borrow()))
}

/// The tax rate when `name` is the configured tax's name, ignoring case.
pub fn tax_rate_named(name: &str) -> Option<Number> {
    current(|s| {
        s.tax
            .as_ref()
            .filter(|t| t.name.eq_ignore_ascii_case(name))
            .map(|t| t.rate.clone())
    })
}
