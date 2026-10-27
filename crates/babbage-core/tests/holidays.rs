//! Days off that workday arithmetic skips, and the user's own names for places.

use babbage_core::calendar::{add_workdays_off, weekday, workdays_between_off};
use babbage_core::settings::{Holiday, Settings, parse_custom_zones, parse_holidays, scoped};
use babbage_core::zones::Zone;
use babbage_core::{FormatOptions, evaluate, format_value};

fn with_holidays(list: &str) -> Settings {
    Settings {
        holidays: parse_holidays(list),
        ..Settings::at(1_793_051_580)
    }
}

fn shown(s: &Settings, line: &str) -> String {
    scoped(s, || {
        format_value(
            &evaluate(line).unwrap_or_else(|e| panic!("{line}: {e}")),
            &FormatOptions::default(),
        )
        .text
    })
}

#[test]
fn holidays_are_read_from_a_list_of_dates() {
    assert_eq!(
        parse_holidays("2026-12-25; 12-26, nonsense; 2026-02-30; 02-29;;"),
        vec![
            Holiday::parse("2026-12-25").unwrap(),
            Holiday::Yearly { month: 12, day: 26 },
            Holiday::Yearly { month: 2, day: 29 },
        ]
    );
    assert_eq!(Holiday::parse("13-01"), None);
    assert_eq!(Holiday::parse("1-1-1-1"), None);
}

/// Day by day: the definition `add_workdays_off` and `workdays_between_off` must agree with.
fn slow_add(s: &Settings, from: i64, n: i64) -> i64 {
    let off = |d: i64| {
        weekday(d) >= 5 || !scoped(s, || babbage_core::settings::holidays_in(d, d + 1)).is_empty()
    };
    let (step, mut left) = if n > 0 { (1, n) } else { (-1, -n) };
    let mut day = from;
    while left > 0 {
        day += step;
        if !off(day) {
            left -= 1;
        }
    }
    day
}

fn slow_between(s: &Settings, from: i64, to: i64) -> i64 {
    let (lo, hi) = (from.min(to), from.max(to));
    let n = (lo..hi)
        .filter(|d| {
            weekday(*d) < 5
                && scoped(s, || babbage_core::settings::holidays_in(*d, d + 1)).is_empty()
        })
        .count() as i64;
    if to < from { -n } else { n }
}

#[test]
fn workday_arithmetic_skips_holidays_exactly() {
    let s = with_holidays("2026-12-25; 2026-12-28; 01-01; 07-04; 2027-01-04; 2027-01-05");
    let start = 20_000; // 2024-10-04, a Friday
    for from in start..start + 800 {
        for n in [-40, -11, -6, -5, -1, 1, 2, 5, 6, 9, 17, 33, 60] {
            let got = scoped(&s, || add_workdays_off(from, n));
            assert_eq!(got, slow_add(&s, from, n), "{from} + {n}");
            assert_eq!(
                weekday(got) < 5,
                weekday(got) < 5
                    && scoped(&s, || babbage_core::settings::holidays_in(got, got + 1)).is_empty(),
                "landed on a day off: {got}"
            );
        }
        for len in [0, 1, 6, 8, 20, 90, 400] {
            let to = from + len;
            assert_eq!(
                scoped(&s, || workdays_between_off(from, to)),
                slow_between(&s, from, to),
                "{from}..{to}"
            );
            assert_eq!(
                scoped(&s, || workdays_between_off(to, from)),
                slow_between(&s, to, from)
            );
        }
    }
}

#[test]
fn with_no_holidays_nothing_changes_and_huge_counts_are_instant() {
    let none = Settings::default();
    scoped(&none, || {
        assert_eq!(
            add_workdays_off(20_000, 5_000_000_000_000),
            babbage_core::calendar::add_workdays(20_000, 5_000_000_000_000)
        );
        assert_eq!(
            workdays_between_off(0, 1_000_000),
            babbage_core::calendar::workdays_between(0, 1_000_000)
        );
    });
    // A yearly holiday still ends: a million workdays is a few thousand years of holidays.
    let s = with_holidays("12-25");
    let far = scoped(&s, || add_workdays_off(20_000, 1_000_000));
    assert!(far > 20_000 + 1_400_000, "{far}");
}

#[test]
fn lines_skip_the_holidays() {
    let s = with_holidays("12-25");
    assert_eq!(shown(&s, "December 24, 2026 + 3 workdays"), "December 30");
    assert_eq!(
        shown(
            &Settings::at(1_793_051_580),
            "December 24, 2026 + 3 workdays"
        ),
        "December 29"
    );
    assert_eq!(
        shown(
            &s,
            "workdays between December 22, 2026 and December 29, 2026"
        ),
        "4 workdays"
    );
    assert_eq!(
        shown(
            &Settings::at(1_793_051_580),
            "workdays between December 22, 2026 and December 29, 2026"
        ),
        "5 workdays"
    );
}

#[test]
fn custom_zone_names_stand_for_their_zones() {
    assert_eq!(
        parse_custom_zones("HQ = Asia/Tokyo; Lab=Europe/Berlin; bad = Not/AZone; = UTC; x"),
        vec![
            ("hq".to_string(), "Asia/Tokyo".to_string()),
            ("lab".to_string(), "Europe/Berlin".to_string())
        ]
    );
    let s = Settings {
        custom_zones: parse_custom_zones("HQ = Asia/Tokyo; Head Office = Europe/Berlin"),
        local_zone: Zone::named("America/Los_Angeles"),
        ..Settings::at(1_793_051_580)
    };
    assert_eq!(shown(&s, "noon HQ in UTC"), "03:00");
    assert_eq!(shown(&s, "noon Head Office in UTC"), "11:00");
    // Without the name, `HQ` is just a word and the line is not a conversion.
    assert_ne!(shown(&Settings::at(1_793_051_580), "noon in UTC"), "03:00");
}
