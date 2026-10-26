use babbage_core::{Error, FormatOptions, evaluate, format_value};

const CORPUS: &str = include_str!("corpus/money.tsv");

fn shown(line: &str) -> String {
    let v = evaluate(line).unwrap_or_else(|e| panic!("{line}: {e}"));
    format_value(&v, &FormatOptions::default()).text
}

#[test]
fn agrees_with_the_money_oracle_on_every_line() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in CORPUS.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let got = evaluate(cols[0]);
        let ok = got.as_ref().is_ok_and(|v| {
            format!("{}/{}", v.number().numer(), v.number().denom()) == cols[1]
                && format_value(v, &FormatOptions::default()).text == cols[2]
        });
        if !ok {
            let shown_got = got.map(|v| {
                (
                    format!("{}/{}", v.number().numer(), v.number().denom()),
                    format_value(&v, &FormatOptions::default()).text,
                )
            });
            failures.push(format!(
                "{}: want {} {}, got {:?}",
                cols[0], cols[1], cols[2], shown_got
            ));
        }
        checked += 1;
    }
    assert!(checked > 5000, "{checked}");
    assert!(
        failures.is_empty(),
        "{} of {checked} differ, first 10:\n{}",
        failures.len(),
        failures
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_reference_money_examples() {
    assert_eq!(
        shown("$700 + $500 + $35 + $1,150 + $300 + $200 + $100"),
        "$2,985.00"
    );
    assert_eq!(shown("$185 × 4"), "$740.00");
    assert_eq!(shown("$55 × 168"), "$9,240.00");
    assert_eq!(shown("$50 × 8"), "$400.00");
    assert_eq!(shown("$999"), "$999.00");
    assert_eq!(shown("$20 + 30"), "$50.00");
    assert_eq!(shown("$30 × 4 days"), "$120.00");
    assert_eq!(shown("$55 + 25%"), "$68.75");
    assert_eq!(shown("$55 - 10%"), "$49.50");
    assert_eq!(shown("$3k ÷ 5"), "$600.00");
    assert_eq!(shown("$386.8M"), "$386.8M");
    assert_eq!(shown("$999 + $1"), "$1,000.00");
}

#[test]
fn currency_symbols_and_codes_all_work() {
    for (line, want) in [
        ("US$5", "$5.00"),
        ("€5", "€5.00"),
        ("£5", "£5.00"),
        ("¥500", "¥500"),
        ("5 USD", "$5.00"),
        ("5 dollars", "$5.00"),
        ("5 euros", "€5.00"),
        ("5 CHF", "5.00 CHF"),
        ("5 BHD", "5.000 BHD"),
        ("A$5", "A$5.00"),
        ("AU$5", "A$5.00"),
        ("C$5", "C$5.00"),
        ("5 BTC", "5 BTC"),
        ("0.5 BTC + 1 satoshi", "0.50000001 BTC"),
        ("1 BTC in satoshi", "100M SAT"),
        ("$1.005", "$1.01"),
        ("$1.004", "$1.00"),
        ("-$5", "-$5.00"),
        ("$-5".trim_start_matches('$'), "-5"),
        ("$5.999", "$6.00"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
}

#[test]
fn money_suffixes_follow_the_reference_table() {
    for (line, want) in [
        ("$3k", "$3,000.00"),
        ("$4K", "$4,000.00"),
        ("$5m", "$5M"),
        ("€6M", "€6M"),
        ("$7B", "$7G"),
        ("¥8b", "¥8G"),
        ("$9bn", "$9G"),
        ("$10T", "$10T"),
        ("€11t", "€11T"),
        ("£12tn", "£12T"),
        ("$1.5 million", "$1.5M"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
}

#[test]
fn different_currencies_need_a_rate() {
    for line in [
        "$200 + €200",
        "5 USD in EUR",
        "5 EUR to USD",
        "$5 - £3",
        "$5 is what % of €10",
        "1 BTC in USD",
        "5 USD in kg",
    ] {
        let err = evaluate(line).unwrap_err();
        assert!(
            matches!(err, Error::CurrencyRateNeeded | Error::IncompatibleUnits),
            "{line}: {err:?}"
        );
    }
    assert_eq!(evaluate("$200 + €200"), Err(Error::CurrencyRateNeeded));
    assert_eq!(evaluate("5 USD in EUR"), Err(Error::CurrencyRateNeeded));
    assert_eq!(evaluate("5 USD in kg"), Err(Error::IncompatibleUnits));
}

#[test]
fn a_rate_written_by_the_user_converts_exactly() {
    assert_eq!(shown("50 EUR in USD at 1.05 USD/EUR"), "$52.50");
    assert_eq!(shown("50 EUR to USD @ 1.05 USD/EUR"), "$52.50");
    assert_eq!(shown("100 USD in EUR at 0.8 EUR/USD"), "€80.00");
    assert_eq!(
        shown(
            "€1,000 in $ at 1.1 USD/EUR"
                .replace(" in $", " in USD")
                .as_str()
        ),
        "$1,100.00"
    );
    assert!(evaluate("50 EUR in USD at 1.05 GBP/EUR").is_err());
    assert!(evaluate("50 EUR in USD at 1.05").is_err());
}

#[test]
fn money_rates_and_ratios() {
    assert_eq!(shown("$120 / 4 days"), "$30.00/day");
    assert_eq!(shown("$30/day × 4 days"), "$120.00");
    assert_eq!(shown("$30/day × 2 weeks"), "$420.00");
    assert_eq!(shown("$100 / $50"), "2");
    assert_eq!(shown("$50 is what % of $200"), "25%");
    assert_eq!(shown("$200 is 25% of what"), "$800.00");
    assert_eq!(shown("$100 to $150 is what %"), "50%");
}

#[test]
fn large_amounts_abbreviate_and_small_ones_keep_their_cents() {
    assert_eq!(shown("$99,999.99"), "$99,999.99");
    assert_eq!(shown("$100,000"), "$100k");
    assert_eq!(shown("$1,234,567.89"), "$1.23456789M");
    assert_eq!(shown("-$250,000"), "-$250k");
    let mut o = FormatOptions::default();
    o.abbreviate_large = false;
    assert_eq!(
        format_value(&evaluate("$100,000").unwrap(), &o).text,
        "$100,000.00"
    );
    o.fixed_money_digits = false;
    assert_eq!(
        format_value(&evaluate("$1.005").unwrap(), &o).text,
        "$1.005"
    );
}
