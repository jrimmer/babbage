use babbage_core::units::{lookup, registry};
use babbage_core::{Error, FormatOptions, Kind, evaluate, format_value};

const FACTORS: &str = include_str!("corpus/units.tsv");

fn shown(line: &str) -> String {
    let v = evaluate(line).unwrap_or_else(|e| panic!("{line:?}: {e}"));
    format_value(&v, &FormatOptions::default()).text
}

#[test]
fn every_unit_factor_matches_pint_exactly() {
    let mut opts = FormatOptions::new(10).unwrap();
    opts.group_thousands = false;
    opts.trim_trailing_zeros = false;
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in FACTORS.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        let (expr, exact, display) = (cols[0], cols[1], cols[2]);
        match evaluate(expr) {
            Err(e) => failures.push(format!("{expr}: {e}")),
            Ok(v) => {
                let ours = format!("{}/{}", v.number().numer(), v.number().denom());
                let text = format_value(&v, &opts).text;
                if ours != exact || text != display {
                    failures.push(format!(
                        "{expr}: want {exact} / {display}, got {ours} / {text}"
                    ));
                }
            }
        }
        checked += 1;
    }
    assert!(checked >= 180, "corpus unexpectedly small: {checked}");
    assert!(
        failures.is_empty(),
        "{} of {checked} unit factors differ, first 20:\n{}",
        failures.len(),
        failures
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn every_spelling_of_a_unit_means_the_same_unit() {
    for (name, unit) in registry().names() {
        assert_eq!(lookup(name).as_ref(), Some(unit), "{name}");
    }
}

#[test]
fn the_example_lines_for_conversion() {
    assert_eq!(shown("10 km in m"), "10,000 m");
    assert_eq!(shown("100 pounds in kg"), "45.359237 kg");
    assert_eq!(shown("5 km to miles"), "3.1068559612 mi");
    assert_eq!(shown("1 km + 1,000 m"), "2 km");
    assert_eq!(shown("300 + 20 km"), "320 km");
    assert_eq!(shown("10m × 10m"), "100 m²");
}

#[test]
fn the_screenshot_volume_lines() {
    assert_eq!(shown("(800mm × 800mm × 1,000mm) / 10"), "64M mm³");
    assert_eq!(shown("256mm x 256mm x 256mm"), "16.777216M mm³");
    assert_eq!(
        shown("(800mm × 800mm × 1,000mm) / 10 / (256mm x 256mm x 256mm)"),
        "3.8146972656"
    );
    assert_eq!(
        evaluate("64000000 mm³ / 16777216 mm³").unwrap().kind(),
        Kind::Plain
    );
}

#[test]
fn unit_arithmetic_rules() {
    assert_eq!(shown("$20 + 30".trim_start_matches('$')), "50");
    assert_eq!(shown("1 km + 1 m"), "1.001 km");
    assert_eq!(shown("1 m + 1 km"), "1.001 km");
    assert_eq!(shown("1500 m + 1 km"), "2.5 km");
    assert_eq!(shown("2 km - 500 m"), "1.5 km");
    assert_eq!(shown("3 × 4 m"), "12 m");
    assert_eq!(shown("4 m × 3"), "12 m");
    assert_eq!(shown("12 m / 4"), "3 m");
    assert_eq!(shown("12 m / 4 m"), "3");
    assert_eq!(shown("1 km / 250 m"), "4");
    assert_eq!(shown("5 km / 2 m"), "2,500");
    assert_eq!(shown("2 m × 3 m × 4 m"), "24 m³");
    assert_eq!(shown("(5 m)^2"), "25 m²");
    assert_eq!(shown("3 m^2 × 2"), "6 m²");
    assert_eq!(shown("10 km / 2 h"), "5 km/h");
    assert_eq!(shown("10 km/h × 3 h"), "30 km");
    assert_eq!(shown("-5 km"), "-5 km");
}

#[test]
fn exponent_spellings() {
    for l in [
        "3 mm³",
        "3 mm^3",
        "3 mm3",
        "3 cubic mm",
        "3 cubic millimeters",
        "3 cumm",
    ] {
        assert_eq!(shown(l), "3 mm³", "{l}");
    }
    assert_eq!(shown("2 m²"), "2 m²");
    assert_eq!(shown("2 sqft"), "2 ft²");
    assert_eq!(shown("2 square feet"), "2 ft²");
    assert_eq!(shown("2 m/s²"), "2 m/s²");
    assert_eq!(shown("2 m/s^2"), "2 m/s²");
    assert_eq!(shown("2 s^-1"), "2 s⁻¹");
}

#[test]
fn conversion_words_and_directions() {
    for w in ["in", "to", "as", "into"] {
        assert_eq!(shown(&format!("2 km {w} m")), "2,000 m", "{w}");
        assert_eq!(shown(&format!("2000 m {w} km")), "2 km", "{w}");
    }
    assert_eq!(shown("1 mi to ft"), "5,280 ft");
    assert_eq!(shown("1 gal in L"), "3.785411784 L");
    assert_eq!(shown("1 yr in days"), "365.2425 day");
    assert_eq!(shown("1 mo in days"), "30.436875 day");
    assert_eq!(shown("60 mph in km/h"), "96.56064 km/h");
    assert_eq!(shown("1 GiB in MiB"), "1,024 MiB");
    assert_eq!(shown("1 GB in MB"), "1,000 MB");
    assert_eq!(shown("1 B in bits"), "8 bit");
}

#[test]
fn incompatible_units_are_refused() {
    for l in [
        "1 m + 1 kg",
        "1 kg - 1 s",
        "1 m in kg",
        "5 in m",
        "1 m to s",
        "1 GB in m",
    ] {
        assert_eq!(evaluate(l), Err(Error::IncompatibleUnits), "{l}");
    }
}

#[test]
fn units_and_percent_work_together() {
    assert_eq!(shown("100 km + 10%"), "110 km");
    assert_eq!(shown("100 km - 10%"), "90 km");
    assert_eq!(shown("10% of 200 m"), "20 m");
    assert_eq!(shown("10% on 200 m"), "220 m");
    assert_eq!(shown("20 m is 10% of what"), "200 m");
    assert_eq!(shown("50 m is what % of 200 m"), "25%");
    assert_eq!(shown("50 cm is what % of 2 m"), "25%");
    assert_eq!(shown("1 km to 1.5 km is what %"), "50%");
    assert!(evaluate("50 m is what % of 200 kg").is_err());
    assert!(evaluate("5 m as %").is_err());
}

const QUANTITIES: &str = include_str!("corpus/quantity.tsv");

#[test]
fn agrees_with_the_quantity_oracle_on_every_line() {
    let mut opts = FormatOptions::new(10).unwrap();
    opts.group_thousands = false;
    opts.trim_trailing_zeros = false;
    let mut failures = Vec::new();
    let mut checked = 0;
    for line in QUANTITIES.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        assert_eq!(cols.len(), 4, "malformed corpus line {line:?}");
        let (expr, kind, exact, display) = (cols[0], cols[1], cols[2], cols[3]);
        let got = evaluate(expr);
        let ok = match (kind, &got) {
            ("ERR:incompatible", Err(Error::IncompatibleUnits)) => true,
            ("plain" | "percent", Ok(v)) => {
                let want = if kind == "plain" {
                    Kind::Plain
                } else {
                    Kind::Percent
                };
                let ours = format!("{}/{}", v.number().numer(), v.number().denom());
                v.kind() == want && ours == exact && format_value(v, &opts).text == display
            }
            _ => false,
        };
        if !ok {
            failures.push(format!(
                "{expr:?} want {kind} {exact} {display}, got {got:?}"
            ));
        }
        checked += 1;
    }
    assert!(checked > 12_000, "corpus unexpectedly small: {checked}");
    assert!(
        failures.is_empty(),
        "{} of {checked} disagree, first 12:\n{}",
        failures.len(),
        failures
            .iter()
            .take(12)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}
