//! The rules behind the owner's Sheet D answers, tested beyond the lines the owner typed: where each rule stops, and
//! what it must leave alone. The lines themselves are replayed in owner_reference.rs.

use babbage_core::settings::{Settings, scoped};
use babbage_core::zones::Zone;
use babbage_core::{FormatOptions, LineKind, SheetOptions, evaluate_sheet, format_value};

/// 2026-10-26 14:53 in Los Angeles.
const NOW: i64 = 1_793_051_580;
/// 2026-06-15 12:00 UTC.
const SUMMER: i64 = 1_781_524_800;

fn at(now: i64, line: &str, opts: &FormatOptions) -> String {
    let s = Settings {
        local_zone: Zone::named("America/Los_Angeles"),
        ..Settings::at(now)
    };
    let sheet = SheetOptions {
        settings: s.clone(),
        ..SheetOptions::default()
    };
    scoped(&s, || {
        let results = evaluate_sheet(&[line], &sheet);
        let r = &results[0];
        match &r.kind {
            LineKind::Value(v) => format_value(v, &r.display_options(opts)).text,
            LineKind::Error(e) => format!("ERROR {e}"),
            other => format!("{other:?}"),
        }
    })
}

fn run(line: &str) -> String {
    at(NOW, line, &FormatOptions::default())
}

fn all(cases: &[(&str, &str)]) {
    for (line, want) in cases {
        assert_eq!(run(line), *want, "{line}");
    }
}

#[test]
fn british_summer_time_is_london_time_so_it_depends_on_the_season() {
    let utc = |now, line| at(now, line, &FormatOptions::default());
    assert_eq!(utc(NOW, "noon BST in UTC"), "12:00");
    assert_eq!(utc(SUMMER, "noon BST in UTC"), "11:00");
}

#[test]
fn feet_and_inches_print_as_words_and_keep_that_layout_through_a_sum() {
    all(&[
        ("5 feet 6 inches", "5 feet 6 inches"),
        ("5 ft 6 in", "5 feet 6 inches"),
        ("5 feet 6 inches + 2 inches", "5 feet 8 inches"),
        ("5' 6\" + 2 inches", "5 feet 8 inches"),
        ("6\"", "6 inches"),
        ("13\"", "1 foot 1 inch"),
        ("1' 0\"", "1 foot"),
        // A part of a foot reads in inches, but a yard (or a conversion) stays a decimal.
        ("5.5 ft", "5 feet 6 inches"),
        ("5 ft + 3 ft", "8 ft"),
        ("2.5 yd", "2.5 yd"),
        ("1 m in ft", "3.280839895 ft"),
    ]);
}

#[test]
fn a_part_of_a_pound_reads_in_ounces_in_the_app_only() {
    all(&[
        ("13.5 lb", "13 lb 8 oz"),
        ("3 lb 4 oz", "3 lb 4 oz"),
        ("0.5 lb", "8 oz"),
        ("40 lb", "40 lb"),
        ("-1.5 lb", "-1 lb 8 oz"),
        ("2.5 kg", "2.5 kg"),
        ("13.5 lb + 1 lb", "14 lb 8 oz"),
        ("0.2 lb", "3.2 oz"),
        // A conversion prints every digit in the unit asked for.
        ("1 kg in lb", "2.2046226218 lb"),
        ("2.5 lb in oz", "40 oz"),
        ("1 stone 3 lb", "1.2142857143 st"),
    ]);
    let plain = FormatOptions::new(10).unwrap();
    assert_eq!(at(NOW, "13.5 lb", &plain), "13.5 lb");
}

#[test]
fn a_time_of_day_alone_has_no_seconds_but_a_date_with_a_time_keeps_them() {
    all(&[
        ("3:45:30 pm", "3:45 PM"),
        ("noon + 90 seconds", "12:01 PM"),
        ("noon UTC in UTC", "12:00"),
    ]);
    // A date from a timestamp ends with the zone's name.
    assert_eq!(
        run("1733823083000 to date"),
        "December 10, 2024 at 1:31:23 AM PST"
    );
    assert_eq!(
        at(NOW, "1781524800 to date", &FormatOptions::default()),
        "June 15, 2026 at 5:00 AM PDT"
    );
}

