use babbage_core::settings::{Settings, scoped};
use babbage_core::{Error, FormatOptions, evaluate_with, format_value};

const NOW: i64 = 1_718_452_800;

fn run(line: &str) -> Result<(String, bool), Error> {
    let s = Settings::at(NOW);
    scoped(&s, || {
        let v = evaluate_with(line, &s)?;
        let f = format_value(&v, &FormatOptions::default());
        Ok((f.text, f.exact))
    })
}

fn shown(line: &str) -> String {
    run(line).unwrap_or_else(|e| panic!("{line}: {e}")).0
}

fn fails(line: &str) {
    assert!(
        run(line).is_err(),
        "{line} should fail, got {:?}",
        run(line)
    );
}

#[test]
fn layouts_refuse_what_cannot_be_written_that_way() {
    for line in [
        "5 km as timespan",
        "5 km as laptime",
        "3 as laptime",
        "3 as timespan",
        "10 m in hours and minutes",
        "5 hours in hours and km",
        "$5 as timespan",
        "5pm as timespan",
    ] {
        fails(line);
    }
}

#[test]
fn zero_negative_and_large_spans() {
    for (line, want) in [
        ("0 minutes as timespan", "0 s"),
        ("-90 minutes as timespan", "-1 hour 30 min"),
        ("0 minutes as laptime", "00:00:00"),
        ("-90 minutes as laptime", "-01:30:00"),
        ("100:00:00 as timespan", "4 days 4 hours"),
        ("1:00:00 - 2:00:00", "-01:00:00"),
        ("00:00:00.001 + 00:00:00.001", "00:00:00.002"),
        ("7pm to 7pm", "0 s"),
        ("0 minutes in hours and minutes", "0 min"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
}

#[test]
fn a_third_of_a_second_is_cut_and_reported_as_inexact() {
    for line in ["1 second / 3 as laptime", "1 second / 3 as timespan"] {
        let (text, exact) = run(line).unwrap();
        assert!(!exact, "{line}: {text}");
        assert!(text.contains("333"), "{line}: {text}");
    }
    assert!(run("1.5 seconds as laptime").unwrap().1);
}

#[test]
fn clock_literals_have_strict_shapes() {
    for line in [
        "12:60", "25:00", "13:00 pm", "0pm", "13pm", "5:5pm", "7:300",
    ] {
        fails(line);
    }
    for (line, want) in [
        ("0:30", "12:30 AM"),
        ("23:59", "11:59 PM"),
        ("12am", "12:00 AM"),
        ("12:30am", "12:30 AM"),
        ("12pm", "12:00 PM"),
        ("9:05 AM", "9:05 AM"),
        ("noon", "12:00 PM"),
        ("midnight", "12:00 AM"),
    ] {
        assert_eq!(shown(line), want, "{line}");
    }
}

#[test]
fn a_time_of_day_wraps_past_midnight_in_both_directions() {
    assert_eq!(shown("11pm + 3 hours"), "Tomorrow at 2:00 AM");
    assert_eq!(shown("1am - 3 hours"), "Yesterday at 10:00 PM");
    assert_eq!(shown("noon + 48 hours"), "June 17, 2024 at 12:00 PM");
}

#[test]
fn m_is_minutes_only_next_to_a_time_unit() {
    assert_eq!(shown("3h 5m 10s in seconds"), "11,110 s");
    assert_eq!(shown("5m 10s in seconds"), "310 s");
    assert_eq!(shown("2 km 5 m in m"), "2,005 m");
    assert_eq!(shown("5m + 10m"), "15 m");
    fails("5m + 10s");
}

#[test]
fn playback_speed_needs_a_time_and_a_speed() {
    for line in ["5 km at 2x", "$5 at 2x", "1 hour at 1.5", "1 hour at 0x"] {
        fails(line);
    }
    assert_eq!(shown("2 hours at 2x"), "1 hour");
    assert_eq!(shown("2 hours at 0.5x"), "4 hours");
    assert_eq!(shown("time saved 2 hours at 2x"), "1 hour");
}

#[test]
fn timestamps_are_written_in_full() {
    assert_eq!(shown("June 15, 2024 to timestamp"), "1718452800");
    assert_eq!(shown("current timestamp"), "1718452800");
    // A date from a timestamp ends with the zone's name (the expected behaviour: `... AM PST`); here the zone is UTC.
    assert_eq!(shown("1718452800 to date"), "12:00 PM UTC");
    assert_eq!(shown("0 to date"), "January 1, 1970");
    assert_eq!(
        shown("99999999999 to date"),
        "November 16, 5138 at 9:46:39 AM UTC"
    );
    assert_eq!(
        shown("100000000000 to date"),
        "March 3, 1973 at 9:46:40 AM UTC"
    );
}

#[test]
fn iso_dates_are_read_strictly() {
    // A date that is not one is arithmetic (the expected answer is `2019-02-30` as 1,987).
    assert_eq!(shown("2019-02-30"), "1,987");
    assert_eq!(shown("2019-13-01"), "2,005");
    assert_eq!(shown("2019-00-10"), "2,009");
    assert_eq!(shown("2019-04-31"), "1,984");
    for line in [
        "2019-04-01T24:00:00",
        "2019-04-01T12:60:00",
        "2019-04-01T12:00:60",
        "2019-04-01T12:00:00+24:00",
    ] {
        fails(line);
    }
    assert_eq!(shown("2019-04-01"), "April 1, 2019");
    assert_eq!(shown("2020-02-29"), "February 29, 2020");
    assert_eq!(shown("2019-4-1"), "2,014");
    assert_eq!(shown("2019-04-01T15:30"), "April 1, 2019 at 3:30 PM");
    assert_eq!(
        shown("2019-04-01T15:30:00.5Z as iso8601"),
        "2019-04-01T15:30:00.5Z"
    );
}

#[test]
fn a_multiplier_literal_needs_nothing_after_it() {
    assert_eq!(shown("1.5x"), "1.5x");
    assert_eq!(shown("2x * 3"), "6");
    assert_eq!(shown("2x * 3x"), "6x");
    assert_eq!(shown("3 x 4"), "12");
    assert_eq!(shown("3x4"), "12");
    assert_eq!(shown("3 x (2)"), "6");
    fails("0x");
}

#[test]
fn frame_words_are_units_first() {
    assert_eq!(shown("1.5 frame in frames"), "1.5 frames");
    assert_eq!(shown("30 fps * 10 seconds"), "300 frames");
    assert_eq!(shown("300 frames / 30 fps"), "10 s");
    fails("10 seconds as frames");
    fails("01:00:00:24");
    fails("01:00:00:00 at 0 fps");
    fails("01:00:00:00 at 29.97 fps");
}

#[test]
fn every_timecode_on_a_line_shares_one_frame_rate() {
    assert_eq!(shown("00:00:01:12 + 00:00:00:15 at 30 fps"), "00:00:01:27");
    assert_eq!(shown("00:00:00:12 + 00:00:00:15"), "00:00:01:03");
    assert_eq!(shown("10:00:00:00 - 00:00:00:01"), "09:59:59:23");
}
