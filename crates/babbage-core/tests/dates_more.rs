//! Edge cases of the date forms: which year a yearless date means, holiday spellings, weekday
//! abbreviations, the words around `to date`, `as iso8601` and `at 1.5x`, and every refusal.

use babbage_core::calendar::{
    Civil, civil_from_days, days_from_civil, nth_weekday, workdays_between,
};
use babbage_core::datefns::{DateFn, call, midpoint};
use babbage_core::settings::{Settings, scoped};
use babbage_core::units::lookup;
use babbage_core::{Error, FormatOptions, Number, Value, evaluate_with, format_value};

/// Saturday 2024-06-15 12:00 UTC.
const NOW: i64 = 1_718_452_800;

fn run_at(now: i64, line: &str) -> Result<String, Error> {
    let s = Settings::at(now);
    scoped(&s, || {
        let v = evaluate_with(line, &s)?;
        Ok(format_value(&v, &FormatOptions::default()).text)
    })
}

fn shown(line: &str) -> String {
    run_at(NOW, line).unwrap_or_else(|e| panic!("{line}: {e}"))
}

fn fails(line: &str) {
    assert!(
        run_at(NOW, line).is_err(),
        "{line} should fail, got {:?}",
        run_at(NOW, line)
    );
}

fn all(cases: &[(&str, &str)]) {
    for (line, want) in cases {
        assert_eq!(shown(line), *want, "{line}");
    }
}

#[test]
fn calendar_helpers_at_their_edges() {
    let c = |year, month, day| Civil { year, month, day };
    // The fifth weekday of a month: it exists in March 2024, and spills into March from February 2023.
    assert_eq!(nth_weekday(2024, 3, 4, 5), c(2024, 3, 29));
    assert_eq!(nth_weekday(2023, 2, 3, 5), c(2023, 2, 23));
    assert_eq!(nth_weekday(2024, 2, 3, 5), c(2024, 2, 29));
    assert_eq!(nth_weekday(2024, 11, 3, 4), c(2024, 11, 28));
    // The last day of every 400-year era is a February 29th.
    for year in (-800..=2800).step_by(400) {
        let leap_day = c(year, 2, 29);
        assert_eq!(
            civil_from_days(days_from_civil(leap_day)),
            leap_day,
            "{year}"
        );
    }
    // A span of a million million days is counted by weeks, not one day at a time.
    assert_eq!(workdays_between(1, 1_000_000_000_000), 714_285_714_285);
    assert_eq!(workdays_between(1_000_000_000_000, 1), -714_285_714_285);
    // The same day at both ends is no days, and it must not recurse.
    assert_eq!(workdays_between(5, 5), 0);
    assert_eq!(workdays_between(-5, -5), 0);
}

#[test]
fn a_word_that_is_not_a_month_is_never_one() {
    for line in ["pizza 5", "Jane 5", "marchx 5", "ago 5", "Maybe 5, 2024"] {
        fails(line);
    }
    all(&[
        ("Sept 5, 2024", "September 5"),
        ("sep 5, 2024", "September 5"),
        ("Sept 5, 2023", "September 5, 2023"),
        ("MAR 5, 2024", "March 5"),
    ]);
}

#[test]
fn weekday_names_and_their_abbreviations() {
    all(&[
        ("next tue", "June 18"),
        ("next tues", "June 18"),
        ("next tuesday", "June 18"),
        ("next wed", "June 19"),
        ("next weds", "June 19"),
        ("next thu", "June 20"),
        ("next thur", "June 20"),
        ("next thurs", "June 20"),
        ("next fri", "June 21"),
        ("this saturday", "June 15"),
        ("next saturday", "June 22"),
        ("last saturday", "June 8"),
        ("last sun", "June 9"),
        ("this mon", "June 17"),
    ]);
    for line in [
        "next tu",
        "next wedn",
        "next thursd",
        "next tuesda",
        "next mond",
        "next su",
        "next x",
    ] {
        fails(line);
    }
}

#[test]
fn until_looks_ahead_and_since_looks_back_and_today_counts_as_both() {
    all(&[
        ("days until June 15", "0 day"),
        ("days since June 15", "0 day"),
        ("days until June 14", "364 day"),
        ("days until June 16", "1 day"),
        ("days since June 16", "365 day"),
        ("days since June 14", "1 day"),
        ("days till June 16", "1 day"),
        ("days until Christmas", "193 day"),
        ("days since Christmas", "173 day"),
        ("days until Easter", "309 day"),
        ("days since Easter", "76 day"),
        ("days until Thanksgiving", "166 day"),
        ("days since Thanksgiving", "205 day"),
        ("days until Black Friday", "167 day"),
        ("days since Black Friday", "204 day"),
        ("days until Halloween", "138 day"),
        ("days since Halloween", "228 day"),
        ("days until New Year", "200 day"),
        ("days since New Year", "166 day"),
        ("days until Valentine's Day", "244 day"),
        ("days since Valentine's Day", "122 day"),
        ("days until February 29", "1,354 day"),
        ("days since February 29", "107 day"),
    ]);
}

