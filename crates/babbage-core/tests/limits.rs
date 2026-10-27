use babbage_core::{Error, evaluate_plain as evaluate};

#[test]
fn daily_compounding_for_thirty_years_is_exact() {
    let n = evaluate("1000 * (1 + 0.05/365)^10950").expect("10,950 periods must be representable");
    assert!(n.numer().bits() < 1 << 20);
}

#[test]
fn zero_to_any_positive_power_is_zero_even_for_huge_exponents() {
    assert_eq!(
        evaluate("0^1000000000"),
        Ok(babbage_core::Number::from_i64(0))
    );
}

#[test]
fn one_and_minus_one_to_huge_powers_follow_parity() {
    assert_eq!(
        evaluate("1^99999999999999999999"),
        Ok(babbage_core::Number::from_i64(1))
    );
    assert_eq!(
        evaluate("(-1)^99999999999999999999"),
        Ok(babbage_core::Number::from_i64(-1))
    );
    assert_eq!(
        evaluate("(-1)^99999999999999999998"),
        Ok(babbage_core::Number::from_i64(1))
    );
}

#[test]
fn results_too_big_to_write_down_are_refused() {
    assert_eq!(evaluate("10^1000000"), Err(Error::Overflow));
    // A power whose exact value does not fit but whose size is modest is approximated (`large_powers.rs`); an
    // exponent past the approximation's range is still refused.
    assert_eq!(evaluate("1.0000001^1000000000"), Err(Error::Overflow));
}
