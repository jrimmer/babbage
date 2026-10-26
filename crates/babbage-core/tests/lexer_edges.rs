//! The lexer's small decisions at their edges: when `m` is minutes, when `x` ends a multiplier, where an
//! underscore may stand, and when an apostrophe belongs to a word. Every value was worked out by hand.

use babbage_core::{FormatOptions, evaluate, format_value};

fn answer(line: &str) -> String {
    match evaluate(line) {
        Ok(v) => format_value(&v, &FormatOptions::default()).text,
        Err(e) => format!("ERROR {e}"),
    }
}

fn all(cases: &[(&str, &str)]) {
    for (line, want) in cases {
        assert_eq!(answer(line), *want, "{line}");
    }
}

fn fails(lines: &[&str]) {
    for line in lines {
        assert!(
            answer(line).starts_with("ERROR"),
            "{line} should fail, got {}",
            answer(line)
        );
    }
}

#[test]
fn m_is_minutes_only_beside_a_number_and_a_unit_of_time() {
    all(&[
        ("3h 5m in seconds", "11,100 s"),
        ("5m 10s in seconds", "310 s"),
        ("1h 2m 3s in seconds", "3,723 s"),
        ("10 s 5 m 3 s in s", "313 s"),
        ("5m 3h in minutes", "185 min"),
        ("1 week 5m in minutes", "10,085 min"),
        ("7 m 2 s in seconds", "422 s"),
        // away from a time unit it stays metres
        ("2 km 5 m in m", "2,005 m"),
        ("5 m in cm", "500 cm"),
        ("5m + 3m in m", "8 m"),
    ]);
    // after an operator it is not part of a run of quantities, so it stays metres and cannot meet hours
    fails(&["3h + 5m", "5m + 3h"]);
}

#[test]
fn a_multiplier_literal_stops_at_the_right_x() {
    all(&[
        ("10x", "10x"),
        ("100x", "100x"),
        ("20x", "20x"),
        ("0.0x", "0x"),
        ("00x", "0x"),
        ("5.0x", "5x"),
        ("2X", "2x"),
        ("(2x)", "2x"),
        ("2x * 3", "6"),
        ("2x*3x", "6x"),
        ("1 hour at 2x", "30 minutes"),
        // `x` between operands is still a times sign
        ("3x4", "12"),
        ("3 x 4", "12"),
        ("10 x 4", "40"),
        ("0 x 5", "0"),
    ]);
    // a bare `0x` is a hex prefix with no digits, and letters after the x make it a word
    fails(&["0x", "2xyz", "2x at", "x"]);
    // a multiplier can only be multiplied
    for line in ["2x - 1", "2x + 1", "2x / 4", "2x % 5", "2x\u{2212}1"] {
        assert_eq!(
            answer(line),
            "ERROR unsupported: a multiplier can only be multiplied",
            "{line}"
        );
    }
}

#[test]
fn underscores_sit_between_two_digits_of_the_same_number() {
    all(&[
        ("1_0", "10"),
        ("1_000_000", "1M"),
        ("0b1_0_1", "0b101"),
        ("0b_101", "0b101"),
        ("0xCAFE_F00D", "0xCAFEF00D"),
        ("0xCAFE_F00D in decimal", "3.405705229G"),
    ]);
    fails(&[
        "1__0", "1_", "_1", "0xFF_", "0b1__0", "0b__1", "0x_", "0xF_G", "0b1_2", "1_000_", "1._5",
        "1_.5",
    ]);
}

#[test]
fn an_apostrophe_joins_two_letters_and_nothing_else() {
    all(&[
        ("Valentine's Day 2025", "February 14, 2025"),
        ("valentines day 2025", "February 14, 2025"),
        ("New Year's Day 2025", "January 1, 2025"),
    ]);
    fails(&["Valentine' Day 2025", "Valentine'1 Day 2025", "'s Day 2025"]);
}
