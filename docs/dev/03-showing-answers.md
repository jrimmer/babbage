# Showing answers

`format_value(&value, &options)` returns a `Formatted`: the `text` to show and an `exact` flag that is false when the text differs from the true value because of rounding.

## FormatOptions

`FormatOptions::default()` is what the app shows with no settings changed: ten decimal places at most, thousands separators, `k` and `M` for large numbers, and the North American number format. Change a copy of it, never the global default.

```rust
use babbage_core::{FormatOptions, evaluate, format_value};

let value = evaluate("2/3")?;
let two = FormatOptions::new(2)?;
assert_eq!(format_value(&value, &two).text, "0.67");

let repeating = FormatOptions::default().with_repeating(true);
assert_eq!(format_value(&value, &repeating).text, "0.(6)");
# Ok::<(), babbage_core::Error>(())
```

`FormatOptions::new` refuses more than 1000 decimal places with `Error::InvalidPrecision`. `with_repeating(true)` writes a repeating decimal with its period in brackets, up to `MAX_REPEAT_PERIOD` digits, and falls back to a rounded decimal beyond that.

## Number formats

`NumberFormat` picks the marks for thousands and for the decimal point. It changes how numbers are written, and how they are read in a line.

```rust
use babbage_core::{FormatOptions, NumberFormat, evaluate, format_value};

let value = evaluate("1234.5")?;
let europe = FormatOptions::default().in_format(NumberFormat::WesternEurope);
assert_eq!(format_value(&value, &europe).text, "1.234,5");
assert_eq!(format_value(&value, &FormatOptions::default()).text, "1,234.5");
# Ok::<(), babbage_core::Error>(())
```

## Large numbers and money

Large numbers are abbreviated by default (`16.7M`). Switch that off for every digit.

```rust
use babbage_core::{FormatOptions, evaluate, format_value};

let value = evaluate("256 * 256 * 256")?;
assert_eq!(format_value(&value, &FormatOptions::default()).text, "16.777216M");
let all = FormatOptions::default().unabbreviated();
assert_eq!(format_value(&value, &all).text, "16,777,216");
# Ok::<(), babbage_core::Error>(())
```

Money keeps its symbol and its cents: `$60.00`, not `60`. The fields `currency_codes` (`5.00 EUR` instead of `€5.00`) and `money_trim_zeros` (`$5` instead of `$5.00`) change that.

## The options a user has set

A program with its own settings screen usually does not build `FormatOptions` by hand. `Settings::format_options()` turns a `Settings` value (see [Settings and time](05-settings-and-time.html)) into the options it implies.
