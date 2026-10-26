//! Random sentences built from every word the date, time, zone and unit forms know. Whatever the engine
//! answers, it must answer: no panic, no stack overflow, no hang, and a sheet of them must survive too.

use std::time::{Duration, Instant};

use babbage_core::settings::{Settings, scoped};
use babbage_core::zones::{Zone, ZoneSource};
use babbage_core::{FormatOptions, SheetOptions, evaluate_sheet, evaluate_with, format_value};
use proptest::prelude::*;

/// Saturday 2024-06-15 12:00 UTC.
const NOW: i64 = 1_718_452_800;

const WORDS: &[&str] = &[
    // numbers and clocks
    "0",
    "1",
    "2",
    "7",
    "24",
    "30",
    "60",
    "2024",
    "1999",
    "1.5",
    "0.5",
    "1_000",
    "100000",
    "99999999999",
    "1700000000000",
    "5pm",
    "17:30",
    "noon",
    "midnight",
    "12am",
    "03:04:05",
    "00:00:01.5",
    "01:02:03:04",
    "9:05am",
    "1.5x",
    "2x",
    "0x",
    // dates
    "today",
    "tomorrow",
    "yesterday",
    "now",
    "June",
    "February",
    "Sept",
    "5",
    "29",
    "31",
    "June 5, 2019",
    "March 14",
    "2019-04-01",
    "2019-04-01T15:30:00Z",
    "Christmas",
    "Easter",
    "Easter 2027",
    "New Year",
    "Valentine's Day",
    "Black Friday",
    "Thanksgiving",
    "Halloween",
    "next friday",
    "last tues",
    "this sunday",
    "1st",
    "2nd",
    "3rd",
    "4th",
    // units and unit-like words
    "day",
    "days",
    "week",
    "weeks",
    "month",
    "months",
    "year",
    "years",
    "hour",
    "hours",
    "minute",
    "minutes",
    "s",
    "ms",
    "workday",
    "workdays",
    "work",
    "hours",
    "fps",
    "frames",
    "frame",
    "m",
    "km",
    "pm",
    "am",
    "st",
    "$",
    "$5",
    "A$5",
    "USD",
    "%",
    // connectors and form words
    "in",
    "to",
    "as",
    "at",
    "on",
    "of",
    "from",
    "until",
    "till",
    "since",
    "between",
    "and",
    "&",
    "through",
    "after",
    "before",
    "ago",
    "the",
    "a",
    "an",
    "is",
    "what",
    "per",
    "over",
    "off",
    "time",
    "date",
    "difference",
    "current",
    "timestamp",
    "iso8601",
    "timespan",
    "laptime",
    "timecode",
    "weekday",
    "number",
    "day of the week",
    "week number",
    "day number",
    "days in",
    "Q1",
    "Q2 2024",
    "midpoint",
    "saved",
    "saved at",
    "half",
    "average",
    "total",
    "sum",
    "mod",
    "round",
    "dp",
    "nearest",
    // formats
    "EEEE",
    "yyyy",
    "MMM d",
    "HH:mm",
    "hh:mm a",
    "EEEE, MMM d, yyyy",
    // places
    "Sydney",
    "Chicago",
    "New York",
    "Tokyo",
    "Paris",
    "Casablanca",
    "PST",
    "EST",
    "GMT",
    "GMT+8",
    "UTC-5:30",
    "Narnia",
    "Germany",
    // operators and punctuation
    "+",
    "-",
    "*",
    "/",
    "^",
    "(",
    ")",
    ",",
    ";",
    "=",
    "<",
    ">",
    "==",
    "!",
    "x",
    "×",
    "÷",
    "'",
    "\"",
    ".",
    ":",
    "e",
    "k",
    "M",
];

fn sentence() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(WORDS), 1..9).prop_map(|words| words.join(" "))
}

fn settings(zone: Option<&str>, source: ZoneSource) -> Settings {
    let mut s = Settings {
        zone_source: source,
        ..Settings::at(NOW)
    };
    s.local_zone = zone.and_then(|name| scoped(&s, || Zone::named(name)));
    s
}

fn answer(s: &Settings, line: &str) -> String {
    scoped(s, || match evaluate_with(line, s) {
        Ok(v) => format_value(&v, &FormatOptions::default()).text,
        Err(e) => e.to_string(),
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 30_000, ..ProptestConfig::default() })]

    #[test]
    fn any_sentence_gets_an_answer_in_good_time(line in sentence()) {
        let started = Instant::now();
        let plain = settings(None, ZoneSource::Bundled);
        let _ = answer(&plain, &line);
        let sydney = settings(Some("Australia/Sydney"), ZoneSource::Bundled);
        let _ = answer(&sydney, &line);
        prop_assert!(started.elapsed() < Duration::from_secs(5), "{line} took {:?}", started.elapsed());
    }

    #[test]
    fn a_sheet_of_sentences_never_crashes(lines in prop::collection::vec(sentence(), 1..6)) {
        let s = settings(None, ZoneSource::Bundled);
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        let opts = SheetOptions { settings: s.clone(), ..SheetOptions::default() };
        let results = scoped(&s, || evaluate_sheet(&refs, &opts));
        prop_assert_eq!(results.len(), lines.len());
    }
}
