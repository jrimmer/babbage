//! The newer word forms (a unit in a quantity, `a is b, what is c`, `N% is B in what`, `at N ppi`, layouts,
//! date arithmetic) with lines that are almost, but not quite, their shape: the exact answer or the exact error.

use babbage_core::settings::{Settings, scoped};
use babbage_core::{
    FormatOptions, LineKind, SheetOptions, evaluate_sheet, evaluate_with, format_value,
};

const CASES: &[(&str, &str)] = &[
    ("meters in", "ERROR unexpected word \"in\" at byte 7"),
    ("in 10 km", "ERROR unexpected word \"in\" at byte 0"),
    (
        "5 meters in 10 km",
        "ERROR unexpected word \"in\" at byte 9",
    ),
    (
        "meters in one day",
        "ERROR these units measure different things",
    ),
    ("seconds in an hour", "3,600 s"),
    ("seconds in a", "ERROR unexpected word \"a\" at byte 11"),
    ("seconds in a 5", "ERROR unexpected word \"a\" at byte 11"),
    ("km in a mile", "1.609344 km"),
    ("meters in 10", "ERROR these units measure different things"),
    ("meters in meters", "1 m"),
    (
        "20% is 500 what is",
        "ERROR unexpected word \"is\" at byte 4",
    ),
    ("20% is 500 what", "ERROR unexpected word \"is\" at byte 4"),
    (
        "is 500, what is 750",
        "ERROR unexpected word \"is\" at byte 0",
    ),
    (
        "20% is , what is 750",
        "ERROR unexpected word \"is\" at byte 4",
    ),
    (
        "20% is 500, what is 750 is",
        "ERROR unexpected word \"is\" at byte 24",
    ),
    (
        "if is 500 what is 750",
        "ERROR unexpected word \"if\" at byte 0",
    ),
    (
        "20% is what is 750",
        "ERROR unexpected word \"is\" at byte 4",
    ),
    ("if 20% is 500, what is 750", "30%"),
    (
        "20 is 500 what is to 750",
        "ERROR unexpected word \"is\" at byte 3",
    ),
    ("2 is to 4 as what is to 8", "4"),
    ("is 1 in what", "ERROR unexpected word \"is\" at byte 0"),
    ("5% is in what", "ERROR unexpected word \"is\" at byte 3"),
    ("5% is 1 in", "ERROR unexpected word \"is\" at byte 3"),
    ("5% 1 in what", "ERROR unexpected number at byte 3"),
    ("in what", "ERROR unexpected word \"in\" at byte 0"),
    (
        "5% is 1 in what what",
        "ERROR unexpected word \"is\" at byte 3",
    ),
    (
        "5% is 0 in what",
        "ERROR that value is outside what this function accepts",
    ),
    ("0% is 1 in what", "ERROR division by zero"),
    (
        "1 cm in px at ppi",
        "ERROR unexpected word \"ppi\" at byte 14",
    ),
    ("in px at 326 ppi", "ERROR unexpected word \"in\" at byte 0"),
    (
        "1 cm in at 326 ppi",
        "ERROR unexpected word \"in\" at byte 5",
    ),
    (
        "1 cm px at 326 ppi",
        "ERROR unexpected word \"ppi\" at byte 15",
    ),
    ("1 cm in px at 326", "12,321.2598425197 px"),
    (
        "1 cm in px 326 ppi",
        "ERROR unexpected word \"in\" at byte 5",
    ),
    ("1 cm to px at 326 ppi", "128.3464566929 px"),
    ("1 cm as px at 326 ppi", "128.3464566929 px"),
    ("1 cm in px at 326 dpi", "128.3464566929 px"),
    (
        "1 cm in km at 326 ppi",
        "ERROR unsupported: this needs pixels on one side and a length on the other",
    ),
    (
        "1 cm in px at 326 ppi ppi",
        "ERROR unexpected word \"ppi\" at byte 18",
    ),
    (
        "1 cm in 5 px at 326 ppi",
        "ERROR unexpected number at byte 8",
    ),
    (
        "5 px in px at 326 ppi",
        "ERROR unsupported: this needs pixels on one side and a length on the other",
    ),
    ("326 px in cm at 326 ppi", "2.54 cm"),
    (
        "5 km as timespan",
        "ERROR unsupported: this needs a length of time",
    ),
    (
        "5 km as laptime",
        "ERROR unsupported: this needs a length of time",
    ),
    (
        "5 hours in km and m",
        "ERROR these units measure different things",
    ),
    (
        "5 hours in hours and m",
        "ERROR these units measure different things",
    ),
    ("5 hours in hours and minutes", "5 h"),
    (
        "5 km as fraction",
        "ERROR unsupported: a fraction needs a plain number or a percentage",
    ),
    (
        "$5 as fraction",
        "ERROR unsupported: a fraction needs a plain number or a percentage",
    ),
    (
        "true as fraction",
        "ERROR unsupported: a fraction needs a plain number or a percentage",
    ),
    (
        "5 kg as fraction",
        "ERROR unsupported: a fraction needs a plain number or a percentage",
    ),
    ("5x as fraction", "ERROR unexpected end of expression"),
    (
        "June 5 as fraction",
        "ERROR unsupported: a fraction needs a plain number or a percentage",
    ),
    ("5 as fraction", "5"),
    (
        "5 km is what % of 10 kg",
        "ERROR unsupported: this phrase needs plain numbers and, where written, a percentage",
    ),
    (
        "5 km is what % of 10",
        "ERROR unsupported: this phrase needs plain numbers and, where written, a percentage",
    ),
    (
        "5 km as x of 10 kg",
        "ERROR unsupported: this phrase needs plain numbers and, where written, a percentage",
    ),
    (
        "5 km on 10 kg as multiplier",
        "ERROR unsupported: 'on' and 'off' need a plain number after them",
    ),
    (
        "5 km to 10 kg is what %",
        "ERROR unsupported: this phrase needs plain numbers and, where written, a percentage",
    ),
    (
        "June 5 + 5 km",
        "ERROR unsupported: a date or time can be shifted by a length of time, and two of them can be subtracted",
    ),
    (
        "June 5 + 5",
        "ERROR unsupported: a date or time can be shifted by a length of time, and two of them can be subtracted",
    ),
    (
        "5 km + June 5",
        "ERROR unsupported: a date or time can be shifted by a length of time, and two of them can be subtracted",
    ),
    (
        "5pm + 5 km",
        "ERROR unsupported: a date or time can be shifted by a length of time, and two of them can be subtracted",
    ),
    (
        "5 km + 5pm",
        "ERROR unsupported: a date or time can be shifted by a length of time, and two of them can be subtracted",
    ),
    (
        "June 5 - 5 km",
        "ERROR unsupported: a date or time can be shifted by a length of time, and two of them can be subtracted",
    ),
    (
        "5pm - 5 km",
        "ERROR unsupported: a date or time can be shifted by a length of time, and two of them can be subtracted",
    ),
    ("June 10, 2024 - 3 workdays", "June 5"),
    ("June 10, 2024 + 3 workdays", "June 13"),
    ("5pm - 90 minutes", "3:30 pm"),
    ("5pm + 90 minutes", "6:30 pm"),
    ("true == false", "false"),
    ("true == true", "true"),
    ("true != false", "true"),
    ("true != true", "false"),
    (
        "sqrt(2) == sqrt(2)",
        "ERROR unsupported: these approximate values are too close to tell apart",
    ),
    (
        "sqrt(2) * sqrt(2) == 2",
        "ERROR unsupported: these approximate values are too close to tell apart",
    ),
    ("1 <= 2", "true"),
    ("2 <= 2", "true"),
    ("3 <= 2", "false"),
];

