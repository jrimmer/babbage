use babbage_core::settings::{Settings, scoped};
use babbage_core::{FormatOptions, evaluate_with, format_value};

/// Saturday 2024-06-15 12:00 UTC.
const NOW: i64 = 1_718_452_800;

const CORPUS: &str = include_str!("corpus/zones.tsv");

fn settings(offset: i64) -> Settings {
    Settings {
        local_utc_offset_seconds: offset,
        ..Settings::at(NOW)
    }
}

fn run(line: &str, offset: i64) -> String {
    let s = settings(offset);
    scoped(&s, || match evaluate_with(line, &s) {
        Ok(v) => format_value(&v, &FormatOptions::default()).text,
        Err(e) => format!("ERROR {e}"),
    })
}

fn shown(line: &str) -> String {
    run(line, 0)
}

#[test]
fn agrees_with_the_zoneinfo_oracle_on_every_line() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in CORPUS.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let got = run(cols[0], cols[2].parse().unwrap());
        if got != cols[1] {
            failures.push(format!(
                "{} @{}: want {}, got {got}",
                cols[0], cols[2], cols[1]
            ));
        }
        checked += 1;
    }
    if let Ok(path) = std::env::var("BABBAGE_DUMP_FAILURES") {
        std::fs::write(path, failures.join("\n")).unwrap();
    }
    assert!(checked > 7_000, "{checked}");
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
fn the_reference_zone_examples() {
    for (line, want) in [
        ("6pm Sydney in Chicago", "3:00 am"),
        ("time in Paris", "2:00 pm"),
        ("Tokyo time", "9:00 pm"),
        ("date in Auckland", "June 16"),
        ("time difference between Sydney and Chicago", "15 hours"),
        ("difference between PST & GMT", "-8 hours"),
        ("3pm PST to GMT", "11:00 pm"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
}
