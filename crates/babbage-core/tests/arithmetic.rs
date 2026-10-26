use babbage_core::{Error, FormatOptions, Number, evaluate_plain as evaluate, format_decimal};

fn exact(expr: &str) -> String {
    let n = evaluate(expr).unwrap_or_else(|e| panic!("{expr:?} failed: {e}"));
    format!("{}/{}", n.numer(), n.denom())
}

fn shown(expr: &str) -> String {
    format_decimal(&evaluate(expr).unwrap(), &FormatOptions::default()).text
}

#[test]
fn exact_values() {
    let table: &[(&str, &str)] = &[
        ("12 + 12 + 15", "39/1"),
        ("85 × 12", "1020/1"),
        ("(800 x 800 x 1,000) / 10", "64000000/1"),
        ("256 x 256 x 256", "16777216/1"),
        ("19/29", "19/29"),
        ("76 × 19/29", "1444/29"),
        ("137 × 1.82", "12467/50"),
        ("108 × 1.36", "3672/25"),
        ("55,368 / 24 / 365", "2307/365"),
        ("1,024 × 16", "16384/1"),
        ("0.92 – .05", "87/100"),
        (".002 ×6", "3/250"),
        ("895,000 / (895,000 + 102,500)", "358/399"),
    ];
    for (expr, want) in table {
        assert_eq!(exact(expr), *want, "{expr}");
    }
}

#[test]
fn shown_values_for_the_screenshots_plain_arithmetic_lines() {
    let table = [
        ("19/29", "0.6551724138"),
        ("76 × 19/29", "49.7931034483"),
        ("137 × 1.82", "249.34"),
        ("108 × 1.36", "146.88"),
        ("55,368 / 24 / 365", "6.3205479452"),
        ("1,024 × 16", "16,384"),
        ("1,024 × 12", "12,288"),
        ("0.92 – .05", "0.87"),
        (".002 ×6", "0.012"),
        ("895,000 / (895,000 + 102,500)", "0.8972431078"),
        ("85 × 12", "1,020"),
    ];
    for (expr, want) in table {
        assert_eq!(shown(expr), want, "{expr}");
    }
}

#[test]
fn binary_float_traps_are_exact() {
    let table = [
        ("0.1 + 0.2", "3/10"),
        ("0.1 * 3", "3/10"),
        ("(0.1 + 0.7) * 10", "8/1"),
        ("4.35 * 100", "435/1"),
        ("0.57 * 100", "57/1"),
        ("9.95 * 100", "995/1"),
        ("1.1 * 1.1", "121/100"),
        ("3 * 1.1", "33/10"),
        ("1 - 0.9", "1/10"),
        ("1/3 * 3", "1/1"),
        ("1/3 + 1/3 + 1/3", "1/1"),
        ("1/7 * 7", "1/1"),
        ("10/3 * 3", "10/1"),
        ("1e-7 * 1e7", "1/1"),
        ("0.1 + 0.2 - 0.3", "0/1"),
        ("123456789 * 987654321", "121932631112635269/1"),
        ("9007199254740993 + 1", "9007199254740994/1"),
        ("9007199254740993 * 3", "27021597764222979/1"),
        ("2^64", "18446744073709551616/1"),
        ("2^64 + 1", "18446744073709551617/1"),
        ("2^100", "1267650600228229401496703205376/1"),
        ("0.1^3", "1/1000"),
        ("(1/3)^3", "1/27"),
    ];
    for (expr, want) in table {
        assert_eq!(exact(expr), *want, "{expr}");
    }
}

#[test]
fn precedence_and_associativity() {
    let table = [
        ("2+3*4", "14/1"),
        ("(2+3)*4", "20/1"),
        ("2*3+4", "10/1"),
        ("10-2-3", "5/1"),
        ("10-3-2-1", "4/1"),
        ("100/10/5", "2/1"),
        ("8/4/2", "1/1"),
        ("2*3/4", "3/2"),
        ("2^3*2", "16/1"),
        ("2*3^2", "18/1"),
        ("2^3^2", "512/1"),
        ("-3^2", "-9/1"),
        ("(-3)^2", "9/1"),
        ("-(-3)", "3/1"),
        ("--3", "3/1"),
        ("- - 3", "3/1"),
        ("+3", "3/1"),
        ("2^-3", "1/8"),
        ("2^-1^2", "1/2"),
        ("1-(-1)", "2/1"),
        ("1--1", "2/1"),
        ("1+-1", "0/1"),
        ("7/-2", "-7/2"),
        ("-7/2", "-7/2"),
        ("-2^2", "-4/1"),
        ("(-2)^3", "-8/1"),
        ("2^(2/1)", "4/1"),
        ("(1/2)^2", "1/4"),
        ("2^0", "1/1"),
        ("5^1", "5/1"),
    ];
    for (expr, want) in table {
        assert_eq!(exact(expr), *want, "{expr}");
    }
}

