# babbage-core

An exact-arithmetic calculation engine for plain-language lines. It is the engine behind the Babbage notepad
calculator: give it `20% of 85`, `150 lb in kg`, `June 3, 2027 + 3 weeks` or `monthly payment on $250,000 over 30
years at 6.5%` and it returns an exact value, and says when a written answer had to be rounded.

```rust
use babbage_core::{FormatOptions, evaluate, format_value};

let value = evaluate("20% of 85")?;
let shown = format_value(&value, &FormatOptions::default());
assert_eq!(shown.text, "17");
```

- Numbers are exact fractions; a result is marked approximate only when it cannot be written exactly.
- Percentages, units, money, dates, times and time zones, finance and the usual functions are read from plain
  language.
- A sheet (`evaluate_sheet`) lets a line use the names and answers of the lines above it.
- `FormatOptions` chooses decimal places, thousands separators, regional number formats and repeating decimals.

The crate has no interface of its own. The documented API is the crate root and the `format`, `number`,
`settings`, `sheet`, `value`, `zones` and `error` modules; the other modules are internals.

MIT licence.
