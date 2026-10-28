//! Forms that sit next to a recognized phrase or unit shape and must not be mistaken for it, and the unit
//! arithmetic that cancels, merges or restates units.

use babbage_core::{FormatOptions, LineKind, SheetOptions, evaluate_sheet, format_value};

/// One line as a sheet reads it, so words that are only prose are skipped as they are in the app.
fn shown(line: &str) -> Result<String, String> {
    match &evaluate_sheet(&[line], &SheetOptions::default())[0].kind {
        LineKind::Value(v) => Ok(format_value(v, &FormatOptions::default()).text),
        LineKind::Error(e) => Err(e.to_string()),
        other => Err(format!("{other:?}")),
    }
}

fn ok(line: &str) -> String {
    shown(line).unwrap_or_else(|e| panic!("{line}: {e}"))
}

#[test]
fn units_of_one_kind_after_in_split_the_amount() {
    assert_eq!(ok("5.5 lb in lb oz"), "5 lb 8 oz");
    assert_eq!(ok("5.5 lb in lb and oz"), "5 lb 8 oz");
    assert!(shown("5.5 lb in lb lb").is_err());
    assert!(shown("5.5 lb in lb m").is_err());
    assert_eq!(ok("5.5 lb in lb foo"), "5.5 lb");
}

#[test]
fn conversions_inside_brackets_do_not_count_as_the_lines_keywords() {
    assert_eq!(ok("(5 m in cm) + (3 m in cm) in m"), "8 m");
    assert_eq!(ok("2 m + (5 m in cm) in m"), "7 m");
    assert_eq!(ok("(1 m in cm) in km"), "0.001 km");
    assert_eq!(ok("(5 m) to cm"), "500 cm");
    assert_eq!(ok("Boing is 5"), "5");
    assert!(shown("(2 + 3) in").is_err());
    assert!(shown("5 in at 3").is_err());
}

#[test]
fn a_unit_after_a_division_sign_is_one_of_it() {
    assert_eq!(ok("3 hours / day"), "3 hours/day");
    assert_eq!(ok("$99 per week"), "$99.00/week");
    assert_eq!(ok("3 hours per day"), "3 hours/day");
}

#[test]
fn rates_meet_in_the_longer_period_and_other_units_in_the_larger() {
    assert_eq!(ok("$20/day + $300/week"), "$440.00/week");
    assert_eq!(ok("$300/week + $20/day"), "$440.00/week");
    assert_eq!(ok("1 km + 500 m"), "1.5 km");
    assert_eq!(ok("500 m + 1 km"), "1.5 km");
    assert_eq!(ok("2 hours/day + 30 min/day"), "0.1041666667");
}

#[test]
fn powers_of_two_length_units_multiply_in_the_left_unit() {
    assert_eq!(ok("2 ft^2 * 12 in"), "2 ft³");
    assert_eq!(ok("2 in * 3 ft^2"), "864 in³");
    assert_eq!(ok("2 ft^2 * 3 ft"), "6 ft³");
    assert_eq!(ok("2 ft^2 * 3 s"), "6 ft² s");
}

#[test]
fn a_rate_is_restated_for_another_period_only_when_it_has_one() {
    assert_eq!(ok("$30/day is what per year"), "$10,957.28/year");
    assert_eq!(ok("$30/day is what per hour"), "$1.25/hour");
    assert_eq!(ok("$30/month is what per day"), "$0.99/day");
    assert!(shown("$30 is what per year").is_err());
    assert!(shown("$30/day is what per meter").is_err());
}

#[test]
fn a_monthly_rate_over_days_is_a_daily_rate_and_over_hours_keeps_both_units() {
    assert_eq!(ok("$500/month / 30 days"), "$16.67/day");
    assert_eq!(ok("$500/month / 2 hours"), "250 $/(hour·month)");
    assert_eq!(ok("$100/day / 2 hours"), "50 $/(hour·day)");
}

#[test]
fn units_of_one_kind_cancel_inside_a_product_or_quotient() {
    assert_eq!(ok("3 km/m"), "3,000");
    assert_eq!(ok("3 km / 2 m"), "1,500");
    assert_eq!(ok("5 km * 2 / m"), "10,000");
    assert_eq!(ok("6 m/s * 2 s"), "12 m");
    assert_eq!(ok("60 mph / 30 mph"), "2");
    assert_eq!(ok("10 kg / 2 lb"), "11.0231131092");
    assert_eq!(ok("3 mph * 2 minutes"), "0.1 mi");
}

