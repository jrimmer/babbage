//! What each date, time and zone form does with a line that is almost, but not quite, its shape: the exact
//! answer or the exact error. A form that starts accepting or refusing a neighboring shape changes one of
//! these, so a broken guard or index cannot go unnoticed. Reviewed by hand when written.

use babbage_core::settings::{Settings, scoped};
use babbage_core::{FormatOptions, evaluate_with, format_value};

const CASES: &[(&str, &str)] = &[
    (
        "day of the week on",
        "ERROR unexpected word \"the\" at byte 7",
    ),
    (
        "week number of",
        "ERROR unexpected word \"number\" at byte 5",
    ),
    (
        "day number in",
        "ERROR unexpected word \"number\" at byte 4",
    ),
    ("weekday on", "ERROR unexpected word \"weekday\" at byte 0"),
    (
        "days between June 5 and",
        "ERROR unexpected word \"between\" at byte 5",
    ),
    (
        "days between and June 5",
        "ERROR unexpected word \"between\" at byte 5",
    ),
    (
        "days between June 5",
        "ERROR unexpected word \"between\" at byte 5",
    ),
    (
        "workdays between June 5 and",
        "ERROR unexpected word \"between\" at byte 9",
    ),
    (
        "workdays between and June 5",
        "ERROR unexpected word \"between\" at byte 9",
    ),
    (
        "work hours between and June 5",
        "ERROR unexpected word \"work\" at byte 0",
    ),
    (
        "work hours between June 5 and",
        "ERROR unexpected word \"work\" at byte 0",
    ),
    (
        "workdays from to June 5",
        "ERROR unexpected word \"from\" at byte 9",
    ),
    (
        "workdays from June 5 to",
        "ERROR unexpected word \"from\" at byte 9",
    ),
    (
        "workdays from to",
        "ERROR unexpected word \"from\" at byte 9",
    ),
    (
        "June 5 through",
        "ERROR unexpected word \"through\" at byte 7",
    ),
    (
        "through June 5",
        "ERROR unexpected word \"through\" at byte 0",
    ),
    ("through", "ERROR unexpected word \"through\" at byte 0"),
    (
        "3 weeks after 5",
        "ERROR unexpected word \"after\" at byte 8",
    ),
    (
        "3 weeks before 5",
        "ERROR unexpected word \"before\" at byte 8",
    ),
    ("3 weeks from 5", "ERROR unexpected word \"from\" at byte 8"),
    (
        "after March 14",
        "ERROR unexpected word \"after\" at byte 0",
    ),
    ("3 after", "ERROR unexpected word \"after\" at byte 2"),
    ("3 weeks after", "ERROR unexpected word \"after\" at byte 8"),
    ("as iso8601", "ERROR unexpected word \"as\" at byte 0"),
    ("5 iso8601", "ERROR unexpected word \"iso\" at byte 2"),
    ("5 as iso 8602", "ERROR unexpected word \"as\" at byte 2"),
    ("June 5 as iso8601", "2024-06-05T00:00:00Z"),
    ("June 5 to iso8601", "2024-06-05T00:00:00Z"),
    ("June 5 in iso8601", "2024-06-05T00:00:00Z"),
    (
        "5 timestamp",
        "ERROR unexpected word \"timestamp\" at byte 2",
    ),
    ("to timestamp", "ERROR unexpected word \"to\" at byte 0"),
    ("timestamp", "ERROR unexpected word \"timestamp\" at byte 0"),
    ("current", "ERROR unexpected word \"current\" at byte 0"),
    (
        "current timestamp 5",
        "ERROR unexpected word \"current\" at byte 0",
    ),
    ("at 1.5x", "ERROR unexpected word \"at\" at byte 0"),
    (
        "5 at 1.5x",
        "ERROR unsupported: this needs a length of time",
    ),
    ("1 hour at", "ERROR unexpected word \"at\" at byte 7"),
    ("time saved", "ERROR unexpected word \"time\" at byte 0"),
    (
        "time saved at 1.5x",
        "ERROR unexpected word \"time\" at byte 0",
    ),
    (
        "time saved 5 at 1.5x",
        "ERROR unsupported: this needs a length of time",
    ),
    (
        "1 hour at at 1.5x",
        "ERROR unexpected word \"at\" at byte 7",
    ),
    ("2 hours at 2x", "1 hour"),
    ("time saved 2 hours at 2x", "1 hour"),
    ("as timecode", "ERROR unexpected word \"as\" at byte 0"),
    (
        "5 as timecode",
        "ERROR unsupported: this needs a length of time or a number of frames",
    ),
    ("5 frames", "5 frames"),
    ("frames", "1 frame"),
    ("as frames", "ERROR unexpected word \"as\" at byte 0"),
    (
        "5 as frames",
        "ERROR unsupported: this needs a length of time",
    ),
    ("10 seconds at 30 fps as frames", "300 frames"),
    (
        "10 seconds at fps as frames",
        "ERROR unexpected word \"at\" at byte 11",
    ),
    (
        "10 seconds at 30 as frames",
        "ERROR unexpected word \"at\" at byte 11",
    ),
    (
        "10 seconds at 30 km as frames",
        "ERROR unexpected word \"at\" at byte 11",
    ),
    ("at 30 fps as frames", "ERROR empty expression"),
    (
        "as at 30 fps as timecode",
        "ERROR unexpected word \"as\" at byte 0",
    ),
    ("10 seconds at 30 fps as timecode", "00:00:10:00"),
    ("noon GMT+5 in UTC", "07:00"),
    ("noon GMT+5:30 in UTC", "06:30"),
    (
        "noon GMT+true in UTC",
        "ERROR unexpected word \"GMT\" at byte 5",
    ),
    (
        "noon GMT+2x in UTC",
        "ERROR unexpected word \"GMT\" at byte 5",
    ),
    (
        "noon GMT+1.5x in UTC",
        "ERROR unexpected word \"GMT\" at byte 5",
    ),
    (
        "days in June xyz",
        "ERROR unexpected word \"June\" at byte 8",
    ),
    ("days in 2024 2025", "ERROR unexpected number at byte 13"),
    (
        "days in Q2 2024 2025",
        "ERROR unexpected word \"Q\" at byte 8",
    ),
    ("workdays in 1 week", "5 workdays"),
];

