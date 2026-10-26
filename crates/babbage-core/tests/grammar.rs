use babbage_core::lexer::{MAX_INPUT_BYTES, lex};
use babbage_core::{Error, Number, evaluate_plain as evaluate};

fn tok(found: &str, at: usize) -> Error {
    Error::UnexpectedToken { found: found.into(), at }
}

#[test]
fn empty_and_blank_input() {
    for s in ["", " ", "   ", "\t\n", "\u{00A0}"] {
        assert_eq!(evaluate(s), Err(Error::Empty), "{s:?}");
    }
}

#[test]
fn incomplete_expressions() {
    for s in ["1 +", "1 -", "1 *", "1 /", "1 ^", "1 **", "(1", "((1)", "(1 +", "-", "+", "2 * (3 +"] {
        assert_eq!(evaluate(s), Err(Error::UnexpectedEnd), "{s:?}");
    }
}

#[test]
fn stray_tokens_report_their_byte_offset() {
    assert_eq!(evaluate("1)"), Err(tok("')'", 1)));
    assert_eq!(evaluate("1 2"), Err(tok("number", 2)));
    assert_eq!(evaluate("*3"), Err(tok("'*'", 0)));
    assert_eq!(evaluate("/3"), Err(tok("'/'", 0)));
    assert_eq!(evaluate("^3"), Err(tok("'^'", 0)));
    assert_eq!(evaluate("()"), Err(tok("')'", 1)));
    assert_eq!(evaluate("1 + * 2"), Err(tok("'*'", 4)));
    assert_eq!(evaluate("1 + )"), Err(tok("')'", 4)));
    assert_eq!(evaluate("(1 2)"), Err(tok("number", 3)));
    assert_eq!(evaluate("2 (3)"), Err(tok("'('", 2)));
    assert_eq!(evaluate("(1)(2)"), Err(tok("'('", 3)));
    assert_eq!(evaluate("1 +* 2"), Err(tok("'*'", 3)));
    assert_eq!(evaluate("x"), Err(tok("'*'", 0)));
}

#[test]
fn bad_characters_report_offsets_in_bytes_not_chars() {
    assert_eq!(evaluate("1 @ 2"), Err(Error::UnexpectedChar { ch: '@', at: 2 }));
    assert_eq!(evaluate("1 × @"), Err(Error::UnexpectedChar { ch: '@', at: 5 }));
    assert_eq!(evaluate("é"), Err(Error::UnexpectedWord { word: "é".into(), at: 0 }));
    assert_eq!(evaluate("１"), Err(Error::UnexpectedChar { ch: '１', at: 0 }));
    assert_eq!(evaluate("٣"), Err(Error::UnexpectedChar { ch: '٣', at: 0 }));
    assert_eq!(evaluate("2xy"), Err(Error::UnexpectedWord { word: "xy".into(), at: 1 }));
    assert_eq!(evaluate("2 xy"), Err(Error::UnexpectedWord { word: "xy".into(), at: 2 }));
    assert_eq!(evaluate("1 + abc"), Err(Error::UnexpectedWord { word: "abc".into(), at: 4 }));
    assert_eq!(evaluate("ABC"), Err(Error::UnexpectedWord { word: "abc".into(), at: 0 }));
    assert_eq!(evaluate("1; 2"), Err(Error::UnexpectedChar { ch: ';', at: 1 }));
    assert_eq!(evaluate("1 — 2"), Err(Error::UnexpectedChar { ch: '—', at: 2 }));
    assert_eq!(evaluate("3 % 2"), Err(tok("'%'", 2)));
}

