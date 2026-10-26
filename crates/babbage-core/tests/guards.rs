use babbage_core::{Error, FormatOptions, evaluate, format_value};

fn shown(line: &str) -> String {
    let v = evaluate(line).unwrap_or_else(|e| panic!("{line:?}: {e}"));
    format_value(&v, &FormatOptions::default()).text
}

fn refused(line: &str) -> Error {
    evaluate(line).expect_err(line)
}

#[test]
fn exponent_suffixes_in_every_spelling() {
    assert_eq!(shown("2 m⁹"), "2 m⁹");
    assert_eq!(shown("2 m¹⁰"), "2 m¹⁰");
    assert_eq!(shown("2 m¹²"), "2 m¹²");
    assert_eq!(shown("2 s⁻¹"), "2 s⁻¹");
    assert_eq!(shown("2 s⁻²"), "2 s⁻²");
    assert_eq!(shown("2 s⁻¹²"), "2 s⁻¹²");
    assert_eq!(shown("2 m^-2"), "2 m⁻²");
    assert_eq!(shown("2 m^12"), "2 m¹²");
    assert_eq!(shown("2 m12"), "2 m¹²");
    assert_eq!(shown("2 m⁰"), "2");
    assert_eq!(shown("2 m^0"), "2");
    assert_eq!(shown("2 m^1"), "2 m");
    assert_eq!(shown("3 m^2 + 1 m²"), "4 m²");
}

#[test]
fn a_caret_without_digits_is_not_an_exponent() {
    assert!(evaluate("2 m^").is_err());
    assert!(evaluate("2 m^-").is_err());
    assert!(evaluate("2 m⁻").is_err());
}

#[test]
fn two_word_unit_names_join_on_a_space_or_a_hyphen() {
    assert_eq!(shown("2 fl oz"), "2 floz");
    assert_eq!(shown("2 acre foot"), "2 af");
    assert_eq!(shown("2 acre-foot"), "2 af");
    assert_eq!(shown("2 oz t"), "2 ozt");
    assert_eq!(shown("2 light years"), "2 ly");
    assert_eq!(shown("2 nautical miles"), "2 NM");
    assert_eq!(shown("2 pound-force"), "2 lbf");
    assert!(evaluate("2 fl").is_err());
    assert!(evaluate("2 fl 3").is_err());
}

#[test]
fn degree_signs_take_the_letters_after_them() {
    assert_eq!(shown("5 °C"), "5 °C");
    assert_eq!(shown("5 ºF"), "5 °F");
    assert_eq!(shown("5°"), "5 deg");
    assert_eq!(shown("5 degrees"), "5 deg");
    assert_eq!(shown("30′"), "30 arcmin");
    assert_eq!(shown("30″"), "30 arcsec");
    assert_eq!(shown("2°²"), "2 deg²");
    assert!(evaluate("5 °X").is_err());
}

#[test]
fn conversion_words_need_a_value_before_and_a_unit_after() {
    for line in [
        "to m",
        "5 km to",
        "5 cast",
        "cast km",
        "in m",
        "5 km to 3",
        "5 km to m m",
    ] {
        assert!(evaluate(line).is_err(), "{line}");
    }
    assert_eq!(shown("km to m"), "1,000 m");
    assert_eq!(shown("5 of km"), "5 km");
    assert_eq!(shown("5 on km"), "6 km");
    assert_eq!(shown("5 off km"), "-4 km");
    for word in ["is", "what", "a", "percent"] {
        assert!(evaluate(&format!("5 {word} km")).is_err(), "5 {word} km");
    }
}

#[test]
fn unit_operations_that_make_no_sense_are_refused() {
    assert!(matches!(
        refused("10% + 5 km"),
        Error::UnsupportedOperation(_)
    ));
    assert!(matches!(
        refused("5 km on 10"),
        Error::UnsupportedOperation(_)
    ));
    assert!(matches!(
        refused("5 km off 10"),
        Error::UnsupportedOperation(_)
    ));
    assert!(matches!(refused("5% km"), Error::UnsupportedOperation(_)));
    assert!(matches!(
        refused("(5 km) m"),
        Error::UnsupportedOperation(_)
    ));
    assert!(matches!(refused("5 km km"), Error::UnsupportedOperation(_)));
    assert!(matches!(refused("2 ^ 3 m"), Error::UnsupportedOperation(_)));
    assert_eq!(shown("10% on 5 km"), "5.5 km");
    assert_eq!(shown("0.1 on 100 km"), "110 km");
    assert_eq!(evaluate("5 km").unwrap().unit().symbol(), "km");
    assert_eq!(evaluate("5").unwrap().unit().symbol(), "");
}

#[test]
fn temperatures_refuse_compound_scales() {
    assert!(matches!(
        refused("20 °C in °F·min/s"),
        Error::UnsupportedOperation(_)
    ));
    assert_eq!(shown("20 °C in °F"), "68 °F");
}

#[test]
fn implicit_sums_need_a_unit_after_the_second_number() {
    assert_eq!(shown("1 m 50 cm"), "1.5 m");
    assert!(evaluate("1 m 50").is_err());
    assert!(evaluate("1 m 50 cm 3").is_err());
    assert_eq!(shown("1 m 50 cm 25 mm"), "1.525 m");
}