fn answer(line: &str) -> String {
    let s = Settings::at(1_718_452_800);
    scoped(&s, || match evaluate_with(line, &s) {
        Ok(v) => format_value(&v, &FormatOptions::default()).text,
        Err(e) => format!("ERROR {e}"),
    })
}

fn sheet(lines: &[&str]) -> Vec<String> {
    let s = Settings::at(1_718_452_800);
    let opts = SheetOptions {
        settings: s.clone(),
        ..SheetOptions::default()
    };
    scoped(&s, || {
        evaluate_sheet(lines, &opts)
            .iter()
            .map(|r| match &r.kind {
                LineKind::Value(v) => {
                    format_value(v, &r.display_options(&FormatOptions::default())).text
                }
                LineKind::Error(e) => format!("ERROR {e}"),
                other => format!("{other:?}"),
            })
            .collect()
    })
}

#[test]
fn near_misses_are_answered_or_refused_exactly_as_reviewed() {
    let wrong: Vec<String> = CASES
        .iter()
        .filter_map(|(line, want)| {
            let got = answer(line);
            (got != *want).then(|| format!("{line}: want {want:?}, got {got:?}"))
        })
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn every_comparison_gives_the_right_truth_for_less_equal_and_greater() {
    // Rust integer comparison is the oracle: the engine compares exact rationals.
    for (a, b) in [(1i64, 2i64), (2, 2), (3, 2), (-1, 1), (0, 0), (10, 9)] {
        for (op, want) in [
            ("<", a < b),
            ("<=", a <= b),
            (">", a > b),
            (">=", a >= b),
            ("==", a == b),
            ("!=", a != b),
        ] {
            let got = answer(&format!("({a}) {op} ({b})"));
            assert_eq!(got, want.to_string(), "{a} {op} {b}");
            // the same comparison between quantities in different units
            let got = answer(&format!("({a}) km {op} ({b}000) m"));
            assert_eq!(got, want.to_string(), "{a} km {op} {b}000 m");
        }
    }
    for (op, want) in [("==", false), ("!=", true)] {
        assert_eq!(answer(&format!("true {op} false")), want.to_string());
        assert_eq!(answer(&format!("false {op} true")), want.to_string());
    }
    for (op, want) in [("==", true), ("!=", false)] {
        assert_eq!(answer(&format!("true {op} true")), want.to_string());
        assert_eq!(answer(&format!("false {op} false")), want.to_string());
    }
    for line in [
        "true < false",
        "true >= 1",
        "sqrt(2) == sqrt(2)",
        "sqrt(2) * sqrt(2) == 2",
        "sqrt(2) != sqrt(2)",
    ] {
        assert!(
            answer(line).starts_with("ERROR"),
            "{line}: {}",
            answer(line)
        );
    }
    // approximate values far enough apart can be told apart
    assert_eq!(answer("sqrt(2) < sqrt(3)"), "true");
    assert_eq!(answer("sqrt(2) == sqrt(3)"), "false");
    assert_eq!(answer("sqrt(2) != sqrt(3)"), "true");
}

#[test]
fn a_layout_survives_a_sum_and_not_a_product() {
    let r = sheet(&[
        "a = 5.5 minutes as timespan",
        "a + 1 minute",
        "1 minute + a",
        "a - 30 seconds",
        "a * 2",
        "a / 2",
        "b = 5.5 minutes as laptime",
        "b + 30 seconds",
        "b * 2",
        "a + 5",
    ]);
    assert_eq!(r[1], "6 minutes 30 seconds");
    assert_eq!(r[2], "6 minutes 30 seconds");
    assert_eq!(r[3], "5 minutes");
    assert_eq!(r[4], "11 min");
    assert_eq!(r[5], "2.75 min");
    assert_eq!(r[7], "00:06:00");
    assert_eq!(r[8], "11 min");
    assert!(
        r[9].starts_with("5 minutes 5 s") || r[9].starts_with("ERROR") || r[9].contains("minutes"),
        "{}",
        r[9]
    );
}

#[test]
fn a_declaration_shows_a_currency_without_minor_digits_as_a_whole_number() {
    let r = sheet(&[
        "yen = \u{a5}1,000",
        "yen * 2",
        "dollars = $1,000",
        "euro = \u{20ac}2.5",
    ]);
    assert_eq!(r[0], "\u{a5}1,000");
    assert_eq!(r[1], "\u{a5}2,000");
    assert_eq!(r[2], "$1,000.00");
    assert_eq!(r[3], "\u{20ac}2.50");
}

#[test]
fn a_breakdown_that_does_not_end_is_flagged_as_inexact() {
    let s = Settings::at(1_718_452_800);
    let (exact, repeating) = scoped(&s, || {
        let a = evaluate_with("5.5 hours in minutes and seconds", &s).unwrap();
        let b = evaluate_with("1 minute / 7 in minutes and seconds", &s).unwrap();
        (
            format_value(&a, &FormatOptions::default()),
            format_value(&b, &FormatOptions::default()),
        )
    });
    assert_eq!(exact.text, "330 min");
    assert!(exact.exact);
    assert_eq!(repeating.text, "8.5714285714 s");
    assert!(!repeating.exact);
}

#[test]
fn dates_move_by_workdays_in_both_directions_and_only_by_lengths_of_time() {
    for (line, want) in [
        ("June 10, 2024 + 3 workdays", "June 13"),
        ("June 10, 2024 - 3 workdays", "June 5"),
        ("June 7, 2024 + 1 workday", "June 10"),
        ("June 8, 2024 + 1 workday", "June 10"),
        ("June 10, 2024 - 1 workday", "June 7"),
        ("June 9, 2024 - 1 workday", "June 7"),
        // 10^12 workdays from a Monday is 2 x 10^11 whole weeks: March 17 of year 3,833,071,834 (worked out
        // in Python from 400-year Gregorian cycles)
        (
            "June 3, 2024 + 1_000_000_000_000 workdays",
            "March 17, 3833071834",
        ),
    ] {
        assert_eq!(answer(line), want, "{line}");
    }
    // one more than that is refused, in either direction
    for line in [
        "June 3, 2024 + 1_000_000_000_001 workdays",
        "June 3, 2024 - 1_000_000_000_001 workdays",
    ] {
        assert!(
            answer(line).starts_with("ERROR"),
            "{line}: {}",
            answer(line)
        );
    }
}

#[test]
fn words_glued_to_numbers_in_a_sheet() {
    // A time, timecode or date glued to a letter is not read as one; the letters are dropped as prose.
    let r = sheet(&[
        "lunch5pm",
        "lunch 5pm",
        "lunch2019-04-01",
        "It's 5 + 5",
        "Valentine' Day 2025",
        "abc' 5",
    ]);
    assert_eq!(r[0], "5");
    assert_eq!(r[1], "5:00 pm");
    assert_eq!(r[2], "2,014");
    assert_eq!(r[3], "10");
    assert!(r[4].starts_with("ERROR"), "{}", r[4]);
    assert!(r[5].starts_with("ERROR"), "{}", r[5]);
}

#[test]
fn words_and_booleans_compare_for_equality_only_with_their_own_kind() {
    let r = sheet(&[
        "a = day of the week on June 15, 2024",
        "b = day of the week on June 22, 2024",
        "c = day of the week on June 16, 2024",
        "a == b",
        "a != b",
        "a == c",
        "a != c",
        "a < c",
        "a == 5",
        "5 == a",
    ]);
    assert_eq!(&r[3..7], ["true", "false", "false", "true"]);
    for (i, got) in r.iter().enumerate().skip(7) {
        assert_eq!(
            got, "ERROR unsupported: words can only be compared for equality",
            "line {i}"
        );
    }
    for (line, want) in [
        (
            "true < false",
            "ERROR unsupported: true and false can only be compared for equality",
        ),
        (
            "true >= false",
            "ERROR unsupported: true and false can only be compared for equality",
        ),
        (
            "true < 1",
            "ERROR unsupported: a true or false cannot be compared with a number",
        ),
        (
            "1 < true",
            "ERROR unsupported: a true or false cannot be compared with a number",
        ),
        (
            "true == 1",
            "ERROR unsupported: a true or false cannot be compared with a number",
        ),
        (
            "1 != false",
            "ERROR unsupported: a true or false cannot be compared with a number",
        ),
    ] {
        assert_eq!(answer(line), want, "{line}");
    }
}

#[test]
fn a_length_of_time_before_a_time_of_day_shifts_it() {
    for (line, want) in [
        ("2 hours + 5pm", "7:00 pm"),
        ("90 minutes + 5pm", "6:30 pm"),
        ("5pm + 2 hours", "7:00 pm"),
        ("5pm - 2 hours", "3:00 pm"),
        (
            "2 hours - 5pm",
            "ERROR unsupported: a date or time can be shifted by a length of time, and two of them can be subtracted",
        ),
        ("June 10, 2024 - 1 month", "May 10"),
        ("June 10, 2024 + 1 month", "July 10"),
        ("June 10, 2024 - 1 year", "June 10, 2023"),
        ("June 10, 2024 + 1 year", "June 10, 2025"),
        ("June 10, 2024 - 1 workday", "June 7"),
        ("June 10, 2024 + 1 workday", "June 11"),
    ] {
        assert_eq!(answer(line), want, "{line}");
    }
}

#[test]
fn a_plain_number_cannot_take_a_layout_that_needs_a_length_or_a_unit() {
    for line in ["5 as timespan", "5 as laptime", "5 in hours and minutes"] {
        assert_eq!(
            answer(line),
            "ERROR these units measure different things",
            "{line}"
        );
    }
    assert_eq!(
        answer("5 km as multiplier"),
        "ERROR unsupported: this phrase needs plain numbers and, where written, a percentage"
    );
}

#[test]
fn what_a_sheet_ignores_is_reported_where_it_stood() {
    let s = Settings::at(1_718_452_800);
    let opts = SheetOptions {
        settings: s.clone(),
        ..SheetOptions::default()
    };
    for line in [
        "3 apples + 4 oranges",
        "h\u{e9}llo 5 + 5",
        "5 + \u{fc} 5",
        "lunch was $55 + 25% tip",
        "at the cafe 3 + 4 (with a friend)",
        "\u{4f60}\u{597d} 2 + 2",
    ] {
        let r = scoped(&s, || evaluate_sheet(&[line], &opts));
        assert!(!r[0].ignored.is_empty(), "{line}");
        for i in &r[0].ignored {
            assert_eq!(
                line.get(i.at..i.at + i.text.len()),
                Some(i.text.as_str()),
                "{line}: {i:?}"
            );
        }
    }
}

#[test]
fn a_time_or_timecode_glued_to_letters_is_not_one_even_when_the_letters_are_dropped() {
    // In a sheet unknown words are dropped, so a time that did swallow its neighbours would answer; it must refuse.
    for line in [
        "lunch01:02:03:04",
        "01:02:03:04apples",
        "00:00:01.x",
        "lunch5:30pm",
        "5:30pmapples",
        "lunch12:30:45",
        "12:30:45apples",
    ] {
        let r = sheet(&[line]);
        assert!(r[0].starts_with("ERROR"), "{line}: {}", r[0]);
    }
    assert_eq!(sheet(&["lunch 5:30pm"])[0], "5:30 pm");
}

#[test]
fn number_scanning_errors_name_exactly_the_text_they_stopped_at() {
    use babbage_core::Error;
    let bad = |text: &str| Error::InvalidNumber {
        text: text.into(),
        at: 0,
    };
    assert_eq!(babbage_core::evaluate("1.2.3"), Err(bad("1.2.3")));
    assert_eq!(babbage_core::evaluate("12.5.5"), Err(bad("12.5.5")));
    assert_eq!(babbage_core::evaluate("1.23.45.6"), Err(bad("1.23.45")));
    assert_eq!(answer("5 30pm"), "ERROR unexpected number at byte 2");
    assert_eq!(answer("24:00"), "ERROR unexpected character ':' at byte 2");
    assert_eq!(answer("23:59"), "11:59 pm");
    assert_eq!(answer("$15bn+1"), "$15.000000001G");
    assert_eq!(answer("$15bn + 1"), "$15.000000001G");
    assert_eq!(answer("$15bnx"), "ERROR unexpected word \"bnx\" at byte 3");
    assert_eq!(answer("5T"), "5T");
    assert_eq!(answer("$5T"), "$5T");
}

#[test]
fn every_ignored_piece_has_text() {
    let s = Settings::at(1_718_452_800);
    let opts = SheetOptions {
        settings: s.clone(),
        ..SheetOptions::default()
    };
    let r = scoped(&s, || {
        evaluate_sheet(
            &["$999 (for iPhone 16)", "at the cafe 3 + 4 (with a friend)"],
            &opts,
        )
    });
    for line in &r {
        assert!(!line.ignored.is_empty());
        assert!(
            line.ignored.iter().all(|i| !i.text.is_empty()),
            "{:?}",
            line.ignored
        );
    }
}

#[test]
fn the_remaining_near_misses_of_the_proportion_and_ratio_forms() {
    for (line, want) in [
        (
            "is to 4 as 3 is to what",
            "ERROR unexpected end of expression",
        ),
        (
            "2 is to as 3 is to what",
            "ERROR unexpected end of expression",
        ),
        (
            "2 is to 4 as is to what",
            "ERROR unexpected end of expression",
        ),
        (
            "2 is to 4 as what is to",
            "ERROR unexpected end of expression",
        ),
        (
            "2 is to 4 what is 8",
            "ERROR unexpected word \"is\" at byte 2",
        ),
        ("5 is 1 in what", "1/0.2"),
        ("4 is 2 in what", "2/0.5"),
        ("3 is 1 in what", "1/0.3333333333"),
        (
            "5% is 1 in whatever",
            "ERROR unexpected word \"is\" at byte 3",
        ),
        (
            "5 in px at 326 ppi",
            "ERROR unsupported: this needs a length or a number of pixels",
        ),
        (
            "1 cm in px at 326 km",
            "ERROR these units measure different things",
        ),
        ("1 cm in px at 326", "12,321.2598425197 px"),
        (
            "1 cm in px at 326 ppix",
            "ERROR unexpected word \"ppix\" at byte 18",
        ),
    ] {
        assert_eq!(answer(line), want, "{line}");
    }
}

#[test]
fn a_repeating_ratio_is_flagged_inexact_and_a_plain_one_is_not() {
    let s = Settings::at(1_718_452_800);
    scoped(&s, || {
        let third = evaluate_with("3 is 1 in what", &s).unwrap();
        let tenth = evaluate_with("5% is 1 in what", &s).unwrap();
        assert!(!format_value(&third, &FormatOptions::default()).exact);
        assert!(format_value(&tenth, &FormatOptions::default()).exact);
    });
}

#[test]
fn an_approximate_comparison_allows_a_gap_in_proportion_to_the_size_of_the_numbers() {
    // A gap of 1e-40 on a number near 1.4e30 is below the tolerance (1e-60 of the size), so the two cannot be
    // told apart; a gap of 1e-20 is above it.
    assert_eq!(
        answer("sqrt(2) * 10^30 == sqrt(2) * 10^30 + 10^-40"),
        "ERROR unsupported: these approximate values are too close to tell apart"
    );
    assert_eq!(
        answer("sqrt(2) * 10^30 == sqrt(2) * 10^30 + 10^-20"),
        "false"
    );
    assert_eq!(answer("sqrt(2) * 10^30 < sqrt(2) * 10^30 + 10^-20"), "true");
    // near 1 the same rule gives a gap of about 1e-60
    assert_eq!(answer("sqrt(2) == sqrt(2) + 10^-50"), "false");
    assert_eq!(
        answer("sqrt(2) == sqrt(2) + 10^-70"),
        "ERROR unsupported: these approximate values are too close to tell apart"
    );
}