#[test]
fn a_plain_ratio_can_be_shown_as_a_rate() {
    assert_eq!(ok("3 hours / 3 days as hour/day"), "1 hour/day");
    assert_eq!(ok("8g/2 lb as grams/lb"), "4 g/lb");
}

fn written(line: &str) -> String {
    let toks = babbage_core::lexer::lex(line).unwrap();
    babbage_core::phrase::parse_line(&toks).unwrap().to_string()
}

#[test]
fn a_parsed_line_is_written_back_with_every_call_and_phrase_argument() {
    assert_eq!(written("max(1, 2, 3)"), "Max(1, 2, 3)");
    assert_eq!(written("10 combinations 3"), "Combinations(10, 3)");
    assert_eq!(written("5 m in cm"), "((5 m) in cm)");
}

#[test]
fn a_plain_number_can_be_shown_as_a_rate() {
    assert_eq!(ok("1 as hour/day"), "24 hours/day");
    assert_eq!(ok("0.5 as hours/day"), "12 hours/day");
    assert_eq!(ok("5 as g/lb"), "2,267.96185 g/lb");
    assert_eq!(ok("5 as km/mi"), "8.04672 km/mi");
    assert!(shown("5 as m").is_err());
    assert_eq!(ok("1 hour / day as hours/day"), "1 hour/day");
}

#[test]
fn a_conversion_at_a_rate_needs_a_target_and_a_rate() {
    for line in ["5 m to cm at", "5 m to at 2", "5 m to", "to cm"] {
        let e = shown(line).unwrap_err();
        assert!(e.starts_with("unexpected word"), "{line}: {e}");
    }
    for line in [
        "5 m to cm at 2",
        "5 m to cm at 2 cm/m",
        "3 hours to minutes at 60",
    ] {
        let e = shown(line).unwrap_err();
        assert!(e.contains("convert without one"), "{line}: {e}");
    }
    assert_eq!(ok("$500 at $20/hour"), "25 hours");
    assert!(
        shown("30 hours at $30")
            .unwrap_err()
            .contains("a rate such as")
    );
}

#[test]
fn a_trailing_prose_word_after_brackets_is_skipped() {
    assert_eq!(ok("(2 + 3) * 2 is"), "10");
    assert_eq!(ok("Boing (2 + 3) is"), "5");
}

#[test]
fn a_spread_of_money_is_money() {
    assert_eq!(ok("stddev($1, $3)"), "$1.41");
    assert_eq!(ok("stddev(1 hour, 3 hour)"), "1.4142135624 hours");
    assert!(shown("stddev(1 m, 3 kg)").is_err());
    assert!(shown("stddev($1, 3)").is_err());
}

#[test]
fn the_phrase_forms_write_back_with_their_arguments() {
    assert_eq!(written("20 is 5% of what"), "IsPercentOfWhat(20, (5%))");
    assert_eq!(written("5 km in 25 min"), "Pace((5 km), (25 min))");
}

#[test]
fn a_plain_number_shown_in_a_unit_of_another_kind_is_refused() {
    for line in ["5 as hour/m", "5 as m/s", "5 as s/m", "5 as rad", "5 as hz"] {
        assert_eq!(
            shown(line).unwrap_err(),
            "these units measure different things",
            "{line}"
        );
    }
    assert_eq!(ok("5 as m/km"), "5,000 m/km");
    assert_eq!(ok("5 as %"), "500%");
}

#[test]
fn fuel_economy_cancels_against_what_it_measures() {
    assert_eq!(ok("10 mpg * 3 gallons"), "30 mi");
    assert_eq!(ok("3 gallons * 10 mpg"), "30 mi");
    assert_eq!(ok("300 mi / 30 mpg"), "10 gal");
    assert_eq!(ok("8 L/100km * 250 km"), "20 L");
    assert_eq!(ok("250 km * 8 L/100km"), "20 L");
    assert_eq!(ok("20 L / 8 L/100km"), "250 km");
    // Another unit of the same kind is converted first, and a mismatched kind keeps both units.
    assert_eq!(ok("30 mpg * 3.785411784 L"), "30 mi");
    assert_eq!(ok("300 miles / 30 mpg in gallons"), "10 gal");
    assert_eq!(ok("10 mpg * 3 m"), "30 MPG m");
    assert_eq!(ok("10 mpg * 2"), "20 MPG");
}

#[test]
fn a_percentage_times_a_plain_number_is_the_share_not_a_percentage() {
    assert_eq!(ok("5% * 2"), "0.1");
    assert_eq!(ok("25% * 200"), "50");
    assert_eq!(ok("2 * 5%"), "0.1");
}
