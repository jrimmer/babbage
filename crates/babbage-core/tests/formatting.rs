use babbage_core::{Error, FormatOptions, Formatted, Number, RoundingMode, evaluate, format_decimal};

fn fmt(expr: &str, dp: u32, mode: RoundingMode, group: bool, trim: bool) -> Formatted {
    let mut o = FormatOptions::new(dp).unwrap();
    o.rounding = mode;
    o.group_thousands = group;
    o.trim_trailing_zeros = trim;
    format_decimal(&evaluate(expr).unwrap(), &o)
}

fn plain(expr: &str, dp: u32, mode: RoundingMode) -> (String, bool) {
    let f = fmt(expr, dp, mode, false, true);
    (f.text, f.exact)
}

use RoundingMode::{AwayFromZero as Away, HalfAwayFromZero as HalfAway, HalfEven, TowardZero as Zero};

#[test]
fn rounding_mode_matrix() {
    // (expr, dp, [HalfAway, HalfEven, TowardZero, AwayFromZero])
    let rows: &[(&str, u32, [&str; 4])] = &[
        ("0.125", 2, ["0.13", "0.12", "0.12", "0.13"]),
        ("0.375", 2, ["0.38", "0.38", "0.37", "0.38"]),
        ("-0.125", 2, ["-0.13", "-0.12", "-0.12", "-0.13"]),
        ("-0.375", 2, ["-0.38", "-0.38", "-0.37", "-0.38"]),
        ("0.5", 0, ["1", "0", "0", "1"]),
        ("1.5", 0, ["2", "2", "1", "2"]),
        ("2.5", 0, ["3", "2", "2", "3"]),
        ("3.5", 0, ["4", "4", "3", "4"]),
        ("-0.5", 0, ["-1", "0", "0", "-1"]),
        ("-1.5", 0, ["-2", "-2", "-1", "-2"]),
        ("-2.5", 0, ["-3", "-2", "-2", "-3"]),
        ("0.49999999999", 0, ["0", "0", "0", "1"]),
        ("0.50000000001", 0, ["1", "1", "0", "1"]),
        ("2.675", 2, ["2.68", "2.68", "2.67", "2.68"]),
        ("1.005", 2, ["1.01", "1", "1", "1.01"]),
        ("1/3", 4, ["0.3333", "0.3333", "0.3333", "0.3334"]),
        ("2/3", 4, ["0.6667", "0.6667", "0.6666", "0.6667"]),
        ("-2/3", 4, ["-0.6667", "-0.6667", "-0.6666", "-0.6667"]),
        ("1/6", 0, ["0", "0", "0", "1"]),
        ("5/6", 0, ["1", "1", "0", "1"]),
        ("7", 3, ["7", "7", "7", "7"]),
        ("0.9995", 3, ["1", "1", "0.999", "1"]),
        ("9.995", 2, ["10", "10", "9.99", "10"]),
        ("99.5", 0, ["100", "100", "99", "100"]),
        ("-99.5", 0, ["-100", "-100", "-99", "-100"]),
    ];
    for (expr, dp, want) in rows {
        for (mode, want) in [HalfAway, HalfEven, Zero, Away].into_iter().zip(want) {
            assert_eq!(plain(expr, *dp, mode).0, *want, "{expr} dp={dp} {mode:?}");
        }
    }
}

#[test]
fn exact_flag_is_true_only_when_the_text_is_the_true_value() {
    let rows: &[(&str, u32, bool)] = &[
        ("5/4", 2, true),
        ("5/4", 1, false),
        ("5/4", 0, false),
        ("1/3", 10, false),
        ("1/3", 1000, false),
        ("0.1 + 0.2", 1, true),
        ("0.1 + 0.2", 0, false),
        ("1,000", 0, true),
        ("1/1024", 10, true),
        ("1/1024", 9, false),
        ("0", 5, true),
        ("1e-11", 10, false),
        ("1e-10", 10, true),
        ("-1/8", 3, true),
        ("-1/8", 2, false),
    ];
    for (expr, dp, want) in rows {
        for mode in [HalfAway, HalfEven, Zero, Away] {
            assert_eq!(plain(expr, *dp, mode).1, *want, "{expr} dp={dp} {mode:?}");
        }
    }
}

#[test]
fn a_value_that_rounds_to_zero_never_prints_a_minus_sign() {
    for mode in [HalfAway, HalfEven, Zero] {
        assert_eq!(plain("-1/3000000", 5, mode), ("0".to_string(), false), "{mode:?}");
    }
    assert_eq!(plain("-1/3000000", 5, Away).0, "-0.00001");
    assert_eq!(plain("-0.4", 0, HalfAway), ("0".to_string(), false));
    assert_eq!(plain("-0", 3, HalfAway), ("0".to_string(), true));
    assert_eq!(plain("0 * -5", 3, HalfAway), ("0".to_string(), true));
    assert_eq!(fmt("-1/3000000", 5, HalfAway, false, false).text, "0.00000");
}

