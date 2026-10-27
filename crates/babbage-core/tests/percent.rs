use babbage_core::{Error, FormatOptions, Kind, Number, evaluate, format_value};
use proptest::prelude::*;

const CORPUS: &str = include_str!("corpus/percent.tsv");

fn shown(line: &str) -> String {
    let v = evaluate(line).unwrap_or_else(|e| panic!("{line:?}: {e}"));
    format_value(&v, &FormatOptions::default()).text
}

fn unsupported(line: &str) {
    assert!(
        matches!(evaluate(line), Err(Error::UnsupportedOperation(_))),
        "{line}: {:?}",
        evaluate(line)
    );
}

#[test]
fn every_example_in_the_reference_table_gives_the_documented_answer() {
    let table = [
        ("10% of 200", "20"),
        ("25% × 200", "50"),
        ("200 + 10%", "220"),
        ("10% on 200", "220"),
        ("200 - 10%", "180"),
        ("10% off 200", "180"),
        ("20 is 10% of what", "200"),
        ("180 is 10% off what", "200"),
        ("220 is 10% on what", "200"),
        ("50 to 75 is what %", "50%"),
        ("40 to 90 as %", "125%"),
        ("180 is what % off 200", "10%"),
        ("180 is what % on 150", "20%"),
        ("20 is what % of 200", "10%"),
        ("20 as a % of 200", "10%"),
        ("20/200 as %", "10%"),
        ("20/200 %", "10%"),
        ("0.35 as %", "35%"),
        ("2/5 as percent", "40%"),
        ("3/20 is what %", "15%"),
        ("3 in 20 is what %", "15%"),
        ("10% + 20%", "30%"),
        ("90% − 40%", "50%"),
        ("30% + 0.4", "70%"),
        ("100% - 1/2", "50%"),
        ("100% + 2 + 30%", "330%"),
        ("50% × 30", "15"),
        ("30 × 50%", "15"),
        ("2/3 of 600", "400"),
        ("50 is 1/5 of what", "250"),
    ];
    for (line, want) in table {
        assert_eq!(shown(line), want, "{line}");
    }
}

#[test]
fn kinds_follow_the_reference_rules() {
    let kinds = [
        ("10%", Kind::Percent),
        ("-10%", Kind::Percent),
        ("10% + 20%", Kind::Percent),
        ("30% + 0.4", Kind::Percent),
        ("200 + 10%", Kind::Plain),
        ("50% × 30", Kind::Plain),
        ("30 × 50%", Kind::Plain),
        ("1 as %", Kind::Percent),
        ("10% of 200", Kind::Plain),
        ("(10%)", Kind::Percent),
        ("(1+1)%", Kind::Percent),
    ];
    for (line, kind) in kinds {
        assert_eq!(evaluate(line).unwrap().kind(), kind, "{line}");
    }
    assert_eq!(
        evaluate("12.5%").unwrap().number(),
        &evaluate("1/8").unwrap().into_plain().unwrap()
    );
    assert_eq!(evaluate("10%"), Ok(evaluate("0.1 as %").unwrap()));
    assert_eq!(
        evaluate("10%").unwrap().into_plain(),
        Err(Error::NotAPlainNumber)
    );
}

#[test]
fn percent_is_exact_not_a_float_trap() {
    assert_eq!(shown("0.07 × 100"), "7");
    assert_eq!(shown("1.1% of 1000"), "11");
    assert_eq!(shown("33.333% of 3000"), "999.99");
    assert_eq!(shown("1/3 as %"), "33.3333333333%");
    assert_eq!(shown("2/3 as %"), "66.6666666667%");
    assert_eq!(shown("0.1% + 0.2%"), "0.3%");
    assert_eq!(shown("19.99 + 8.25%"), "21.639175");
    assert_eq!(shown("100 + 10% - 10%"), "99");
    assert_eq!(shown("100 - 10% + 10%"), "99");
    assert_eq!(shown("0 as %"), "0%");
    assert_eq!(shown("100% as %"), "100%");
}

