//! A word the engine skips beside a number is flagged, never refused, and the words with one clear meaning are read:
//! `cents` and `pence` are hundredths of a currency.

use babbage_core::{
    FormatOptions, LineKind, LineResult, SheetOptions, evaluate_sheet, format_value,
};

/// The line as the app evaluates it: a one-line sheet, where unknown words are prose.
fn shown(line: &str) -> String {
    let r = run(line);
    match &r.kind {
        LineKind::Value(v) => format_value(v, &r.display_options(&FormatOptions::default())).text,
        other => panic!("{line}: {other:?}"),
    }
}

fn run(line: &str) -> LineResult {
    evaluate_sheet(&[line], &SheetOptions::default()).remove(0)
}

fn unclear(line: &str) -> Vec<String> {
    run(line)
        .ignored
        .into_iter()
        .filter(|i| i.unclear)
        .map(|i| i.text)
        .collect()
}

#[test]
fn cents_are_hundredths_of_the_dollar() {
    assert_eq!(shown("50 cents"), "$0.50");
    assert_eq!(shown("50 cents + $1"), "$1.50");
    assert_eq!(shown("$1 + 50 cents"), "$1.50");
    assert_eq!(shown("150 cents"), "$1.50");
    assert_eq!(shown("1 cent"), "$0.01");
    assert_eq!(shown("2 cents × 3"), "$0.06");
}

#[test]
fn cents_follow_the_currency_beside_them() {
    assert_eq!(shown("€1 + 50 cents"), "€1.50");
    assert_eq!(shown("£2 + 75 cents"), "£2.75");
}

#[test]
fn pence_are_hundredths_of_a_pound() {
    assert_eq!(shown("75 pence"), "£0.75");
    assert_eq!(shown("£2 + 75 pence"), "£2.75");
    assert_eq!(shown("1 penny"), "£0.01");
}

#[test]
fn a_cents_that_is_not_after_a_number_is_prose_or_refused_like_any_other_word() {
    // No number straight before it: it is an unknown word, skipped where it is harmless and refused where it
    // stands in for a value.
    assert!(matches!(run("cents + $1").kind, LineKind::Error(_)));
}

#[test]
fn an_unknown_word_where_a_unit_would_stand_is_unclear() {
    assert_eq!(unclear("3 widgets × 4"), ["widgets"]);
    assert_eq!(unclear("5 + 3 apples"), ["apples"]);
    assert_eq!(unclear("hotel, 3 nights × $120"), ["nights"]);
    assert_eq!(unclear("$5 gizmo"), ["gizmo"]);
    assert_eq!(unclear("5 kmm + 3 km"), ["kmm"]);
}

#[test]
fn prose_elsewhere_in_the_line_is_left_alone() {
    for line in [
        "I spent $45 on clothes",
        "lunch was $55",
        "Rent 5 + 3",
        "hotel, 3 × $120",
        "10 kg of flour",
        "150 lb in kg",
    ] {
        assert!(unclear(line).is_empty(), "{line}: {:?}", unclear(line));
    }
}

#[test]
fn a_skipped_word_never_turns_a_line_into_an_error() {
    for (line, want) in [
        ("3 widgets × 4", "12"),
        ("5 + 3 apples", "8"),
        ("I spent $45 on clothes", "$45.00"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
    assert!(matches!(run("3 widgets × 4").kind, LineKind::Value(_)));
}

#[test]
fn a_hint_about_a_typo_only_follows_a_plain_number() {
    let kmm = run("5 kmm + 3 km");
    assert!(
        kmm.warnings
            .iter()
            .any(|w| w.text == "kmm" && w.looks_like == "km")
    );
    // After money a word is flagged but not corrected: a tip is not a teaspoon. A word before the number is prose.
    assert!(run("$5 tip").warnings.is_empty());
    assert!(run("tip $5").warnings.is_empty());
    assert!(!run("5 tsq").warnings.is_empty());
}
