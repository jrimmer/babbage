use babbage_core::sheet::{floating_total, floating_total_left_out};
use babbage_core::{
    Error, FormatOptions, LineKind, LineResult, SheetOptions, evaluate_sheet, format_value,
};

fn run(lines: &[&str]) -> Vec<LineResult> {
    evaluate_sheet(lines, &SheetOptions::default())
}

fn shown(r: &LineResult) -> String {
    match &r.kind {
        LineKind::Value(v) => format_value(v, &r.display_options(&FormatOptions::default())).text,
        other => format!("{other:?}"),
    }
}

fn answers(lines: &[&str]) -> Vec<String> {
    run(lines).iter().map(shown).collect()
}

fn error_of(r: &LineResult) -> &Error {
    match &r.kind {
        LineKind::Error(e) => e,
        other => panic!("expected an error, got {other:?}"),
    }
}

#[test]
fn variables_are_declared_then_used() {
    assert_eq!(
        answers(&["discount = 10%", "cost = $200", "cost - discount"]),
        ["10%", "$200.00", "$180.00"].map(String::from)
    );
}

#[test]
fn multi_word_variable_names() {
    let r = answers(&[
        "monthly rent = $1,200",
        "monthly rent × 12",
        "monthly rent + 10%",
    ]);
    assert_eq!(
        r,
        ["$1,200.00", "$14,400.00", "$1,320.00"].map(String::from)
    );
}

#[test]
fn a_variable_can_be_redefined_and_nudged() {
    let r = answers(&[
        "x = 5", "x + 1", "x = 10", "x + 1", "x += 3", "x", "x -= 20", "x",
    ]);
    assert_eq!(
        r,
        ["5", "6", "10", "11", "13", "13", "-7", "-7"].map(String::from)
    );
}

#[test]
fn variables_must_be_declared_before_use() {
    let r = run(&["y + 1", "y = 2", "y + 1"]);
    assert!(matches!(error_of(&r[0]), Error::UnexpectedWord { word, .. } if word == "y"));
    assert_eq!(shown(&r[2]), "3");
}

#[test]
fn a_variable_keeps_its_units_and_percent() {
    let r = answers(&[
        "tax = 8.5%",
        "price = 19.99 USD",
        "price + tax",
        "length = 3 m",
        "length × 4",
        "length + 50 cm",
    ]);
    assert_eq!(r[2], "$21.69");
    assert_eq!(r[4], "12 m");
    assert_eq!(r[5], "3.5 m");
}

#[test]
fn variable_names_are_matched_on_case_when_exact_and_loosely_when_unique() {
    assert_eq!(
        answers(&["Rent = 100", "Rent + 1", "rent + 1"]),
        ["100", "101", "101"].map(String::from)
    );
    let r = answers(&[
        "Rent = 100",
        "rent = 200",
        "Rent + 1",
        "rent + 1",
        "RENT + 1",
    ]);
    assert_eq!(&r[2..4], ["101", "201"].map(String::from));
    assert!(r[4].contains("Error"));
}

#[test]
fn line_references_use_the_answer_above() {
    let r = answers(&[
        "5",
        "6",
        "line1 + line2",
        "line 3 × 2",
        "line3 to line4 as %",
    ]);
    assert_eq!(r, ["5", "6", "11", "22", "100%"].map(String::from));
}

#[test]
fn line_references_keep_full_precision() {
    let r = answers(&["19/29", "76 × line1"]);
    assert_eq!(r[0], "0.6551724138");
    assert_eq!(r[1], "49.7931034483");
}

#[test]
fn line_references_only_point_up_at_a_result() {
    let r = run(&[
        "line1", "5", "line3", "line9", "line0", "# Title", "line6", "// note", "line8",
    ]);
    for i in [0, 2, 3, 4, 6, 8] {
        assert!(
            matches!(error_of(&r[i]), Error::BadReference { .. }),
            "line {}",
            i + 1
        );
    }
}

