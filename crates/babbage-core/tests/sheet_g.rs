//! Behaviour settled by the owner's Sheet G (see `owner_reference.rs` for the lines as typed) and the cases around
//! them: the rule each answer implies, tried on neighbouring lines.

use babbage_core::{FormatOptions, LineKind, SheetOptions, evaluate, evaluate_sheet, format_value};

fn shown(line: &str) -> String {
    let v = evaluate(line).unwrap_or_else(|e| panic!("{line}: {e}"));
    format_value(&v, &FormatOptions::default()).text
}

/// The last line of a sheet, as the app shows it.
fn last_of(lines: &[&str]) -> String {
    let opts = SheetOptions::default();
    let results = evaluate_sheet(lines, &opts);
    let r = results.last().expect("a line");
    match &r.kind {
        LineKind::Value(v) => format_value(v, &r.display_options(&FormatOptions::default())).text,
        LineKind::Error(e) => format!("ERROR {e}"),
        other => format!("{other:?}"),
    }
}

#[test]
fn a_temperature_takes_a_plain_number_a_percentage_or_its_own_scale() {
    assert_eq!(shown("20 C + 5 C"), "25 °C");
    assert_eq!(shown("20 C - 5 C"), "15 °C");
    assert_eq!(shown("20 C + 5"), "25 °C");
    assert_eq!(shown("5 + 20 C"), "25 °C");
    assert_eq!(shown("20 C + 10%"), "22 °C");
    assert_eq!(shown("20 C - 10%"), "18 °C");
}

#[test]
fn a_temperature_is_scaled_by_a_plain_number_and_converts_afterwards() {
    assert_eq!(shown("20 C × 2"), "40 °C");
    assert_eq!(shown("2 × 20 C"), "40 °C");
    assert_eq!(shown("20 C / 2"), "10 °C");
    assert_eq!(shown("20 C + 5 C in F"), "77 °F");
}

#[test]
fn two_scales_or_a_product_of_temperatures_stay_refused() {
    assert!(evaluate("20 C + 5 F").is_err());
    assert!(evaluate("20 C × 20 C").is_err());
    assert!(evaluate("2 / 20 C").is_err());
}

#[test]
fn a_number_straight_before_a_constant_multiplies_it() {
    for line in ["2pi", "2 pi", "2π", "2 * pi"] {
        assert_eq!(shown(line), "6.2831853072", "{line}");
    }
    assert_eq!(shown("3 tau"), "18.8495559215");
    assert_eq!(shown("5 phi"), "8.0901699437");
    assert_eq!(shown("2 pi + 1"), "7.2831853072");
    assert_eq!(shown("2 pi radians in degrees"), "360°");
}

#[test]
fn trig_of_a_multiple_of_pi_is_zero_not_a_remainder_of_the_precision() {
    for line in [
        "sin(pi)",
        "sin(2pi)",
        "sin(3pi)",
        "cos(pi/2)",
        "tan(pi)",
        "sin(2pi)^2",
    ] {
        assert_eq!(shown(line), "0", "{line}");
    }
    assert_eq!(shown("cos(2pi)"), "1");
}

#[test]
fn only_the_precision_remainder_is_snapped_to_zero() {
    // A tiny argument has a tiny sine, and an argument that is not a multiple of pi keeps its small answer.
    assert_eq!(shown("sin(1e-200)"), "1e-200");
    assert_eq!(shown("sin(0.001)"), "0.0009999998");
    assert_eq!(shown("cos(1.5707963267948966)"), "1.9231321692e-17");
    assert_eq!(shown("sin(1)"), "0.8414709848");
}

#[test]
fn a_tag_followed_by_total_adds_the_tagged_lines() {
    let sheet = ["$5 #jude", "$7 #jude", "$9", "#jude total"];
    assert_eq!(last_of(&sheet), "$12.00");
    // The long form is the same.
    let sheet = ["$5 #jude", "$7 #jude", "$9", "total of #jude"];
    assert_eq!(last_of(&sheet), "$12.00");
}

#[test]
fn a_statistics_word_alone_on_a_line_shows_no_answer() {
    for word in ["average", "avg", "count", "median", "Average", "  count  "] {
        assert_eq!(last_of(&["4", "6", word]), "Text", "{word}");
    }
    // With something to work on it is still a function; other words alone are unchanged.
    assert_eq!(last_of(&["average of 3 and 5"]), "4");
    assert_eq!(last_of(&["average 3"]), "3");
}

#[test]
fn an_hour_is_fifteen_degrees_of_turning() {
    assert_eq!(shown("12h 30m in degrees"), "187.5°");
    assert_eq!(shown("1 hour in degrees"), "15°");
    assert_eq!(shown("6 hours in radians"), "1.5707963268 rad");
    assert_eq!(shown("1 degree in minutes"), "4 min");
    assert_eq!(shown("10 degrees in seconds"), "2,400 s");
}

#[test]
fn degrees_in_hours_read_hours_minutes_seconds() {
    assert_eq!(shown("90 degrees in hours"), "6h 0m 0s");
    assert_eq!(shown("100 degrees in hours"), "6h 40m 0s");
    assert_eq!(shown("0.5 degrees in hours"), "0h 2m 0s");
    assert_eq!(shown("-45 degrees in hours"), "-3h 0m 0s");
    // Other time units are plain amounts.
    assert_eq!(shown("90 degrees in days"), "0.25 days");
}

#[test]
fn time_is_still_not_a_length() {
    assert!(evaluate("3 hours in km").is_err());
    assert!(evaluate("3 degrees in kg").is_err());
}

