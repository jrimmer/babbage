use babbage_core::settings::{Settings, scoped};
use babbage_core::{Error, FormatOptions, evaluate_with, format_value};

fn run(settings: &Settings, line: &str) -> Result<String, Error> {
    scoped(settings, || {
        let v = evaluate_with(line, settings)?;
        Ok(format_value(&v, &FormatOptions::default().unabbreviated()).text)
    })
}

fn dollar(code: &'static str) -> Settings {
    Settings {
        dollar: Some(code),
        ..Settings::default()
    }
}

#[test]
fn a_bare_dollar_is_us_dollars_unless_the_region_says_otherwise() {
    let us = Settings::default();
    assert_eq!(run(&us, "$5 + $5").as_deref(), Ok("$10.00"));
    assert_eq!(run(&us, "A$5 + A$5").as_deref(), Ok("A$10.00"));
    assert_eq!(run(&us, "US$5 + $5").as_deref(), Ok("$10.00"));
    assert_eq!(run(&us, "$5 + A$5"), Err(Error::CurrencyRateNeeded));
}

#[test]
fn in_australia_a_dollar_is_australian_and_us_dollars_are_marked() {
    let au = dollar("AUD");
    assert_eq!(run(&au, "$5 + $5").as_deref(), Ok("$10.00"));
    assert_eq!(run(&au, "$5 + A$5").as_deref(), Ok("$10.00"));
    assert_eq!(run(&au, "US$5 + US$5").as_deref(), Ok("US$10.00"));
    assert_eq!(run(&au, "$5 + US$5"), Err(Error::CurrencyRateNeeded));
    assert_eq!(
        run(&au, "$5 in USD at 0.5 USD/AUD").as_deref(),
        Ok("US$2.50")
    );
    assert_eq!(run(&au, "$5 + 5 AUD").as_deref(), Ok("$10.00"));
}

#[test]
fn every_region_dollar_works_and_a_non_dollar_is_ignored() {
    for (code, mark) in [("CAD", "$"), ("NZD", "$"), ("SGD", "$"), ("HKD", "$")] {
        assert_eq!(
            run(&dollar(code), "$2 + $3").as_deref(),
            Ok(format!("{mark}5.00").as_str()),
            "{code}"
        );
        assert_eq!(
            run(&dollar(code), "US$2 + US$3").as_deref(),
            Ok("US$5.00"),
            "{code}"
        );
    }
    assert_eq!(run(&dollar("EUR"), "$2 + $3").as_deref(), Ok("$5.00"));
}

#[test]
fn work_hours_set_the_workday() {
    let seven = Settings {
        work_hours_per_day: Some(7),
        ..Settings::default()
    };
    assert_eq!(run(&seven, "21 h in workdays").as_deref(), Ok("3 workdays"));
    assert_eq!(
        run(&Settings::default(), "24 h in workdays").as_deref(),
        Ok("3 workdays")
    );
    assert_eq!(
        run(
            &seven,
            "work hours between March 4, 2024 and March 11, 2024"
        )
        .as_deref(),
        Ok("35 hours")
    );
}
