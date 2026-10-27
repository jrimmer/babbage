use babbage_core::settings::{Settings, scoped};
use babbage_core::{FormatOptions, evaluate_with, format_value};

/// Saturday 2024-06-15 12:00 UTC.
const NOW: i64 = 1_718_452_800;

fn settings(offset: i64) -> Settings {
    Settings {
        local_utc_offset_seconds: offset,
        ..Settings::at(NOW)
    }
}

fn shown(line: &str) -> String {
    let s = settings(0);
    scoped(&s, || {
        let v = evaluate_with(line, &s).unwrap_or_else(|e| panic!("{line}: {e}"));
        format_value(&v, &FormatOptions::default()).text
    })
}

const CORPUS: &str = include_str!("corpus/time.tsv");

#[test]
fn agrees_with_the_time_oracle_on_every_line() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in CORPUS.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let (expr, want) = (cols[0], cols[1]);
        let s = settings(cols[2].parse().unwrap());
        let got = scoped(&s, || match evaluate_with(expr, &s) {
            Ok(v) => format_value(&v, &FormatOptions::default()).text,
            Err(e) => format!("ERROR {e}"),
        });
        if got != want {
            failures.push(format!("{expr} @{}: want {want}, got {got}", cols[2]));
        }
        checked += 1;
    }
    if let Ok(path) = std::env::var("BABBAGE_DUMP_FAILURES") {
        std::fs::write(path, failures.join("\n")).unwrap();
    }
    assert!(checked > 50_000, "{checked}");
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
fn the_reference_time_examples() {
    for (line, want) in [
        ("4.54 hours as timespan", "4 hours 32 min 24 s"),
        ("72 days as timespan", "10 weeks 2 days"),
        ("3h 5m 10s in seconds", "11,110 s"),
        ("5.5 minutes as laptime", "00:05:30"),
        ("03:04:05 + 01:02:03", "04:06:08"),
        ("00:12:05 − 00:04:09", "00:07:56"),
        ("03:04:05 as timespan", "3 hours 4 min 5 s"),
        ("3 hours 4 minutes 5 seconds as laptime", "03:04:05"),
        ("12.5 minutes in minutes and seconds", "12 min 30 s"),
        ("1.4 weeks in hours and minutes", "235 hours 12 min"),
        ("1 hour 30 minutes at 1.5x", "1 hour"),
        ("16:00 + 3 hours 12 minutes", "7:12 PM"),
        ("7:30am to 8:45pm", "13 hours 15 min"),
        ("5pm - 3pm", "2 hours"),
        (
            "1733823083000 to date",
            "December 10, 2024 at 9:31:23 AM UTC",
        ),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
}

#[test]
#[ignore = "ticket 0018: show the unit spelling the user typed"]
fn the_reference_spells_units_as_typed() {
    assert_eq!(shown("5.5 minutes as timespan"), "5 min 30 s");
    assert_eq!(shown("4.5 weeks in days and hours"), "31 days 12 hours");
    assert_eq!(shown("1.4 weeks in hours and minutes"), "235 hours 12 min");
    assert_eq!(shown("time saved 5 min at 1.5x"), "1 min 40 s");
}