#[test]
fn attached_and_detached_percent_mean_different_things() {
    assert_eq!(shown("20/200 %"), "10%");
    // An attached percent is its fraction, so this is 20 / 2 (the reference app: `20 / 50%` is 40).
    assert_eq!(shown("20/200%"), "10");
    assert_eq!(shown("20 / 200 %"), "10%");
    assert_eq!(shown("(20/200)%"), "0.1%");
    assert_eq!(shown("50 %"), "5,000%");
    assert_eq!(shown("50%"), "50%");
}

#[test]
fn word_forms_are_case_and_spacing_insensitive() {
    for line in ["10% OF 200", "10% Of 200", "  10%   of   200  "] {
        assert_eq!(shown(line), "20", "{line}");
    }
    for line in [
        "20 IS 10% OF WHAT",
        "20 Is 10% Of What",
        "20  is   10%  of  what",
    ] {
        assert_eq!(shown(line), "200", "{line}");
    }
    assert_eq!(shown("20 as PERCENT"), "2,000%");
    assert_eq!(shown("20 is what percent of 200"), "10%");
    assert_eq!(
        shown("20 is what % of 200"),
        shown("20 is what percent of 200")
    );
}

#[test]
fn undocumented_combinations_are_refused_not_guessed() {
    for line in ["10% / 20%", "2^10%", "10% on 20%", "10% off 20%", "10%%"] {
        unsupported(line);
    }
}

#[test]
fn percentages_multiply_divide_and_power_as_the_reference_app_does() {
    // Sheet G (ticket 0012): The reference app gave 1%, 40, 25% and 0.01.
    assert_eq!(shown("10% × 10%"), "1%");
    assert_eq!(shown("20 / 50%"), "40");
    assert_eq!(shown("50% / 2"), "25%");
    assert_eq!(shown("10%^2"), "0.01");
    // The rule, tried on other numbers: a percentage is its fraction, and only the product and the quotient by a
    // plain number stay a percentage.
    assert_eq!(shown("200% × 25%"), "50%");
    assert_eq!(shown("30 kg / 25%"), "120 kg");
    assert_eq!(shown("1 / 4%"), "25");
    assert_eq!(shown("5% / 2"), "2.5%");
    assert_eq!(shown("200%^3"), "8");
    assert_eq!(evaluate("5 / 0%"), Err(Error::DivisionByZero));
    assert_eq!(evaluate("5% / 0"), Err(Error::DivisionByZero));
}