#[test]
fn a_yearless_date_means_the_closest_one_unless_the_line_says_which() {
    let at = |now, line: &str| run_at(now, line).unwrap_or_else(|e| panic!("{line}: {e}"));
    all(&[
        ("December 25 as yyyy", "2023"),
        ("December 1 as yyyy", "2024"),
        ("January 5 as yyyy", "2024"),
    ]);
    // Late December: the next January is closer than the last one.
    assert_eq!(at(1_734_696_000, "January 5 as yyyy"), "2025");
    assert_eq!(at(1_734_696_000, "December 25 as yyyy"), "2024");
    // Early January: the last December is closer.
    assert_eq!(at(1_704_283_200, "December 28 as yyyy"), "2023");
    // A leap day is found in whichever year has one, up to four years either way.
    assert_eq!(at(1_835_265_600, "days since February 29"), "1,459 day");
    assert_eq!(at(1_709_294_400, "days until February 29"), "1,460 day");
    assert_eq!(at(1_709_294_400, "February 29 as yyyy"), "2024");
}

#[test]
fn every_spelling_of_a_holiday() {
    all(&[
        ("Christmas 2025", "December 25, 2025"),
        ("christmas day 2025", "December 25, 2025"),
        ("Halloween 2025", "October 31, 2025"),
        ("New Year 2025", "January 1, 2025"),
        ("new years 2025", "January 1, 2025"),
        ("New Year's 2025", "January 1, 2025"),
        ("new year day 2025", "January 1, 2025"),
        ("new years day 2025", "January 1, 2025"),
        ("New Year's Day 2025", "January 1, 2025"),
        ("Valentine's Day 2025", "February 14, 2025"),
        ("valentines day 2025", "February 14, 2025"),
        ("Valentine's 2025", "February 14, 2025"),
        ("valentines 2025", "February 14, 2025"),
        ("Easter 2027", "March 28, 2027"),
        ("Thanksgiving 2025", "November 27, 2025"),
        ("Black Friday 2025", "November 28, 2025"),
    ]);
}

#[test]
fn ordinal_suffixes_and_the_years_a_date_may_carry() {
    all(&[
        ("June 1st, 2019", "June 1, 2019"),
        ("June 2nd 2019", "June 2, 2019"),
        ("June 3rd 2019", "June 3, 2019"),
        ("June 4th, 2019", "June 4, 2019"),
        ("March 5, 1000", "March 5, 1000"),
        ("March 5, 9999", "March 5, 9999"),
        ("5 March 2019", "March 5, 2019"),
        ("February 29, 2020", "February 29, 2020"),
    ]);
    for line in [
        "March 5, 999",
        "March 5, 10000",
        "February 29, 2019",
        "February 30",
        "April 31, 2020",
        "June 0",
    ] {
        fails(line);
    }
}

#[test]
fn today_tomorrow_yesterday_and_now() {
    all(&[
        ("today as EEEE", "Saturday"),
        ("tomorrow as EEEE", "Sunday"),
        ("yesterday as EEEE", "Friday"),
        ("tomorrow - yesterday", "2 days"),
        ("now to timestamp", "1718452800"),
        ("now as HH:mm", "12:00"),
        ("now as h:mm a", "12:00 PM"),
        ("now as hh:mm a", "12:00 PM"),
        ("now as m:ss", "0:00"),
    ]);
    assert_eq!(run_at(NOW + 45, "now as HH:mm:ss").unwrap(), "12:00:45");
    assert_eq!(run_at(NOW + 86_400, "tomorrow as EEEE").unwrap(), "Monday");
}

#[test]
fn the_words_after_a_long_expression_still_find_their_place() {
    all(&[
        ("June 5, 2019 + 3 days to date", "June 8, 2019"),
        (
            "(1_000_000_000 + 700_000_000) to date",
            "November 14, 2023 at 10:13:20 pm",
        ),
        (
            "(1_700_000_000 * 1_000) to date",
            "November 14, 2023 at 10:13:20 pm",
        ),
        ("June 5, 2019 + 3 days to timestamp", "1559952000"),
        ("June 5, 2019 + 3 days as iso8601", "2019-06-08T00:00:00Z"),
        ("June 5, 2019 + 3 days as EEEE", "Saturday"),
        (
            "June 5, 2019 + 3 days as EEEE, MMM d, yyyy",
            "Saturday, Jun 8, 2019",
        ),
        ("3 weeks + 2 days after June 5, 2019", "June 28, 2019"),
        (
            "2 days + 1 hour before June 5, 2019",
            "June 2, 2019 at 11:00 pm",
        ),
        ("5 days + 2 hours ago", "June 10 at 10:00 am"),
        ("3 hours + 30 minutes from now", "June 15 at 3:30 pm"),
        (
            "1 hour 30 minutes + 5 minutes at 1.5x",
            "1 hour 3 minutes 20 seconds",
        ),
        (
            "time saved 1 hour 30 minutes + 5 minutes at 1.5x",
            "31 minutes 40 seconds",
        ),
        (
            "300 frames + 60 frames at 30 fps as timecode",
            "00:00:12:00",
        ),
        ("00:00:01:00 + 00:00:01:00 at 30 fps as frames", "60 frame"),
        ("10 seconds + 5 seconds at 30 fps as frames", "450 frame"),
        ("5 minutes + 5 minutes to laptime", "00:10:00"),
    ]);
}

