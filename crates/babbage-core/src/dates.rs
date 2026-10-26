//! Dates and times of day: reading `June 17`, `5 May 2019` and `Easter 2027`, and writing them back.
//!
//! A date is held as days since 1970-01-01 in local time. A fraction of a day is the time of day.

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::ToPrimitive;

use crate::calendar::{self, Civil, MONTHS, WEEKDAYS};
use crate::error::Error;
use crate::lexer::{Tok, Token};
use crate::number::Number;
use crate::settings;
use crate::value::Value;

pub fn month_number(word: &str) -> Option<u32> {
    let w = word.to_lowercase();
    MONTHS
        .iter()
        .position(|m| {
            m.to_lowercase() == w
                || (w.len() == 3 && m.to_lowercase().starts_with(&w))
                || (w == "sept" && *m == "September")
        })
        .map(|i| i as u32 + 1)
}

pub fn weekday_index(word: &str) -> Option<u32> {
    let w = word.to_lowercase();
    WEEKDAYS
        .iter()
        .position(|d| {
            d.to_lowercase() == w
                || (w.len() >= 3 && d.to_lowercase().starts_with(&w) && w.len() <= 4)
        })
        .map(|i| i as u32)
}

fn date_value(days: i64) -> Value {
    Value::date(Number::from_i64(days))
}

fn int(n: &Number) -> Option<i64> {
    n.is_integer().then(|| n.numer().to_i64()).flatten()
}

/// Which occurrence of a yearless date a line means: the closest, the next, or the last.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Bias {
    Nearest,
    Next,
    Last,
}

impl Bias {
    /// `days until Christmas` looks ahead and `days since Christmas` looks back.
    fn before(out: &[Token]) -> Bias {
        match out.last().and_then(|t| word_of(Some(t))).as_deref() {
            Some("until" | "till") => Bias::Next,
            Some("since") => Bias::Last,
            _ => Bias::Nearest,
        }
    }

    fn pick(self, today: i64, candidates: impl Iterator<Item = i64>) -> Option<i64> {
        match self {
            Bias::Nearest => candidates.min_by_key(|d| (d - today).abs()),
            Bias::Next => candidates.filter(|d| *d >= today).min(),
            Bias::Last => candidates.filter(|d| *d <= today).max(),
        }
    }
}

/// The day of the year that falls on `month` and `day`, chosen by `bias`.
fn nearest_occurrence(month: u32, day: u32, bias: Bias) -> Option<i64> {
    let (today, _) = settings::local_now();
    let this_year = calendar::civil_from_days(today).year;
    let candidates = [this_year - 1, this_year, this_year + 1, this_year + 2]
        .into_iter()
        .filter(|y| {
            calendar::is_valid(Civil {
                year: *y,
                month,
                day,
            })
        })
        .map(|y| {
            calendar::days_from_civil(Civil {
                year: y,
                month,
                day,
            })
        });
    bias.pick(today, candidates)
}

fn holiday(words: &[&str], year: Option<i64>, bias: Bias) -> Option<i64> {
    let at = |month: u32, day: u32| match year {
        Some(y) => calendar::is_valid(Civil {
            year: y,
            month,
            day,
        })
        .then(|| {
            calendar::days_from_civil(Civil {
                year: y,
                month,
                day,
            })
        }),
        None => nearest_occurrence(month, day, bias),
    };
    let (today, _) = settings::local_now();
    let this_year = year.unwrap_or_else(|| calendar::civil_from_days(today).year);
    let movable = |f: &dyn Fn(i64) -> Civil| match year {
        Some(y) => Some(calendar::days_from_civil(f(y))),
        None => bias.pick(
            today,
            [this_year - 1, this_year, this_year + 1, this_year + 2]
                .into_iter()
                .map(|y| calendar::days_from_civil(f(y))),
        ),
    };
    match words {
        ["christmas"] | ["christmas", "day"] => at(12, 25),
        ["new", "year"] | ["new", "years"] | ["new", "year's"] | ["new", "year's", "day"] => {
            at(1, 1)
        }
        ["halloween"] => at(10, 31),
        ["valentine's", "day"] | ["valentines", "day"] | ["valentine's"] => at(2, 14),
        ["easter"] => movable(&calendar::easter),
        ["thanksgiving"] => movable(&|y| calendar::nth_weekday(y, 11, 3, 4)),
        ["black", "friday"] => movable(&|y| {
            let thanksgiving = calendar::days_from_civil(calendar::nth_weekday(y, 11, 3, 4));
            calendar::civil_from_days(thanksgiving + 1)
        }),
        _ => None,
    }
}

