//! A power whose exact value would not fit the number size limit falls back to about 115 digits and is marked
//! approximate, never silently. The reference values are Python `Fraction` powers (exact), printed to 58 digits.

use babbage_core::{Number, Value, evaluate};

/// |a - b| <= 1e-50 * max(1, |b|), in exact rational arithmetic.
fn close(a: &Number, reference: &str) -> bool {
    let b = Number::from_decimal_str(reference).unwrap();
    let tolerance = Number::from_decimal_str("1e-50").unwrap();
    let scale = if b.abs() > Number::from_i64(1) {
        b.abs()
    } else {
        Number::from_i64(1)
    };
    a.sub(&b).unwrap().abs() <= tolerance.mul(&scale).unwrap()
}

/// Marked approximate by the engine (as against rounded for display, which also shows `≈`).
fn marked_approximate(v: &Value) -> bool {
    v.is_approximate()
}

#[test]
fn a_hundred_years_of_daily_compounding_is_approximate_and_right() {
    let v = evaluate("(1+0.03/365)^36500").expect("an approximate answer");
    assert!(marked_approximate(&v));
    assert!(close(
        v.number(),
        "20.0830609124197764914056975630459394161439888096702857735137"
    ));
}

#[test]
fn a_base_close_to_one_raised_to_a_big_power() {
    let v = evaluate("1.0000001^100000").expect("an approximate answer");
    assert!(marked_approximate(&v));
    assert!(close(
        v.number(),
        "1.01005016657914300779467405984247419491294661115678912148073"
    ));
}

#[test]
fn the_sign_of_a_negative_base_follows_the_exponent() {
    let odd = evaluate("(-1.0001)^100001").expect("an approximate answer");
    assert!(close(
        odd.number(),
        "-22017.6575941570538655660267273168813921497015912981232616414"
    ));
    let even = evaluate("(-1.0001)^100000").expect("an approximate answer");
    assert!(even.number().is_positive());
}

#[test]
fn what_fits_stays_exact() {
    let v = evaluate("(1+0.05/365)^10950").expect("exact");
    assert!(!marked_approximate(&v));
    let v = evaluate("2^10").expect("exact");
    assert!(!marked_approximate(&v));
}

#[test]
fn a_power_beyond_the_approximation_is_still_refused() {
    assert!(evaluate("1.0000001^1000000000").is_err());
    assert!(evaluate("10^1000000").is_err());
}

#[test]
fn the_exponent_limit_is_four_hundred_thousand() {
    assert!(evaluate("1.0000001^400000").is_ok());
    assert!(evaluate("1.0000001^400001").is_err());
    assert!(evaluate("1.0000001^(0-400000)").is_ok());
    assert!(evaluate("1.0000001^(0-400001)").is_err());
}
