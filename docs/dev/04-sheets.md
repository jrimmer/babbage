# Sheets

A sheet is a list of lines evaluated from the top. A line can use the names and the answers of the lines above it. `evaluate_sheet` takes the lines and returns one `LineResult` for each.

```rust
use babbage_core::{FormatOptions, LineKind, SheetOptions, evaluate_sheet, format_value};

let lines = ["# Trip", "hotel = $180", "3 * hotel", "// a note", "Boing is $5"];
let results = evaluate_sheet(&lines, &SheetOptions::default());
assert_eq!(results.len(), lines.len());

assert_eq!(results[0].kind, LineKind::Heading("Trip".into()));
assert_eq!(results[3].kind, LineKind::Comment);

let LineKind::Value(nights) = &results[2].kind else { panic!("a value") };
assert_eq!(format_value(nights, &FormatOptions::default()).text, "$540.00");
```

## What a line can be

`LineKind` is one of `Blank`, `Heading`, `Divider` (`---`), `Comment` (`//`), `Text` (nothing to calculate), `Value` or `Error`. A line that fails does not stop the sheet: its result is `LineKind::Error` and the lines after it are still evaluated.

```rust
use babbage_core::{Error, LineKind, SheetOptions, evaluate_sheet};

let results = evaluate_sheet(&["1 / 0", "2 + 2"], &SheetOptions::default());
assert_eq!(results[0].kind, LineKind::Error(Error::DivisionByZero));
assert!(matches!(results[1].kind, LineKind::Value(_)));
```

## Names and references

`name = value` defines a variable for the lines below it. `line3` refers to the answer of line 3, counting from 1, and only to a line above.

```rust
use babbage_core::{FormatOptions, LineKind, SheetOptions, evaluate_sheet, format_value};

let results = evaluate_sheet(&["19 / 4", "line1 * 4"], &SheetOptions::default());
let LineKind::Value(v) = &results[1].kind else { panic!("a value") };
assert_eq!(format_value(v, &FormatOptions::default()).text, "19");
assert_eq!(results[0].value().is_some(), true);
```

A line's `declares` field holds the name it defines, if any. `value()` returns the answer of a value line and `counted_value()` the one that counts toward totals.

## Words that were not understood

A sheet reads like notes, so by default words the engine does not know are skipped, and each is reported in `ignored` with its position. A skipped word right after a number is flagged `unclear`, because the line may not mean what it says. A skipped word one letter from a unit also appears in `warnings`.

```rust
use babbage_core::{SheetOptions, evaluate_sheet};

let results = evaluate_sheet(&["Boing is $5"], &SheetOptions::default());
let words: Vec<&str> = results[0].ignored.iter().map(|w| w.text.as_str()).collect();
assert_eq!(words, ["Boing", "is"]);
```

Set `SheetOptions { natural: false, ..Default::default() }` to refuse such lines instead. Use that when the lines come from a program and not from a person.

## Totals and tags

`floating_total` adds up the sheet the way the app's Total chip does, and `floating_total_left_out` counts the lines it had to leave out. Lines can carry `#tags`, which the result lists in `tags`, and a line such as `total of #food` adds up only those.

```rust
use babbage_core::{FormatOptions, SheetOptions, evaluate_sheet, format_value};
use babbage_core::sheet::floating_total;

let opts = SheetOptions::default();
let results = evaluate_sheet(&["$12 lunch #food", "$30 taxi", "$8 coffee #food"], &opts);
let total = floating_total(&results, &opts).expect("something to add")?;
assert_eq!(format_value(&total, &FormatOptions::default()).text, "$50.00");
# Ok::<(), babbage_core::Error>(())
```

## Variables every sheet has

`SheetOptions::globals` holds lines such as `rent = $900` that every sheet starts with. They survive a divider line, which otherwise ends the scope of the names above it.
