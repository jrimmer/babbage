use babbage_core::settings::{Settings, scoped};
use babbage_core::{FormatOptions, evaluate_with, format_value};

const CORPUS: &str = include_str!("corpus/workdays.tsv");

fn run(hours: u32, line: &str) -> String {
    let s = Settings {
        work_hours_per_day: Some(hours),
        ..Settings::default()
    };
    scoped(&s, || match evaluate_with(line, &s) {
        Ok(v) => format_value(&v, &FormatOptions::default().unabbreviated()).text,
        Err(e) => format!("ERROR {e}"),
    })
}

fn shown(line: &str) -> String {
    run(8, line)
}

#[test]
fn agrees_with_the_working_time_oracle_for_every_work_hours_setting() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in CORPUS.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let got = run(cols[0].parse().unwrap(), cols[1]);
        if got != cols[2] {
            failures.push(format!(
                "{}h {}: want {}, got {got}",
                cols[0], cols[1], cols[2]
            ));
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

#[test]
fn a_week_is_five_workdays_as_in_the_reference() {
    for (line, want) in [
        ("$500/workday * 4 weeks", "$10,000.00"),
        ("$500 * 5 * 4", "$10,000.00"),
        ("4 weeks in workdays", "20 workdays"),
        ("1 week in workdays", "5 workdays"),
        ("20 workdays in weeks", "4 weeks"),
        ("55h in work days", "6.875 workdays"),
        ("1 week in hours", "168 hours"),
        ("8 hours in workdays", "1 workday"),
        ("1 week == 5 workdays", "true"),
        ("1 week > 4 workdays", "true"),
        ("1 week < 6 workdays", "true"),
        ("$500/workday in $/week", "$2,500.00/week"),
        // Checked on a real reference app by the owner.
        ("1 day in workdays", "0.7142857143 workdays"),
        ("1 month in workdays", "21.740625 workdays"),
        ("1 year in workdays", "260.8875 workdays"),
        ("$500/workday * 10 days", "$3,571.43"),
        ("$500/workday * 10 workdays", "$5,000.00"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
}

#[test]
fn working_time_follows_the_work_hours_setting_but_a_week_stays_five_workdays() {
    assert_eq!(run(6, "1 week in workdays"), "5 workdays");
    assert_eq!(run(6, "12 hours in workdays"), "2 workdays");
    assert_eq!(run(10, "1 workday in hours"), "10 hours");
    assert_eq!(run(10, "$100/workday * 2 weeks"), "$1,000.00");
}

#[test]
fn time_with_no_workday_in_it_is_left_alone() {
    for (line, want) in [
        ("1 week in days", "7 days"),
        ("1 week in hours", "168 hours"),
        ("2 weeks + 3 days", "2.4285714286 weeks"),
        ("1 month in days", "30.436875 days"),
        ("1 year in weeks", "52.1775 weeks"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
}

#[test]
fn workdays_still_move_dates_over_weekends() {
    assert_eq!(shown("January 5, 2024 + 1 workday"), "January 8, 2024");
    assert_eq!(shown("workdays in 3 weeks"), "15 workdays");
}
