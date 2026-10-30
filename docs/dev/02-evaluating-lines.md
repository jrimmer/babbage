# Evaluating lines

`evaluate` takes one line and returns a `Value` or an `Error`. A line is what you would type in the app: numbers, units and the words of the grammar, such as `plus`, `of`, `in` and `to`. It is strict. A word it does not know is an error, so the line is either a calculation or refused. (A sheet is looser: it skips unknown words the way notes read. See [Sheets](04-sheets.html).)

```rust
use babbage_core::{FormatOptions, evaluate, format_value};

let show = |line: &str| -> Result<String, babbage_core::Error> {
    Ok(format_value(&evaluate(line)?, &FormatOptions::default()).text)
};
assert_eq!(show("12 plus 3")?, "15");
assert_eq!(show("150 lb in kg")?, "68.0388555 kg");
assert_eq!(show("$50 + 20%")?, "$60.00");
assert_eq!(show("3 hours 20 minutes in seconds")?, "12,000 s");
assert!(show("I spent $45 on clothes").is_err());
# Ok::<(), babbage_core::Error>(())
```

## What is inside a value

A `Value` is a number, a kind and a unit. The number is exact.

```rust
use babbage_core::{Kind, evaluate};

let half = evaluate("1/3 + 1/6")?;
assert_eq!(half.number().numer().to_string(), "1");
assert_eq!(half.number().denom().to_string(), "2");
assert!(!half.is_approximate());
assert!(evaluate("pi")?.is_approximate());
assert_eq!(half.kind(), Kind::Plain);

assert_eq!(evaluate("20%")?.kind(), Kind::Percent);
assert_eq!(evaluate("June 3, 2027")?.kind(), Kind::Date);
# Ok::<(), babbage_core::Error>(())
```

`Kind` says what the number means: a plain number, a percentage (held as a fraction, so 10% is 1/10), a date, a time of day, a multiplier, a tax rate, a true or false, or a word. A value with a unit, such as `5 km`, is a plain number whose `unit()` is not empty.

`is_approximate()` is true when the engine had to approximate, for example `pi`, which no fraction holds exactly. It is a property of the value, separate from the rounding that `format_value` does.

## When you need a plain number

`evaluate_plain` is for callers that want a number and nothing else. A percentage, a date or a unit is an error, not a surprise.

```rust
use babbage_core::{Error, evaluate_plain};

let n = evaluate_plain("2 + 3 * 4")?;
assert_eq!(n.to_exact_decimal().as_deref(), Some("14"));
assert!(evaluate_plain("20%").is_err());
assert!(matches!(evaluate_plain("1 / 0"), Err(Error::DivisionByZero)));
# Ok::<(), Error>(())
```

## What `evaluate` does not do

A single line has no neighbours, so it cannot use a name or the answer of another line. `rent * 12` fails on its own because `rent` is not defined. For lines that refer to each other, use a sheet (see [Sheets](04-sheets.html)).