#[test]
fn an_hour_typed_h_stays_h_after_a_slash() {
    all(&[
        ("100 km/h", "100 km/h"),
        ("100 km/hour", "100 km/hour"),
        ("100 km/hr", "100 km/hour"),
        ("5 miles/hour", "5 mi/hour"),
        ("5 mph", "5 mph"),
        ("30 mi / 2 hours", "15 mi/hour"),
        ("10 km / 2 hours", "5 km/hour"),
        ("100 km/h × 2 h", "200 km"),
        ("60 mph in km/h", "96.56064 km/h"),
        ("100 km/h in mph", "62.1371192237 mph"),
        ("100 km/h + 20 km/hour", "120 km/h"),
    ]);
}

#[test]
fn a_product_of_units_keeps_every_unit_typed() {
    all(&[
        ("2 N m", "2 N⋅m"),
        ("3 W s", "3 W s"),
        // the reference app prints `3 kg`; dropping the `m` would turn momentum into a flow rate.
        ("3 kg m", "3 kg m"),
        ("3 kg m/s", "3 kg m/s"),
        ("2 kg m²", "2 kg m²"),
        ("3 kg m + 2 kg m", "5 kg m"),
        ("5 feet 3 inches", "5 feet 3 inches"),
    ]);
}

#[test]
fn an_amount_below_a_millionth_is_scientific() {
    all(&[
        ("0.000000001", "1e-9"),
        ("0.00000000004", "4e-11"),
        ("-0.000000002", "-2e-9"),
        ("0.0000000015", "1.5e-9"),
        // the reference app: a millionth is still a decimal, a ten-millionth is not.
        ("0.00001", "0.00001"),
        ("0.000001", "0.000001"),
        ("0.0000001", "1e-7"),
        ("0.00000001", "1e-8"),
        ("0.00000000999", "9.99e-9"),
        ("0", "0"),
        ("1 ng in kg", "1e-12 kg"),
    ]);
}

#[test]
fn the_imperial_gallon_is_exact_and_its_pint_is_the_rounded_one() {
    all(&[
        ("1 imperial gallon in L", "4.54609 L"),
        ("1 liter in imperial gallons", "0.2199692483 gal"),
        ("1 liter in imperial pints", "1.7597547606 pt"),
        ("1 imperial quart in L", "1.13652 L"),
        ("1 imperial fluid ounce in mL", "28.4130625 mL"),
        ("1 gallon in L", "3.785411784 L"),
    ]);
}

#[test]
fn a_year_less_date_is_this_years_wherever_it_falls() {
    all(&[
        ("January 1", "January 1"),
        ("January 12 + 3 weeks", "February 2"),
        ("December 28", "December 28"),
        ("days until January 1", "67 days"),
        ("days since January 1", "298 days"),
    ]);
}

#[test]
fn chinese_new_year_goes_past_2099() {
    all(&[
        ("Chinese New Year 2100", "February 9, 2100"),
        ("Chinese New Year 2025", "January 29, 2025"),
    ]);
}

#[test]
fn a_run_of_digits_joined_by_dots_is_not_a_number_and_is_skipped() {
    all(&[
        ("01.05.2005 + 1 day", "1 day"),
        ("version 1.2.3 costs $5", "$5.00"),
        // One dot is still a decimal point.
        ("1.5 + 1", "2.5"),
        ("1.5.2 + 1", "1"),
    ]);
}

#[test]
fn a_comma_after_a_stray_word_is_part_of_the_remark() {
    all(&[
        ("hotel, 3 nights × $120", "$360.00"),
        ("lunch, dinner, 3 × 4", "12"),
        // Commas that separate things stay.
        ("min(3, 4)", "3"),
        ("average of 10, 20, 30", "20"),
        ("April 1, 2019 + 1 day", "April 2, 2019"),
    ]);
}

#[test]
fn money_per_month_over_a_length_of_time_takes_that_time_as_its_period() {
    all(&[
        ("$500/month / 30 days", "$16.67/day"),
        // Any other rate over a time keeps both units: the rule is only the owner's one form.
        ("$100/day / 2 hours", "50 $/(hour·day)"),
        ("$500/month / 2 weeks", "250 $/(week·month)"),
        ("100 m/s / 2 s", "50 m/s²"),
        ("$500 / 30 days", "$16.67/day"),
    ]);
}

