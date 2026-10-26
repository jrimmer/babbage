use babbage_core::{Error, FormatOptions, evaluate, format_value};

fn shown(line: &str) -> String {
    let v = evaluate(line).unwrap_or_else(|e| panic!("{line}: {e}"));
    format_value(&v, &FormatOptions::default()).text
}

fn approx(line: &str) -> bool {
    evaluate(line).unwrap().is_approximate()
}

#[test]
fn the_reference_compound_interest_examples() {
    for (line, want) in [
        ("$1,000 after 3 years at 7%", "$1,225.04"),
        (
            "$1,000 after 3 years at 7% compounding monthly",
            "$1,232.93",
        ),
        (
            "$1,000 after 3 years at 7% compounding quarterly",
            "$1,231.44",
        ),
        ("interest on $1,000 after 3 years @ 7%", "$225.04"),
        ("present value of $1,000 after 20 years at 10%", "$148.64"),
        // `for` reads as `after`, as the reference note writes it.
        ("$1,000 for 3 years at 7%", "$1,225.04"),
        ("$1,000 for 3 years at 7% compounding monthly", "$1,232.93"),
        (
            "$1,000 for 3 years at 7% compounding quarterly",
            "$1,231.44",
        ),
        (
            "interest on $1,000 for 3 years at 7% compounding monthly",
            "$232.93",
        ),
        (
            "interest on $1,000 for 3 years at 7% compounding quarterly",
            "$231.44",
        ),
        ("interest on $1,000 for 3 years @ 7%", "$225.04"),
        ("present value of $1,000 for 20 years at 10%", "$148.64"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
    assert!(!approx("$1,000 after 3 years at 7%"));
    for line in ["$1,000 for", "$1,000 for 3 years", "$1,000 for at 7%"] {
        assert!(evaluate(line).is_err(), "{line}");
    }
}

#[test]
fn the_reference_loan_examples() {
    for (line, want) in [
        ("daily repayment on $10,000 over 6 years at 6%", "$5.45"),
        ("monthly repayment on $10,000 over 6 years at 6%", "$165.73"),
        (
            "annual repayment on $10,000 over 6 years at 6%",
            "$1,988.75",
        ),
        (
            "total repayment on $10,000 over 6 years at 6%",
            "$11,932.48",
        ),
        ("daily interest on $10,000 over 6 years at 6%", "$0.88"),
        ("monthly interest on $10,000 over 6 years at 6%", "$26.84"),
        ("annual interest on $10,000 over 6 years at 6%", "$322.08"),
        ("total interest on $10,000 over 6 years at 6%", "$1,932.48"),
    ] {
        assert_eq!(shown(line), want, "{line}");
        assert!(!approx(line), "{line} is exact");
    }
}

#[test]
fn the_reference_growth_examples() {
    assert_eq!(shown("20k after 6 months at 10% per month"), "35,431.22");
    assert_eq!(
        shown("time from $20k to $40k at $2k every month in months"),
        "10 months"
    );
    assert!(shown("time from 20k to 100k at 10% growth per month").starts_with("16.88"));
    assert!(approx("time from 20k to 100k at 10% growth per month"));
    assert!(shown("growth per day from 10k to 15k over 1 week").starts_with("5.96"));
    assert!(approx("growth per day from 10k to 15k over 1 week"));
    assert!(!shown("growth per day from 10k to 20k over 1 week").is_empty());
}

#[test]
fn returns_and_multiples() {
    assert_eq!(shown("$500 invested $1,500 returned"), "3x");
    assert!(
        shown("annual return on $1,000 invested $2,500 returned after 7 years")
            .starts_with("13.98")
    );
    assert_eq!(
        shown("annual return on $1,000 invested $2,500 returned after 7 years"),
        "13.9852281048%"
    );
    assert_eq!(
        shown("annual return on $1,000 invested $2,000 returned after 1 year"),
        "100%"
    );
    assert!(!approx(
        "annual return on $1,000 invested $4,000 returned after 2 years"
    ));
    assert_eq!(
        shown("annual return on $1,000 invested $4,000 returned after 2 years"),
        "100%"
    );
}

#[test]
fn the_reference_multiplier_examples() {
    for (line, want) in [
        ("20/5 as multiplier", "4x"),
        ("50 as x of 5", "10x"),
        ("2 as multiplier of 1", "2x"),
        ("2 as multiplier on 1", "1x"),
        ("1 as x off 2", "0.5x"),
        ("50 to 75 is what x", "1.5x"),
        ("20 to 40 as x", "2x"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
}

#[test]
fn the_reference_rate_examples() {
    assert_eq!(shown("$99 per week"), "$99.00/week");
    assert_eq!(shown("30 hours at $30/hour"), "$900.00");
    assert_eq!(shown("$500 at $20/hour"), "25 hours");
    assert_eq!(shown("90 km over 3 days"), "30 km/day");
    assert_eq!(shown("time to upload 3GB at 10 MB/s"), "300 s");
    assert_eq!(shown("$24 a day for a year"), "$8,765.82");
    assert_eq!(shown("$10/day in $/month"), "$304.37/month");
    assert_eq!(shown("$30 per hour × 8 hours"), "$240.00");
}

#[test]
fn zero_rates_and_edge_cases() {
    assert_eq!(
        shown("monthly repayment on $1,200 over 1 year at 0%"),
        "$100.00"
    );
    assert_eq!(shown("$1,000 after 0 years at 7%"), "$1,000.00");
    assert_eq!(shown("$1,000 after 3 years at 0%"), "$1,000.00");
    assert!(shown("$1,000 after 6 months at 100%").starts_with("$1,414"));
    assert!(approx("$1,000 after 6 months at 100%"));
    assert_eq!(
        shown("$1,000 after 2 years at 100% compounding annually"),
        "$4,000.00"
    );
    assert!(evaluate("monthly repayment on $1,000 over 0 years at 5%").is_err());
    assert_eq!(
        evaluate("$500 at 5"),
        Err(Error::UnsupportedOperation(
            "a rate such as $30/hour is needed after `at`"
        ))
    );
}

const CORPUS: &str = include_str!("corpus/finance.tsv");

fn digest(n: &babbage_core::Number) -> String {
    use sha2::{Digest, Sha256};
    let text = format!("{}/{}", n.numer(), n.denom());
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn close(a: &babbage_core::Number, b: &babbage_core::Number) -> bool {
    let tolerance = babbage_core::Number::from_decimal_str("1e-60").unwrap();
    let scale = if b.abs() > babbage_core::Number::from_i64(1) {
        b.abs()
    } else {
        babbage_core::Number::from_i64(1)
    };
    a.sub(b).unwrap().abs() <= tolerance.mul(&scale).unwrap()
}

#[test]
fn agrees_with_the_finance_oracle_on_every_line() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in CORPUS.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let (expr, kind, want) = (cols[0], cols[1], cols[2]);
        let got = evaluate(expr);
        let ok = match (&got, kind) {
            (Ok(v), "E") => !v.is_approximate() && digest(v.number()) == want,
            (Ok(v), "A") => {
                v.is_approximate()
                    && close(
                        v.number(),
                        &babbage_core::Number::from_decimal_str(want).unwrap(),
                    )
            }
            _ => false,
        };
        if !ok {
            failures.push(format!(
                "{expr:?} want {kind} {want:.50}, got {:?}",
                got.map(|v| (
                    format!("{}/{}", v.number().numer(), v.number().denom())
                        .chars()
                        .take(60)
                        .collect::<String>(),
                    v.is_approximate()
                ))
            ));
        }
        checked += 1;
    }
    assert!(checked > 7_000, "{checked}");
    assert!(
        failures.is_empty(),
        "{} of {checked} differ, first 10:\n{}",
        failures.len(),
        failures
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
