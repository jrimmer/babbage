use babbage_core::settings::{Settings, scoped};
use babbage_core::zones::Zone;
use babbage_core::{FormatOptions, evaluate_with, format_value};

/// Saturday 2024-06-15 12:00 UTC.
const NOW: i64 = 1_718_452_800;

const CORPUS: &str = include_str!("corpus/localzone.tsv");

fn run(zone: &str, line: &str, now: i64) -> String {
    let s = Settings {
        local_zone: Some(Zone::named(zone).unwrap_or_else(|| panic!("{zone}"))),
        ..Settings::at(now)
    };
    scoped(&s, || match evaluate_with(line, &s) {
        Ok(v) => format_value(&v, &FormatOptions::default()).text,
        Err(e) => format!("ERROR {e}"),
    })
}

#[test]
fn agrees_with_the_zoneinfo_oracle_for_the_users_own_zone() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in CORPUS.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let got = run(cols[0], cols[1], NOW);
        if got != cols[2] {
            failures.push(format!(
                "{} {}: want {}, got {got}",
                cols[0], cols[1], cols[2]
            ));
        }
        checked += 1;
    }
    assert!(checked > 30_000, "{checked}");
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
fn now_follows_daylight_saving_through_the_year() {
    let january = 1_705_320_000; // 2024-01-15 12:00 UTC
    assert_eq!(run("America/New_York", "now", NOW), "June 15 at 8:00 AM");
    assert_eq!(
        run("America/New_York", "now", january),
        "January 15 at 7:00 AM"
    );
    assert_eq!(run("Australia/Sydney", "now", NOW), "June 15 at 10:00 PM");
    assert_eq!(
        run("Australia/Sydney", "now", january),
        "January 15 at 11:00 PM"
    );
}

#[test]
fn a_date_in_another_season_uses_that_seasons_offset() {
    assert_eq!(
        run("America/New_York", "January 15, 2024 3pm to timestamp", NOW),
        "1705348800"
    );
    assert_eq!(
        run("America/New_York", "June 15, 2024 3pm to timestamp", NOW),
        "1718478000"
    );
    assert_eq!(
        run("America/New_York", "January 15, 2024 3pm as iso8601", NOW),
        "2024-01-15T15:00:00-05:00"
    );
    assert_eq!(
        run("America/New_York", "June 15, 2024 3pm as iso8601", NOW),
        "2024-06-15T15:00:00-04:00"
    );
}

#[test]
fn a_time_with_no_place_is_in_the_users_zone() {
    assert_eq!(
        run("America/New_York", "3pm in Tokyo", NOW),
        "June 16 at 4:00 AM"
    );
    assert!(run("America/New_York", "3pm Tokyo in local", NOW).starts_with("ERROR"));
    assert_eq!(run("Asia/Tokyo", "3pm in Tokyo", NOW), "3:00 PM");
}
