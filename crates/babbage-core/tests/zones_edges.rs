use babbage_core::settings::{Settings, scoped};
use babbage_core::{Error, FormatOptions, evaluate_with, format_value};

/// Saturday 2024-06-15 12:00 UTC.
const NOW: i64 = 1_718_452_800;

fn run(line: &str) -> Result<String, Error> {
    let s = Settings::at(NOW);
    scoped(&s, || {
        let v = evaluate_with(line, &s)?;
        Ok(format_value(&v, &FormatOptions::default()).text)
    })
}

fn shown(line: &str) -> String {
    run(line).unwrap_or_else(|e| panic!("{line}: {e}"))
}

fn fails(line: &str) {
    assert!(
        run(line).is_err(),
        "{line} should fail, got {:?}",
        run(line)
    );
}

#[test]
fn an_unknown_place_is_an_error_not_a_guess() {
    for line in [
        "6pm Atlantis in Chicago",
        "6pm Sydney in Narnia",
        "6pm in Narnia",
        "time in Narnia",
        "Narnia time",
        "date in Narnia",
        "time difference between Sydney and Narnia",
        "6pm Sydney in",
        "6pm Sydney in GMT+25",
        "6pm Sydney in GMT+8:99",
        "6pm Sydney in GMT+",
    ] {
        fails(line);
    }
}

#[test]
fn a_time_with_no_place_is_in_the_local_zone() {
    assert_eq!(shown("6pm in Chicago"), "1:00 PM");
    assert_eq!(shown("6pm UTC in Chicago"), "1:00 PM");
    assert_eq!(shown("6pm Sydney in Sydney"), "6:00 PM");
}

#[test]
fn case_and_spacing_do_not_matter() {
    for line in [
        "6pm SYDNEY in chicago",
        "6pm sydney IN Chicago",
        "6pm   Sydney   in   Chicago",
        "6pm Sydney to Chicago",
    ] {
        assert_eq!(shown(line), "3:00 AM", "{line}");
    }
}

#[test]
fn a_us_abbreviation_is_its_region_all_year_and_the_others_are_fixed() {
    assert_eq!(shown("3pm PDT to PST"), "3:00 PM");
    assert_eq!(shown("12pm PST in PDT"), "12:00 PM");
    assert_eq!(shown("12pm EST in New York"), "12:00 PM");
    assert_eq!(shown("3pm CET to CEST"), "4:00 PM");
    assert_eq!(shown("12pm eastern time in New York"), "12:00 PM");
    assert_eq!(shown("12pm pacific time in eastern time"), "3:00 PM");
}

#[test]
fn offsets_are_written_gmt_or_utc_with_a_sign() {
    assert_eq!(shown("noon GMT+5:45 in UTC"), "06:15");
    assert_eq!(shown("noon UTC+14 in GMT-12"), "10:00 AM");
    assert_eq!(shown("noon gmt-12 in utc+14"), "Tomorrow at 2:00 PM");
}

#[test]
fn a_repeated_or_skipped_hour_is_decided_the_same_way_every_time() {
    // 2024-03-10 02:30 does not exist in New York: the offset from before the change applies.
    assert_eq!(
        shown("March 10, 2024 2:30am New York in London"),
        "March 10, 2024 at 7:30 AM"
    );
    // 2024-11-03 01:30 happens twice in New York: the first reading, daylight time, applies.
    assert_eq!(
        shown("November 3, 2024 1:30am New York in London"),
        "November 3, 2024 at 5:30 AM"
    );
    assert_eq!(
        shown("November 3, 2024 3:30am New York in London"),
        "November 3, 2024 at 8:30 AM"
    );
}

#[test]
fn a_country_means_its_capitals_zone() {
    assert_eq!(shown("noon Germany in Berlin"), "12:00 PM");
    assert_eq!(shown("noon USA in New York"), "12:00 PM");
    assert_eq!(shown("noon Australia in Sydney"), "12:00 PM");
    assert_eq!(shown("noon China in Beijing"), "12:00 PM");
}

#[test]
fn the_time_difference_is_how_far_ahead_the_second_place_is() {
    assert_eq!(
        shown("time difference between Tokyo and London"),
        "-8 hours"
    );
    assert_eq!(shown("time difference between London and Tokyo"), "8 hours");
    assert_eq!(
        shown("time difference between Kathmandu and Delhi"),
        "-15 min"
    );
    assert_eq!(shown("time difference between Tokyo and Tokyo"), "0 s");
}

#[test]
fn zone_words_do_not_disturb_other_lines() {
    assert_eq!(shown("5 km to m"), "5,000 m");
    assert_eq!(shown("3pm to 5pm"), "2 hours");
    assert_eq!(shown("10 inch to cm"), "25.4 cm");
}

/// A moment given as hours after 00:00 UTC on June 15 (the test clock's day), written as the answer shows it: just
/// the time on June 15 itself, `Yesterday at` or `Tomorrow at` a day either side, and the date with its year beyond;
/// exactly midnight is a bare date.
fn shown_moment(hours_after: i32) -> String {
    let (day, hour) = (hours_after.div_euclid(24), hours_after.rem_euclid(24));
    let clock = format!(
        "{}:00 {}",
        if hour % 12 == 0 { 12 } else { hour % 12 },
        if hour < 12 { "AM" } else { "PM" }
    );
    match (day, hour) {
        (d, 0) => format!("June {}", 15 + d),
        (0, _) => clock,
        (-1, _) => format!("Yesterday at {clock}"),
        (1, _) => format!("Tomorrow at {clock}"),
        (d, _) => format!("June {}, 2024 at {clock}", 15 + d),
    }
}