fn word_of(t: Option<&Token>) -> Option<String> {
    match t.map(|t| &t.tok) {
        Some(Tok::Word(w)) => Some(w.to_lowercase()),
        _ => None,
    }
}

fn number_of(t: Option<&Token>) -> Option<i64> {
    match t.map(|t| &t.tok) {
        Some(Tok::Num(n)) => int(n),
        _ => None,
    }
}

/// Reads an optional `st`/`nd`/`rd`/`th`, then an optional comma, then an optional four-digit year after a day number.
fn year_after(toks: &[Token], mut i: usize) -> (Option<i64>, usize) {
    if matches!(
        word_of(toks.get(i)).as_deref(),
        Some("st" | "nd" | "rd" | "th")
    ) {
        i += 1;
    }
    let mut j = i;
    if matches!(toks.get(j).map(|t| &t.tok), Some(Tok::Comma)) {
        j += 1;
    }
    match number_of(toks.get(j)) {
        Some(y) if (1000..=9999).contains(&y) => (Some(y), j + 1),
        _ => (None, i),
    }
}

/// Replaces date words and patterns with date values.
pub fn rewrite(toks: Vec<Token>) -> Vec<Token> {
    let (today, seconds) = settings::local_now();
    let mut out: Vec<Token> = Vec::with_capacity(toks.len());
    let mut i = 0;
    while i < toks.len() {
        let at = toks[i].at;
        let bias = Bias::before(&out);
        let push = |out: &mut Vec<Token>, v: Value| {
            out.push(Token {
                tok: Tok::Value(v),
                at,
            })
        };
        if let Some(w) = word_of(toks.get(i)) {
            let day_offset = match w.as_str() {
                "today" => Some(0),
                "tomorrow" => Some(1),
                "yesterday" => Some(-1),
                _ => None,
            };
            if let Some(offset) = day_offset {
                push(&mut out, date_value(today + offset));
                i += 1;
                continue;
            }
            match w.as_str() {
                "now" => {
                    let moment = Number::from_ratio(
                        BigInt::from(today * 86_400 + seconds),
                        BigInt::from(86_400),
                    )
                    .expect("nonzero");
                    push(&mut out, Value::date(moment));
                    i += 1;
                    continue;
                }
                "noon" => {
                    push(&mut out, Value::clock(Number::from_i64(43_200)));
                    i += 1;
                    continue;
                }
                "midnight" => {
                    push(&mut out, Value::clock(Number::from_i64(0)));
                    i += 1;
                    continue;
                }
                "next" | "last" | "this" => {
                    if let Some(target) = word_of(toks.get(i + 1)).and_then(|d| weekday_index(&d)) {
                        let current = i64::from(calendar::weekday(today));
                        let ahead = (i64::from(target) - current).rem_euclid(7);
                        let day = match w.as_str() {
                            "next" => today + if ahead == 0 { 7 } else { ahead },
                            "last" => today - if ahead == 0 { 7 } else { 7 - ahead },
                            _ => today + ahead,
                        };
                        push(&mut out, date_value(day));
                        i += 2;
                        continue;
                    }
                }
                _ => {}
            }
            if let Some(month) = month_number(&w)
                && let Some(day) = number_of(toks.get(i + 1)).filter(|d| (1..=31).contains(d))
            {
                let (year, next) = year_after(&toks, i + 2);
                let days = match year {
                    Some(y) => calendar::is_valid(Civil {
                        year: y,
                        month,
                        day: day as u32,
                    })
                    .then(|| {
                        calendar::days_from_civil(Civil {
                            year: y,
                            month,
                            day: day as u32,
                        })
                    }),
                    None => nearest_occurrence(month, day as u32, bias),
                };
                if let Some(d) = days {
                    push(&mut out, date_value(d));
                    i = next;
                    continue;
                }
            }
            // Holidays, one to three words, with an optional year after.
            let hit = [3usize, 2, 1].into_iter().find_map(|length| {
                let words: Vec<String> = (0..length)
                    .filter_map(|k| word_of(toks.get(i + k)))
                    .collect();
                if words.len() != length {
                    return None;
                }
                let refs: Vec<&str> = words.iter().map(String::as_str).collect();
                let year = number_of(toks.get(i + length)).filter(|y| (1000..=9999).contains(y));
                holiday(&refs, year, bias).map(|d| (d, length + usize::from(year.is_some())))
            });
            if let Some((d, used)) = hit {
                push(&mut out, date_value(d));
                i += used;
                continue;
            }
        }
        // `5 May 2019`, `5 May`
        if let (Some(day), Some(month)) = (
            number_of(toks.get(i)).filter(|d| (1..=31).contains(d)),
            word_of(toks.get(i + 1)).and_then(|w| month_number(&w)),
        ) {
            let (year, next) = year_after_month(&toks, i + 2);
            let days = match year {
                Some(y) => calendar::is_valid(Civil {
                    year: y,
                    month,
                    day: day as u32,
                })
                .then(|| {
                    calendar::days_from_civil(Civil {
                        year: y,
                        month,
                        day: day as u32,
                    })
                }),
                None => nearest_occurrence(month, day as u32, bias),
            };
            if let Some(d) = days {
                push(&mut out, date_value(d));
                i = next;
                continue;
            }
        }
        out.push(toks[i].clone());
        i += 1;
    }
    join_date_and_time(out)
}

