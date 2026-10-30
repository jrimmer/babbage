# Settings and time

Some answers depend on things outside the line: what day it is, where you are, which dollar a bare `$` means, whether a sales tax is on. `settings::Settings` holds them, and nothing is read from the machine unless you leave it unset.

## A clock you control

`today` and `now` read the system clock by default. Pin the clock and the answer is the same every time, which is what a test needs.

```rust
use babbage_core::settings::Settings;
use babbage_core::{FormatOptions, evaluate_with, format_value};

// 2027-01-01 00:00:00 UTC
let settings = Settings::at(1_798_761_600);
let value = evaluate_with("today + 3 weeks", &settings)?;
assert_eq!(format_value(&value, &FormatOptions::default()).text, "January 22, 2027");
# Ok::<(), babbage_core::Error>(())
```

`Settings` has a field for each thing a line can depend on, and `Settings::default()` leaves every one unset. Build one with struct update syntax and set only what you need.

## Regional dollars and number formats

A bare `$` means US dollars unless `dollar` says otherwise. `number_format` changes how numbers are read as well as written, and `day_first` decides whether `12/02/1988` is 12 February or 2 December.

```rust
use babbage_core::settings::Settings;
use babbage_core::{FormatOptions, evaluate_with, format_value};

let australia = Settings { dollar: Some("AUD"), day_first: true, ..Settings::default() };
let value = evaluate_with("$50 * 2", &australia)?;
assert_eq!(format_value(&value, &FormatOptions::default()).text, "A$100.00");
# Ok::<(), babbage_core::Error>(())
```

## Sales tax

A named tax lets a line add it by name (`$100 + VAT`). The rate is a `Number`.

```rust
use babbage_core::settings::{Settings, Tax};
use babbage_core::{FormatOptions, Number, evaluate_with, format_value};

let rate = Number::from_decimal_str("0.2")?;
let settings = Settings { tax: Some(Tax { name: "VAT".into(), rate }), ..Settings::default() };
let value = evaluate_with("$100 + VAT", &settings)?;
assert_eq!(format_value(&value, &FormatOptions::default()).text, "$120.00");
# Ok::<(), babbage_core::Error>(())
```

## Time zones and holidays

`local_zone` is the user's own zone, with its daylight saving rules; without it, `local_utc_offset_seconds` is one fixed offset. `holidays` lists days off that workday arithmetic skips, and `custom_zones` gives your own names to places. Zone rules come from a bundled database by default, so answers do not depend on the machine's files.

## Settings for a whole sheet

`SheetOptions` carries a `Settings` in its `settings` field, so every line of a sheet sees the same clock, tax and region. `Settings::format_options()` gives the matching `FormatOptions` for showing the answers.

```rust
use babbage_core::settings::Settings;
use babbage_core::{FormatOptions, LineKind, SheetOptions, evaluate_sheet, format_value};

let settings = Settings::at(1_798_761_600);
let opts = SheetOptions { settings: settings.clone(), ..SheetOptions::default() };
let results = evaluate_sheet(&["today", "today + 1 day"], &opts);
let show = settings.format_options();
let texts: Vec<String> = results
    .iter()
    .map(|r| match &r.kind {
        LineKind::Value(v) => format_value(v, &show).text,
        other => format!("{other:?}"),
    })
    .collect();
assert_eq!(texts, ["January 1, 2027", "January 2, 2027"]);
let _ = FormatOptions::default();
```
