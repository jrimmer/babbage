//! Edges of the number-theory, music, angle, screen and random-number functions: the smallest and largest
//! inputs each form accepts, and the first one it refuses.

use babbage_core::functions::seed_random;
use babbage_core::{FormatOptions, evaluate, format_value};

fn shown(line: &str) -> Result<String, String> {
    evaluate(line)
        .map(|v| format_value(&v, &FormatOptions::default()).text)
        .map_err(|e| e.to_string())
}

fn ok(line: &str) -> String {
    shown(line).unwrap_or_else(|e| panic!("{line}: {e}"))
}

#[test]
fn primes_are_found_by_trial_division_up_to_a_limit() {
    for p in [2, 3, 5, 7, 11, 13, 97, 7919] {
        assert_eq!(ok(&format!("is {p} prime")), "true", "{p}");
    }
    for c in [0, 1, 4, 6, 9, 15, 25, 35, 49, 91, 7921] {
        assert_eq!(ok(&format!("is {c} prime")), "false", "{c}");
    }
    assert_eq!(ok("is -7 prime"), "false");
    assert_eq!(ok("is 999999999989 prime"), "true");
    assert_eq!(ok("is 1000000000000 prime"), "false");
    assert!(shown("is 1000000000001 prime").is_err());
}

#[test]
fn a_frequency_is_named_by_the_nearest_note() {
    assert_eq!(ok("440 hz as pitch"), "A4");
    assert_eq!(ok("261.63 hz as pitch"), "C4");
    assert_eq!(ok("1 khz as pitch"), "B5");
    assert_eq!(ok("466.16 hz as pitch"), "A#4");
    assert_eq!(ok("8.18 hz as pitch"), "C-1");
    assert_eq!(ok("8 hz as pitch"), "C-1");
    assert_eq!(ok("12543.85 hz as pitch"), "G9");
    assert_eq!(ok("27.5 hz as pitch"), "A0");
    assert!(shown("7 hz as pitch").is_err());
    assert!(shown("13000 hz as pitch").is_err());
    assert!(shown("0 hz as pitch").is_err());
    assert!(shown("-440 hz as pitch").is_err());
    assert!(shown("440 as pitch").is_err());
    assert!(shown("440 m as pitch").is_err());
}

#[test]
fn an_angle_is_written_as_degrees_minutes_and_seconds() {
    assert_eq!(ok("156.742° as dms"), "156° 44′ 31.2″");
    assert_eq!(ok("5 as dms"), "5° 0′ 0″");
    assert_eq!(ok("-156.742 as dms"), "-156° 44′ 31.2″");
    assert_eq!(ok("1 rad as dms"), "57° 17′ 44.8062470964″");
    assert_eq!(ok("0.99999999999999 as dms"), "1° 0′ 0″");
    assert_eq!(ok("10.016666666666666 as dms"), "10° 1′ 0″");
    assert!(shown("1 m as dms").is_err());
    assert!(shown("5 kg as dms").is_err());
}

#[test]
fn the_density_of_a_screen_needs_positive_sizes() {
    assert_eq!(ok("ppi of 15\" screen at 1024x768"), "85.3333333333 PPI");
    assert_eq!(ok("ppi of 15 screen at 1024x768"), "85.3333333333 PPI");
    assert_eq!(ok("ppi of 38.1 cm screen at 1024x768"), "85.3333333333 PPI");
    assert!(shown("ppi of 0 screen at 1024x768").is_err());
    assert!(shown("ppi of -15 screen at 1024x768").is_err());
    assert!(shown("ppi of 15 screen at 0x768").is_err());
    assert!(shown("ppi of 15 screen at 1024x0").is_err());
    assert!(shown("ppi of 15 screen at -1024x768").is_err());
}

#[test]
fn combinations_use_the_smaller_side_and_refuse_impossible_counts() {
    assert_eq!(ok("10 combinations 8"), "45");
    assert_eq!(ok("100000 combinations 99998"), "4.99995G");
    assert_eq!(ok("5 combinations 5"), "1");
    assert_eq!(ok("5 permutations 5"), "120");
    assert!(shown("5 combinations 6").is_err());
    assert!(shown("-5 combinations 2").is_err());
    assert!(shown("5 combinations -2").is_err());
    assert!(shown("5 permutations -2").is_err());
}

#[test]
fn a_root_index_is_between_one_and_a_thousand() {
    assert_eq!(ok("1 root of 8"), "8");
    assert_eq!(ok("3 root of 8"), "2");
    assert_eq!(ok("1000 root of 1"), "1");
    assert!(shown("0 root of 8").is_err());
    assert!(shown("1001 root of 8").is_err());
}