#[test]
fn a_huge_number_of_workdays_answers_at_once_or_is_refused() {
    // Values worked out in Python from the calendar (a Monday plus whole weeks of seven days).
    all(&[
        ("June 3, 2024 + 1_000_000 workdays", "June 29, 5857"),
        ("June 3, 2024 + 1_000_001 workdays", "June 30, 5857"),
        ("June 3, 2024 - 500_000 workdays", "November 21, 107"),
        ("June 3, 2024 - 500_001 workdays", "November 18, 107"),
    ]);
    for line in [
        "June 5 + 5_000_000_000_000 workdays",
        "June 5 - 99_999_999_999_999 workdays",
    ] {
        fails(line);
    }
    // Large but allowed: it must come back immediately, which the suite's runtime enforces.
    assert!(run_at(NOW, "June 5 + 99999999999 workdays").is_ok());
    assert!(run_at(NOW, "June 5 - 99999999999 workdays").is_ok());
}

#[test]
fn a_span_of_no_time_has_no_workdays_and_does_not_crash() {
    all(&[
        ("workdays between June 5 and June 5", "0 workday"),
        ("work hours between June 5 and June 5", "0 h"),
        ("workdays from June 5 to June 5", "0 workday"),
        ("workdays in 0 days", "0 workday"),
        ("workdays in 2024", "0 workday"),
        ("workdays in 1 day", "0 workday"),
        ("workdays in 2 days", "0 workday"),
        ("workdays in 3 days", "1 workday"),
        ("workdays between June 17 and June 14", "-1 workday"),
    ]);
}

#[test]
fn a_form_missing_a_side_is_an_error_not_a_guess() {
    for line in [
        "days between June 5",
        "days between and June 5",
        "days between June 5 and",
        "workdays between June 5",
        "workdays between and June 5",
        "work hours between June 5 and",
        "June 5 through",
        "through June 5",
        "workdays from to June 5",
        "workdays from June 5 to",
        "days until",
        "days since",
        "day of the week on",
        "weekday on",
        "week number on",
        "day number of",
        "to date",
        "as iso8601",
        "to timestamp",
        "as EEEE",
        "at 1.5x",
        "time saved at 1.5x",
        "as timecode",
        "as frames",
        "6pm Sydney Chicago",
        "6pm in",
        "time in",
        "date in",
        "time difference between Sydney",
        "time difference between and Sydney",
    ] {
        fails(line);
    }
}

#[test]
fn days_in_a_period() {
    all(&[
        ("days in Q1", "91 day"),
        ("days in Q2", "91 day"),
        ("days in Q3", "92 day"),
        ("days in Q4", "92 day"),
        ("days in Q2 2023", "91 day"),
        ("days in Q1 2023", "90 day"),
        ("days in February", "29 day"),
        ("days in February 2023", "28 day"),
        ("days in 2023", "365 day"),
        ("days in 2024", "366 day"),
        ("days in 1900", "365 day"),
        ("days in 2000", "366 day"),
    ]);
    for line in [
        "days in Q5 2024",
        "days in Q0",
        "days in 2024.5",
        "days in 24.5",
        "days in Q2 2024 2025",
        "days in",
        "days in Smarch",
    ] {
        fails(line);
    }
}

#[test]
fn offsets_and_months_written_with_a_number() {
    all(&[
        ("noon GMT+5:30 in UTC", "6:30 am"),
        ("noon GMT+ 5 in UTC", "7:00 am"),
        ("noon UTC-3 in UTC", "3:00 pm"),
        ("workdays in June 2024", "20 workday"),
        ("workdays in June", "20 workday"),
        ("work hours in June", "160 h"),
        ("work hours in February 2023", "160 h"),
    ]);
    for line in [
        "noon GMT+8.5 in UTC",
        "noon GMT+19 in UTC",
        "noon GMT+5:99 in UTC",
        "work hours in June 2024.5",
        "workdays in Smarch",
    ] {
        fails(line);
    }
}

