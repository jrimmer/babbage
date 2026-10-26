use babbage_core::{Error, FormatOptions, evaluate, format_value};

fn shown(line: &str) -> String {
    let v = evaluate(line).unwrap_or_else(|e| panic!("{line}: {e}"));
    format_value(&v, &FormatOptions::default()).text
}

fn word(w: &str, at: usize) -> Error {
    Error::UnexpectedWord { word: w.into(), at }
}

#[test]
fn a_currency_symbol_needs_a_number_right_after_it() {
    assert_eq!(shown("$5"), "$5.00");
    assert_eq!(shown("$.5"), "$0.50");
    assert_eq!(shown("$0.5"), "$0.50");
    assert_eq!(
        evaluate("$ 5"),
        Err(Error::UnexpectedChar { ch: '$', at: 0 })
    );
    assert_eq!(evaluate("$"), Err(Error::UnexpectedChar { ch: '$', at: 0 }));
    assert_eq!(
        evaluate("$."),
        Err(Error::UnexpectedChar { ch: '$', at: 0 })
    );
    assert_eq!(
        evaluate("$x"),
        Err(Error::UnexpectedChar { ch: '$', at: 0 })
    );
    assert_eq!(
        evaluate("5$"),
        Err(Error::UnexpectedChar { ch: '$', at: 1 })
    );
    assert_eq!(shown("-$5"), "-$5.00");
    assert_eq!(shown("($5)"), "$5.00");
    assert_eq!(shown("2 × $5"), "$10.00");
}

#[test]
fn a_symbol_made_of_letters_cannot_start_inside_a_word() {
    assert_eq!(shown("A$5"), "A$5.00");
    assert_eq!(shown("(A$5)"), "A$5.00");
    assert_eq!(shown("1 + A$5"), "A$6.00");
    assert_eq!(evaluate("2A$5"), Err(Error::IncompatibleUnits));
    assert!(evaluate("xA$5").is_err());
    assert_eq!(shown("NZ$5"), "NZ$5.00");
    assert_eq!(shown("HK$5"), "HK$5.00");
    assert_eq!(shown("S$5"), "S$5.00");
    assert_eq!(shown("R$5"), "R$5.00");
    assert_eq!(shown("NT$5"), "NT$5.00");
}

#[test]
fn plain_number_suffixes() {
    assert_eq!(shown("5k"), "5,000");
    assert_eq!(shown("5M"), "5M");
    assert_eq!(shown("5G"), "5G");
    assert_eq!(shown("5T"), "5T");
    assert_eq!(shown("5K"), "5 K");
    assert_eq!(shown("5m"), "5 m");
    assert_eq!(shown("5g"), "5 g");
    assert_eq!(shown("5t"), "5 t");
    assert_eq!(evaluate("5b"), Err(word("b", 1)));
    assert_eq!(evaluate("5kx"), Err(word("kx", 1)));
    assert_eq!(evaluate("5Mx"), Err(word("Mx", 1)));
    assert_eq!(shown("5 million"), "5M");
    assert_eq!(shown("5Million"), "5M");
    assert_eq!(shown("5 BILLION"), "5G");
    assert_eq!(shown("2 trillion"), "2T");
    assert_eq!(shown("2 thousand"), "2,000");
}

#[test]
fn money_suffixes() {
    for (line, want) in [
        ("$5k", "$5,000.00"),
        ("$5K", "$5,000.00"),
        ("$5m", "$5M"),
        ("$5M", "$5M"),
        ("$5b", "$5G"),
        ("$5B", "$5G"),
        ("$5bn", "$5G"),
        ("$5t", "$5T"),
        ("$5T", "$5T"),
        ("$5tn", "$5T"),
        ("$1.5k", "$1,500.00"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
    assert_eq!(evaluate("$5G"), Err(word("G", 2)));
    assert_eq!(evaluate("$5bnx"), Err(word("bnx", 2)));
    assert_eq!(evaluate("$5tnx"), Err(word("tnx", 2)));
    assert_eq!(evaluate("$5kx"), Err(word("kx", 2)));
    assert_eq!(evaluate("$5bx"), Err(word("bx", 2)));
}

#[test]
fn conversion_words_need_something_on_each_side() {
    for line in [
        "to km",
        "5 km to",
        "5 km in",
        "in USD",
        "5 EUR in USD at",
        "5 EUR in at 1.05 USD/EUR",
        "in USD at 1.05 USD/EUR",
        "5 EUR in USD at at",
    ] {
        assert!(evaluate(line).is_err(), "{line}");
    }
    assert_eq!(shown("50 EUR in USD at 1.05 USD/EUR"), "$52.50");
    assert_eq!(shown("50 EUR to USD at 1.05 USD/EUR"), "$52.50");
    assert_eq!(shown("50 EUR as USD @ 1.05 USD/EUR"), "$52.50");
    assert_eq!(shown("50 EUR into USD at 1.05 USD/EUR"), "$52.50");
}

#[test]
fn abbreviation_climbs_through_every_step() {
    assert_eq!(shown("10^5"), "100k");
    assert_eq!(shown("10^6"), "1M");
    assert_eq!(shown("10^9"), "1G");
    assert_eq!(shown("10^12"), "1T");
    assert_eq!(shown("10^15"), "1,000T");
    assert_eq!(shown("10^18"), "1,000,000T");
    assert_eq!(shown("999999999999999.9999999999"), "1,000T");
    assert_eq!(shown("999999999.9999999999"), "1G");
    assert_eq!(shown("999999.9999999999"), "1M");
    assert_eq!(shown("-10^9"), "-1G");
}

#[test]
fn display_options_helpers() {
    let default = FormatOptions::default();
    assert!(default.abbreviate_large);
    assert!(!default.unabbreviated().abbreviate_large);
    assert!(!default.with_places(2).unwrap().abbreviate_large);
    assert_eq!(default.with_places(2).unwrap().decimal_places(), 2);
    assert!(default.with_places(1001).is_err());
}

#[test]
fn equal_factors_keep_the_left_unit_and_larger_units_win() {
    assert_eq!(shown("1 mcg + 1 µg"), "2 mcg");
    assert_eq!(shown("1 µg + 1 mcg"), "2 µg");
    assert_eq!(shown("1 g + 1 mg"), "1.001 g");
    assert_eq!(shown("1 mg + 1 g"), "1.001 g");
    assert_eq!(shown("2 g - 1 mg"), "1.999 g");
}

#[test]
fn a_dimensionless_compound_unit_becomes_a_plain_number() {
    assert_eq!(shown("5 N / (1 kg × 1 m / 1 s^2)"), "5");
    assert_eq!(shown("6 kg × 1 m / 1 s^2 / 3 N"), "2");
    assert_eq!(shown("1 km / 250 m"), "4");
    assert_eq!(shown("1 kb / 1 B"), "125");
    assert_eq!(shown("1 h / 1 min"), "60");
    assert_eq!(shown("1 wk / 1 day"), "7");
}

#[test]
fn money_rates_show_their_denominator() {
    assert_eq!(shown("$120 / 4 days"), "$30.00/day");
    assert_eq!(shown("€120 / 4 days"), "€30.00/day");
    assert_eq!(shown("120 CHF / 4 days"), "30.00 CHF/day");
    assert!(shown("$120 / (4 days × 2 kg)").contains("kg"));
}
