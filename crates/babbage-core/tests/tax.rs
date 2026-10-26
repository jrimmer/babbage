use babbage_core::settings::{Settings, Tax};
use babbage_core::{Error, FormatOptions, Number, evaluate, evaluate_with, format_value};

fn vat() -> Settings {
    Settings {
        tax: Some(Tax {
            name: "VAT".into(),
            rate: Number::from_decimal_str("0.15").unwrap(),
        }),
    }
}

fn shown(line: &str) -> String {
    let v = evaluate_with(line, &vat()).unwrap_or_else(|e| panic!("{line}: {e}"));
    format_value(&v, &FormatOptions::default()).text
}

#[test]
fn the_reference_tax_examples_at_fifteen_percent() {
    for (line, want) in [
        ("$300 + VAT", "$345.00"),
        ("VAT on $300", "$45.00"),
        ("$300 - VAT", "$260.87"),
        ("VAT off $300", "$260.87"),
        ("VAT in $300", "$39.13"),
        ("VAT of $300", "$39.13"),
        ("VAT from $300", "$39.13"),
        ("vat on $300", "$45.00"),
        ("VAT", "15%"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
}

#[test]
fn removing_tax_divides_it_is_not_a_percentage_off() {
    assert_eq!(shown("$115 - VAT"), "$100.00");
    assert_eq!(shown("($115 - VAT) + VAT"), "$115.00");
    assert_eq!(shown("$300 - 15%"), "$255.00");
}

#[test]
fn tax_works_on_plain_numbers_and_quantities() {
    assert_eq!(shown("100 + VAT"), "115");
    assert_eq!(shown("VAT on 200"), "30");
    assert_eq!(shown("10 kg + VAT"), "11.5 kg");
}

#[test]
fn a_tax_name_means_nothing_without_a_setting() {
    assert!(evaluate("$300 + VAT").is_err());
    let gst = Settings {
        tax: Some(Tax {
            name: "GST".into(),
            rate: Number::from_decimal_str("0.1").unwrap(),
        }),
    };
    assert_eq!(
        format_value(
            &evaluate_with("$110 - GST", &gst).unwrap(),
            &FormatOptions::default()
        )
        .text,
        "$100.00"
    );
    assert!(evaluate_with("$300 + VAT", &gst).is_err());
}

#[test]
fn nonsense_with_a_tax_is_refused() {
    for line in [
        "VAT × VAT",
        "VAT + VAT",
        "VAT / 2",
        "5 on VAT",
        "VAT + $5",
        "$5 × VAT",
        "VAT in VAT",
    ] {
        assert!(
            matches!(
                evaluate_with(line, &vat()),
                Err(Error::UnsupportedOperation(_)) | Err(Error::UnexpectedWord { .. })
            ),
            "{line}: {:?}",
            evaluate_with(line, &vat())
        );
    }
}

#[test]
fn a_sheet_uses_the_tax_setting() {
    let opts = babbage_core::SheetOptions {
        settings: vat(),
        ..babbage_core::SheetOptions::default()
    };
    let r = babbage_core::evaluate_sheet(&["price = $300", "price + VAT", "VAT on price"], &opts);
    let text = |i: usize| match &r[i].kind {
        babbage_core::LineKind::Value(v) => {
            format_value(v, &r[i].display_options(&FormatOptions::default())).text
        }
        other => format!("{other:?}"),
    };
    assert_eq!(text(1), "$345.00");
    assert_eq!(text(2), "$45.00");
}
