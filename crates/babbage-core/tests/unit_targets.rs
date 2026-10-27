//! A target of a conversion that measures something else is adapted when it can be: a speed `to minutes` replaces
//! the time it is divided by, and a power of a length `to` a length is the same power of the target.

use babbage_core::{FormatOptions, evaluate, format_value};

fn shown(line: &str) -> Result<String, String> {
    evaluate(line)
        .map(|v| format_value(&v, &FormatOptions::default()).text)
        .map_err(|e| e.to_string())
}

#[test]
fn a_speed_to_a_unit_of_time_replaces_its_denominator() {
    assert_eq!(shown("3 mph to minutes").unwrap(), "0.05 mi/min");
    assert_eq!(shown("60 mph to minutes").unwrap(), "1 mi/min");
    assert_eq!(shown("5 km/h to minutes").unwrap(), "0.0833333333 km/min");
}

#[test]
fn an_area_or_volume_to_a_length_is_that_power_of_the_length() {
    assert_eq!(shown("10 cm^3 to m").unwrap(), "0.00001 m³");
    assert_eq!(shown("12 ft^2 to yd").unwrap(), "1.3333333333 yd²");
}

#[test]
fn a_conversion_that_only_restates_the_question_or_mixes_kinds_is_refused() {
    assert!(shown("10 cm^3 to cm").is_err());
    assert!(shown("1 cm in px at 326 km").is_err());
}