#[test]
fn litres_per_hundred_kilometres_is_a_unit_and_keeps_its_name() {
    assert_eq!(shown("8 L/100km"), "8 L/100km");
    assert_eq!(shown("8 l/100 km"), "8 L/100km");
    assert_eq!(shown("8 liters/100km"), "8 L/100km");
    assert_eq!(shown("8 L/100km + 2 L/100km"), "10 L/100km");
    assert_eq!(shown("8 L/100km × 2"), "16 L/100km");
    // With spaces around the slash it is still a division of a volume by a distance.
    assert_eq!(shown("8 L / 100 km"), "0.08 L/km");
}

#[test]
fn fuel_economy_converts_both_ways_through_the_reciprocal() {
    assert_eq!(shown("8 L/100km in mpg"), "29.4018229167 MPG");
    assert_eq!(shown("30 mpg in L/100km"), "7.8404861111 L/100km");
    assert_eq!(shown("8 L/100km in km/L"), "12.5 km/L");
    assert_eq!(shown("10 km/L in L/100km"), "10 L/100km");
    assert_eq!(shown("10 km/L in mpg"), "23.5214583333 MPG");
    assert_eq!(shown("1 mpg in km/L"), "0.4251437074 km/L");
}

#[test]
fn only_litres_per_hundred_kilometres_is_turned_upside_down() {
    assert!(evaluate("8 L/100km in m").is_err());
    assert!(evaluate("0 L/100km in mpg").is_err());
    assert!(evaluate("5 m in 1/m").is_err());
}

#[test]
fn molar_is_moles_per_litre() {
    assert_eq!(shown("1 molar in mol/L"), "1 mol/L");
    assert_eq!(shown("2 molar in mmol/L"), "2,000 mmol/L");
    assert_eq!(shown("0.5 mol/L in molar"), "0.5 molar");
    assert!(evaluate("1 molar in kg").is_err());
}

#[test]
fn two_dates_without_a_year_subtract_to_the_span_forward_from_the_first() {
    // Sheet G (ticket 0041): `January 10 - February 5` is `3 weeks 5 days`, and `February 5 - January 10` is the
    // span on to the next January 10.
    let at_noon = |line: &str| {
        let s = babbage_core::settings::Settings::at(1_793_039_340);
        babbage_core::settings::scoped(&s, || shown(line))
    };
    assert_eq!(at_noon("January 10 - February 5"), "3 weeks 5 days");
    assert_eq!(at_noon("February 5 - January 10"), "11 months 5 days");
    assert_eq!(at_noon("Jan 10 - Feb 5"), "3 weeks 5 days");
    assert_eq!(at_noon("5 February - 10 January"), "11 months 5 days");
    assert_eq!(at_noon("December 25 - January 5"), "1 week 4 days");
    // With a year written the difference keeps its sign, as before.
    assert_eq!(
        at_noon("February 5, 2026 - January 10, 2026"),
        "3 weeks 5 days"
    );
    assert_eq!(
        at_noon("January 10, 2026 - February 5, 2026"),
        "-3 weeks 5 days"
    );
}

#[test]
fn every_tag_form_has_both_spellings_and_every_statistic() {
    let tagged = |last: &str| last_of(&["4 #a", "6 #a", "8 #b", last]);
    for (line, want) in [
        ("total of #a", "10"),
        ("#a total", "10"),
        ("sum of #a", "10"),
        ("#a sum", "10"),
        ("average of #a", "5"),
        ("#a average", "5"),
        ("avg of #a", "5"),
        ("#a avg", "5"),
        ("count of #a", "2"),
        ("#a count", "2"),
        ("#b total", "8"),
    ] {
        assert_eq!(tagged(line), want, "{line}");
    }
}

#[test]
fn a_tag_name_may_hold_letters_digits_underscores_and_hyphens_only() {
    let sheet = |tag: &str| {
        let line = format!("5 #{tag}");
        let total = format!("#{tag} total");
        last_of(&[line.as_str(), total.as_str()])
    };
    for tag in ["trip", "trip2", "my_trip", "my-trip", "2024"] {
        assert_eq!(sheet(tag), "5", "{tag}");
    }
    // A dot ends the tag, so what follows is not a tag total.
    assert_ne!(last_of(&["5 #a", "#a.b total"]), "5");
    assert_ne!(last_of(&["5 #a", "#a! total"]), "5");
}

/// The refusal says it is about temperatures, not some other rule.
fn refused_as_temperature(line: &str) {
    match evaluate(line) {
        Err(babbage_core::Error::UnsupportedOperation(m)) => {
            assert!(m.contains("temperatures can be converted"), "{line}: {m}")
        }
        other => panic!("{line}: {other:?}"),
    }
}

#[test]
fn temperatures_are_refused_with_the_temperature_message_in_every_other_combination() {
    for line in [
        "20 C + 5 kg",
        "20 C + 5 F",
        "10% + 20 C",
        "20 C / 2 kg",
        "20 C / 2 C",
        "20 C / 10%",
        "5 kg / 20 C",
        "20 C × 20 C",
        "2 / 20 C",
    ] {
        refused_as_temperature(line);
    }
}

#[test]
fn a_percentage_divided_by_a_quantity_is_refused() {
    assert!(evaluate("50% / 2 kg").is_err());
    assert!(evaluate("50% / 2 m").is_err());
}

#[test]
fn hours_minutes_seconds_are_only_exact_when_every_part_is() {
    let exact = |line: &str| {
        let v = evaluate(line).expect("a value");
        format_value(&v, &FormatOptions::default()).exact
    };
    assert!(exact("90 degrees in hours"));
    assert!(!exact("(1/7) degrees in hours"));
    assert!(!exact("1 radian in hours"));
}
