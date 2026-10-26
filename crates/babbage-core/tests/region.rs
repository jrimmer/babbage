use babbage_core::settings::{Settings, scoped};
use babbage_core::{NumberFormat, evaluate_with, format_value};

const CORPUS: &str = include_str!("corpus/region.tsv");

fn settings(format: NumberFormat) -> Settings {
    Settings {
        number_format: format,
        ..Settings::default()
    }
}

fn format_of(code: &str) -> NumberFormat {
    match code {
        "na" => NumberFormat::NorthAmerica,
        "we" => NumberFormat::WesternEurope,
        "ee" => NumberFormat::EasternEurope,
        other => panic!("unknown region {other}"),
    }
}

fn run(format: NumberFormat, line: &str) -> Result<String, String> {
    let s = settings(format);
    scoped(&s, || {
        evaluate_with(line, &s)
            .map(|v| format_value(&v, &s.format_options().unabbreviated()).text)
            .map_err(|e| e.to_string())
    })
}

#[test]
fn agrees_with_the_decimal_oracle_in_every_region() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in CORPUS.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let got = run(format_of(cols[0]), cols[1]);
        if got.as_deref() != Ok(cols[2]) {
            failures.push(format!(
                "{} {}: want {:?}, got {got:?}",
                cols[0], cols[1], cols[2]
            ));
        }
        checked += 1;
    }
    assert!(checked > 20_000, "{checked}");
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

#[test]
fn a_mark_that_is_not_this_regions_is_an_error_or_a_separator_never_a_guess() {
    use NumberFormat::{EasternEurope as Ee, NorthAmerica as Na, WesternEurope as We};
    for (format, line) in [
        (We, "3.14"),
        (We, "1.5 + 1"),
        (We, "1.2345"),
        (We, "1.23"),
        (We, "1,2,3"),
        (Ee, "1 23"),
        (Ee, "1,2,3"),
        (Ee, "1.2.3"),
        (Na, "1.2.3"),
        (Na, "1,23 + 1"),
        (Na, "1234,567"),
        (We, "1234.567"),
    ] {
        assert!(
            run(format, line).is_err(),
            "{line} should fail in {format:?}: {:?}",
            run(format, line)
        );
    }
}

#[test]
fn the_reference_region_examples() {
    use NumberFormat::{EasternEurope as Ee, NorthAmerica as Na, WesternEurope as We};
    for (format, line, want) in [
        (Na, "1_000_000 + 2_000", "1,002,000"),
        (Na, "0b_1100_0001 in decimal", "193"),
        (Na, "0xCAFE_F00D in decimal", "3,405,705,229"),
        (We, "1.234,5 + 1", "1.235,5"),
        (Ee, "1 234,5 + 1", "1\u{a0}235,5"),
        (Ee, "1 234.5 + 1", "1\u{a0}235,5"),
        (Ee, "1\u{a0}234,5 + 1", "1\u{a0}235,5"),
        (Ee, "1\u{202f}234,5 + 1", "1\u{a0}235,5"),
        (We, "max(1,5; 2,5)", "2,5"),
        (Na, "max(1;5)", "5"),
    ] {
        assert_eq!(
            run(format, line).as_deref(),
            Ok(want),
            "{line} in {format:?}"
        );
    }
}

#[test]
fn money_and_percentages_use_the_regions_marks() {
    use NumberFormat::{EasternEurope as Ee, NorthAmerica as Na, WesternEurope as We};
    for (format, line, want) in [
        (We, "$1.234,5 + $1", "$1.235,50"),
        (Ee, "$1 234,5 + $1", "$1\u{a0}235,50"),
        (Na, "$1,234.5 + $1", "$1,235.50"),
        (We, "12,5% of 80", "10"),
        (We, "1.234,5 km in m", "1.234.500 m"),
        (We, "1.234,5678 to 2 dp", "1.234,57"),
    ] {
        assert_eq!(
            run(format, line).as_deref(),
            Ok(want),
            "{line} in {format:?}"
        );
    }
}