/// A date followed by a time of day, with or without `at`, is one moment: `April 1, 2019 3:30pm`.
fn join_date_and_time(toks: Vec<Token>) -> Vec<Token> {
    let date_of = |t: Option<&Token>| match t.map(|t| &t.tok) {
        Some(Tok::Value(v)) if v.kind() == crate::value::Kind::Date && v.number().is_integer() => {
            Some(v.number().clone())
        }
        _ => None,
    };
    let clock_of = |t: Option<&Token>| match t.map(|t| &t.tok) {
        Some(Tok::Value(v)) if v.kind() == crate::value::Kind::Clock => Some(v.number().clone()),
        _ => None,
    };
    let mut out: Vec<Token> = Vec::with_capacity(toks.len());
    let mut i = 0;
    while i < toks.len() {
        if let Some(days) = date_of(toks.get(i)) {
            let has_at = word_of(toks.get(i + 1)).as_deref() == Some("at");
            let clock_at = i + 1 + usize::from(has_at);
            if let Some(seconds) = clock_of(toks.get(clock_at))
                && let Ok(moment) = seconds
                    .div(&Number::from_i64(86_400))
                    .and_then(|fraction| days.add(&fraction))
            {
                out.push(Token {
                    tok: Tok::Value(Value::date(moment)),
                    at: toks[i].at,
                });
                i = clock_at + 1;
                continue;
            }
        }
        out.push(toks[i].clone());
        i += 1;
    }
    out
}

fn year_after_month(toks: &[Token], i: usize) -> (Option<i64>, usize) {
    match number_of(toks.get(i)) {
        Some(y) if (1000..=9999).contains(&y) => (Some(y), i + 1),
        _ => (None, i),
    }
}