fn answer(line: &str) -> String {
    let s = Settings::at(1_718_452_800);
    scoped(&s, || match evaluate_with(line, &s) {
        Ok(v) => format_value(&v, &FormatOptions::default()).text,
        Err(e) => format!("ERROR {e}"),
    })
}

#[test]
fn near_misses_are_answered_or_refused_exactly_as_reviewed() {
    let wrong: Vec<String> = CASES
        .iter()
        .filter_map(|(line, want)| {
            let got = answer(line);
            (got != *want).then(|| format!("{line}: want {want:?}, got {got:?}"))
        })
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn both_spellings_of_a_frame_count_and_a_non_clock_offset() {
    for (line, want) in [
        ("10 seconds at 30 fps as frames", "300 frames"),
        ("10 seconds at 30 fps as frame", "300 frames"),
        ("10 seconds at 30 fps to frames", "300 frames"),
        ("10 seconds at 30 fps in frames", "300 frames"),
        ("10 seconds at 30 fps as timecode", "00:00:10:00"),
        ("10 seconds at 24 fps as timecode", "00:00:10:00"),
    ] {
        assert_eq!(answer(line), want, "{line}");
    }
    for line in [
        "noon GMT+true in UTC",
        "noon UTC-true in UTC",
        "noon GMT+2x in UTC",
    ] {
        assert!(
            answer(line).starts_with("ERROR"),
            "{line}: {}",
            answer(line)
        );
    }
}