#[test]
fn thousands_separators_are_strict() {
    assert_eq!(evaluate("1,2"), Err(Error::UnexpectedChar { ch: ',', at: 1 }));
    assert_eq!(evaluate("1,23"), Err(Error::UnexpectedChar { ch: ',', at: 1 }));
    assert_eq!(evaluate("1,2345"), Err(Error::UnexpectedChar { ch: ',', at: 1 }));
    assert_eq!(evaluate("1,234,5"), Err(Error::UnexpectedChar { ch: ',', at: 5 }));
    assert_eq!(evaluate("1234,567"), Err(Error::InvalidNumber { text: "1234,567".into(), at: 0 }));
    assert_eq!(evaluate(",5"), Err(Error::UnexpectedChar { ch: ',', at: 0 }));
    assert_eq!(evaluate("1,000,00"), Err(Error::UnexpectedChar { ch: ',', at: 5 }));
    assert_eq!(evaluate("1, 000"), Err(Error::UnexpectedChar { ch: ',', at: 1 }));
    assert_eq!(evaluate("0,123"), Ok(Number::from_i64(123)));
    assert_eq!(evaluate("1,234,567,890"), Ok(Number::from_i64(1_234_567_890)));
}

#[test]
fn decimal_point_strictness() {
    assert_eq!(evaluate("5."), Err(Error::UnexpectedChar { ch: '.', at: 1 }));
    assert_eq!(evaluate("."), Err(Error::UnexpectedChar { ch: '.', at: 0 }));
    assert_eq!(evaluate("1.2.3"), Err(Error::InvalidNumber { text: "1.2.3".into(), at: 0 }));
    assert_eq!(evaluate("1..2"), Err(Error::UnexpectedChar { ch: '.', at: 1 }));
    assert_eq!(evaluate("1.2.3.4"), Err(Error::InvalidNumber { text: "1.2.3".into(), at: 0 }));
}

#[test]
fn exponent_notation_boundaries() {
    assert_eq!(evaluate("1e100001"), Err(Error::Overflow));
    assert_eq!(evaluate("1e-100001"), Err(Error::Overflow));
    assert!(evaluate("1e100000").is_ok());
    assert!(evaluate("1e-100000").is_ok());
    assert_eq!(evaluate("1e999999999999999999999"), Err(Error::Overflow));
    assert_eq!(evaluate("1e"), Err(Error::UnexpectedWord { word: "e".into(), at: 1 }));
    assert_eq!(evaluate("1e+"), Err(Error::UnexpectedWord { word: "e".into(), at: 1 }));
    assert_eq!(evaluate("1ex"), Err(Error::UnexpectedWord { word: "ex".into(), at: 1 }));
}

#[test]
fn deep_structures_evaluate_without_recursion() {
    let n = 20_000;
    assert_eq!(evaluate(&format!("{}1{}", "(".repeat(n), ")".repeat(n))), Ok(Number::from_i64(1)));
    assert_eq!(evaluate(&format!("{}1", "-".repeat(n))), Ok(Number::from_i64(1)));
    assert_eq!(evaluate(&format!("{}1", "-".repeat(n + 1))), Ok(Number::from_i64(-1)));
    assert_eq!(evaluate(&format!("{}1", "+".repeat(n))), Ok(Number::from_i64(1)));
    assert_eq!(evaluate(&format!("1{}", "+1".repeat(n))), Ok(Number::from_i64(n as i64 + 1)));
    assert_eq!(evaluate(&format!("1{}", "^1".repeat(n))), Ok(Number::from_i64(1)));
    let negated_parens = format!("{}1{}", "-(".repeat(n / 2), ")".repeat(n / 2));
    assert_eq!(evaluate(&negated_parens), Ok(Number::from_i64(1)));
}

#[test]
fn deep_structures_do_not_need_a_big_stack() {
    on_small_stack(128, || {
        let n = 20_000;
        assert!(evaluate(&format!("{}1{}", "(".repeat(n), ")".repeat(n))).is_ok());
        assert!(evaluate(&format!("{}1", "-".repeat(n))).is_ok());
        assert!(evaluate(&format!("1{}", "+1".repeat(n))).is_ok());
        assert!(evaluate(&format!("1{}", "*1".repeat(n))).is_ok());
        assert!(evaluate(&format!("1{}", "^1".repeat(n))).is_ok());
        assert!(evaluate(&format!("{}1{}", "-(".repeat(n / 2), ")".repeat(n / 2))).is_ok());
        let parsed = babbage_core::parse_expression(&format!("1{}", "+1".repeat(n))).unwrap();
        drop(parsed);
    });
}

