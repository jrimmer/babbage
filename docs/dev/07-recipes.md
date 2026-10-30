# Recipes

## A calculator for lines on standard input

Read the lines, evaluate them as one sheet, and print each answer beside its line. This is the whole idea of the `bbg` command, minus its options.

```rust
use babbage_core::{FormatOptions, LineKind, SheetOptions, evaluate_sheet, format_value};

fn answers(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let results = evaluate_sheet(&lines, &SheetOptions::default());
    let opts = FormatOptions::default();
    lines
        .iter()
        .zip(&results)
        .map(|(line, result)| match &result.kind {
            LineKind::Value(v) => format!("{line}  =  {}", format_value(v, &opts).text),
            LineKind::Error(e) => format!("{line}  !  {e}"),
            _ => (*line).to_string(),
        })
        .collect()
}

let out = answers("rent = $900\nrent * 12\n5 kg in m");
assert_eq!(out[1], "rent * 12  =  $10,800.00");
assert_eq!(out[2], "5 kg in m  !  these units measure different things");
```

## One call that gives text

When all you need is the answer to one line as a string, wrap the two calls once and keep the error:

```rust
use babbage_core::{Error, FormatOptions, evaluate, format_value};

fn calc(line: &str) -> Result<String, Error> {
    Ok(format_value(&evaluate(line)?, &FormatOptions::default()).text)
}

assert_eq!(calc("20% of 85")?, "17");
assert!(calc("20% of").is_err());
# Ok::<(), Error>(())
```

## Answers inside text

A note-taking program can answer calculations written in running text. Treat a line that ends in `=` as a question:

```rust
use babbage_core::{FormatOptions, LineKind, SheetOptions, evaluate_sheet, format_value};

fn fill_in(line: &str) -> String {
    let Some(question) = line.trim_end().strip_suffix('=') else {
        return line.to_string();
    };
    // A sheet of one line reads the words around the sum the way the app does: "Tip on the bill:" is a label.
    match &evaluate_sheet(&[question], &SheetOptions::default())[0].kind {
        LineKind::Value(v) => format!("{question}= {}", format_value(v, &FormatOptions::default()).text),
        _ => line.to_string(),
    }
}

assert_eq!(fill_in("Tip on the bill: 18% of 64 ="), "Tip on the bill: 18% of 64 = 11.52");
assert_eq!(fill_in("nothing to do here"), "nothing to do here");
```

Lines that are not a calculation are left alone, which is the safe thing for a text editor to do. A whole note can go to `evaluate_sheet` in one call, so that names and earlier answers work too; read `ignored` on each result to see which words were skipped.
