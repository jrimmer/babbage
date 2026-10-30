# Errors and guarantees

## Errors

Every fallible call returns `Error`, which implements `std::error::Error` and `Display`. The messages are written for the person typing the line.

```rust
use babbage_core::{Error, evaluate};

assert_eq!(evaluate("1 / 0").unwrap_err(), Error::DivisionByZero);
assert_eq!(evaluate("5 kg in m").unwrap_err(), Error::IncompatibleUnits);
assert_eq!(
    evaluate("5 kg in m").unwrap_err().to_string(),
    "these units measure different things"
);
assert_eq!(evaluate("10 EUR in USD").unwrap_err(), Error::CurrencyRateNeeded);
assert_eq!(evaluate("2 ^ 3000000").unwrap_err(), Error::Overflow);
```

The last one is on purpose. Babbage fetches no exchange rates and never guesses one: a conversion between currencies needs a rate in the line (`10 EUR in USD at 1.08 USD/EUR`), and without one it is an error. Match on `Error` with a catch-all arm, so a variant added later does not break your build.

## Exactness

- Numbers are exact fractions (`Number` wraps a big rational), so `1/3 + 1/6` is exactly one half and `0.1 + 0.2` is exactly `0.3`.
- The engine uses no floating point for arithmetic. A project check fails the build if it ever does.
- A value is marked approximate only when it cannot be held exactly, such as `pi`. `is_approximate()` reports it and the app shows a small ≈.
- Numbers are limited to about a million bits (`number::MAX_BITS`). Past that a line fails with `Error::Overflow` (`2 ^ 3000000` does) instead of exhausting memory.

## What 1.x promises

- The crate root and the `format`, `number`, `settings`, `sheet`, `value`, `zones` and `error` modules follow semantic versioning. A change that breaks code written against them waits for 2.0.
- The other modules are hidden from the documentation and are internals. They can change in any release, so do not depend on them.
- The text of error messages is not part of the interface; match on the variant, not on the string.
- Answers can change within 1.x when an answer was wrong. A bug fix that corrects a result is not a breaking change. The project tests every documented behavior, and the decisions behind them are in `docs/decisions.md` in the repository.