#[test]
fn subtotals_stop_at_the_previous_subtotal_heading_or_divider() {
    let r = answers(&[
        "$10", "$20", "subtotal", "$5", "subtotal", "# Next", "$7", "$8", "subtotal", "---", "$1",
        "subtotal",
    ]);
    assert_eq!(r[2], "$30.00");
    assert_eq!(r[4], "$5.00");
    assert_eq!(r[8], "$15.00");
    assert_eq!(r[11], "$1.00");
}

#[test]
fn a_percentage_line_is_applied_to_the_subtotal() {
    let r = answers(&["$100", "$50", "10%", "subtotal"]);
    assert_eq!(r[3], "$165.00");
}

#[test]
fn a_subtotal_can_become_a_variable() {
    let r = answers(&["$10", "$20", "groceries =", "groceries × 2"]);
    assert_eq!(r[2], "$30.00");
    assert_eq!(r[3], "$60.00");
}

#[test]
fn tags_group_lines_for_totals_and_averages() {
    let r = answers(&[
        "lunch $10 #food",
        "dinner $20 #food",
        "bus $2 #travel",
        "total of #food",
        "average of #food",
        "count of #food",
        "total of #travel",
        "total of #nothing",
    ]);
    assert_eq!(r[3], "$30.00");
    assert_eq!(r[4], "$15.00");
    assert_eq!(r[5], "2");
    assert_eq!(r[6], "$2.00");
    assert!(r[7].contains("Error"));
}

#[test]
fn a_divider_starts_a_fresh_set_of_tags_and_variables() {
    let r = run(&["a = 5 #t", "---", "total of #t", "a + 1"]);
    assert!(matches!(r[2].kind, LineKind::Error(_)));
    assert!(matches!(error_of(&r[3]), Error::UnexpectedWord { .. }));
    let keep = evaluate_sheet(
        &["a = 5 #t", "---", "total of #t", "a + 1"],
        &SheetOptions {
            dividers_reset_scope: false,
            ..SheetOptions::default()
        },
    );
    assert_eq!(shown(&keep[3]), "6");
}

#[test]
fn labels_comments_headings_and_dividers() {
    let r = run(&[
        "Cost of 128 GB iPhone 16: $999",
        "// 1 + 2",
        "1 + 2 // note",
        "# This is a heading",
        "---",
        "",
        "$999 (for iPhone 16)",
        "Boing \"747\" is $386.8M",
        "## Sub",
    ]);
    assert_eq!(shown(&r[0]), "$999.00");
    assert_eq!(r[0].label.as_deref(), Some("Cost of 128 GB iPhone 16"));
    assert_eq!(r[1].kind, LineKind::Comment);
    assert_eq!(shown(&r[2]), "3");
    assert_eq!(r[3].kind, LineKind::Heading("This is a heading".into()));
    assert_eq!(r[4].kind, LineKind::Divider);
    assert_eq!(r[5].kind, LineKind::Blank);
    assert_eq!(shown(&r[6]), "$999.00");
    assert_eq!(shown(&r[7]), "$386.8M");
    assert_eq!(r[8].kind, LineKind::Heading("Sub".into()));
}

#[test]
fn a_clock_time_is_not_a_label() {
    let r = run(&["Meeting at 14:45"]);
    assert!(r[0].label.is_none());
}

#[test]
fn natural_language_around_a_calculation_is_ignored_and_reported() {
    let r = run(&[
        "lunch was $55 + 25% tip",
        "$3k earnings ÷ 5 people",
        "3 apples + 4 oranges",
    ]);
    assert_eq!(shown(&r[0]), "$68.75");
    assert_eq!(
        r[0].ignored
            .iter()
            .map(|i| i.text.as_str())
            .collect::<Vec<_>>(),
        ["lunch", "was", "tip"]
    );
    assert_eq!(shown(&r[1]), "$600.00");
    assert_eq!(shown(&r[2]), "7");
    assert_eq!(r[2].ignored.len(), 2);
}

#[test]
fn text_without_a_calculation_is_just_text() {
    let r = run(&["hello there", "remember the milk"]);
    assert!(r.iter().all(|l| l.kind == LineKind::Text));
}