fn plain(n: i64) -> Value {
    Value::plain(Number::from_i64(n))
}

fn qty(n: i64, unit: &str) -> Value {
    Value::quantity(Number::from_i64(n), lookup(unit).unwrap()).unwrap()
}

fn date(days: i64) -> Value {
    Value::date(Number::from_i64(days))
}

#[test]
fn date_functions_refuse_what_they_cannot_use() {
    let s = Settings::at(NOW);
    scoped(&s, || {
        let bad = |f: DateFn, args: &[Value]| {
            assert!(call(f, args).is_err(), "{f:?} {args:?} should fail");
        };
        // a whole number is needed for a year or a quarter
        bad(
            DateFn::DaysInYear,
            &[Value::plain(Number::from_decimal_str("2024.5").unwrap())],
        );
        bad(DateFn::DaysInYear, &[qty(2024, "m")]);
        bad(DateFn::DaysInYear, &[date(0)]);
        bad(DateFn::DaysInQuarter, &[plain(0), plain(2024)]);
        bad(DateFn::DaysInQuarter, &[plain(5), plain(2024)]);
        bad(DateFn::DaysInQuarter, &[qty(2, "m"), plain(2024)]);
        // dates and times of day
        bad(DateFn::DateAt, &[date(0), plain(5)]);
        bad(DateFn::DateAt, &[date(0), date(5)]);
        bad(DateFn::Weekday, &[plain(5)]);
        bad(DateFn::Weekday, &[plain(1), plain(2)]);
        assert!(midpoint(&plain(1), &date(2)).is_err());
        assert!(midpoint(&date(1), &plain(2)).is_err());
        assert!(midpoint(&plain(1), &plain(2)).is_err());
        // intervals
        bad(
            DateFn::Interval,
            &[date(1), Value::clock(Number::from_i64(5))],
        );
        bad(DateFn::Interval, &[plain(1), plain(2)]);
        // timestamps
        bad(DateFn::ToDate, &[qty(5, "km")]);
        bad(DateFn::ToDate, &[Value::clock(Number::from_i64(5))]);
        bad(DateFn::ToTimestamp, &[plain(5)]);
        bad(DateFn::AsIso, &[plain(5)]);
        // frame rates
        let ok_length = qty(10, "s");
        for rate in [
            plain(0),
            plain(-24),
            Value::plain(Number::from_decimal_str("29.97").unwrap()),
            qty(24, "s"),
            date(24),
        ] {
            bad(DateFn::AsTimecodeAt, &[ok_length.clone(), rate.clone()]);
            bad(DateFn::AsFramesAt, &[ok_length.clone(), rate]);
        }
        bad(DateFn::AsTimecode, &[plain(10)]);
        bad(DateFn::AsTimecode, &[qty(10, "m")]);
        bad(DateFn::AsTimecode, &[date(10)]);
        bad(DateFn::AsFrames, std::slice::from_ref(&ok_length));
        bad(DateFn::AsFramesAt, &[plain(10), plain(24)]);
        bad(DateFn::AsFramesAt, &[qty(10, "m"), plain(24)]);
        // playback
        bad(DateFn::Playback, &[qty(10, "s"), plain(2)]);
        bad(
            DateFn::Playback,
            &[plain(10), Value::multiplier(Number::from_i64(2))],
        );
        bad(
            DateFn::Playback,
            &[qty(10, "m"), Value::multiplier(Number::from_i64(2))],
        );
        bad(
            DateFn::TimeSaved,
            &[qty(10, "m"), Value::multiplier(Number::from_i64(2))],
        );
        // and the working cases beside them
        assert!(call(DateFn::AsTimecodeAt, &[ok_length.clone(), plain(24)]).is_ok());
        assert!(call(DateFn::AsFramesAt, &[ok_length, plain(24)]).is_ok());
        assert!(
            call(
                DateFn::Playback,
                &[qty(10, "s"), Value::multiplier(Number::from_i64(2))]
            )
            .is_ok()
        );
    });
}

#[test]
fn date_to_date_intervals_and_midpoints_by_function() {
    let s = Settings::at(NOW);
    scoped(&s, || {
        let v = call(DateFn::Interval, &[date(10), date(14)]).unwrap();
        assert_eq!(format_value(&v, &FormatOptions::default()).text, "4 days");
        let v = call(DateFn::Interval, &[date(14), date(10)]).unwrap();
        assert_eq!(format_value(&v, &FormatOptions::default()).text, "-4 days");
        let m = midpoint(&date(10), &date(14)).unwrap();
        assert_eq!(m.number(), &Number::from_i64(12));
    });
    assert_eq!(shown("March 1 to March 5"), "4 days");
    assert_eq!(shown("March 5 to March 1"), "-4 days");
    assert_eq!(shown("midpoint between March 12 and April 5"), "March 24");
}