#[test]
fn a_clock_time_is_hours_as_a_decimal() {
    assert_eq!(ok("10:15 to decimal"), "10.25");
    assert_eq!(ok("12.5 to decimal"), "12.5");
}

#[test]
fn a_standard_deviation_needs_values_of_one_kind() {
    assert_eq!(
        ok("standard deviation of 2, 4, 4, 4, 5, 5, 7, 9"),
        "2.1380899353"
    );
    assert_eq!(ok("standard deviation of 1, 2, 3"), "1");
    assert_eq!(ok("standard deviation of 1 m, 200 cm, 3 m"), "1 m");
    assert!(shown("standard deviation of 1 m, 2 kg").is_err());
    assert!(shown("standard deviation of 1 m, 2").is_err());
    assert!(shown("standard deviation of 5").is_err());
}

#[test]
fn a_rounding_step_must_be_positive() {
    assert_eq!(ok("37 to nearest 10"), "40");
    assert!(shown("37 to nearest 0").is_err());
    assert!(shown("37 to nearest -10").is_err());
}

#[test]
fn random_numbers_follow_the_seed_and_the_limits() {
    seed_random(7);
    let first: Vec<String> = (0..8)
        .map(|_| ok("random number between 1 and 100"))
        .collect();
    assert_eq!(first, ["28", "53", "44", "8", "51", "26", "66", "49"]);
    seed_random(7);
    let again: Vec<String> = (0..8)
        .map(|_| ok("random number between 1 and 100"))
        .collect();
    assert_eq!(first, again);
    seed_random(8);
    let other: Vec<String> = (0..8)
        .map(|_| ok("random number between 1 and 100"))
        .collect();
    assert_ne!(first, other);

    seed_random(7);
    let zero_based: Vec<String> = (0..12)
        .map(|_| ok("random number between 0 and 5"))
        .collect();
    assert_eq!(
        zero_based,
        ["3", "0", "3", "1", "4", "1", "1", "0", "3", "5", "1", "4"]
    );
    seed_random(7);
    let fractions: Vec<String> = (0..6)
        .map(|_| ok("random number between 0 and 1"))
        .collect();
    assert_eq!(
        fractions,
        [
            "0.7575888327",
            "0.7144956564",
            "0.5666571708",
            "0.8406936777",
            "0.1114063188",
            "0.5483879899"
        ]
    );
    seed_random(7);
    let wide: Vec<String> = (0..6)
        .map(|_| ok("random number between 10 and 2.5"))
        .collect();
    assert_eq!(
        wide,
        [
            "3.2575888327",
            "6.2844438974",
            "3.1874004805",
            "8.4160813997",
            "4.6848483162",
            "9.1622060429"
        ]
    );
    seed_random(7);
    let negative: Vec<String> = (0..6)
        .map(|_| ok("random number between -3 and 3"))
        .collect();
    assert_eq!(negative, ["-3", "-3", "3", "2", "-2", "3"]);
    seed_random(7);
    let tiny: Vec<String> = (0..6)
        .map(|_| ok("random number between 0 and 0.0000000001"))
        .collect();
    assert_eq!(tiny, ["1e-10", "0", "1e-10", "1e-10", "0", "1e-10"]);
    assert_eq!(ok("random number between 4 and 4"), "4");
    for bad in [
        "random number between 1 m and 5",
        "random number between 1 and 5 m",
        "random number between $1 and 5",
        "random number between 9:00 and 5",
    ] {
        assert!(shown(bad).is_err(), "{bad}");
    }
}

#[test]
fn refusals_say_which_kind_of_input_was_wrong() {
    let outside = "that value is outside what this function accepts";
    for line in [
        "-5 combinations 2",
        "5 combinations -2",
        "5 combinations 7",
        "-5 permutations 2",
        "0 hz as pitch",
    ] {
        assert_eq!(shown(line).unwrap_err(), outside, "{line}");
    }
    assert_eq!(ok("1000000 combinations 999998"), "499.9995G");
    assert_eq!(ok("midpoint(2024-01-01, 2024-01-03)"), "January 2, 2024");
    assert_eq!(ok("midpoint(4, 10)"), "7");
    let units = shown("stddev($1, 3)").unwrap_err();
    assert_eq!(units, shown("stddev(1 m, 2)").unwrap_err());
}

#[test]
fn a_spread_is_exact_only_when_the_values_and_the_root_are() {
    let approximate = |line: &str| evaluate(line).unwrap().is_approximate();
    assert!(!approximate("standard deviation of 1, 2, 3"));
    assert!(approximate("standard deviation of 1, 2"));
    assert!(approximate(
        "standard deviation of sqrt(2), sqrt(2) + 1, sqrt(2) + 2"
    ));
}
