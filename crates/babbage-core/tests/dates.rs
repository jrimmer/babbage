use babbage_core::settings::{Settings, scoped};
use babbage_core::{FormatOptions, evaluate_with, format_value};

/// Saturday 2024-06-15 12:00 UTC.
const NOW: i64 = 1_718_452_800;

fn shown(line: &str) -> String {
    let settings = Settings::at(NOW);
    scoped(&settings, || {
        let v = evaluate_with(line, &settings).unwrap_or_else(|e| panic!("{line}: {e}"));
        format_value(&v, &FormatOptions::default()).text
    })
}

#[test]
fn the_reference_date_examples() {
    for (line, want) in [
        ("10 June + 3 weeks", "July 1"),
        ("3 weeks after March 14, 2019", "April 4, 2019"),
        ("Feb 28 + 1 month", "March 28"),
        ("January 31 2020 + 1 month", "February 29, 2020"),
        // Both without a year: the span forward from the first date (the reference app, Sheet G).
        ("February 5 - January 10", "11 months 5 days"),
        ("January 10 - February 5", "3 weeks 5 days"),
        ("Easter 2027", "March 28, 2027"),
        ("week number on march 12, 2021", "10"),
        ("day number on March 15, 2024", "75"),
        ("days in February 2020", "29 days"),
        ("midpoint between March 12 and April 5", "March 24"),
        (
            "March 12, 2023 as EEEE, MMM d, yyyy",
            "Sunday, Mar 12, 2023",
        ),
        ("day of the week on January 24, 1984", "Tuesday"),
        ("workdays in 3 weeks", "15 workdays"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
}

#[test]
#[ignore = "ticket 0018: show the unit spelling the user typed"]
fn the_reference_shows_the_plural_unit_names() {
    assert_eq!(shown("days in February 2020"), "29 days");
    assert_eq!(shown("workdays in 3 weeks"), "15 workdays");
}

const CORPUS: &str = include_str!("corpus/dates.tsv");

/// Whether `v` is exactly `want` of the unit named `unit`.
fn counts(v: &babbage_core::Value, want: &str, unit: &str) -> bool {
    let n = v.number();
    n.is_integer()
        && n.numer().to_string() == want
        && babbage_core::units::lookup(unit).is_some_and(|u| v.unit() == &u)
}

#[test]
fn agrees_with_the_date_oracle_on_every_line() {
    let settings = Settings::at(NOW);
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in CORPUS.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let (expr, mode, want) = (cols[0], cols[1], cols[2]);
        let ok = scoped(&settings, || match evaluate_with(expr, &settings) {
            Err(e) => Err(e.to_string()),
            Ok(v) => {
                let good = match mode {
                    "text" => format_value(&v, &FormatOptions::default()).text == want,
                    "days" => counts(&v, want, "day"),
                    "workdays" => counts(&v, want, "workday"),
                    "hours" => counts(&v, want, "h"),
                    other => panic!("unknown mode {other}"),
                };
                if good {
                    Ok(())
                } else {
                    Err(format!(
                        "got {}",
                        format_value(&v, &FormatOptions::default()).text
                    ))
                }
            }
        });
        if let Err(why) = ok {
            failures.push(format!("{expr}: want {want} ({mode}), {why}"));
        }
        checked += 1;
    }
    assert!(checked > 20_000, "{checked}");
    if let Ok(path) = std::env::var("BABBAGE_DUMP_FAILURES") {
        std::fs::write(path, failures.join("\n")).unwrap();
    }
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
