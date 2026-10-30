# Getting started

`babbage-core` is the calculation engine behind the Babbage notepad calculator, published as a Rust crate. It reads a line of plain language such as `20% of 85`, `150 lb in kg` or `June 3, 2027 + 3 weeks` and returns an exact value. It has no interface of its own: no window, no files, no network.

## Add it

```sh
cargo add babbage-core
```

The crate uses Rust edition 2024, so it needs Rust 1.85 or later. It is MIT licensed.

## Your first answer

Two calls do the work. `evaluate` turns a line into a `Value`, and `format_value` writes that value the way the app shows it.

```rust
use babbage_core::{FormatOptions, evaluate, format_value};

let value = evaluate("20% of 85")?;
let shown = format_value(&value, &FormatOptions::default());
assert_eq!(shown.text, "17");
assert!(shown.exact);
# Ok::<(), babbage_core::Error>(())
```

`shown.exact` tells you whether the text is the true value. A third is not representable as a decimal, so the text is cut and the flag says so:

```rust
use babbage_core::{FormatOptions, evaluate, format_value};

let third = format_value(&evaluate("1/3")?, &FormatOptions::default());
assert_eq!(third.text, "0.3333333333");
assert!(!third.exact);
# Ok::<(), babbage_core::Error>(())
```

Underneath, the value is still exactly one third. Nothing is lost until you ask for text.

## What is where

| You want to | Use |
|---|---|
| Calculate one line | `evaluate`, or `evaluate_with` to give it settings |
| Calculate a list of lines that refer to each other | `evaluate_sheet` |
| Get text from a value | `format_value` and `FormatOptions` |
| Set the clock, the time zone, tax or the region | `settings::Settings` |
| Look inside a value | `Value`, `Kind` and `Number` |
| Handle a failure | `Error` |

The chapters that follow take these in turn. The crate root and the `format`, `number`, `settings`, `sheet`, `value`, `zones` and `error` modules are the documented interface. Every other module is internal and can change in any release.
