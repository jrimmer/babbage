use babbage_core::{Error, FormatOptions, Number, evaluate, format_value};

const CORPUS: &str = include_str!("corpus/functions.tsv");

fn shown(line: &str) -> String {
    let v = evaluate(line).unwrap_or_else(|e| panic!("{line}: {e}"));
    format_value(&v, &FormatOptions::default()).text
}

#[test]
fn agrees_with_the_function_oracle_on_every_line() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in CORPUS.lines() {
        let (expr, exact) = line.split_once('\t').expect("tab-separated");
        let got = evaluate(expr).map(|v| format!("{}/{}", v.number().numer(), v.number().denom()));
        if got.as_deref() != Ok(exact) {
            failures.push(format!("{expr:?}: want {exact}, got {got:?}"));
        }
        checked += 1;
    }
    assert!(checked > 100_000, "{checked}");
    assert!(
        failures.is_empty(),
        "{} of {checked} differ, first 15:\n{}",
        failures.len(),
        failures
            .iter()
            .take(15)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_example_lines() {
    for (line, want) in [
        ("30 plus 20", "50"),
        ("3,000 minus 12", "2,988"),
        ("3 multiplied by 4", "12"),
        ("1,000 divided by 200", "5"),
        ("3 to the power of 2", "9"),
        ("remainder of 21 divided by 5", "1"),
        ("21 mod 5", "1"),
        ("1/3 to 2 dp", "0.33"),
        ("5.5 rounded", "6"),
        ("5.5 rounded down", "5"),
        ("5.5 rounded up", "6"),
        ("37 to nearest 10", "40"),
        ("$490 rounded to nearest hundred", "$500.00"),
        ("2,100 to nearest thousand", "2,000"),
        ("21 rounded up to nearest 5", "25"),
        ("17 rounded down to nearest 3", "15"),
        ("0.534 to nearest 16th", "0.5625"),
        ("total of 3, 4, 7 and 9", "23"),
        ("average of 36, 42, 19 and 81", "44.5"),
        ("count of 1, 2, 3, 4, 5", "5"),
        ("median of 10, 20 and 30", "20"),
        ("6 is to 60 as 8 is to what", "80"),
        ("5 is to 10 as what is to 80", "40"),
        ("larger of 100 and 200", "200"),
        ("greater of 100 and 200", "200"),
        ("smaller of 5 and 10", "5"),
        ("lesser of 5 and 10", "5"),
        ("half of 175", "87.5"),
        ("midpoint between 150 and 300", "225"),
        ("gcd of 20 and 30", "10"),
        ("lcm of 5 and 8", "40"),
        ("10 permutation 3", "720"),
        ("25 combination 3", "2,300"),
        ("3 permutations of 10", "720"),
        ("3 combinations of 25", "2,300"),
        ("clamp 26 between 5 and 25", "25"),
        ("clamp 4 from 5 to 25", "5"),
        ("$100 as number", "100"),
        ("40/day to decimal", "40"),
        ("20% as dec", "0.2"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
}

#[test]
fn functions_keep_units_and_money() {
    assert_eq!(shown("max(3 m, 250 cm, 1 km)"), "1 km");
    assert_eq!(shown("min(3 m, 250 cm)"), "250 cm");
    assert_eq!(shown("average of 10 m, 20 m and 60 m"), "30 m");
    assert_eq!(shown("sum($10, $20.5)"), "$30.50");
    assert_eq!(shown("median of $1, $5, $3"), "$3.00");
    assert_eq!(shown("round($1.5)"), "$2.00");
    assert_eq!(shown("floor(-2.5 m)"), "-3 m");
    assert_eq!(shown("clamp 5 km between 1 m and 2 km"), "2 km");
    assert_eq!(shown("half of 5 km"), "2.5 km");
    assert_eq!(shown("midpoint between 1 km and 500 m"), "0.75 km");
    assert_eq!(shown("5 km as number"), "5");
    assert_eq!(shown("5 km is to 500 m as 5 cm is to what"), "0.005 m");
}

#[test]
fn rounding_ties_go_away_from_zero() {
    for (line, want) in [
        ("round(2.5)", "3"),
        ("round(-2.5)", "-3"),
        ("round(0.5)", "1"),
        ("round(-0.5)", "-1"),
        ("round(1.4999)", "1"),
        ("-5.5 rounded", "-6"),
        ("2.5 to nearest 5", "5"),
        ("-2.5 to nearest 5", "-5"),
        ("1.005 to 2 dp", "1.01"),
        ("-1.005 to 2 dp", "-1.01"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
}

#[test]
fn statistics_of_one_value_and_of_even_counts() {
    assert_eq!(shown("median(7)"), "7");
    assert_eq!(shown("median(1, 2, 3, 4)"), "2.5");
    assert_eq!(shown("average(5)"), "5");
    assert_eq!(shown("count(5)"), "1");
    assert_eq!(shown("max(5)"), "5");
}

#[test]
fn modulo_semantics() {
    assert_eq!(shown("-7 mod 3"), "-1");
    assert_eq!(shown("7 mod -3"), "1");
    assert_eq!(shown("5.5 mod 2"), "1.5");
    assert_eq!(shown("10 m mod 3 m"), "1 m");
    assert_eq!(shown("1 km mod 300 m"), "0.1 km");
    assert_eq!(evaluate("5 mod 0"), Err(Error::DivisionByZero));
    assert!(evaluate("5 m mod 2 kg").is_err());
}

#[test]
fn domain_and_argument_errors() {
    assert_eq!(evaluate("fact(-1)"), Err(Error::OutOfDomain));
    assert!(evaluate("fact(2.5)").is_err());
    assert_eq!(evaluate("5 permutation 6"), Err(Error::OutOfDomain));
    assert!(evaluate("gcd(2.5, 5)").is_err());
    assert!(evaluate("clamp(5, 10, 1)").is_err());
    assert!(evaluate("max()").is_err());
    assert!(evaluate("abs(1, 2)").is_err());
    assert!(evaluate("average of 3 m, 2 kg").is_err());
    assert!(evaluate("sum(1,)").is_err());
    assert!(evaluate("(1, 2)").is_err());
    assert!(evaluate("max 5 3").is_err());
}

#[test]
fn a_large_factorial_is_exact_and_a_huge_one_is_refused() {
    assert_eq!(
        evaluate("fact(30)").unwrap().number(),
        &Number::from_decimal_str("265252859812191058636308480000000").unwrap()
    );
    assert_eq!(evaluate("fact(100000)"), Err(Error::Overflow));
}

#[test]
fn prefix_functions_bind_to_the_next_operand_only() {
    assert_eq!(shown("round 2.6 + 1"), "4");
    assert_eq!(shown("abs -5 + 1"), "6");
    assert_eq!(shown("-abs 5"), "-5");
    assert_eq!(shown("abs(-5) × 2"), "10");
}

#[test]
fn random_numbers_stay_inside_their_range() {
    babbage_core::functions::seed_random(42);
    for _ in 0..200 {
        let v = evaluate("random number between 1 and 10").unwrap();
        let n = v.number();
        assert!(n.is_integer() && *n >= Number::from_i64(1) && *n <= Number::from_i64(10));
        let f = evaluate("random number between 0 and 1").unwrap();
        assert!(*f.number() >= Number::from_i64(0) && *f.number() <= Number::from_i64(1));
        assert!(f.number().terminating_places().is_some_and(|p| p <= 10));
    }
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..500 {
        seen.insert(
            evaluate("random number between 1 and 3")
                .unwrap()
                .number()
                .clone(),
        );
    }
    assert_eq!(seen.len(), 3);
}