#[test]
fn phrase_forms_work_inside_parentheses() {
    // Ticket 0013: a form in parentheses is one operand, with the answer the bare form gives.
    for (line, want) in [
        ("(20 as %) × 1", "20"),
        ("1 + (20 is what % of 200)", "1.1"),
        // 3 to 4 is +1/3, and 1/3 + 1 = 4/3 as a percentage.
        ("(3 to 4 as %) + 1", "133.3333333333%"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
}

#[test]
fn zero_divisors_in_every_form() {
    let lines = [
        "5 is 0% of what",
        "5 is 100% off what",
        "5 is what % of 0",
        "5 is what % on 0",
        "5 is what % off 0",
        "0 to 5 is what %",
        "0 to 5 as %",
        "5 in 0 is what %",
        "5/0 as %",
        "5/0 %",
        "5 is 0/3 of what",
    ];
    for line in lines {
        assert_eq!(evaluate(line), Err(Error::DivisionByZero), "{line}");
    }
}

#[test]
fn incomplete_or_misplaced_phrases_are_errors() {
    for line in [
        "is what %",
        "of 200",
        "10% of",
        "20 is of what",
        "20 is what % of",
        "to 5 as %",
        "20 as",
        "as %",
        "%",
        "is",
    ] {
        assert!(
            evaluate(line).is_err(),
            "{line} should fail, got {:?}",
            evaluate(line)
        );
    }
    assert!(matches!(
        evaluate("20 foo 5"),
        Err(Error::UnexpectedWord { .. })
    ));
    assert!(matches!(
        evaluate("lunch was 55"),
        Err(Error::UnexpectedWord { .. })
    ));
}

#[test]
fn phrases_nest_inside_parentheses_and_other_arithmetic() {
    assert_eq!(shown("(10% of 200) + 5"), "25");
    assert_eq!(shown("2 × (10% of 200)"), "40");
    assert_eq!(shown("(200 + 10%) × 2"), "440");
    assert_eq!(shown("10% of (100 + 100)"), "20");
    assert_eq!(shown("10% of 200 + 5"), "25");
    assert_eq!(shown("5 + 10% of 200"), "25");
    assert_eq!(shown("(20 + 20) is 10% of what"), "400");
    assert_eq!(shown("20 is (5 + 5)% of what"), "200");
    assert_eq!(shown("(1 + 1) to (3 + 3) is what %"), "200%");
}

#[test]
fn agrees_with_the_percentage_oracle_on_every_line() {
    let mut failures = Vec::new();
    let mut opts = FormatOptions::new(10).unwrap();
    opts.group_thousands = false;
    opts.trim_trailing_zeros = false;
    let mut checked = 0;
    for line in CORPUS.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        assert_eq!(cols.len(), 4, "malformed corpus line {line:?}");
        let (expr, kind, exact, display) = (cols[0], cols[1], cols[2], cols[3]);
        let got = evaluate(expr);
        let ok = match (kind, &got) {
            ("ERR:div0", Err(Error::DivisionByZero)) => true,
            ("ERR:unsupported", Err(Error::UnsupportedOperation(_))) => true,
            ("plain" | "percent", Ok(v)) => {
                let want_kind = if kind == "plain" {
                    Kind::Plain
                } else {
                    Kind::Percent
                };
                let ours = format!("{}/{}", v.number().numer(), v.number().denom());
                v.kind() == want_kind && ours == exact && format_value(v, &opts).text == display
            }
            _ => false,
        };
        if !ok {
            failures.push(format!(
                "{expr:?} want {kind} {exact} {display}, got {got:?}"
            ));
        }
        checked += 1;
    }
    assert!(checked > 40_000, "corpus unexpectedly small: {checked}");
    assert!(
        failures.is_empty(),
        "{} of {checked} disagree, first 12:\n{}",
        failures.len(),
        failures
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn dec() -> impl Strategy<Value = String> {
    (0u32..=100_000, 0u32..=999).prop_map(|(i, f)| format!("{i}.{f:03}"))
}

fn pos_dec() -> impl Strategy<Value = String> {
    dec().prop_filter("nonzero", |s| s.parse::<f64>().unwrap() > 0.0)
}

fn plain(line: &str) -> Number {
    evaluate(line).unwrap().into_plain().unwrap()
}

fn pct_value(line: &str) -> Number {
    let v = evaluate(line).unwrap();
    assert_eq!(v.kind(), Kind::Percent, "{line}");
    v.number().clone()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 3000, ..ProptestConfig::default() })]

    #[test]
    fn on_and_off_are_the_same_as_adding_and_subtracting(x in dec(), p in dec()) {
        prop_assert_eq!(plain(&format!("{p}% on {x}")), plain(&format!("{x} + {p}%")));
        prop_assert_eq!(plain(&format!("{p}% off {x}")), plain(&format!("{x} - {p}%")));
        prop_assert_eq!(plain(&format!("{p}% of {x}")), plain(&format!("{x} × {p}%")));
        prop_assert_eq!(plain(&format!("{p}% of {x}")), plain(&format!("({p}/100) * {x}")));
    }

    #[test]
    fn reverse_forms_recover_the_original(w in pos_dec(), p in pos_dec()) {
        let f = pct_value(&format!("{p}%"));
        prop_assume!(f != Number::from_i64(1));
        let part = plain(&format!("{p}% of {w}"));
        let shown_part = babbage_core::format_decimal(&part, &exact_opts(part.terminating_places().unwrap())).text;
        prop_assert_eq!(plain(&format!("{shown_part} is {p}% of what")), plain(&w));
        let up = plain(&format!("{w} + {p}%"));
        let up_text = babbage_core::format_decimal(&up, &exact_opts(up.terminating_places().unwrap())).text;
        prop_assert_eq!(plain(&format!("{up_text} is {p}% on what")), plain(&w));
        let down = plain(&format!("{w} - {p}%"));
        let down_text = babbage_core::format_decimal(&down, &exact_opts(down.terminating_places().unwrap())).text;
        prop_assert_eq!(plain(&format!("{down_text} is {p}% off what")), plain(&w));
    }

    #[test]
    fn change_forms_recover_the_rate(w in pos_dec(), p in pos_dec()) {
        let up = plain(&format!("{w} + {p}%"));
        let up_text = babbage_core::format_decimal(&up, &exact_opts(up.terminating_places().unwrap())).text;
        prop_assert_eq!(pct_value(&format!("{up_text} is what % on {w}")), pct_value(&format!("{p}%")));
        prop_assert_eq!(pct_value(&format!("{w} to {up_text} as %")), pct_value(&format!("{p}%")));
        let down = plain(&format!("{w} - {p}%"));
        let down_text = babbage_core::format_decimal(&down, &exact_opts(down.terminating_places().unwrap())).text;
        prop_assert_eq!(pct_value(&format!("{down_text} is what % off {w}")), pct_value(&format!("{p}%")));
    }

    #[test]
    fn as_percent_then_back_is_the_identity(x in dec()) {
        prop_assert_eq!(pct_value(&format!("{x} as %")), plain(&x));
        prop_assert_eq!(pct_value(&format!("{x}% as %")), pct_value(&format!("{x}%")));
    }

    #[test]
    fn percent_addition_is_exact_and_associative(p in dec(), q in dec(), r in dec()) {
        prop_assert_eq!(pct_value(&format!("{p}% + {q}%")), pct_value(&format!("{q}% + {p}%")));
        prop_assert_eq!(pct_value(&format!("({p}% + {q}%) + {r}%")), pct_value(&format!("{p}% + ({q}% + {r}%)")));
        prop_assert_eq!(pct_value(&format!("{p}% - {p}%")), Number::from_i64(0));
    }

    #[test]
    fn stacked_adjustments_multiply_not_add(x in dec(), p in dec(), q in dec()) {
        let stacked = plain(&format!("{x} + {p}% + {q}%"));
        let by_hand = plain(&format!("{x} * (1 + {p}/100) * (1 + {q}/100)"));
        prop_assert_eq!(stacked, by_hand);
    }

    #[test]
    fn arbitrary_phrase_soup_never_panics(s in "[0-9. %+*/()a-z-]{0,60}") {
        let _ = evaluate(&s);
    }
}

