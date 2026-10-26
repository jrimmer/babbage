use babbage_core::{Error, FormatOptions, Number, Value, evaluate, format_value};

const CORPUS: &str = include_str!("corpus/irrational.tsv");

/// |a - b| <= 1e-60 * max(1, |b|), in exact rational arithmetic.
fn close(a: &Number, b: &Number) -> bool {
    let tolerance = Number::from_decimal_str("1e-60").unwrap();
    let scale = if b.abs() > Number::from_i64(1) {
        b.abs()
    } else {
        Number::from_i64(1)
    };
    a.sub(b).unwrap().abs() <= tolerance.mul(&scale).unwrap()
}

fn exact_text(v: &Value) -> String {
    format!("{}/{}", v.number().numer(), v.number().denom())
}

#[test]
fn agrees_with_mpmath_on_every_line() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in CORPUS.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let (expr, kind, want) = (cols[0], cols[1], cols[2]);
        let got = evaluate(expr);
        let ok = match (&got, kind) {
            (Ok(v), "E") => !v.is_approximate() && exact_text(v) == want && v.unit().is_empty(),
            (Ok(v), "A") => {
                v.is_approximate()
                    && close(v.number(), &Number::from_decimal_str(want).unwrap())
                    && v.unit().is_empty()
            }
            (Ok(v), "D") => {
                !v.is_approximate() && exact_text(v) == want && v.unit().symbol() == "deg"
            }
            (Ok(v), "DA") => {
                v.is_approximate()
                    && close(v.number(), &Number::from_decimal_str(want).unwrap())
                    && v.unit().symbol() == "deg"
            }
            _ => false,
        };
        if !ok {
            failures.push(format!(
                "{expr:?} want {kind} {want:.40}, got {:?}",
                got.map(|v| (exact_text(&v), v.is_approximate(), v.unit().symbol()))
            ));
        }
        checked += 1;
    }
    assert!(checked > 4000, "{checked}");
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

fn shown(line: &str) -> (String, bool) {
    let v = evaluate(line).unwrap_or_else(|e| panic!("{line}: {e}"));
    let f = format_value(&v, &FormatOptions::default());
    (f.text, f.exact)
}

#[test]
fn the_example_lines() {
    for (line, want) in [
        ("sqrt(16)", "4"),
        ("cbrt(343)", "7"),
        ("square root of 81", "9"),
        ("cube root of 27", "3"),
        ("root 5 of 100", "2.5118864315"),
        ("log 20 base 4", "2.1609640474"),
        ("81 is 9 to what power", "2"),
        ("27 is 3 to the what power", "3"),
        ("sin(90 degrees)", "1"),
        ("sind(90)", "1"),
        ("π", "3.1415926536"),
        ("tau", "6.2831853072"),
        ("phi", "1.6180339887"),
        ("π to 5 digits", "3.14159"),
        ("standard deviation of 20, 30 and 40", "10"),
    ] {
        assert_eq!(shown(line).0, want, "{line}");
    }
    assert_eq!(shown("asind(0.5)").0, "30 deg");
    assert_eq!(shown("asin(0.5 degrees)").0, "30 deg");
}

#[test]
fn exactness_is_reported_honestly() {
    for exact in [
        "sqrt(16)",
        "sqrt(9/4)",
        "cbrt(-27)",
        "log10(1000)",
        "log10(0.001)",
        "log2(1024)",
        "sin(30 degrees)",
        "ln(1)",
        "exp(0)",
        "cos(0)",
        "log 81 base 3",
        "sqrt(0)",
        "standard deviation of 20, 30 and 40",
    ] {
        assert!(shown(exact).1, "{exact} is exact");
        assert!(!evaluate(exact).unwrap().is_approximate(), "{exact}");
    }
    for approximate in [
        "sqrt(2)",
        "ln(2)",
        "sin(1)",
        "π",
        "log10(2)",
        "exp(1)",
        "sin(45 degrees)",
        "tan(1 degree)",
        "cbrt(2)",
        "log 20 base 4",
    ] {
        assert!(!shown(approximate).1, "{approximate} is not exact");
        assert!(
            evaluate(approximate).unwrap().is_approximate(),
            "{approximate}"
        );
    }
}

