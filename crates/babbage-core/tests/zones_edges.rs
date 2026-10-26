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
    assert_eq!(shown("6pm in Chicago"), "1:00 pm");
    assert_eq!(shown("6pm UTC in Chicago"), "1:00 pm");
    assert_eq!(shown("6pm Sydney in Sydney"), "6:00 pm");
}

#[test]
fn case_and_spacing_do_not_matter() {
    for line in [
        "6pm SYDNEY in chicago",
        "6pm sydney IN Chicago",
        "6pm   Sydney   in   Chicago",
        "6pm Sydney to Chicago",
    ] {
        assert_eq!(shown(line), "3:00 am", "{line}");
    }
}

#[test]
fn an_abbreviation_is_a_fixed_offset_all_year() {
    assert_eq!(shown("3pm PDT to PST"), "2:00 pm");
    assert_eq!(shown("12pm PST in PDT"), "1:00 pm");
    assert_eq!(shown("12pm EST in New York"), "1:00 pm");
    assert_eq!(shown("12pm eastern time in New York"), "12:00 pm");
    assert_eq!(shown("12pm pacific time in eastern time"), "3:00 pm");
}

#[test]
fn offsets_are_written_gmt_or_utc_with_a_sign() {
    assert_eq!(shown("noon GMT+5:45 in UTC"), "6:15 am");
    assert_eq!(shown("noon UTC+14 in GMT-12"), "June 15 at 10:00 am");
    assert_eq!(shown("noon gmt-12 in utc+14"), "June 16 at 2:00 pm");
}

#[test]
fn a_repeated_or_skipped_hour_is_decided_the_same_way_every_time() {
    // 2024-03-10 02:30 does not exist in New York: the offset from before the change applies.
    assert_eq!(
        shown("March 10, 2024 2:30am New York in London"),
        "March 10 at 7:30 am"
    );
    // 2024-11-03 01:30 happens twice in New York: the first reading, daylight time, applies.
    assert_eq!(
        shown("November 3, 2024 1:30am New York in London"),
        "November 3 at 5:30 am"
    );
    assert_eq!(
        shown("November 3, 2024 3:30am New York in London"),
        "November 3 at 8:30 am"
    );
}

#[test]
fn a_country_means_its_capitals_zone() {
    assert_eq!(shown("noon Germany in Berlin"), "12:00 pm");
    assert_eq!(shown("noon USA in New York"), "12:00 pm");
    assert_eq!(shown("noon Australia in Sydney"), "12:00 pm");
    assert_eq!(shown("noon China in Beijing"), "12:00 pm");
}

#[test]
fn the_time_difference_is_how_far_ahead_the_first_place_is() {
    assert_eq!(shown("time difference between Tokyo and London"), "8 hours");
    assert_eq!(
        shown("time difference between London and Tokyo"),
        "-8 hours"
    );
    assert_eq!(
        shown("time difference between Kathmandu and Delhi"),
        "15 minutes"
    );
    assert_eq!(
        shown("time difference between Tokyo and Tokyo"),
        "0 seconds"
    );
}

#[test]
fn zone_words_do_not_disturb_other_lines() {
    assert_eq!(shown("5 km to m"), "5,000 m");
    assert_eq!(shown("3pm to 5pm"), "2 hours");
    assert_eq!(shown("10 inch to cm"), "25.4 cm");
}
