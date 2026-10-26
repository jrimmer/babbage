use babbage_core::{
    Error, FormatOptions, LineKind, LineResult, SheetOptions, evaluate, evaluate_sheet,
    format_value,
};

const CORPUS: &str = include_str!("corpus/logic.tsv");

fn shown(line: &str) -> String {
    let v = evaluate(line).unwrap_or_else(|e| panic!("{line}: {e}"));
    format_value(&v, &FormatOptions::default()).text
}

#[test]
fn agrees_with_the_logic_oracle_on_every_line() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in CORPUS.lines() {
        let (expr, want) = line.rsplit_once('\t').expect("tab-separated");
        let got = evaluate(expr).map(|v| format_value(&v, &FormatOptions::default()).text);
        if got.as_deref() != Ok(want) {
            failures.push(format!("{expr:?}: want {want}, got {got:?}"));
        }
        checked += 1;
    }
    assert!(checked > 150_000, "{checked}");
    assert!(
        failures.is_empty(),
        "{} of {checked} differ, first 12:\n{}",
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
fn the_reference_comparisons() {
    for (line, want) in [
        ("20km == 20,000 m", "true"),
        ("1 > 2", "false"),
        ("1 != 2", "true"),
        ("2 >= 2", "true"),
        ("2 <= 1", "false"),
        ("true", "true"),
        ("false", "false"),
        ("$500 > $499.99", "true"),
        ("10% < 20%", "true"),
        ("1 < 2 and 2 < 3", "true"),
        ("1 < 2 && 3 < 2", "false"),
        ("1 > 2 || 2 > 1", "true"),
        ("true == true", "true"),
        ("true != false", "true"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
}

#[test]
fn the_reference_bases() {
    for (line, want) in [
        ("256 as hex", "0x100"),
        ("99 in binary", "0b1100011"),
        ("0x9F31 to decimal", "40,753"),
        ("0b1000101 to octal", "0o105"),
        ("0b101101 as base 8", "0o55"),
        ("0x2D as base 2", "0b101101"),
        ("int(0o55)", "45"),
        ("hex(99)", "0x63"),
        ("bin(0x73)", "0b1110011"),
        ("255 in hex", "0xFF"),
        ("0xFF", "0xFF"),
        ("0b101", "0b101"),
        ("0o17", "0o17"),
        ("0xFF + 1", "256"),
        ("0xFF & 0x0F", "0xF"),
        ("6 AND 3", "2"),
        ("6 OR 3", "7"),
        ("6 xor 3", "5"),
        ("6 XOR 3", "5"),
        ("1 << 10", "1,024"),
        ("1024 >> 3", "128"),
        ("255 as base 3", "100110₃"),
        ("0 as hex", "0x0"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
}

#[test]
fn comparisons_and_logic_refuse_nonsense() {
    for line in [
        "1 < true",
        "true < false",
        "true + 1",
        "true × 2",
        "1 && 2",
        "true & false",
        "-1 & 1",
        "1.5 & 1",
        "5 m & 1",
        "1 << -1",
        "5 m < 3 kg",
        "50% < 3",
        "$5 < €5",
        "1 < 2 < 3",
        "5 and true",
        "0x",
        "0xG",
        "0b2",
        "255 as base 1",
        "255 as base 37",
        "2.5 as hex",
        "-0x10 & 1",
    ] {
        assert!(
            evaluate(line).is_err(),
            "{line} should fail, got {:?}",
            evaluate(line)
        );
    }
    assert_eq!(evaluate("$5 < €5"), Err(Error::CurrencyRateNeeded));
}

#[test]
fn approximate_values_are_not_compared_for_equality() {
    assert!(evaluate("sqrt(2) × sqrt(2) == 2").is_err());
    assert_eq!(shown("sqrt(2) > 1.4"), "true");
    assert_eq!(shown("sqrt(2) < 1.5"), "true");
    assert_eq!(shown("sqrt(16) == 4"), "true");
    assert_eq!(shown("π > 3"), "true");
    assert!(evaluate("π == π").is_err());
}

#[test]
fn precedence_follows_c() {
    assert_eq!(shown("1 + 2 < 4"), "true");
    assert_eq!(shown("2 × 3 == 6"), "true");
    assert_eq!(shown("1 < 2 == true"), "true");
    assert_eq!(shown("true or false and false"), "true");
    assert_eq!(shown("false and false or true"), "true");
    assert_eq!(shown("1 << 2 + 1"), "8");
    assert_eq!(shown("6 | 1 & 3"), "7");
    assert_eq!(shown("6 xor 3 & 1"), "7");
}

fn sheet(lines: &[&str]) -> Vec<LineResult> {
    evaluate_sheet(lines, &SheetOptions::default())
}

fn answers(lines: &[&str]) -> Vec<String> {
    sheet(lines)
        .iter()
        .map(|r| match &r.kind {
            LineKind::Value(v) => {
                format_value(v, &r.display_options(&FormatOptions::default())).text
            }
            other => format!("{other:?}"),
        })
        .collect()
}

#[test]
fn the_reference_conditional_sheets() {
    let r = answers(&[
        "earnings = $45k",
        "if earnings > $30k then tax = 20% else tax = 5%",
        "My tax paid: earnings × tax",
    ]);
    assert_eq!(r, ["$45,000.00", "20%", "$9,000.00"].map(String::from));

    let r = answers(&[
        "income = $35k",
        "expenses = $21.5k",
        "profitable = true if income > expenses",
        "insolvent = false unless expenses > income",
    ]);
    assert_eq!(&r[2..], ["true", "false"].map(String::from));

    let r = answers(&[
        "BMI = 24",
        "Underweight = BMI < 18.5",
        "Healthy Weight = BMI >= 18.5 and BMI < 25",
        "Overweight = BMI >= 25 and BMI < 30",
        "Obese = BMI >= 30",
    ]);
    assert_eq!(
        &r[1..],
        ["false", "true", "false", "false"].map(String::from)
    );

    let r = answers(&[
        "cost = $500",
        "discount = true",
        "if discount then cost = cost - 10%",
        "cost",
    ]);
    assert_eq!(&r[2..], ["$450.00", "$450.00"].map(String::from));
}

#[test]
fn a_false_condition_without_an_else_leaves_everything_alone() {
    let r = answers(&[
        "x = 5",
        "if x > 10 then x = 99",
        "x",
        "x = 7 if x > 10",
        "x",
    ]);
    assert_eq!(r[2], "5");
    assert_eq!(r[1], "Text");
    assert_eq!(r[3], "Text");
    assert_eq!(r[4], "5");
}

#[test]
fn the_other_branch_runs_and_conditions_can_be_variables() {
    let r = answers(&[
        "go = false",
        "if go then 1 else 2",
        "if not_defined > 1 then 3",
        "x = 3 unless go",
        "x",
    ]);
    assert_eq!(r[1], "2");
    assert!(r[2].contains("Error"), "{}", r[2]);
    assert_eq!(&r[3..], ["3", "3"].map(String::from));
}

#[test]
fn a_condition_that_is_not_true_or_false_is_refused_in_the_prefix_form() {
    let r = sheet(&["if 5 then 1 else 2"]);
    assert!(matches!(
        r[0].kind,
        LineKind::Error(Error::UnsupportedOperation(_))
    ));
}

#[test]
fn prose_that_happens_to_contain_if_still_evaluates() {
    let r = answers(&["what if 5 + 5", "5 + 5 if you like"]);
    assert_eq!(r, ["10", "10"].map(String::from));
}