#[test]
fn approximation_spreads_through_arithmetic() {
    for line in [
        "sqrt(2) × sqrt(2)",
        "π + 1",
        "2 × π",
        "sqrt(2) - sqrt(2)",
        "π / π",
        "(1 + sqrt(2))^2",
        "max(sqrt(2), 1)",
        "π × 0",
    ] {
        let v = evaluate(line).unwrap();
        assert!(v.is_approximate(), "{line}");
        assert!(!format_value(&v, &FormatOptions::default()).exact, "{line}");
    }
    assert!(!evaluate("sqrt(16) + 1").unwrap().is_approximate());
}

#[test]
fn degrees_and_radians() {
    assert_eq!(shown("1 rad in degrees").0, "57.2957795131 deg");
    assert!(evaluate("1 rad in degrees").unwrap().is_approximate());
    assert_eq!(shown("180 degrees in rad").0, "3.1415926536 rad");
    assert_eq!(shown("1 rev in degrees").0, "360 deg");
    assert!(!evaluate("1 rev in degrees").unwrap().is_approximate());
    assert_eq!(shown("sin(30 degrees)").0, "0.5");
    assert!(!evaluate("2 rad + 3 rad").unwrap().is_approximate());
    assert!(evaluate("2 rad + 3 degrees").unwrap().is_approximate());
    assert_eq!(shown("sin(π / 2)").0, "1");
    assert_eq!(shown("cos(π)").0, "-1");
    assert_eq!(shown("tan(π / 4)").0, "1");
}

#[test]
fn domain_errors() {
    for line in [
        "sqrt(-1)",
        "ln(0)",
        "ln(-5)",
        "log10(0)",
        "log 5 base 1",
        "log 5 base 0",
        "asin(2)",
        "acos(-2)",
        "acosh(0.5)",
        "atanh(1)",
        "atanh(-1)",
        "tan(90 degrees)",
        "tan(270 degrees)",
        "root 2 of -4",
        "root 0 of 4",
        "asind(1.5)",
        "atanh(2)",
    ] {
        assert_eq!(evaluate(line), Err(Error::OutOfDomain), "{line}");
    }
    assert_eq!(evaluate("exp(1000000)"), Err(Error::Overflow));
    assert!(evaluate("sqrt(5 m)").is_err());
    assert!(evaluate("sin(5 kg)").is_err());
    assert!(evaluate("sin(10%)").is_err());
}

#[test]
fn negative_inputs_that_are_fine() {
    assert_eq!(shown("cbrt(-8)").0, "-2");
    assert_eq!(shown("root 3 of -27").0, "-3");
    assert_eq!(shown("sin(-30 degrees)").0, "-0.5");
    assert_eq!(shown("cos(-60 degrees)").0, "0.5");
    assert_eq!(shown("tan(-45 degrees)").0, "-1");
    assert_eq!(shown("sin(390 degrees)").0, "0.5");
    assert_eq!(shown("asind(-1)").0, "-90 deg");
    assert_eq!(shown("sinh(0)").0, "0");
    assert_eq!(shown("ln(1/2)").0, "-0.6931471806");
}

#[test]
fn huge_and_tiny_arguments_do_not_hang() {
    assert!(evaluate("sqrt(10^300)").is_ok());
    assert!(evaluate("ln(10^300)").is_ok());
    assert!(evaluate("sin(10^200)").is_ok());
    assert!(evaluate("sqrt(1/10^300)").is_ok());
    let text = shown("sqrt(10^300)").0;
    assert!(text.starts_with("1,000,") && text.ends_with("T"), "{text}");
}
