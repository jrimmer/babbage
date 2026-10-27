//! Forms a reader types in words or with a different mark: fractional powers, `not`, factorials, `int`, `half(...)`,
//! `round ... to N dp`, pence, `% discount on`, `percent of`, unit spellings and a currency code before an amount.

use babbage_core::{FormatOptions, SheetOptions, evaluate, evaluate_sheet, format_value};

fn shown(line: &str) -> String {
    format_value(
        &evaluate(line).unwrap_or_else(|e| panic!("{line}: {e}")),
        &FormatOptions::default(),
    )
    .text
}

fn exact(line: &str) -> bool {
    format_value(&evaluate(line).unwrap(), &FormatOptions::default()).exact
}

#[test]
fn a_fraction_as_the_exponent_is_a_root() {
    for (line, want) in [
        ("4 ^ 0.5", "2"),
        ("8 ^ (1/3)", "2"),
        ("27 ^ (2/3)", "9"),
        ("100 ^ 1.5", "1,000"),
        ("2 ^ 0.5", "1.4142135624"),
        ("2 ^ -0.5", "0.7071067812"),
        ("0 ^ 0.5", "0"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
    assert!(exact("4 ^ 0.5"));
    assert!(!exact("2 ^ 0.5"));
    assert!(evaluate("(-4) ^ 0.5").is_err());
    assert!(evaluate("5 m ^ 0.5").is_err());
}

#[test]
fn not_factorial_and_the_whole_part() {
    for (line, want) in [
        ("not true", "false"),
        ("!true", "false"),
        ("not (1 == 2)", "true"),
        ("5 factorial", "120"),
        ("5!", "120"),
        ("(2 + 3)!", "120"),
        ("3 != 4", "true"),
        ("int(3.9)", "3"),
        ("int(-3.9)", "-3"),
        ("trunc(7.5)", "7"),
        ("half(50)", "25"),
        ("half of 50", "25"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
}

#[test]
fn rounding_to_places_in_words() {
    assert_eq!(shown("round 2.567 to 1 dp"), "2.6");
    assert_eq!(shown("round 2.567 to 2 decimal places"), "2.57");
    assert_eq!(shown("round 2.567"), "3");
    assert_eq!(shown("round(2.567)"), "3");
}

#[test]
fn pence_percent_words_and_discounts() {
    for (line, want) in [
        ("£2 + 50p", "£2.50"),
        ("£2 + 50 pence", "£2.50"),
        ("30% discount on $200", "$140.00"),
        ("25% off $120", "$90.00"),
        ("15 percent of 80", "12"),
        ("15 % of 200", "30"),
        ("15 % 4", "3"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
}

#[test]
fn unit_spellings_and_a_code_before_the_amount() {
    for (line, want) in [
        ("5 ml in L", "0.005 L"),
        ("5 cl in mL", "50 mL"),
        ("5 KB in B", "5,000 B"),
        ("5 uF in farad", "0.000005 farad"),
        ("CHF 5", "5.00 CHF"),
        ("3 * CHF 5", "15.00 CHF"),
        ("CHF 5 + CHF 2", "7.00 CHF"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
}

#[test]
fn a_rate_written_after_the_hours_cancels_them() {
    for line in [
        "3 hours x $25/hour",
        "$25/hour * 3 hours",
        "3 hours * $25 / hour",
    ] {
        assert_eq!(shown(line), "$75.00", "{line}");
    }
    assert_eq!(shown("10 m * 5 / s"), "50 m/s");
    assert_eq!(shown("6 * 4 / 2"), "12");
}

#[test]
fn a_word_beside_a_number_or_before_a_bracket_is_marked_unclear() {
    let results = evaluate_sheet(
        &["3 widgets * 4", "blorp (45)", "I spent $45 on clothes"],
        &SheetOptions::default(),
    );
    let unclear = |i: usize| -> Vec<&str> {
        results[i]
            .ignored
            .iter()
            .filter(|w| w.unclear)
            .map(|w| w.text.as_str())
            .collect()
    };
    assert_eq!(unclear(0), ["widgets"]);
    assert_eq!(unclear(1), ["blorp"]);
    assert!(unclear(2).is_empty());
}

#[test]
fn total_alone_adds_every_line_above() {
    use babbage_core::{LineKind, sheet::floating_total};
    let lines = ["$10", "$5", "subtotal", "$20", "total", "$1", "grand total"];
    let opts = SheetOptions::default();
    let results = evaluate_sheet(&lines, &opts);
    let text = |i: usize| match &results[i].kind {
        LineKind::Value(v) => format_value(v, &FormatOptions::default()).text,
        other => panic!("line {i}: {other:?}"),
    };
    assert_eq!(text(2), "$15.00");
    assert_eq!(text(4), "$35.00");
    assert_eq!(text(6), "$36.00");
    // Total lines are not counted again by the floating total.
    let floating = floating_total(&results, &opts).unwrap().unwrap();
    assert_eq!(
        format_value(&floating, &FormatOptions::default()).text,
        "$36.00"
    );
    // Nothing above it: the line stays empty.
    let alone = evaluate_sheet(&["total"], &opts);
    assert!(matches!(alone[0].kind, LineKind::Text));
}

/// The answer of one line as a sheet gives it, where a word that is not understood is skipped; `None` for an error.
fn in_sheet(line: &str) -> Option<String> {
    use babbage_core::LineKind;
    match &evaluate_sheet(&[line], &SheetOptions::default())[0].kind {
        LineKind::Value(v) => Some(format_value(v, &FormatOptions::default()).text),
        _ => None,
    }
}

#[test]
fn near_misses_of_the_spoken_forms_are_left_alone() {
    // `round` is dropped only for `to N dp` (or places) after the number.
    assert_eq!(in_sheet("round 2.567 to 1 dp").as_deref(), Some("2.6"));
    assert_eq!(in_sheet("round 2.567 in 1 dp"), None);
    assert_eq!(in_sheet("round 2.567 to 1 inch"), None);
    assert_eq!(in_sheet("round 2.567 to foo dp").as_deref(), Some("3"));
    // `discount` is `off` only after a percentage and before on/off.
    assert_eq!(in_sheet("discount on $200"), None);
    assert_eq!(in_sheet("30% discount $200"), None);
    assert_eq!(in_sheet("30% tax on $200").as_deref(), Some("$260.00"));
    // A factorial of a bracket takes the whole bracket, wherever it stands in the line.
    assert_eq!(in_sheet("1 + (2 + 3)!").as_deref(), Some("121"));
    assert_eq!(in_sheet("2 * (1 + 2)!").as_deref(), Some("12"));
}

#[test]
fn total_alone_counts_declaration_lines_like_the_chip() {
    use babbage_core::LineKind;
    let results = evaluate_sheet(&["rent = $900", "$10", "total"], &SheetOptions::default());
    let LineKind::Value(v) = &results[2].kind else {
        panic!("a value");
    };
    assert_eq!(format_value(v, &FormatOptions::default()).text, "$910.00");
}
