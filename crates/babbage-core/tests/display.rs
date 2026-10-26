use babbage_core::{FormatOptions, evaluate, format_value};

const CORPUS: &str = include_str!("corpus/display.tsv");

fn shown(line: &str) -> String {
    format_value(
        &evaluate(line).unwrap_or_else(|e| panic!("{line}: {e}")),
        &FormatOptions::default(),
    )
    .text
}

#[test]
fn default_display_matches_the_decimal_oracle() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in CORPUS.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let got = evaluate(cols[0]);
        let ok = got.as_ref().is_ok_and(|v| {
            format!("{}/{}", v.number().numer(), v.number().denom()) == cols[1]
                && format_value(v, &FormatOptions::default()).text == cols[2]
        });
        if !ok {
            failures.push(format!(
                "{}: want {} {}, got {:?}",
                cols[0],
                cols[1],
                cols[2],
                got.map(|v| format_value(&v, &FormatOptions::default()).text)
            ));
        }
        checked += 1;
    }
    assert!(checked > 4000);
    assert!(
        failures.is_empty(),
        "{} of {checked} differ, first 10:\n{}",
        failures.len(),
        failures
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn example_lines() {
    assert_eq!(shown("100,000 + 200,000"), "300k");
    assert_eq!(shown("3 million + 10%"), "3.3M");
    assert_eq!(shown("$0".trim_start_matches('$')), "0");
    assert_eq!(shown("99,999"), "99,999");
    assert_eq!(shown("100,000"), "100k");
    assert_eq!(shown("-250,000"), "-250k");
    assert_eq!(shown("1,234,567"), "1.234567M");
    assert_eq!(shown("999,999.99999999999"), "1M");
    assert_eq!(shown("2.5k + 2.5k"), "5,000");
    assert_eq!(shown("5M"), "5M");
    assert_eq!(shown("5M km"), "5M km");
    assert_eq!(shown("64000000 mm³"), "64M mm³");
}

#[test]
fn abbreviation_never_hides_a_digit() {
    let n = evaluate("123456789.123456789").unwrap();
    let text = format_value(&n, &FormatOptions::default());
    assert_eq!(text.text, "123.4567891235M");
    assert!(!text.exact);
    let exact = format_value(&evaluate("123456789").unwrap(), &FormatOptions::default());
    assert_eq!(exact.text, "123.456789M");
    assert!(exact.exact);
}

#[test]
fn abbreviation_can_be_switched_off() {
    let mut o = FormatOptions::default();
    o.abbreviate_large = false;
    assert_eq!(
        format_value(&evaluate("300000").unwrap(), &o).text,
        "300,000"
    );
}

#[test]
fn suffixes_are_only_suffixes_when_they_end_the_word() {
    assert!(evaluate("5 kg").is_ok());
    assert_eq!(shown("5kg"), "5 kg");
    assert_eq!(shown("5Mb"), "5 Mb");
    assert_eq!(shown("5GB"), "5 GB");
    assert_eq!(shown("5 TB"), "5 TB");
    assert!(evaluate("5 M").is_err());
}