#[test]
fn a_dropped_word_may_not_stand_in_for_a_number() {
    let lines = [
        "x = 5",
        "y + 1",
        "5 + y",
        "(y) + 1",
        "-y",
        "5 × y × 2",
        "x + oops",
        "oops × 3",
        "a + 1",
        "5 × (y)",
    ];
    let r = run(&lines);
    for (line, result) in lines.iter().zip(&r).skip(1) {
        assert!(
            matches!(result.kind, LineKind::Error(Error::UnexpectedWord { .. })),
            "{line}: {:?}",
            result.kind
        );
    }
}

#[test]
fn a_misspelt_unit_after_a_number_is_ignored_but_warned_about() {
    let r = run(&["5 kmm + 3 km"]);
    assert_eq!(shown(&r[0]), "8 km");
    assert_eq!(
        r[0].ignored
            .iter()
            .map(|i| i.text.as_str())
            .collect::<Vec<_>>(),
        ["kmm"]
    );
    assert!(
        r[0].warnings
            .iter()
            .any(|w| w.text == "kmm" && w.looks_like == "km")
    );
}

#[test]
fn a_near_miss_in_prose_before_a_number_is_not_flagged() {
    // `was` is near `wa`, but it stands before the number, in prose: no hint.
    let r = run(&["lunch was $55"]);
    assert!(r[0].warnings.is_empty());
    assert!(r[0].ignored.iter().all(|i| !i.unclear));
}

#[test]
fn strict_mode_refuses_every_unknown_word() {
    let strict = SheetOptions {
        natural: false,
        ..SheetOptions::default()
    };
    let r = evaluate_sheet(&["lunch was $55 + 25% tip", "$55 + 25%"], &strict);
    assert!(matches!(
        r[0].kind,
        LineKind::Error(Error::UnexpectedWord { .. })
    ));
    assert_eq!(shown(&r[1]), "$68.75");
}

#[test]
fn stray_keywords_in_prose_are_dropped_but_phrase_forms_survive() {
    let r = run(&[
        "Boing is $5",
        "20 is 10% of what",
        "20 is what % of",
        "this is 7",
    ]);
    assert_eq!(shown(&r[0]), "$5.00");
    assert_eq!(shown(&r[1]), "200");
    assert!(matches!(r[2].kind, LineKind::Error(_)));
    assert_eq!(shown(&r[3]), "7");
}

#[test]
fn declaration_lines_show_money_without_rounding() {
    let r = run(&["price = $1.005", "price × 1"]);
    assert_eq!(shown(&r[0]), "$1.005");
    assert_eq!(shown(&r[1]), "$1.01");
}

#[test]
fn the_floating_total_adds_every_result_except_subtotal_lines() {
    let lines = ["$10", "$20", "subtotal", "# H", "$5", "text only", "// c"];
    let results = run(&lines);
    let total = floating_total(&results, &SheetOptions::default())
        .unwrap()
        .unwrap();
    assert_eq!(
        format_value(&total, &FormatOptions::default()).text,
        "$35.00"
    );
    assert!(floating_total(&run(&["hello"]), &SheetOptions::default()).is_none());
}

/// Lines that cannot all be added: the sum starts again from the line that does not fit, so the Total is the sum of
/// the lines since the last such line, plain numbers joining the unit (Sheets C and F of the expected behaviour,
/// ticket 0050).
#[test]
fn a_total_restarts_at_a_line_that_does_not_fit() {
    let total = |lines: &[&str]| {
        let results = run(lines);
        let t = floating_total(&results, &SheetOptions::default())
            .unwrap()
            .unwrap();
        format_value(&t, &FormatOptions::default()).text
    };
    assert_eq!(total(&["$10", "5", "3 gallons"]), "3 gal");
    assert_eq!(total(&["3 gallons", "5", "$10"]), "$10.00");
    assert_eq!(total(&["3 gallons", "5", "2 km", "4 kg"]), "4 kg");
    // A plain number before the conflict is part of the old sum; after it, of the new one.
    assert_eq!(total(&["$10", "3 gallons", "5"]), "8 gal");
    assert_eq!(total(&["3 gallons", "$10", "5", "$1"]), "$16.00");
    assert_eq!(
        total(&["5", "3 gallons"]),
        "8 gal",
        "no clash, so the plain number joins in"
    );
    assert_eq!(total(&["5", "$10", "$20", "7"]), "$42.00");
}

