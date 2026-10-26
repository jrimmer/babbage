//! The proportion, `in what` and pixel-density functions called directly, with the kinds of argument the
//! parser never builds: every refusal and every way the units change the answer.

use babbage_core::functions::{Func, call};
use babbage_core::units::lookup;
use babbage_core::{Error, FormatOptions, Number, Value, format_value};

fn n(v: i64) -> Value {
    Value::plain(Number::from_i64(v))
}

fn q(v: i64, unit: &str) -> Value {
    Value::quantity(Number::from_i64(v), lookup(unit).unwrap()).unwrap()
}

fn pct(v: &str) -> Value {
    Value::percent(Number::from_decimal_str(v).unwrap())
}

fn text(v: Result<Value, Error>) -> String {
    match v {
        Ok(v) => format_value(&v, &FormatOptions::default()).text,
        Err(e) => format!("ERROR {e}"),
    }
}

fn prop(a: Value, b: Value, c: Value) -> String {
    text(call(Func::Proportion, &[a, b, c]))
}

#[test]
fn a_proportion_of_bare_numbers_ignores_nothing_and_keeps_the_kind_of_the_first() {
    assert_eq!(prop(n(2), n(4), n(10)), "5");
    assert_eq!(prop(pct("0.2"), n(500), n(750)), "30%");
    assert_eq!(prop(n(20), pct("0.3"), pct("0.6")), "40");
    assert_eq!(prop(n(20), n(30), n(60)), "40");
    assert!(prop(n(1), n(0), n(5)).starts_with("ERROR"));
}

#[test]
fn a_proportion_with_a_unit_anywhere_is_worked_out_with_the_units() {
    // 2 km x 10 / 4 m = 5,000: reading the numbers alone would give 5
    assert_eq!(prop(q(2, "km"), q(4, "m"), n(10)), "5,000");
    // one quantity among bare numbers keeps its unit
    assert_eq!(prop(q(2, "km"), n(4), n(10)), "5 km");
    assert_eq!(prop(n(2), n(4), q(10, "km")), "5 km");
    assert_eq!(prop(n(2), q(4, "m"), n(10)), "5 m\u{207b}\u{b9}");
}

#[test]
fn a_proportion_of_an_approximate_number_is_approximate() {
    let approx = |v: i64| n(v).approximate();
    for (a, b, c) in [
        (approx(2), n(4), n(10)),
        (n(2), approx(4), n(10)),
        (n(2), n(4), approx(10)),
    ] {
        let v = call(Func::Proportion, &[a, b, c]).unwrap();
        assert!(v.is_approximate());
    }
    let v = call(Func::Proportion, &[n(2), n(4), n(10)]).unwrap();
    assert!(!v.is_approximate());
}

#[test]
fn n_percent_is_b_in_what_needs_two_bare_numbers_that_are_not_zero() {
    let w = |a: Value, b: Value| text(call(Func::InWhat, &[a, b]));
    assert_eq!(w(pct("0.05"), n(1)), "1/20");
    assert_eq!(w(pct("0.4"), n(1)), "1/2.5");
    assert_eq!(w(pct("0.1"), n(3)), "3/30");
    assert_eq!(w(n(5), n(10)), "10/2");
    for (a, b) in [
        (q(2, "km"), n(1)),
        (n(2), q(1, "km")),
        (q(2, "km"), q(1, "km")),
        (Value::date(Number::from_i64(5)), n(1)),
        (n(2), Value::date(Number::from_i64(5))),
    ] {
        assert!(w(a, b).starts_with("ERROR unsupported"));
    }
    assert_eq!(w(n(0), n(1)), "ERROR division by zero");
    assert_eq!(
        w(pct("0.05"), n(0)),
        "ERROR that value is outside what this function accepts"
    );
}

