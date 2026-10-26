use babbage_core::settings::{Settings, scoped};
use babbage_core::{FormatOptions, evaluate_with, format_value};

const CORPUS: &str = include_str!("corpus/iso.tsv");

fn answer(line: &str) -> String {
    let s = Settings::at(1_718_452_800);
    scoped(&s, || match evaluate_with(line, &s) {
        Ok(v) => format_value(&v, &FormatOptions::default()).text,
        Err(e) => format!("ERR {e}"),
    })
}

#[test]
fn agrees_with_python_datetime_on_iso_8601_literals() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in CORPUS.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let got = answer(cols[0]);
        let ok = if cols[1] == "ERR" {
            got.starts_with("ERR")
        } else {
            got == cols[1]
        };
        if !ok {
            failures.push(format!("{}: want {}, got {got}", cols[0], cols[1]));
        }
        checked += 1;
    }
    assert!(checked > 6_000, "{checked}");
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
fn what_surrounds_an_iso_date_decides_whether_it_is_one() {
    // A date glued to a digit or a point is a different number, not an ISO date.
    for line in [
        "12019-04-01",
        "2019-04-011",
        "2019-04-01.5",
        "2019-04-01.",
        "x2019-04-01",
    ] {
        assert!(
            !answer(line).starts_with("April 1, 2019"),
            "{line}: {}",
            answer(line)
        );
    }
    assert_eq!(answer("(2019-04-01)"), "April 1, 2019");
    assert_eq!(
        answer("1 + 2019-04-01 - 1"),
        "ERR unsupported: a date or time can be shifted by a length of time, and two of them can be subtracted"
    );
    assert_eq!(answer("2019-04-01 + 1 day"), "April 2, 2019");
    assert_eq!(answer("2019-04-01 - 2019-03-01"), "4 weeks 3 days");
}