#[test]
fn declarations_can_be_left_out_of_totals() {
    let lines = ["rate = $2", "$10", "subtotal"];
    let default = run(&lines);
    assert_eq!(shown(&default[2]), "$12.00");
    let opts = SheetOptions {
        declarations_in_totals: false,
        ..SheetOptions::default()
    };
    assert_eq!(shown(&evaluate_sheet(&lines, &opts)[2]), "$10.00");
}

#[test]
fn errors_stay_on_their_own_line() {
    let r = run(&["1 / 0", "2 + 2", "line1 + 1"]);
    assert!(matches!(error_of(&r[0]), Error::DivisionByZero));
    assert_eq!(shown(&r[1]), "4");
    assert!(matches!(error_of(&r[2]), Error::BadReference { line: 1 }));
}

#[test]
fn a_variable_named_x_still_leaves_the_times_sign_alone() {
    let r = answers(&["x = 4", "x × 2", "3 x 5", "x x x", "x + x"]);
    assert_eq!(r, ["4", "8", "15", "16", "8"].map(String::from));
}

const SHEETS: &str = include_str!("corpus/sheets.tsv");

fn kind_label(r: &LineResult) -> String {
    match &r.kind {
        LineKind::Blank => ".blank".into(),
        LineKind::Heading(_) => ".heading".into(),
        LineKind::Divider => ".divider".into(),
        LineKind::Comment => ".comment".into(),
        LineKind::Text => ".text".into(),
        LineKind::Error(_) => ".error".into(),
        LineKind::Value(_) => shown(r),
    }
}

