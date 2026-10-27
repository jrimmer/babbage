//! Horsepower: `hp` is mechanical (550 ft·lbf/s), metric horsepower is 75 kgf·m/s (735.49875 W) and electrical
//! horsepower is 746 W exactly. A "metric" or "electrical" in front of `hp` is part of the unit, not a skipped word.

use babbage_core::{FormatOptions, evaluate, format_value};

fn show(line: &str) -> String {
    let v = evaluate(line).unwrap_or_else(|e| panic!("{line}: {e}"));
    format_value(&v, &FormatOptions::default()).text
}

#[test]
fn the_three_horsepowers_in_watts() {
    assert_eq!(show("1 hp in W"), "745.6998715823 W");
    assert_eq!(show("1 metric hp in W"), "735.49875 W");
    assert_eq!(show("1 electrical hp in W"), "746 W");
}

#[test]
fn spellings() {
    assert_eq!(show("100 cv in kW"), "73.549875 kW");
    assert_eq!(show("2 metric horsepower in W"), "1,470.9975 W");
    assert_eq!(show("1 electric horsepower in W"), "746 W");
    assert_eq!(show("1 ehp in W"), "746 W");
}

#[test]
fn they_add_with_each_other() {
    assert_eq!(
        show("2 metric horsepower + 1 hp in W"),
        "2,216.6973715823 W"
    );
}