#[test]
fn pixel_density_needs_a_positive_bare_number_and_a_length_or_pixel_count() {
    let d =
        |v: Value, density: Value, to: &str| text(call(Func::AtDensity, &[v, density, q(1, to)]));
    assert_eq!(d(q(1, "inch"), n(96), "px"), "96 px");
    assert_eq!(d(q(96, "px"), n(96), "inch"), "1 in");
    // the density
    for bad in [
        n(0),
        n(-96),
        pct("0.5"),
        q(96, "km"),
        Value::date(Number::from_i64(1)),
    ] {
        assert_eq!(
            d(q(1, "inch"), bad, "px"),
            "ERROR unsupported: pixels per inch must be a number above zero"
        );
    }
    // the value
    for bad in [
        n(5),
        pct("0.5"),
        q(5, "kg"),
        q(5, "s"),
        Value::date(Number::from_i64(1)),
    ] {
        assert_eq!(
            d(bad, n(96), "px"),
            "ERROR unsupported: this needs a length or a number of pixels"
        );
    }
    // the target
    assert_eq!(
        d(q(1, "inch"), n(96), "kg"),
        "ERROR unsupported: the answer must be a length or pixels"
    );
    // pixels on one side and a length on the other
    for (v, to) in [
        (q(1, "cm"), "km"),
        (q(5, "px"), "px"),
        (q(1, "inch"), "inch"),
    ] {
        assert_eq!(
            d(v, n(96), to),
            "ERROR unsupported: this needs pixels on one side and a length on the other"
        );
    }
}

fn phrase(p: babbage_core::parser::Phrase, args: &[Value]) -> String {
    text(Value::phrase(p, args))
}

#[test]
fn rounding_phrases_take_a_plain_count_of_places_or_a_plain_step() {
    use babbage_core::parser::Phrase;
    let one = Value::plain(Number::from_decimal_str("3.14159").unwrap());
    assert_eq!(phrase(Phrase::RoundPlaces, &[one.clone(), n(2)]), "3.14");
    assert_eq!(
        phrase(
            Phrase::RoundNearest,
            &[
                one.clone(),
                Value::plain(Number::from_decimal_str("0.5").unwrap())
            ]
        ),
        "3"
    );
    assert_eq!(phrase(Phrase::RoundUp, &[one.clone(), n(1)]), "4");
    assert_eq!(phrase(Phrase::RoundDown, &[one.clone(), n(1)]), "3");
    // a step or a count that is not a bare number is refused, never read as its number
    for bad in [q(2, "km"), pct("0.5"), Value::date(Number::from_i64(2))] {
        for p in [
            Phrase::RoundPlaces,
            Phrase::RoundNearest,
            Phrase::RoundUp,
            Phrase::RoundDown,
        ] {
            assert!(
                phrase(p, &[one.clone(), bad.clone()]).starts_with("ERROR"),
                "{p:?} {bad:?}"
            );
        }
    }
    // a quantity keeps its unit when it is the one rounded
    assert_eq!(
        phrase(
            Phrase::RoundPlaces,
            &[
                Value::quantity(
                    Number::from_decimal_str("5.555").unwrap(),
                    lookup("km").unwrap()
                )
                .unwrap(),
                n(1)
            ]
        ),
        "5.6 km"
    );
    // places must be a whole number between 0 and 1000
    for bad in [
        Value::plain(Number::from_decimal_str("1.5").unwrap()),
        n(-1),
        n(1001),
    ] {
        assert!(phrase(Phrase::RoundPlaces, &[one.clone(), bad]).starts_with("ERROR"));
    }
    assert!(!phrase(Phrase::RoundPlaces, &[one.clone(), n(1000)]).starts_with("ERROR"));
    // as multiplier
    assert_eq!(phrase(Phrase::AsMultiplier, &[n(4)]), "4x");
    assert!(phrase(Phrase::AsMultiplier, &[q(4, "km")]).starts_with("ERROR"));
    assert!(phrase(Phrase::AsMultiplier, &[pct("0.5")]).starts_with("ERROR"));
}