#[test]
fn agrees_with_the_sheet_simulator_on_every_line_of_600_sheets() {
    let mut sheets: Vec<Vec<(String, String)>> = Vec::new();
    for line in SHEETS.lines() {
        if line.starts_with("SHEET ") {
            sheets.push(Vec::new());
            continue;
        }
        let (text, want) = line.rsplit_once('\t').expect("tab-separated");
        sheets
            .last_mut()
            .expect("a sheet header comes first")
            .push((text.to_string(), want.to_string()));
    }
    assert_eq!(sheets.len(), 600);
    let mut failures = Vec::new();
    let mut lines_checked = 0;
    for (n, sheet) in sheets.iter().enumerate() {
        let texts: Vec<&str> = sheet.iter().map(|(t, _)| t.as_str()).collect();
        let results = run(&texts);
        for (i, ((text, want), got)) in sheet.iter().zip(&results).enumerate() {
            let got = kind_label(got);
            if &got != want {
                failures.push(format!(
                    "sheet {n} line {}: {text:?} want {want:?}, got {got:?}",
                    i + 1
                ));
            }
            lines_checked += 1;
        }
    }
    assert!(lines_checked > 6000, "{lines_checked}");
    assert!(
        failures.is_empty(),
        "{} of {lines_checked} lines differ, first 12:\n{}",
        failures.len(),
        failures
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn a_function_name_is_never_dropped_as_prose() {
    // In natural mode unknown words are dropped, and a function name once went with them: `sqrt(16)` was 16.
    for (line, want) in [
        ("sqrt(16)", "4"),
        ("cbrt(343)", "7"),
        ("max(3, 9, 4)", "9"),
        ("min(3, 9, 4)", "3"),
        ("abs(-5)", "5"),
        ("round(2.567)", "3"),
        ("floor(2.9)", "2"),
        ("ceil(2.1)", "3"),
        ("sin(30°)", "0.5"),
        ("ln(1)", "0"),
        ("sqrt 16", "4"),
        ("3 + sqrt(16) + 1", "8"),
        ("the total is sqrt(16) items", "4"),
    ] {
        let r = run(&[line]);
        assert_eq!(shown(&r[0]), want, "{line}");
        assert!(
            r[0].ignored.iter().all(|i| ![
                "sqrt", "cbrt", "max", "min", "abs", "round", "floor", "ceil", "sin", "ln"
            ]
            .contains(&i.text.as_str())),
            "{line}: {:?}",
            r[0].ignored
        );
    }
}

#[test]
fn layout_words_are_not_dropped_as_prose() {
    assert_eq!(shown(&run(&["5.5 minutes as timespan"])[0]), "5 min 30 s");
    assert_eq!(shown(&run(&["5.5 minutes as laptime"])[0]), "00:05:30");
    assert_eq!(
        shown(&run(&["03:04:05 as timespan"])[0]),
        "3 hours 4 min 5 s"
    );
    assert_eq!(shown(&run(&["5.5 minutes to laptime"])[0]), "00:05:30");
    // `timespan` standing on its own is still just a word.
    assert!(!run(&["my timespan is 5 + 5"])[0].ignored.is_empty());
}

#[test]
fn a_layout_word_at_the_start_of_a_line_is_just_a_word() {
    // The layout words are only special after `as`, `in`, `to` or `into`; at the start nothing comes before them.
    for line in [
        "timespan",
        "laptime",
        "fraction",
        "timespan 5 + 5",
        "fraction of 3 + 3",
    ] {
        let r = run(&[line]);
        assert_eq!(r.len(), 1, "{line}");
    }
    assert_eq!(shown(&run(&["timespan 5 + 5"])[0]), "10");
}

#[test]
fn the_total_says_how_many_lines_it_left_out() {
    let left_out = |lines: &[&str]| floating_total_left_out(&run(lines), &SheetOptions::default());
    assert_eq!(left_out(&["$10", "5", "3 gallons"]), 2);
    assert_eq!(left_out(&["3 gallons", "5", "$10"]), 2);
    assert_eq!(left_out(&["$10", "3 gallons"]), 1);
    assert_eq!(left_out(&["$10", "3 gallons", "5"]), 1);
    assert_eq!(left_out(&["3 gallons", "5", "2 km", "4 kg"]), 3);
    // Lines that can all be added leave nothing out, plain numbers joining the unit.
    assert_eq!(left_out(&["5", "3 gallons"]), 0);
    assert_eq!(left_out(&["5", "$10", "$20", "7"]), 0);
    assert_eq!(left_out(&["hello"]), 0);
    assert_eq!(left_out(&[]), 0);
}

#[test]
fn global_variables_are_in_every_sheet_and_survive_a_divider() {
    let opts = |globals: &[&str], reset: bool| SheetOptions {
        globals: globals.iter().map(|g| (*g).to_string()).collect(),
        dividers_reset_scope: reset,
        ..SheetOptions::default()
    };
    let lines = ["rent * 2", "mine = 5", "---", "rent + 1", "mine"];
    let shown = |o: &SheetOptions| -> Vec<String> {
        evaluate_sheet(&lines, o)
            .iter()
            .map(|r| match &r.kind {
                babbage_core::LineKind::Value(v) => format_value(v, &FormatOptions::default()).text,
                other => format!("{other:?}"),
            })
            .collect()
    };
    let globals = ["rent = $900", "bad line = ", "rate = 5%"];
    let kept = shown(&opts(&globals, true));
    assert_eq!(kept[0], "$1,800.00");
    // After the divider the sheet's own names are gone and the global one stays.
    assert_eq!(kept[3], "$901.00");
    assert_ne!(
        kept[4], "5",
        "the sheet's own name is gone after the divider"
    );
    // With dividers not resetting, the sheet's own names stay too.
    assert_eq!(shown(&opts(&globals, false))[4], "5");
    // No globals: `rent` is nothing.
    assert_ne!(shown(&opts(&[], true))[0], "$1,800.00");
    // A sheet's own line wins over a global of the same name.
    let mut own = lines.to_vec();
    own[1] = "rent = 1";
    let r = evaluate_sheet(&own, &opts(&["rent = $900"], true));
    assert_eq!(
        match &r[0].kind {
            babbage_core::LineKind::Value(v) => format_value(v, &FormatOptions::default()).text,
            other => format!("{other:?}"),
        },
        "$1,800.00",
        "the line above the declaration still sees the global"
    );
}