#[test]
fn squared_and_cubed_are_powers_after_anything_that_ends_an_operand() {
    all(&[
        ("5 squared", "25"),
        ("2 cubed", "8"),
        ("(1 + 2) squared", "9"),
        ("5 m squared", "25 m²"),
        ("3 squared + 1", "10"),
        ("2 × 3 squared", "18"),
        ("-3 squared", "-9"),
        // With nothing before it the word is prose.
        ("squared 5", "5"),
    ]);
    assert_eq!(
        sheet(&["side = 4", "side squared"]),
        ["4", "16"],
        "a name that stands for a value can be squared"
    );
}

fn sheet(lines: &[&str]) -> Vec<String> {
    let s = Settings {
        local_zone: Zone::named("America/Los_Angeles"),
        ..Settings::at(NOW)
    };
    let opts = SheetOptions {
        settings: s.clone(),
        ..SheetOptions::default()
    };
    scoped(&s, || {
        evaluate_sheet(lines, &opts)
            .iter()
            .map(|r| match &r.kind {
                LineKind::Value(v) => {
                    format_value(v, &r.display_options(&FormatOptions::default())).text
                }
                other => format!("{other:?}"),
            })
            .collect()
    })
}

#[test]
fn a_skipped_dotted_run_is_shown_as_ignored_not_dropped_silently() {
    let s = Settings {
        local_zone: Zone::named("America/Los_Angeles"),
        ..Settings::at(NOW)
    };
    let opts = SheetOptions {
        settings: s.clone(),
        ..SheetOptions::default()
    };
    scoped(&s, || {
        let r = &evaluate_sheet(&["01.05.2005 + 1 day"], &opts)[0];
        let skipped: Vec<(String, usize)> =
            r.ignored.iter().map(|i| (i.text.clone(), i.at)).collect();
        assert_eq!(skipped, [("01.05.2005".to_string(), 0)]);
        let plain = &evaluate_sheet(&["1.5 + 1"], &opts)[0];
        assert!(plain.ignored.is_empty());
    });
}

#[test]
fn interest_spread_over_a_period_is_the_total_over_the_time() {
    all(&[
        // One year: The reference app agrees to every digit.
        (
            "hourly interest on $1k for 1 year at 4% compounding quarterly",
            "$0.0046320835",
        ),
        // Three years: the total interest over all of it (the reference app spreads it over one year's hours).
        (
            "daily interest on $1k for 3 years at 4% compounding quarterly",
            "$0.12",
        ),
        (
            "monthly interest on $1k for 1 year at 12% compounding monthly",
            "$10.57",
        ),
        ("yearly interest on $1k for 2 years at 10%", "$105.00"),
        // The loan form is another phrase and is unchanged.
        ("daily payment on $1M over 30 years at 6.5%", "$207.67"),
    ]);
}

#[test]
fn an_amount_of_money_too_small_for_cents_keeps_its_digits() {
    all(&[
        ("$0.004", "$0.004"),
        ("$0.005", "$0.01"),
        ("$0.00", "$0.00"),
        ("$1.995", "$2.00"),
    ]);
}

#[test]
fn a_silly_pile_of_unit_powers_is_an_answer_or_an_error_never_a_panic() {
    // Found by the property test: products of units keep every unit, so exponents can pile up.
    for line in [
        "0°²²²°",
        "5 m²²²²²²²²²²²² m²²²²²²²²²²²²",
        "3 kg^100 kg^100 kg^100",
        "2 s⁻¹⁻¹⁻¹⁻¹ s⁻¹⁻¹⁻¹",
    ] {
        let _ = run(line);
    }
}

#[test]
fn money_can_show_its_code_and_hide_trailing_zeros() {
    let with = |f: fn(&mut FormatOptions), line: &str| {
        let mut o = FormatOptions::default();
        f(&mut o);
        at(NOW, line, &o)
    };
    assert_eq!(with(|_| {}, "$5 + $0.5"), "$5.50");
    assert_eq!(with(|o| o.money_trim_zeros = true, "$5"), "$5");
    assert_eq!(with(|o| o.money_trim_zeros = true, "$5 + $0.5"), "$5.5");
    assert_eq!(with(|o| o.money_trim_zeros = true, "$5.25"), "$5.25");
    assert_eq!(with(|o| o.currency_codes = true, "$5"), "5.00 USD");
    assert_eq!(with(|o| o.currency_codes = true, "€3 + €1"), "4.00 EUR");
    assert_eq!(
        with(
            |o| {
                o.currency_codes = true;
                o.money_trim_zeros = true
            },
            "$5"
        ),
        "5 USD"
    );
}