/// Noon in a zone `offset` hours ahead of UTC means noon on that zone's own date, which the test clock
/// (12:00 UTC on June 15) puts at `floor((12 + offset) / 24)` days after June 15. Shown in UTC the answer is a time
/// of day on the 24-hour clock when it lands on that same date, and a date otherwise.
fn noon_in_zone_as_utc(offset: i32) -> String {
    let zone_day = (12 + offset).div_euclid(24);
    let moment = zone_day * 24 + 12 - offset;
    if moment.div_euclid(24) == zone_day {
        format!("{:02}:00", moment.rem_euclid(24))
    } else {
        shown_moment(moment)
    }
}

#[test]
fn every_abbreviation_is_the_fixed_offset_it_names() {
    // Offsets in hours east of UTC, written here independently of the engine's table.
    for (abbr, hours) in [
        ("UTC", 0),
        ("GMT", 0),
        ("UT", 0),
        ("Z", 0),
        ("WET", 0),
        ("WEST", 1),
        ("CET", 1),
        ("CEST", 2),
        ("EET", 2),
        ("EEST", 3),
        ("MSK", 3),
        ("JST", 9),
        ("KST", 9),
        ("AWST", 8),
        ("AEST", 10),
        ("AEDT", 11),
        ("NZST", 12),
        ("NZDT", 13),
    ] {
        assert_eq!(
            shown(&format!("noon {abbr} in UTC")),
            noon_in_zone_as_utc(hours),
            "{abbr} to UTC"
        );
        let to_abbr = if ["UTC", "GMT", "UT", "Z"].contains(&abbr) {
            "12:00".to_string()
        } else {
            shown_moment(12 + hours)
        };
        assert_eq!(
            shown(&format!("noon UTC in {abbr}")),
            to_abbr,
            "UTC to {abbr}"
        );
    }
}

/// The US abbreviations are their regions, summer time included: on 15 June `PST` is UTC-7 like `PDT`.
#[test]
fn the_us_abbreviations_are_their_regions() {
    for (abbr, hours) in [
        ("EST", -4),
        ("EDT", -4),
        ("CST", -5),
        ("CDT", -5),
        ("MST", -6),
        ("MDT", -6),
        ("PST", -7),
        ("PDT", -7),
        ("AKST", -8),
        ("AKDT", -8),
        ("HST", -10),
    ] {
        assert_eq!(
            shown(&format!("noon {abbr} in UTC")),
            noon_in_zone_as_utc(hours),
            "{abbr} to UTC"
        );
        assert_eq!(
            shown(&format!("noon UTC in {abbr}")),
            shown_moment(12 + hours),
            "UTC to {abbr}"
        );
    }
}

#[test]
fn a_place_name_of_four_words_works_and_five_does_not() {
    assert_eq!(
        shown("noon united states of america in New York"),
        "12:00 PM"
    );
    assert_eq!(
        shown("noon new york in united states of america"),
        "12:00 PM"
    );
    for line in [
        "noon a b c d e in New York",
        "noon New York in a b c d e",
        "time in united states of america now",
        "time difference between united states of america and a b c d e",
    ] {
        assert!(run(line).is_err(), "{line}: {:?}", run(line));
    }
}

#[test]
fn zones_loaded_from_files_compare_by_identity() {
    use babbage_core::zones::{Zone, ZoneSource};
    let fixtures =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zoneinfo");
    let s = Settings {
        zone_source: ZoneSource::System { dir: fixtures },
        ..Settings::at(NOW)
    };
    scoped(&s, || {
        let (a, b, tokyo) = (
            Zone::named("Asia/Tokyo"),
            Zone::named("Asia/Tokyo"),
            Zone::named("Europe/Berlin"),
        );
        assert!(matches!(a, Some(Zone::Loaded(_))));
        assert_eq!(a, b);
        assert_ne!(a, tokyo);
        assert_ne!(Zone::named("Europe/Berlin"), Zone::named("Asia/Kolkata"));
    });
}

#[test]
fn an_offset_has_bounds_on_hours_and_minutes() {
    use babbage_core::zones::{Zone, offset_zone};
    assert_eq!(offset_zone(false, 18, 0), Some(Zone::Fixed(18 * 3600)));
    assert_eq!(offset_zone(false, 19, 0), None);
    assert_eq!(
        offset_zone(false, 5, 59),
        Some(Zone::Fixed(5 * 3600 + 59 * 60))
    );
    assert_eq!(offset_zone(false, 5, 60), None);
    assert_eq!(
        offset_zone(true, 5, 30),
        Some(Zone::Fixed(-(5 * 3600 + 30 * 60)))
    );
    assert_eq!(offset_zone(true, 0, 0), Some(Zone::Fixed(0)));
    assert_eq!(babbage_core::zones::place(&[]), None);
}
