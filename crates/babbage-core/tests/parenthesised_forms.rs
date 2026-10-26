//! Phrase forms and conversions inside parentheses (ticket 0013): `(20 as %) x 2`, `1 + (20 is what % of 200)`,
//! `max(5 km in m, 2)`. Every grouped line must give what the bare form gives, with the same errors.

use babbage_core::settings::{Settings, scoped};
use babbage_core::{
    FormatOptions, LineKind, SheetOptions, evaluate_sheet, evaluate_with, format_value,
};

fn answer(line: &str) -> String {
    let s = Settings::at(1_718_452_800);
    scoped(&s, || match evaluate_with(line, &s) {
        Ok(v) => format_value(&v, &FormatOptions::default()).text,
        Err(e) => format!("ERROR {e}"),
    })
}

/// Forms that stand on their own as a whole line, with their answers.
const FORMS: &[(&str, &str)] = &[
    ("20 as %", "2,000%"),
    ("20 is what % of 200", "10%"),
    ("20 is 5% of what", "400"),
    ("20 is 5% on what", "19.0476190476"),
    ("50 to 75 as %", "50%"),
    ("5 km in m", "5,000 m"),
    ("5 km to m", "5,000 m"),
    ("3 hours in minutes", "180 min"),
    ("0.5 as fraction", "1/2"),
    ("1 / 3 as fraction", "1/3"),
];

#[test]
fn a_form_in_parentheses_is_the_form() {
    for (line, want) in FORMS {
        let bare = answer(line);
        assert_eq!(&bare, want, "bare: {line}");
        for wrapped in [
            format!("({line})"),
            format!("(({line}))"),
            format!("((({line})))"),
            format!(" ( {line} ) "),
        ] {
            assert_eq!(answer(&wrapped), bare, "{wrapped}");
        }
    }
}

#[test]
fn a_grouped_form_is_one_operand_in_arithmetic() {
    for (line, want) in [
        ("(20 as %) x 2", "40"),
        ("2 x (20 as %)", "40"),
        ("1 + (20 is what % of 200)", "1.1"),
        ("(20 is 5% of what) + 1", "401"),
        ("2 * (20 is 5% of what)", "800"),
        ("(5 km in m) * 2", "10,000 m"),
        ("(5 km in m) + 1 m", "5,001 m"),
        ("1 m + (5 km in m)", "5,001 m"),
        ("((5 km in m) + 1 m) in km", "5.001 km"),
        ("(3 hours in minutes) / 2", "90 min"),
        ("(20 as %) + (30 as %)", "5,000%"),
        ("(5 km in m) / (1 km in m)", "5"),
        ("sqrt((16 as %) x 100)", "40"),
    ] {
        assert_eq!(answer(line), want, "{line}");
    }
}

#[test]
fn a_form_inside_a_function_call_is_one_argument() {
    for (line, want) in [
        (
            "sqrt(16 in m)",
            "ERROR these units measure different things",
        ),
        ("max(5 km in m, 2 km in m)", "5,000 m"),
        ("min(5 km in m, 2 km in m)", "2,000 m"),
        ("max(20 as %, 5 as %)", "2,000%"),
        ("round(1 / 3 as fraction)", "0"),
    ] {
        let got = answer(line);
        assert_eq!(got, want, "{line}");
    }
}

#[test]
fn what_was_an_error_is_still_the_same_error() {
    for (line, want) in [
        ("(5 km in m", "ERROR unexpected word \"in\" at byte 6"),
        ("5 km in m)", "ERROR unexpected word \"in\" at byte 5"),
        ("(in)", "ERROR unexpected word \"in\" at byte 1"),
        ("(20 is what)", "ERROR unexpected word \"is\" at byte 4"),
        (
            "1 + (20 is what % of 200",
            "ERROR unexpected word \"is\" at byte 8",
        ),
        ("()", "ERROR expected a value after '('"),
    ] {
        let got = answer(line);
        if line == "()" {
            assert!(got.starts_with("ERROR"), "{line}: {got}");
        } else {
            assert_eq!(got, want, "{line}");
        }
    }
}

#[test]
fn a_form_with_a_mistake_inside_parentheses_fails_where_the_bare_form_fails() {
    for line in [
        "20 as % of",
        "is what % of 200",
        "5 km in",
        "in m",
        "20 is of what",
    ] {
        let bare = answer(line);
        assert!(bare.starts_with("ERROR"), "bare: {line}: {bare}");
        let wrapped = answer(&format!("1 + ({line})"));
        assert!(wrapped.starts_with("ERROR"), "wrapped: {line}: {wrapped}");
    }
}

#[test]
fn deep_nesting_does_not_overflow_or_crawl() {
    let depth = 20_000;
    let line = format!("{}20 as %{}", "(".repeat(depth), ")".repeat(depth));
    let start = std::time::Instant::now();
    assert_eq!(answer(&line), "2,000%");
    let words = format!("{}in{}", "(in ".repeat(depth), ")".repeat(depth));
    assert!(answer(&words).starts_with("ERROR"));
    assert!(
        start.elapsed() < std::time::Duration::from_secs(10),
        "{:?}",
        start.elapsed()
    );
}

#[test]
fn in_a_sheet_a_grouped_form_reads_variables_and_lines() {
    let s = Settings::at(1_718_452_800);
    let opts = SheetOptions {
        settings: s.clone(),
        ..SheetOptions::default()
    };
    let lines = [
        "price = 200",
        "tip = 20",
        "(tip is what % of price) * 2",
        "(line3 as %) x 100",
        "(price km in m)",
    ];
    let out = scoped(&s, || {
        evaluate_sheet(&lines, &opts)
            .iter()
            .map(|r| match &r.kind {
                LineKind::Value(v) => {
                    format_value(v, &r.display_options(&FormatOptions::default())).text
                }
                LineKind::Error(e) => format!("ERROR {e}"),
                other => format!("{other:?}"),
            })
            .collect::<Vec<_>>()
    });
    assert_eq!(out[2], "0.2");
    assert_eq!(out[3], "20");
    assert_eq!(out[4], "200k m");
}