fn exact_opts(places: u32) -> FormatOptions {
    let mut o = FormatOptions::new(places).unwrap();
    o.group_thousands = false;
    o.trim_trailing_zeros = false;
    o
}

const PHRASE_NEEDS_PLAIN: &str = "this phrase needs plain numbers and, where written, a percentage";
const PERCENT_COMBO: &str = "this combination of percentages is not defined";
const ON_OFF_NEEDS_PLAIN: &str = "'on' and 'off' need a plain number after them";

#[test]
fn phrase_forms_reject_a_percentage_where_a_plain_number_is_required() {
    let lines = [
        "10% is 10% of what",
        "10% is 10% on what",
        "10% is 10% off what",
        "10% is what % of 200",
        "20 is what % of 10%",
        "10% is what % on 200",
        "20 is what % on 10%",
        "10% is what % off 200",
        "20 is what % off 10%",
        "10% to 20 is what %",
        "20 to 10% is what %",
        "10% to 20 as %",
        "20 to 10% as %",
        "10% in 20 is what %",
        "20 in 10% is what %",
        "10% as a % of 20",
        "20 as a % of 10%",
    ];
    for line in lines {
        assert_eq!(
            evaluate(line),
            Err(Error::UnsupportedOperation(PHRASE_NEEDS_PLAIN)),
            "{line}"
        );
    }
}