/// `3:30 pm` style: no leading zero, `am`/`pm` in lower case, seconds only when there are some.
pub fn format_clock(seconds: &Number) -> String {
    let whole = BigInt::div_floor(seconds.numer(), seconds.denom());
    let total = whole.mod_floor(&BigInt::from(86_400)).to_i64().unwrap_or(0);
    let fraction = seconds
        .sub(&Number::from_ratio(whole, BigInt::from(1)).expect("integer"))
        .ok();
    let (h, m, s) = (total / 3600, total % 3600 / 60, total % 60);
    let hour12 = match h % 12 {
        0 => 12,
        n => n,
    };
    let suffix = if h < 12 { "am" } else { "pm" };
    let sub_second = fraction
        .filter(|f| !f.is_zero())
        .and_then(|f| f.to_exact_decimal())
        .map(|d| d.trim_start_matches('0').to_string())
        .unwrap_or_default();
    if s == 0 && sub_second.is_empty() {
        format!("{hour12}:{m:02} {suffix}")
    } else {
        format!("{hour12}:{m:02}:{s:02}{sub_second} {suffix}")
    }
}

fn split_days(value: &Number) -> (i64, Number) {
    let day = BigInt::div_floor(value.numer(), value.denom());
    let rest = value
        .sub(&Number::from_ratio(day.clone(), BigInt::from(1)).expect("integer"))
        .expect("small");
    (
        day.to_i64().unwrap_or(0),
        rest.mul(&Number::from_i64(86_400)).expect("small"),
    )
}

/// `June 17`, or `June 17, 2019` when the year is not the current one; with ` at 3:30 pm` for a time of day.
pub fn format_date(days: &Number) -> String {
    let (day, seconds) = split_days(days);
    let civil = calendar::civil_from_days(day);
    let (today, _) = settings::local_now();
    let this_year = calendar::civil_from_days(today).year;
    let mut text = format!("{} {}", MONTHS[civil.month as usize - 1], civil.day);
    if civil.year != this_year {
        text.push_str(&format!(", {}", civil.year));
    }
    if !seconds.is_zero() {
        text.push_str(" at ");
        text.push_str(&format_clock(&seconds));
    }
    text
}

/// The seconds of a length of time as text, and whether that text is exact: a fraction of a second that
/// does not end (a third of a second) is cut at three places and reported as inexact.
pub(crate) fn seconds_text(whole_seconds: i64, fraction: &Number) -> (String, bool) {
    if fraction.is_zero() {
        return (whole_seconds.to_string(), true);
    }
    match fraction.to_exact_decimal() {
        Some(d) => (
            format!("{whole_seconds}{}", d.trim_start_matches('0')),
            true,
        ),
        None => {
            let three = fraction
                .mul(&Number::from_i64(1000))
                .map(|n| n.floor())
                .unwrap_or_default();
            (format!("{whole_seconds}.{three:03}"), false)
        }
    }
}

pub(crate) fn split_seconds(seconds: &Number) -> (bool, i64, Number) {
    let negative = seconds.is_negative();
    let whole = BigInt::div_floor(&seconds.abs().numer().clone(), seconds.denom());
    let fraction = seconds
        .abs()
        .sub(&Number::from_ratio(whole.clone(), BigInt::from(1)).expect("integer"))
        .expect("small");
    (negative, whole.to_i64().unwrap_or(i64::MAX), fraction)
}

/// A length of time as `3 weeks 5 days` or `4 hours 32 minutes 24 seconds`.
pub fn format_timespan(seconds: &Number) -> (String, bool) {
    let (negative, total, fraction) = split_seconds(seconds);
    let parts = [
        (total / 604_800, "week"),
        (total % 604_800 / 86_400, "day"),
        (total % 86_400 / 3600, "hour"),
        (total % 3600 / 60, "minute"),
    ];
    let mut text: Vec<String> = parts
        .iter()
        .filter(|(n, _)| *n != 0)
        .map(|(n, name)| format!("{n} {name}{}", if *n == 1 { "" } else { "s" }))
        .collect();
    let (secs_text, exact) = seconds_text(total % 60, &fraction);
    if total % 60 != 0 || !fraction.is_zero() || text.is_empty() {
        text.push(format!(
            "{secs_text} second{}",
            if secs_text == "1" { "" } else { "s" }
        ));
    }
    (
        format!("{}{}", if negative { "-" } else { "" }, text.join(" ")),
        exact,
    )
}

