use babbage_core::{FormatOptions, evaluate, format_value};

const CORPUS: &str = include_str!("corpus/forms.tsv");

#[test]
fn agrees_with_the_fraction_oracle_on_the_word_forms() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in CORPUS.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let got = match evaluate(cols[0]) {
            Ok(v) => format_value(&v, &FormatOptions::default()).text,
            Err(e) => format!("ERROR {e}"),
        };
        if got != cols[1] {
            failures.push(format!("{}: want {}, got {got}", cols[0], cols[1]));
        }
        checked += 1;
    }
    assert!(checked > 8_000, "{checked}");
    assert!(
        failures.is_empty(),
        "{} of {checked} differ, first 15:\n{}",
        failures.len(),
        failures
            .iter()
            .take(15)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