#[test]
fn refusal_messages_say_which_rule_was_hit() {
    assert_eq!(
        evaluate("10% on 20%"),
        Err(Error::UnsupportedOperation(ON_OFF_NEEDS_PLAIN))
    );
    assert_eq!(
        evaluate("10% off 20%"),
        Err(Error::UnsupportedOperation(ON_OFF_NEEDS_PLAIN))
    );
    assert_eq!(
        evaluate("5 on 10%"),
        Err(Error::UnsupportedOperation(ON_OFF_NEEDS_PLAIN))
    );
    assert_eq!(
        evaluate("10% / 20%"),
        Err(Error::UnsupportedOperation(PERCENT_COMBO))
    );
    assert_eq!(
        evaluate("2^10%"),
        Err(Error::UnsupportedOperation(PERCENT_COMBO))
    );
    assert_eq!(
        evaluate("10%%"),
        Err(Error::UnsupportedOperation("a percentage of a percentage"))
    );
}

#[test]
fn keywords_inside_unbalanced_parentheses_are_not_phrase_keywords() {
    assert_eq!(
        evaluate("(5 as %"),
        Err(Error::UnexpectedWord {
            word: "as".into(),
            at: 3
        })
    );
    assert_eq!(
        evaluate("(5 %"),
        Err(Error::UnexpectedToken {
            found: "'%'".into(),
            at: 3
        })
    );
    assert_eq!(
        evaluate("(5 is what %"),
        Err(Error::UnexpectedWord {
            word: "is".into(),
            at: 3
        })
    );
    assert_eq!(
        evaluate("(20 is 10% of what"),
        Err(Error::UnexpectedWord {
            word: "is".into(),
            at: 4
        })
    );
    assert_eq!(
        evaluate("(20 is 10 on what"),
        Err(Error::UnexpectedWord {
            word: "is".into(),
            at: 4
        })
    );
    assert_eq!(
        evaluate("(20 is 10 off what"),
        Err(Error::UnexpectedWord {
            word: "is".into(),
            at: 4
        })
    );
    assert_eq!(
        evaluate("20 is 10% of what foo"),
        Err(Error::UnexpectedWord {
            word: "is".into(),
            at: 3
        })
    );
    assert_eq!(
        evaluate("20 is 0.1 of what 5"),
        Err(Error::UnexpectedWord {
            word: "is".into(),
            at: 3
        })
    );
}

#[test]
fn phrases_print_in_a_readable_form() {
    let rendered = |l: &str| babbage_core::parse_expression(l).unwrap().to_string();
    assert_eq!(rendered("20 is 10% of what"), "IsPercentOfWhat(20, (10%))");
    assert_eq!(rendered("20 as %"), "AsPercent(20)");
    assert_eq!(rendered("1 + 2 to 4 as %"), "PercentChange((1 + 2), 4)");
}

#[test]
fn text_after_what_never_changes_the_answer() {
    for line in [
        "20 is 0.1 of what 5",
        "20 is 0.1 on what 7",
        "20 is 0.1 off what 7",
        "20 is 10% of what (5)",
        "20 is 10% of what + 1",
        "20 is 10% of what %",
        "20 is what % of 10 what",
    ] {
        assert!(
            evaluate(line).is_err(),
            "{line} must not produce an answer, got {:?}",
            evaluate(line)
        );
    }
    for line in [
        "is 10% of what 20",
        "20 is what 10% of what",
        "20 10% is of what",
        "20 is of 10% what",
    ] {
        assert!(evaluate(line).is_err(), "{line} must not produce an answer");
    }
}
