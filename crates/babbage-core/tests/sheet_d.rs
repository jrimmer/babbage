//! The rules behind the owner's Sheet D answers, tested beyond the lines the owner typed: where each rule stops, and
//! what it must leave alone. The lines themselves are replayed in owner_reference.rs.

use babbage_core::settings::{Settings, scoped};
use babbage_core::zones::Zone;
use babbage_core::{FormatOptions, LineKind, SheetOptions, evaluate_sheet, format_value};

/// 2026-10-26 14:53 in Los Angeles.
const NOW: i64 = 1_793_051_580;
/// 2026-06-15 12:00 UTC.
const SUMMER: i64 = 1_781_524_800;

fn at(now: i64, line: &str, opts: &FormatOptions) -> String {
    let s = Settings {
        local_zone: Zone::named("America/Los_Angeles"),
        ..Settings::at(now)
    };
    let sheet = SheetOptions {
        settings: s.clone(),
        ..SheetOptions::default()
    };
    scoped(&s, || {
        let results = evaluate_sheet(&[line], &sheet);
        let r = &results[0];
        match &r.kind {
            LineKind::Value(v) => format_value(v, &r.display_options(opts)).text,
            LineKind::Error(e) => format!("ERROR {e}"),
            other => format!("{other:?}"),
        }
    })
}

fn run(line: &str) -> String {
    at(NOW, line, &FormatOptions::default())
}

fn all(cases: &[(&str, &str)]) {
    for (line, want) in cases {
        assert_eq!(run(line), *want, "{line}");
    }
}

#[test]
fn british_summer_time_is_london_time_so_it_depends_on_the_season() {
    let utc = |now, line| at(now, line, &FormatOptions::default());
    assert_eq!(utc(NOW, "noon BST in UTC"), "12:00");
    assert_eq!(utc(SUMMER, "noon BST in UTC"), "11:00");
}

#[test]
fn feet_and_inches_print_as_words_and_keep_that_layout_through_a_sum() {
    all(&[
        ("5 feet 6 inches", "5 feet 6 inches"),
        ("5 ft 6 in", "5 feet 6 inches"),
        ("5 feet 6 inches + 2 inches", "5 feet 8 inches"),
        ("5' 6\" + 2 inches", "5 feet 8 inches"),
        ("6\"", "6 inches"),
        ("13\"", "1 foot 1 inch"),
        ("1' 0\"", "1 foot"),
        // Alone, a length stays a decimal: only feet and inches written together are composite.
        ("5.5 ft", "5.5 ft"),
        ("5 ft + 3 ft", "8 ft"),
    ]);
}

#[test]
fn a_part_of_a_pound_reads_in_ounces_in_the_app_only() {
    all(&[
        ("13.5 lb", "13 lb 8 oz"),
        ("3 lb 4 oz", "3 lb 4 oz"),
        ("0.5 lb", "8 oz"),
        ("40 lb", "40 lb"),
        ("-1.5 lb", "-1 lb 8 oz"),
        ("2.5 kg", "2.5 kg"),
    ]);
    let plain = FormatOptions::new(10).unwrap();
    assert_eq!(at(NOW, "13.5 lb", &plain), "13.5 lb");
}

#[test]
fn a_time_of_day_alone_has_no_seconds_but_a_date_with_a_time_keeps_them() {
    all(&[
        ("3:45:30 pm", "3:45 PM"),
        ("noon + 90 seconds", "12:01 PM"),
        ("noon UTC in UTC", "12:00"),
    ]);
    assert_eq!(
        run("1733823083000 to date"),
        "December 10, 2024 at 1:31:23 AM"
    );
}

#[test]
fn an_hour_typed_h_stays_h_after_a_slash() {
    all(&[
        ("100 km/h", "100 km/h"),
        ("100 km/hour", "100 km/hour"),
        ("10 km / 2 hours", "5 km/hour"),
        ("100 km/h × 2 h", "200 km"),
        ("60 mph in km/h", "96.56064 km/h"),
        ("100 km/h in mph", "62.1371192237 mph"),
        ("100 km/h + 20 km/hour", "120 km/h"),
    ]);
}

#[test]
fn a_product_of_units_is_a_unit_only_if_it_measures_something() {
    all(&[
        ("2 N m", "2 N⋅m"),
        ("3 W s", "3 W s"),
        ("3 kg m", "3 kg"),
        ("3 kg m + 2 kg", "5 kg"),
        ("5 feet 3 inches", "5 feet 3 inches"),
    ]);
}

#[test]
fn an_amount_below_a_hundred_millionth_is_scientific() {
    all(&[
        ("0.000000001", "1e-9"),
        ("0.00000000004", "4e-11"),
        ("-0.000000002", "-2e-9"),
        ("0.0000000015", "1.5e-9"),
        ("0.00000001", "0.00000001"),
        ("0.00000000999", "9.99e-9"),
        ("0", "0"),
        ("1 ng in kg", "1e-12 kg"),
    ]);
}

#[test]
fn the_imperial_gallon_is_exact_and_its_pint_is_the_rounded_one() {
    all(&[
        ("1 imperial gallon in L", "4.54609 L"),
        ("1 liter in imperial gallons", "0.2199692483 gal"),
        ("1 liter in imperial pints", "1.7597547606 pt"),
        ("1 gallon in L", "3.785411784 L"),
    ]);
}

#[test]
fn a_year_less_date_is_this_years_wherever_it_falls() {
    all(&[
        ("January 1", "January 1"),
        ("January 12 + 3 weeks", "February 2"),
        ("December 28", "December 28"),
        ("days until January 1", "67 days"),
        ("days since January 1", "298 days"),
    ]);
}

#[test]
fn chinese_new_year_goes_past_2099() {
    all(&[
        ("Chinese New Year 2100", "February 9, 2100"),
        ("Chinese New Year 2025", "January 29, 2025"),
    ]);
}

#[test]
fn a_run_of_digits_joined_by_dots_is_not_a_number_and_is_skipped() {
    all(&[
        ("01.05.2005 + 1 day", "1 day"),
        ("version 1.2.3 costs $5", "$5.00"),
        // One dot is still a decimal point.
        ("1.5 + 1", "2.5"),
        ("1.5.2 + 1", "1"),
    ]);
}

#[test]
fn a_comma_after_a_stray_word_is_part_of_the_remark() {
    all(&[
        ("hotel, 3 nights × $120", "$360.00"),
        ("lunch, dinner, 3 × 4", "12"),
        // Commas that separate things stay.
        ("min(3, 4)", "3"),
        ("average of 10, 20, 30", "20"),
        ("April 1, 2019 + 1 day", "April 2, 2019"),
    ]);
}

#[test]
fn money_per_month_over_a_length_of_time_takes_that_time_as_its_period() {
    all(&[
        ("$500/month / 30 days", "$16.67/day"),
        ("$500/month / 2 weeks", "$250.00/week"),
        // Not money, so nothing changes: a speed over a time stays a plain division of units.
        ("100 m/s / 2 s", "50 m/s²"),
        ("$500 / 30 days", "$16.67/day"),
    ]);
}
