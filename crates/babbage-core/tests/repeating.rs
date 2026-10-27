use babbage_core::{FormatOptions, MAX_REPEAT_PERIOD, NumberFormat, evaluate, format_value};

fn repeating(line: &str) -> String {
    let opts = FormatOptions::default().with_repeating(true);
    format_value(
        &evaluate(line).unwrap_or_else(|e| panic!("{line}: {e}")),
        &opts,
    )
    .text
}

fn ordinary(line: &str) -> String {
    format_value(
        &evaluate(line).unwrap_or_else(|e| panic!("{line}: {e}")),
        &FormatOptions::default(),
    )
    .text
}

#[test]
fn a_fraction_that_never_ends_shows_its_repeating_part() {
    for (line, want) in [
        ("1/3", "0.(3)"),
        ("2/3", "0.(6)"),
        ("1/6", "0.1(6)"),
        ("1/12", "0.08(3)"),
        ("1/7", "0.(142857)"),
        ("22/7", "3.(142857)"),
        ("1/11", "0.(09)"),
        ("-1/3", "-0.(3)"),
        ("10000/3", "3,333.(3)"),
    ] {
        assert_eq!(repeating(line), want, "{line}");
    }
}

#[test]
fn a_number_that_ends_is_written_as_usual() {
    for line in ["1/2", "3/8", "5", "0.125", "1/1024", "2.5"] {
        assert_eq!(repeating(line), ordinary(line), "{line}");
    }
}

#[test]
fn a_repeating_part_in_the_ordinary_answer_is_marked_rounded() {
    let v = evaluate("1/3").unwrap();
    assert!(!format_value(&v, &FormatOptions::default()).exact);
    assert!(format_value(&v, &FormatOptions::default().with_repeating(true)).exact);
}

#[test]
fn a_period_longer_than_the_cap_is_written_as_usual() {
    // The period of 1/p is p - 1 digits for a full-reptend prime: 53 digits for 1/59 is over the cap.
    const { assert!(MAX_REPEAT_PERIOD < 58) };
    assert_eq!(repeating("1/59"), ordinary("1/59"));
    // 1/49 has a 42-digit period, which fits.
    let text = repeating("1/49");
    assert!(
        text.starts_with("0.(020408163265306122448979591836734693877551"),
        "{text}"
    );
    assert!(text.ends_with(')'));
}

#[test]
fn percentages_and_quantities_repeat_too() {
    assert_eq!(
        repeating("1/3 as percent"),
        ordinary("1/3 as percent").replace("33.3333333333%", "33.(3)%")
    );
    assert_eq!(repeating("1 m / 3"), "0.(3) m");
}

#[test]
fn money_keeps_the_currency_digits() {
    assert_eq!(repeating("$10 / 3"), "$3.33");
}

#[test]
fn the_decimal_mark_follows_the_number_format() {
    let opts = FormatOptions::default()
        .with_repeating(true)
        .in_format(NumberFormat::WesternEurope);
    assert_eq!(format_value(&evaluate("1/3").unwrap(), &opts).text, "0,(3)");
}