#[test]
fn every_operator_spelling_means_the_same() {
    let muls = ["*", "×", "x", "X", "·", "⋅", " x ", " × "];
    let divs = ["/", "÷", "∕"];
    let subs = ["-", "–", "−"];
    let pows = ["^", "**"];
    for m in muls {
        assert_eq!(exact(&format!("6{m}7")), "42/1", "mul {m:?}");
    }
    for d in divs {
        assert_eq!(exact(&format!("6{d}8")), "3/4", "div {d:?}");
    }
    for s in subs {
        assert_eq!(exact(&format!("6{s}8")), "-2/1", "sub {s:?}");
        assert_eq!(exact(&format!("{s}6")), "-6/1", "neg {s:?}");
    }
    for p in pows {
        assert_eq!(exact(&format!("2{p}10")), "1024/1", "pow {p:?}");
    }
}

#[test]
fn number_literal_forms() {
    let table = [
        ("007", "7/1"),
        ("0.50", "1/2"),
        ("00.5", "1/2"),
        (".5", "1/2"),
        ("1e3", "1000/1"),
        ("1E3", "1000/1"),
        ("1e+3", "1000/1"),
        ("1.5e3", "1500/1"),
        ("2e-3", "1/500"),
        (".5e1", "5/1"),
        ("5e0", "5/1"),
        ("0e0", "0/1"),
        ("1,000", "1000/1"),
        ("1,000,000", "1000000/1"),
        ("1,000.5", "2001/2"),
        ("12,288", "12288/1"),
        ("999,999,999,999", "999999999999/1"),
        ("0.000000000000000000000001", "1/1000000000000000000000000"),
        ("123456789012345678901234567890", "123456789012345678901234567890/1"),
    ];
    for (expr, want) in table {
        assert_eq!(exact(expr), *want, "{expr}");
    }
}

#[test]
fn whitespace_variants_are_ignored() {
    for ws in [" ", "  ", "\t", "\u{00A0}", "\u{2009}", "\n"] {
        assert_eq!(exact(&format!("1{ws}+{ws}2")), "3/1", "{ws:?}");
    }
    assert_eq!(exact("  \t 4 * ( 5 + 1 )   "), "24/1");
}

#[test]
fn arithmetic_errors() {
    assert_eq!(evaluate("1/0"), Err(Error::DivisionByZero));
    assert_eq!(evaluate("0/0"), Err(Error::DivisionByZero));
    assert_eq!(evaluate("1/(2-2)"), Err(Error::DivisionByZero));
    assert_eq!(evaluate("5/(0.1-0.1)"), Err(Error::DivisionByZero));
    assert_eq!(evaluate("0^0"), Err(Error::ZeroToTheZero));
    assert_eq!(evaluate("0^-1"), Err(Error::DivisionByZero));
    assert_eq!(evaluate("0^(3-3)"), Err(Error::ZeroToTheZero));
    assert_eq!(evaluate("2^0.5"), Err(Error::NonIntegerExponent));
    assert_eq!(evaluate("2^(1/2)"), Err(Error::NonIntegerExponent));
    assert_eq!(evaluate("2^(1/3*3)"), Ok(Number::from_i64(2)));
    assert_eq!(evaluate("1 + 1/0"), Err(Error::DivisionByZero));
    assert_eq!(evaluate("(1/0) * 0"), Err(Error::DivisionByZero));
}

#[test]
fn zero_results_are_a_single_value() {
    for expr in ["0", "-0", "0*-5", "5-5", "0/7", "-0/7", "0.0", "0e5", "0^3"] {
        assert_eq!(evaluate(expr), Ok(Number::from_i64(0)), "{expr}");
        assert!(!evaluate(expr).unwrap().is_negative(), "{expr}");
    }
}

#[test]
fn representation_is_canonical() {
    let half = Number::from_i64(1).div(&Number::from_i64(2)).unwrap();
    assert_eq!(evaluate("2/4").unwrap(), half);
    assert_eq!(evaluate("-1/-2").unwrap(), half);
    assert_eq!(evaluate("50/100").unwrap(), half);
    assert_eq!(evaluate("0.5").unwrap(), half);
    let neg = evaluate("1/-2").unwrap();
    assert!(neg.denom().sign() == num_bigint::Sign::Plus);
    assert_eq!(neg, half.neg());
}
