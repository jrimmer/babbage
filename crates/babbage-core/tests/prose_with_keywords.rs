//! How a line that mixes prose with the phrase keywords (`is`, `of`, `to`, `in`, `at`, `for`, `what`) is read.
//!
//! The rule: a skipped word never changes the maths, prose that spells a complete form is read as that form, and a
//! keyword that opens a form that cannot be completed makes the line an error rather than a guess. A comment hides
//! its keywords. Sentences beyond these shapes are left to be found in use.

use babbage_core::{FormatOptions, LineKind, SheetOptions, evaluate_sheet, format_value};

fn shown(line: &str) -> Result<String, String> {
    match &evaluate_sheet(&[line], &SheetOptions::default())[0].kind {
        LineKind::Value(v) => Ok(format_value(v, &FormatOptions::default()).text),
        LineKind::Error(e) => Err(e.to_string()),
        other => Err(format!("{other:?}")),
    }
}

fn answers(cases: &[(&str, &str)]) {
    for (line, want) in cases {
        assert_eq!(shown(line).as_deref(), Ok(*want), "{line}");
    }
}

#[test]
fn stray_words_around_the_maths_are_skipped() {
    answers(&[
        ("Boing is $5", "$5.00"),
        ("Lunch was $12", "$12.00"),
        ("Total is $45", "$45.00"),
        ("Boing is $5 + $3", "$8.00"),
        ("Dinner is $86 / 4", "$21.50"),
        ("Dinner for four is $86", "$86.00"),
        ("Dinner for 4: $86", "$86.00"),
        ("I spent $128 + $45 on clothes", "$173.00"),
        ("is this 5 + 3", "8"),
        ("5 + 3 is", "8"),
        ("tip is 20% of $45", "$9.00"),
        ("a 20% tip on $45", "$54.00"),
        ("The rent is $1,200 a month", "$1,200.00/month"),
    ]);
}

#[test]
fn a_comment_hides_its_keywords() {
    answers(&[
        ("Groceries $45 // is a 5 in the notes", "$45.00"),
        ("$45 + $5 // meeting in 5 minutes", "$50.00"),
    ]);
}

#[test]
fn prose_that_spells_a_complete_form_is_read_as_the_form() {
    answers(&[
        ("Book 3 of 5 volumes", "15"),
        ("Room 5 to 7", "2"),
        ("Chapter 3 to 5 pages", "2"),
        ("Send 5 to Sam", "5"),
    ]);
}

#[test]
fn a_keyword_that_opens_a_form_it_cannot_complete_is_an_error_not_a_guess() {
    for line in [
        "Meeting in 5 minutes",
        "Flight to Denver $320",
        "Flight to Denver $320 + $45 bag",
        "Hotel in Paris 3 nights $150",
        "Dinner for 4 is $86",
        "Dinner for 4 at $21.50 each",
        "It is 5 to 7",
        "Table for 2 at 7",
        "Paid $20 + tip",
    ] {
        assert!(shown(line).is_err(), "{line} gave {:?}", shown(line));
    }
}

#[test]
fn a_percentage_of_what_is_an_amount_asks_for_the_whole() {
    answers(&[
        ("$30 is 20% of what", "$150.00"),
        ("20% of what is $30", "$150.00"),
        ("20% of what is 30", "150"),
        ("5% of what is 10", "200"),
        ("what is 20% of $30", "$6.00"),
        ("20% on what is 120", "100"),
        ("20% off what is 80", "100"),
        ("120 is 20% on what", "100"),
        ("80 is 20% off what", "100"),
    ]);
}
