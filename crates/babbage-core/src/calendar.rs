//! The proleptic Gregorian calendar in exact integer arithmetic.
//!
//! Days are counted from 1970-01-01. The conversions are Howard Hinnant's civil-date algorithms, which are
//! exact over the whole range of `i64` years that fit the arithmetic.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Civil {
    pub year: i64,
    pub month: u32,
    pub day: u32,
}

pub fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if is_leap(year) => 29,
        _ => 28,
    }
}

/// True when the day exists: month 1 to 12 and a day inside that month.
pub fn is_valid(c: Civil) -> bool {
    (1..=12).contains(&c.month) && c.day >= 1 && c.day <= days_in_month(c.year, c.month)
}

pub fn days_from_civil(c: Civil) -> i64 {
    let y = if c.month <= 2 { c.year - 1 } else { c.year };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = i64::from(c.month);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(c.day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn civil_from_days(z: i64) -> Civil {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    Civil { year, month, day }
}

/// Monday is 0 and Sunday is 6.
pub fn weekday(days: i64) -> u32 {
    // 1970-01-01 was a Thursday.
    (days + 3).rem_euclid(7) as u32
}

pub const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
pub const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Adds calendar months, keeping the day of the month or the last day of a shorter month:
/// 31 January plus a month is the last day of February.
pub fn add_months(c: Civil, months: i64) -> Civil {
    let total = c.year * 12 + i64::from(c.month) - 1 + months;
    let year = total.div_euclid(12);
    let month = (total.rem_euclid(12) + 1) as u32;
    Civil {
        year,
        month,
        day: c.day.min(days_in_month(year, month)),
    }
}

pub fn day_of_year(c: Civil) -> u32 {
    (days_from_civil(c)
        - days_from_civil(Civil {
            year: c.year,
            month: 1,
            day: 1,
        })) as u32
        + 1
}

/// The ISO 8601 week number (1 to 53) and the year it belongs to.
pub fn iso_week(c: Civil) -> (i64, u32) {
    let days = days_from_civil(c);
    // The Thursday of this week decides which year the week belongs to.
    let thursday = days - i64::from(weekday(days)) + 3;
    let year = civil_from_days(thursday).year;
    let first = days_from_civil(Civil {
        year,
        month: 1,
        day: 1,
    });
    (year, ((thursday - first) / 7) as u32 + 1)
}

/// Western Easter Sunday (the anonymous Gregorian algorithm).
pub fn easter(year: i64) -> Civil {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = ((h + l - 7 * m + 114) / 31) as u32;
    let day = ((h + l - 7 * m + 114) % 31 + 1) as u32;
    Civil { year, month, day }
}

/// The `n`th (1 to 4, or 5 for the last) given weekday of a month, such as the 4th Thursday of November.
pub fn nth_weekday(year: i64, month: u32, weekday_index: u32, n: u32) -> Civil {
    let first = days_from_civil(Civil {
        year,
        month,
        day: 1,
    });
    let offset = (i64::from(weekday_index) - i64::from(weekday(first))).rem_euclid(7);
    let mut day = first + offset + 7 * i64::from(n - 1);
    if n == 5 && civil_from_days(day).month != month {
        day -= 7;
    }
    civil_from_days(day)
}

/// Monday to Friday between two days, counting the first and not the last (a half-open range).
pub fn workdays_between(from: i64, to: i64) -> i64 {
    if to <= from {
        return -workdays_between(to, from);
    }
    let full_weeks = (to - from) / 7;
    let mut count = full_weeks * 5;
    let mut day = from + full_weeks * 7;
    while day < to {
        if weekday(day) < 5 {
            count += 1;
        }
        day += 1;
    }
    count
}

/// The day that is `n` workdays after `from` (Monday to Friday only; before it for negative `n`).
pub fn add_workdays(from: i64, n: i64) -> i64 {
    let step = if n < 0 { -1 } else { 1 };
    let mut day = from;
    let mut left = n.abs();
    while left > 0 {
        day += step;
        if weekday(day) < 5 {
            left -= 1;
        }
    }
    day
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(year: i64, month: u32, day: u32) -> Civil {
        Civil { year, month, day }
    }

    #[test]
    fn the_epoch_and_a_few_known_days() {
        assert_eq!(days_from_civil(d(1970, 1, 1)), 0);
        assert_eq!(days_from_civil(d(2000, 3, 1)), 11_017);
        assert_eq!(days_from_civil(d(1969, 12, 31)), -1);
        assert_eq!(civil_from_days(19_000), d(2022, 1, 8));
        assert_eq!(weekday(0), 3);
        assert_eq!(weekday(days_from_civil(d(1984, 1, 24))), 1);
    }

    #[test]
    fn every_day_round_trips() {
        for z in (-800_000..800_000).step_by(37) {
            assert_eq!(days_from_civil(civil_from_days(z)), z);
        }
    }

    #[test]
    fn month_arithmetic_clamps_to_the_end_of_the_month() {
        assert_eq!(add_months(d(2020, 1, 31), 1), d(2020, 2, 29));
        assert_eq!(add_months(d(2019, 2, 28), 1), d(2019, 3, 28));
        assert_eq!(add_months(d(2019, 1, 31), 1), d(2019, 2, 28));
        assert_eq!(add_months(d(2020, 3, 31), -1), d(2020, 2, 29));
        assert_eq!(add_months(d(2020, 12, 15), 1), d(2021, 1, 15));
        assert_eq!(add_months(d(2020, 1, 15), -1), d(2019, 12, 15));
        assert_eq!(add_months(d(2020, 2, 29), 12), d(2021, 2, 28));
        assert_eq!(add_months(d(2020, 2, 29), 48), d(2024, 2, 29));
    }

    #[test]
    fn leap_years() {
        assert!(is_leap(2000) && is_leap(2024) && !is_leap(1900) && !is_leap(2023));
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2023, 2), 28);
    }

    #[test]
    fn weeks_and_days_of_the_year() {
        assert_eq!(iso_week(d(2021, 3, 12)), (2021, 10));
        assert_eq!(day_of_year(d(2024, 3, 15)), 75);
        assert_eq!(iso_week(d(2021, 1, 1)), (2020, 53));
        assert_eq!(iso_week(d(2018, 12, 31)), (2019, 1));
    }

    #[test]
    fn easter_dates() {
        assert_eq!(easter(2027), d(2027, 3, 28));
        assert_eq!(easter(2024), d(2024, 3, 31));
        assert_eq!(easter(2019), d(2019, 4, 21));
        assert_eq!(easter(1961), d(1961, 4, 2));
    }

    #[test]
    fn thanksgiving_is_the_fourth_thursday_of_november() {
        assert_eq!(nth_weekday(2024, 11, 3, 4), d(2024, 11, 28));
        assert_eq!(nth_weekday(2023, 11, 3, 4), d(2023, 11, 23));
        assert_eq!(nth_weekday(2024, 5, 0, 5), d(2024, 5, 27));
    }

    #[test]
    fn workdays() {
        let monday = days_from_civil(d(2024, 3, 4));
        assert_eq!(workdays_between(monday, monday + 7), 5);
        assert_eq!(workdays_between(monday, monday + 5), 5);
        assert_eq!(workdays_between(monday, monday + 6), 5);
        assert_eq!(workdays_between(monday + 7, monday), -5);
        assert_eq!(add_workdays(monday + 4, 1), monday + 7);
        assert_eq!(add_workdays(monday, 5), monday + 7);
        assert_eq!(add_workdays(monday + 7, -1), monday + 4);
    }
}