#[test]
fn trailing_zero_handling() {
    assert_eq!(fmt("1.5", 4, HalfAway, false, true).text, "1.5");
    assert_eq!(fmt("1.5", 4, HalfAway, false, false).text, "1.5000");
    assert_eq!(fmt("2", 3, HalfAway, false, true).text, "2");
    assert_eq!(fmt("2", 3, HalfAway, false, false).text, "2.000");
    assert_eq!(fmt("0.99999", 2, HalfAway, false, true).text, "1");
    assert_eq!(fmt("0.99999", 2, HalfAway, false, false).text, "1.00");
    assert_eq!(fmt("100", 0, HalfAway, false, true).text, "100");
    assert_eq!(fmt("100", 2, HalfAway, false, true).text, "100");
    assert_eq!(fmt("10.50", 5, HalfAway, false, true).text, "10.5");
    assert_eq!(fmt("0.05", 5, HalfAway, false, true).text, "0.05");
    assert_eq!(fmt("1e-9", 10, HalfAway, false, true).text, "0.000000001");
}

#[test]
fn thousands_grouping() {
    let rows = [
        ("0", "0"),
        ("1", "1"),
        ("12", "12"),
        ("123", "123"),
        ("1234", "1,234"),
        ("12345", "12,345"),
        ("123456", "123,456"),
        ("1234567", "1,234,567"),
        ("-1234567", "-1,234,567"),
        ("1000000000", "1,000,000,000"),
        ("1234567.891", "1,234,567.891"),
        ("0.123456", "0.123456"),
        ("999.9999999999", "999.9999999999"),
        ("999.99999999999", "1,000"),
        ("-999.99999999999", "-1,000"),
        ("999999.99999999999", "1,000,000"),
    ];
    for (expr, want) in rows {
        assert_eq!(fmt(expr, 10, HalfAway, true, true).text, want, "{expr}");
        assert_eq!(fmt(expr, 10, HalfAway, false, true).text, want.replace(',', ""), "{expr} ungrouped");
    }
    let big = "1,".to_string() + &"000,".repeat(16) + "000";
    assert_eq!(fmt("10^51", 0, HalfAway, true, true).text, big);
}

#[test]
fn precision_bounds() {
    assert!(FormatOptions::new(0).is_ok());
    assert!(FormatOptions::new(1000).is_ok());
    assert_eq!(FormatOptions::new(1001), Err(Error::InvalidPrecision));
    assert_eq!(FormatOptions::new(u32::MAX), Err(Error::InvalidPrecision));
    let f = fmt("1/3", 1000, HalfAway, false, true);
    assert_eq!(f.text.len(), 2 + 1000);
    assert!(f.text.starts_with("0.3333") && f.text.ends_with("3333"));
    assert!(!f.exact);
    assert_eq!(FormatOptions::new(7).unwrap().decimal_places(), 7);
}

#[test]
fn defaults_match_the_reference_app() {
    let o = FormatOptions::default();
    assert_eq!(o.decimal_places(), 10);
    assert_eq!(o.rounding, HalfAway);
    assert!(o.group_thousands && o.trim_trailing_zeros);
}

#[test]
fn exact_decimal_rendering() {
    let rows: &[(&str, Option<&str>)] = &[
        ("1/8", Some("0.125")),
        ("-3/20", Some("-0.15")),
        ("1/1024", Some("0.0009765625")),
        ("1/625", Some("0.0016")),
        ("7", Some("7")),
        ("0", Some("0")),
        ("-0.5", Some("-0.5")),
        ("1e3", Some("1000")),
        ("1/3", None),
        ("1/6", None),
        ("1/7", None),
        ("1/12", None),
        ("22/7", None),
        ("1/(2^10*5^10)", Some("0.0000000001")),
        ("1/(2^3*5^7)", Some("0.0000016")),
    ];
    for (expr, want) in rows {
        let n = evaluate(expr).unwrap();
        assert_eq!(n.to_exact_decimal().as_deref(), *want, "{expr}");
        assert_eq!(n.is_terminating(), want.is_some(), "{expr}");
    }
    assert_eq!(evaluate("1/8").unwrap().terminating_places(), Some(3));
    assert_eq!(evaluate("1/625").unwrap().terminating_places(), Some(4));
    assert_eq!(evaluate("12").unwrap().terminating_places(), Some(0));
    assert_eq!(evaluate("1/3").unwrap().terminating_places(), None);
    assert_eq!(Number::from_i64(0).terminating_places(), Some(0));
}