/// A length of time as `03:04:05`, with a fraction of a second after a point: `00:00:01.5`.
pub fn format_laptime(seconds: &Number) -> (String, bool) {
    let (negative, total, fraction) = split_seconds(seconds);
    let (secs, exact) = seconds_text(total % 60, &fraction);
    let secs = if secs.split('.').next().is_some_and(|w| w.len() < 2) {
        format!("0{secs}")
    } else {
        secs
    };
    (
        format!(
            "{}{:02}:{:02}:{secs}",
            if negative { "-" } else { "" },
            total / 3600,
            total % 3600 / 60
        ),
        exact,
    )
}

/// A length of time as `01:02:03:04`: hours, minutes, seconds and the frame within the second. A time that
/// falls between two frames is cut to the frame before it and reported as inexact.
pub fn format_timecode(seconds: &Number, frames_per_second: &Number) -> (String, bool) {
    let negative = seconds.is_negative();
    let frames = seconds
        .abs()
        .mul(frames_per_second)
        .unwrap_or_else(|_| Number::from_i64(0));
    let whole = frames.floor();
    let exact = frames
        .sub(&Number::from_ratio(whole.clone(), BigInt::from(1)).expect("integer"))
        .is_ok_and(|rest| rest.is_zero());
    let rate = frames_per_second.floor();
    let (total_seconds, frame) = BigInt::div_rem(&whole, &rate);
    let total = total_seconds.to_i64().unwrap_or(i64::MAX);
    (
        format!(
            "{}{:02}:{:02}:{:02}:{:02}",
            if negative { "-" } else { "" },
            total / 3600,
            total % 3600 / 60,
            total % 60,
            frame
        ),
        exact,
    )
}

/// Writes a date with an `NSDateFormatter`-style pattern: `EEEE, MMM d, yyyy`.
pub fn format_pattern(days: &Number, pattern: &str) -> Result<String, Error> {
    let (day, seconds) = split_days(days);
    let civil = calendar::civil_from_days(day);
    let secs = seconds
        .numer()
        .div_floor(seconds.denom())
        .to_i64()
        .unwrap_or(0);
    let (h, m, s) = (secs / 3600, secs % 3600 / 60, secs % 60);
    let weekday = WEEKDAYS[calendar::weekday(day) as usize];
    let mut out = String::new();
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\'' {
            let close = chars[i + 1..]
                .iter()
                .position(|x| *x == '\'')
                .ok_or(Error::UnexpectedEnd)?;
            out.extend(&chars[i + 1..i + 1 + close]);
            i += close + 2;
            continue;
        }
        if !c.is_ascii_alphabetic() {
            out.push(c);
            i += 1;
            continue;
        }
        let run = chars[i..].iter().take_while(|x| **x == c).count();
        let piece = match (c, run) {
            ('y', 2) => format!("{:02}", civil.year.rem_euclid(100)),
            ('y', _) => format!("{:0width$}", civil.year, width = run.max(1)),
            ('M', 1) => civil.month.to_string(),
            ('M', 2) => format!("{:02}", civil.month),
            ('M', 3) => MONTHS[civil.month as usize - 1][..3].to_string(),
            ('M', _) => MONTHS[civil.month as usize - 1].to_string(),
            ('d', 1) => civil.day.to_string(),
            ('d', _) => format!("{:02}", civil.day),
            ('E', 1..=3) => weekday[..3].to_string(),
            ('E', _) => weekday.to_string(),
            ('H', 1) => h.to_string(),
            ('H', _) => format!("{h:02}"),
            ('h', 1) => (if h % 12 == 0 { 12 } else { h % 12 }).to_string(),
            ('h', _) => format!("{:02}", if h % 12 == 0 { 12 } else { h % 12 }),
            ('m', 1) => m.to_string(),
            ('m', _) => format!("{m:02}"),
            ('s', 1) => s.to_string(),
            ('s', _) => format!("{s:02}"),
            ('a', _) => (if h < 12 { "AM" } else { "PM" }).to_string(),
            _ => {
                return Err(Error::UnsupportedOperation(
                    "this date format letter is not supported",
                ));
            }
        };
        out.push_str(&piece);
        i += run;
    }
    Ok(out)
}
