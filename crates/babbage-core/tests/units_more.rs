use babbage_core::units::registry;
use babbage_core::{Error, FormatOptions, Number, evaluate, format_value};
use proptest::prelude::*;

fn shown(line: &str) -> String {
    let v = evaluate(line).unwrap_or_else(|e| panic!("{line:?}: {e}"));
    format_value(&v, &FormatOptions::default()).text
}

#[test]
fn several_quantities_in_a_row_are_a_sum() {
    // Several time units written in a row print as a timespan, as the reference app does.
    assert_eq!(shown("5 hours 30 minutes"), "5 hours 30 min");
    assert_eq!(shown("5 hours 30 minutes to seconds"), "19,800 s");
    assert_eq!(shown("1 m 50 cm"), "1.5 m");
    assert_eq!(shown("1 h 30 min 30 s"), "1 hour 30 min 30 s");
    assert_eq!(shown("2 ft 3 inches in inches"), "27 in");
    assert_eq!(shown("1 lb 8 oz in oz"), "24 oz");
    assert_eq!(
        shown("2 × 1 h 30 min"),
        "2 × 1 h 30 min".replace("2 × 1 h 30 min", "3 h")
    );
}

#[test]
fn a_number_after_a_unit_without_a_unit_after_it_is_not_a_sum() {
    assert!(evaluate("5 km 30").is_err());
    assert!(evaluate("5 km 30 foo").is_err());
}

#[test]
fn cast_relabels_without_converting() {
    assert_eq!(shown("5 km cast miles"), "5 mi");
    assert_eq!(shown("5 km cast kg"), "5 kg");
    assert!(evaluate("5 cast km").is_err());
}

#[test]
fn temperature_rules() {
    assert_eq!(shown("20 °C + 5"), "25 °C");
    assert_eq!(shown("5 + 20 °C"), "25 °C");
    assert_eq!(shown("20 °C - 5"), "15 °C");
    assert_eq!(shown("-5 °C"), "-5 °C");
    assert_eq!(shown("100 celsius in fahrenheit"), "212 °F");
    assert_eq!(shown("0 K in C"), "-273.15 °C");
    for refused in [
        "20 °C + 5 °C",
        "20 °C × 2",
        "2 × 20 °F",
        "20 °C / 2",
        "20 °C + 10%",
        "20 °C is what % of 40 °C",
        "(20 °C)^2",
        "20 °C in kg",
    ] {
        assert!(
            evaluate(refused).is_err(),
            "{refused}: {:?}",
            evaluate(refused)
        );
    }
    assert!(matches!(
        evaluate("20 °C × 2"),
        Err(Error::UnsupportedOperation(_))
    ));
    assert_eq!(evaluate("20 °C in kg"), Err(Error::IncompatibleUnits));
}

#[test]
fn plain_letters_c_and_f_are_temperatures_only_after_a_number() {
    assert_eq!(shown("100 C in F"), "212 °F");
    assert!(evaluate("C").is_ok());
}

fn linear_units() -> Vec<(String, babbage_core::units::UnitExpr)> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for (name, unit) in registry().names() {
        if unit.is_single()
            && !unit.has_offset()
            && !unit.dim().has_money()
            && seen.insert(unit.clone())
        {
            out.push((name.clone(), unit.clone()));
        }
    }
    out.sort();
    out
}

fn same_kind_pair() -> impl Strategy<Value = (String, String)> {
    let units = linear_units();
    (0..units.len(), 0..units.len()).prop_filter_map("same kind", move |(i, j)| {
        (units[i].1.dim() == units[j].1.dim()).then(|| (units[i].0.clone(), units[j].0.clone()))
    })
}

fn amount() -> impl Strategy<Value = String> {
    (1u32..=100_000, 0u32..=9999).prop_map(|(i, f)| format!("{i}.{f:04}"))
}

fn exact(line: &str) -> Number {
    let v = evaluate(line).unwrap_or_else(|e| panic!("{line}: {e}"));
    v.number().clone()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 2000, ..ProptestConfig::default() })]

    #[test]
    fn converting_there_and_back_is_exact((a, b) in same_kind_pair(), x in amount()) {
        let there = evaluate(&format!("{x} {a} in {b}")).unwrap();
        let back = evaluate(&format!("({}) {b} in {a}", format_exact(there.number()))).unwrap();
        prop_assert_eq!(back.number().clone(), exact(&x));
    }

    #[test]
    fn addition_of_quantities_commutes_in_value_and_unit((a, b) in same_kind_pair(), x in amount(), y in amount()) {
        let l = evaluate(&format!("{x} {a} + {y} {b}")).unwrap();
        let r = evaluate(&format!("{y} {b} + {x} {a}")).unwrap();
        let si = |v: &babbage_core::Value| v.number().mul(&v.unit().factor().unwrap()).unwrap();
        prop_assert_eq!(si(&l), si(&r));
        prop_assert_eq!(l.unit().dim(), r.unit().dim());
    }

    #[test]
    fn adding_then_subtracting_returns_the_start((a, b) in same_kind_pair(), x in amount(), y in amount()) {
        let back = evaluate(&format!("(({x} {a} + {y} {b}) - {y} {b}) in {a}")).unwrap();
        prop_assert_eq!(back.number().clone(), exact(&x));
    }

    #[test]
    fn scaling_by_a_plain_number_and_back_is_exact((a, _b) in same_kind_pair(), x in amount(), k in amount()) {
        let q = evaluate(&format!("({x} {a} × {k}) / {k}")).unwrap();
        prop_assert_eq!(q, evaluate(&format!("{x} {a}")).unwrap());
    }

    #[test]
    fn a_quantity_divided_by_itself_is_one((a, b) in same_kind_pair(), x in amount()) {
        prop_assert_eq!(evaluate(&format!("{x} {a} / {x} {a}")).unwrap().into_plain().unwrap(), Number::from_i64(1));
        prop_assert_eq!(evaluate(&format!("{x} {a} / {x} {a}")).unwrap(), evaluate(&format!("{x} {b} / {x} {b}")).unwrap());
    }

    #[test]
    fn unit_words_and_text_never_panic(s in "[0-9. a-zA-Zµ°²³^/*·-]{0,50}") {
        let _ = evaluate(&s);
    }
}

fn format_exact(n: &Number) -> String {
    format!("{}/{}", n.numer(), n.denom())
}
