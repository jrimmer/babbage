//! Holidays that follow the Hebrew, Islamic and Chinese calendars, as days since 1970-01-01.
//!
//! Hanukkah comes from the Hebrew calendar rules (Dershowitz and Reingold), Ramadan from the arithmetic Islamic
//! calendar, and Chinese New Year from a table (`chinese_new_year.rs`). The reference app gives the evening a
//! festival begins and the day it ends: `Hanukkah 2026` is December 4, though the first full day is the 5th.

use crate::calendar::{self, Civil};
use crate::chinese_new_year as table;

/// The day numbering of the Hebrew calendar counts from this fixed day, and Unix time from this one.
const HEBREW_EPOCH: i64 = -1_373_427;
const UNIX_EPOCH_FIXED_DAY: i64 = 719_163;

fn hebrew_elapsed_days(year: i64) -> i64 {
    let months = (235 * year - 234).div_euclid(19);
    let parts = 12_084 + 13_753 * months;
    let days = 29 * months + parts.div_euclid(25_920);
    if (3 * (days + 1)).rem_euclid(7) < 3 {
        days + 1
    } else {
        days
    }
}

fn hebrew_new_year(year: i64) -> i64 {
    let (before, this, after) = (
        hebrew_elapsed_days(year - 1),
        hebrew_elapsed_days(year),
        hebrew_elapsed_days(year + 1),
    );
    let delay = if after - this == 356 {
        2
    } else if this - before == 382 {
        1
    } else {
        0
    };
    HEBREW_EPOCH + this + delay
}

/// The first full day of Hanukkah (25 Kislev) in the Hebrew year that begins in the Gregorian `year`.
fn hanukkah_first_day(year: i64) -> i64 {
    let hebrew_year = year + 3761;
    let start = hebrew_new_year(hebrew_year);
    let length = hebrew_new_year(hebrew_year + 1) - start;
    // Tishri has 30 days, Heshvan 30 in a complete year and 29 otherwise, and Kislev starts after them.
    let heshvan = if length % 10 == 5 { 30 } else { 29 };
    start + 30 + heshvan + 24 - UNIX_EPOCH_FIXED_DAY
}

/// The evening Hanukkah begins.
pub fn hanukkah(year: i64) -> Option<i64> {
    Some(hanukkah_first_day(year) - 1)
}

/// The last day of Hanukkah, eight days after the first.
pub fn end_of_hanukkah(year: i64) -> Option<i64> {
    Some(hanukkah_first_day(year) + 7)
}

/// Days since 1970-01-01 of a day in the arithmetic Islamic calendar.
fn islamic_day(year: i64, month: i64, day: i64) -> i64 {
    // ceil(29.5 * (month - 1)) is 30 * (month - 1) - (month - 1) / 2, and the epoch is 16 July 622 (Julian).
    let months = 29 * (month - 1) + month / 2;
    day + months + (year - 1) * 354 + (3 + 11 * year).div_euclid(30) - 492_148 - 1
}

/// The first Ramadan that starts in the Gregorian `year`, as (first day, first day of Shawwal).
fn ramadan_in(year: i64) -> Option<(i64, i64)> {
    let near = (year - 622) * 33 / 32;
    (near - 3..=near + 3).find_map(|hijri| {
        let first = islamic_day(hijri, 9, 1);
        (calendar::civil_from_days(first).year == year).then(|| (first, islamic_day(hijri, 10, 1)))
    })
}

/// The evening before the first day of Ramadan.
pub fn ramadan(year: i64) -> Option<i64> {
    ramadan_in(year).map(|(first, _)| first - 1)
}

/// The last day of Ramadan, the day before Shawwal begins.
pub fn end_of_ramadan(year: i64) -> Option<i64> {
    ramadan_in(year).map(|(_, shawwal)| shawwal - 1)
}

/// Chinese New Year, for the years the table covers.
pub fn chinese_new_year(year: i64) -> Option<i64> {
    let index = usize::try_from(year - table::FIRST_YEAR).ok()?;
    let after = *table::DAYS_AFTER_JANUARY_21.get(index)?;
    Some(
        calendar::days_from_civil(Civil {
            year,
            month: 1,
            day: 21,
        }) + i64::from(after),
    )
}

pub fn chinese_new_years_eve(year: i64) -> Option<i64> {
    chinese_new_year(year).map(|day| day - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn civil(day: i64) -> (i64, u32, u32) {
        let c = calendar::civil_from_days(day);
        (c.year, c.month, c.day)
    }

    #[test]
    fn the_owners_three_answers() {
        assert_eq!(civil(hanukkah(2026).unwrap()), (2026, 12, 4));
        assert_eq!(civil(chinese_new_year(2025).unwrap()), (2025, 1, 29));
        assert_eq!(civil(ramadan(2026).unwrap()), (2026, 2, 17));
    }

    #[test]
    fn the_festivals_end_where_the_calendars_say() {
        assert_eq!(civil(end_of_hanukkah(2026).unwrap()), (2026, 12, 12));
        assert_eq!(civil(end_of_ramadan(2026).unwrap()), (2026, 3, 19));
        assert_eq!(civil(chinese_new_years_eve(2025).unwrap()), (2025, 1, 28));
    }

    #[test]
    fn the_owners_chinese_new_year_2100_comes_from_the_sky_not_the_table() {
        assert_eq!(civil(chinese_new_year(2100).unwrap()), (2100, 2, 9));
    }

    #[test]
    fn the_chinese_table_ends_with_its_years() {
        assert!(chinese_new_year(1899).is_none());
        assert!(chinese_new_year(1900).is_some());
        assert!(chinese_new_year(2299).is_some());
        assert!(chinese_new_year(2300).is_none());
    }
}