#[test]
fn unbalanced_deep_input_is_an_error_not_a_crash() {
    let n = 20_000;
    assert_eq!(evaluate(&"(".repeat(n)), Err(Error::UnexpectedEnd));
    assert_eq!(evaluate(&format!("{}1", "(".repeat(n))), Err(Error::UnexpectedEnd));
    assert_eq!(evaluate(&format!("1{}", ")".repeat(n))), Err(tok("')'", 1)));
    assert_eq!(evaluate(&"-".repeat(n)), Err(Error::UnexpectedEnd));
}

#[test]
fn power_towers_hit_the_size_guard_quickly() {
    assert!(evaluate("2^2^2^2^2").is_ok());
    assert_eq!(evaluate("2^2^2^2^2^2"), Err(Error::Overflow));
    assert_eq!(evaluate("9^9^9^9"), Err(Error::Overflow));
}

#[test]
fn input_size_limit() {
    let at_limit = format!("{}1", " ".repeat(MAX_INPUT_BYTES - 1));
    assert_eq!(at_limit.len(), MAX_INPUT_BYTES);
    assert_eq!(evaluate(&at_limit), Ok(Number::from_i64(1)));
    assert_eq!(evaluate(&format!("{at_limit} ")), Err(Error::InputTooLong));
    assert_eq!(lex(&"1".repeat(MAX_INPUT_BYTES + 1)).unwrap_err(), Error::InputTooLong);
}


fn on_small_stack(kib: usize, f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new().stack_size(kib * 1024).spawn(f).unwrap().join().unwrap();
}

#[test]
fn error_spans_stop_exactly_where_the_bad_number_stops() {
    assert_eq!(evaluate("1234,567+1"), Err(Error::InvalidNumber { text: "1234,567".into(), at: 0 }));
    assert_eq!(evaluate("2 * 1234,567 + 1"), Err(Error::InvalidNumber { text: "1234,567".into(), at: 4 }));
    assert_eq!(evaluate("1.2.3+1"), Err(Error::InvalidNumber { text: "1.2.3".into(), at: 0 }));
    assert_eq!(evaluate("1.2.345 + 1"), Err(Error::InvalidNumber { text: "1.2.345".into(), at: 0 }));
    assert_eq!(evaluate("7 + 1.2.3 * 2"), Err(Error::InvalidNumber { text: "1.2.3".into(), at: 4 }));
}

#[test]
fn a_comma_is_only_a_group_separator_before_exactly_three_digits() {
    assert_eq!(evaluate("1,a23"), Err(Error::UnexpectedChar { ch: ',', at: 1 }));
    assert_eq!(evaluate("1,2a3"), Err(Error::UnexpectedChar { ch: ',', at: 1 }));
    assert_eq!(evaluate("1,23a"), Err(Error::UnexpectedChar { ch: ',', at: 1 }));
    assert_eq!(evaluate("1,"), Err(Error::UnexpectedChar { ch: ',', at: 1 }));
    assert_eq!(evaluate("12,345+1"), Ok(Number::from_i64(12346)));
    assert_eq!(evaluate("123,456,789"), Ok(Number::from_i64(123_456_789)));
}

#[test]
fn a_dot_after_a_fraction_is_only_a_second_point_before_a_digit() {
    assert_eq!(evaluate("1.5."), Err(Error::UnexpectedChar { ch: '.', at: 3 }));
    assert_eq!(evaluate("1.5.x"), Err(Error::UnexpectedChar { ch: '.', at: 3 }));
    assert_eq!(evaluate("1.5. + 1"), Err(Error::UnexpectedChar { ch: '.', at: 3 }));
}

#[test]
fn exponent_with_a_fraction_keeps_the_full_range() {
    let tiny = evaluate("1.5e-100000").unwrap();
    assert_eq!(tiny.numer().to_string(), "3");
    assert!(evaluate("1.5e100000").is_ok());
    assert!(evaluate("0.000001e-100000").is_ok());
    assert!(evaluate("123456.789e100000").is_ok());
}
