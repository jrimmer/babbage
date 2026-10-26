//! Forms about dates, written in words: `day of the week on March 12, 2023`, `days until Christmas`,
//! `3 weeks after March 14, 2019`, `April 1 through April 30`, `March 12, 2023 as EEEE, MMM d, yyyy`.

use crate::calendar;
use crate::datefns::{self, DateFn};
use crate::error::Error;
use crate::functions::Func;
use crate::lexer::{Tok, Token};
use crate::number::Number;
use crate::parser::{BinOp, Expr, Node, parse};
use crate::settings;
use crate::units;
use crate::value::{Kind, Value};

/// A word, or a unit written as that word: `week` and `days` lex as units.
fn is_word(t: Option<&Token>, w: &str) -> bool {
    match t.map(|t| &t.tok) {
        Some(Tok::Word(x)) => x.eq_ignore_ascii_case(w),
        Some(Tok::Unit(u)) => units::lookup(w).is_some_and(|k| &k == u),
        _ => false,
    }
}

fn starts_with_words(toks: &[Token], words: &[&str]) -> bool {
    words
        .iter()
        .enumerate()
        .all(|(i, w)| is_word(toks.get(i), w))
}

fn word_of(t: &Token) -> Option<String> {
    match &t.tok {
        Tok::Word(w) => Some(w.to_lowercase()),
        _ => None,
    }
}

fn today_node() -> Node {
    let (today, _) = settings::local_now();
    Node::Value(Value::date(Number::from_i64(today)))
}

fn finish(mut rpn: Vec<Node>, f: DateFn) -> Result<Expr, Error> {
    rpn.push(Node::Call(Func::Date(f), f.arity()));
    Ok(Expr::from_nodes(rpn))
}

fn rpn_of(toks: &[Token]) -> Result<Vec<Node>, Error> {
    Ok(parse(toks)?.nodes().to_vec())
}

/// `<lead words> <connector> <date>`: one function of one date.
const ONE_DATE: &[(&[&str], DateFn)] = &[
    (&["day", "of", "the", "week"], DateFn::Weekday),
    (&["weekday"], DateFn::Weekday),
    (&["what", "day", "of", "the", "week"], DateFn::Weekday),
    (&["what", "day"], DateFn::Weekday),
    (&["week", "number"], DateFn::WeekNumber),
    (&["week", "of", "the", "year"], DateFn::WeekNumber),
    (&["day", "number"], DateFn::DayNumber),
    (&["day", "of", "the", "year"], DateFn::DayNumber),
    (&["day", "of", "the", "month"], DateFn::DayOfMonth),
];

const CONNECTORS: &[&str] = &["on", "of", "for", "in", "is"];

pub fn parse_date_forms(toks: &[Token]) -> Option<Result<Expr, Error>> {
    one_date(toks)
        .or_else(|| days_in(toks))
        .or_else(|| spans(toks))
        .or_else(|| through(toks))
        .or_else(|| relative(toks))
        .or_else(|| formatted(toks))
        .or_else(|| timestamps(toks))
        .or_else(|| interval(toks))
        .or_else(|| playback(toks))
        .or_else(|| timecodes(toks))
}

fn one_date(toks: &[Token]) -> Option<Result<Expr, Error>> {
    for (lead, f) in ONE_DATE {
        if starts_with_words(toks, lead)
            && CONNECTORS.iter().any(|c| is_word(toks.get(lead.len()), c))
            && toks.len() > lead.len() + 1
        {
            return Some(rpn_of(&toks[lead.len() + 1..]).and_then(|rpn| finish(rpn, *f)));
        }
    }
    None
}

/// `days in February 2020`, `days in Q3 2024`, `days in 2024`.
fn days_in(toks: &[Token]) -> Option<Result<Expr, Error>> {
    if !(starts_with_words(toks, &["days", "in"])
        || starts_with_words(toks, &["number", "of", "days", "in"]))
    {
        return None;
    }
    let rest = &toks[toks.iter().position(|t| is_word(Some(t), "in"))? + 1..];
    let (today, _) = settings::local_now();
    let this_year = calendar::civil_from_days(today).year;
    let whole = |n: i64| Node::Num(Number::from_i64(n));
    let year_of = |t: Option<&Token>| match t.map(|t| &t.tok) {
        Some(Tok::Num(n)) if n.is_integer() => n.numer().to_string().parse::<i64>().ok(),
        _ => None,
    };
    match rest {
        [t] if year_of(Some(t)).is_some() => {
            Some(finish(vec![whole(year_of(Some(t))?)], DateFn::DaysInYear))
        }
        [q, n, ..] if is_word(Some(q), "q") && rest.len() <= 3 => {
            let q = year_of(Some(n))?;
            let year = if rest.len() == 3 {
                year_of(rest.get(2))?
            } else {
                this_year
            };
            Some(finish(vec![whole(q), whole(year)], DateFn::DaysInQuarter))
        }
        [w, y] if year_of(Some(y)).is_some() => {
            let month = crate::dates::month_number(&word_of(w)?)?;
            let first = datefns::first_of(year_of(Some(y))?, month)?;
            Some(finish(vec![Node::Value(first)], DateFn::DaysInMonthOf))
        }
        [w] => {
            let month = crate::dates::month_number(&word_of(w)?)?;
            let first = datefns::first_of(this_year, month)?;
            Some(finish(vec![Node::Value(first)], DateFn::DaysInMonthOf))
        }
        _ => None,
    }
}

/// `days until D`, `days between A and B`, `workdays in 3 weeks`, `work hours between A and B`.
fn spans(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let lead_len = |lead: &[&str]| starts_with_words(toks, lead).then_some(lead.len());
    let between = |at: usize, f: DateFn| {
        let rest = &toks[at..];
        let and = rest.iter().position(|t| is_word(Some(t), "and"))?;
        let (a, b) = (&rest[..and], &rest[and + 1..]);
        if a.is_empty() || b.is_empty() {
            return None;
        }
        Some(rpn_of(a).and_then(|mut x| {
            x.extend(rpn_of(b)?);
            finish(x, f)
        }))
    };
    let from_today = |at: usize, f: DateFn| {
        let rest = &toks[at..];
        if rest.is_empty() {
            return None;
        }
        Some(rpn_of(rest).and_then(|r| {
            let mut x = vec![today_node()];
            x.extend(r);
            finish(x, f)
        }))
    };
    let single = |at: usize, f: DateFn| {
        let rest = &toks[at..];
        (!rest.is_empty()).then(|| rpn_of(rest).and_then(|r| finish(r, f)))
    };
    if let Some(n) =
        lead_len(&["days", "between"]).or_else(|| lead_len(&["number", "of", "days", "between"]))
    {
        return between(n, DateFn::DaysBetween);
    }
    if let Some(n) =
        lead_len(&["workdays", "between"]).or_else(|| lead_len(&["work", "days", "between"]))
    {
        return between(n, DateFn::WorkdaysBetween);
    }
    if let Some(n) = lead_len(&["work", "hours", "between"]) {
        return between(n, DateFn::WorkHoursBetween);
    }
    if let Some(n) = lead_len(&["days", "until"]).or_else(|| lead_len(&["days", "till"])) {
        return single(n, DateFn::Until);
    }
    if let Some(n) = lead_len(&["days", "since"]) {
        return single(n, DateFn::Since);
    }
    for lead in [
        &["workdays", "until"][..],
        &["workdays", "till"],
        &["work", "days", "until"],
    ] {
        if let Some(n) = lead_len(lead) {
            return from_today(n, DateFn::WorkdaysBetween);
        }
    }
    for lead in [&["workdays", "in"][..], &["work", "days", "in"]] {
        if let Some(n) = lead_len(lead) {
            return single(n, DateFn::WorkdaysIn);
        }
    }
    None
}

/// `April 1 through April 30`: both ends counted.
fn through(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let at = toks.iter().position(|t| is_word(Some(t), "through"))?;
    let (a, b) = (&toks[..at], &toks[at + 1..]);
    if a.is_empty() || b.is_empty() {
        return None;
    }
    Some(rpn_of(a).and_then(|mut x| {
        x.extend(rpn_of(b)?);
        finish(x, DateFn::Inclusive)
    }))
}

fn date_token(t: &Token) -> bool {
    matches!(&t.tok, Tok::Value(v) if v.kind() == Kind::Date)
}

/// `3 weeks after March 14, 2019`, `2 days before Christmas`, `3 hours from now`, `5 days ago`.
fn relative(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let n = toks.len();
    if n >= 2 && is_word(toks.last(), "ago") {
        let left = &toks[..n - 1];
        let (today, seconds) = settings::local_now();
        let now = Value::date(
            Number::from_ratio(
                num_bigint::BigInt::from(today * 86_400 + seconds),
                num_bigint::BigInt::from(86_400),
            )
            .ok()?,
        );
        return Some(rpn_of(left).map(|l| {
            let mut x = vec![Node::Value(now)];
            x.extend(l);
            x.push(Node::Bin(BinOp::Sub));
            Expr::from_nodes(x)
        }));
    }
    if n >= 3 && date_token(&toks[n - 1]) {
        let connector = &toks[n - 2];
        let sign = if is_word(Some(connector), "from") || is_word(Some(connector), "after") {
            BinOp::Add
        } else if is_word(Some(connector), "before") {
            BinOp::Sub
        } else {
            return None;
        };
        let Tok::Value(date) = &toks[n - 1].tok else {
            return None;
        };
        let date = date.clone();
        return Some(rpn_of(&toks[..n - 2]).map(|l| {
            let mut x = vec![Node::Value(date)];
            x.extend(l);
            x.push(Node::Bin(sign));
            Expr::from_nodes(x)
        }));
    }
    None
}

/// `March 12, 2023 as EEEE, MMM d, yyyy`.
fn formatted(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let n = toks.len();
    if n < 3 {
        return None;
    }
    let Tok::Format(pattern) = &toks[n - 1].tok else {
        return None;
    };
    if !is_word(toks.get(n - 2), "as") {
        return None;
    }
    Some(rpn_of(&toks[..n - 2]).and_then(|mut x| {
        x.push(Node::Value(Value::text(pattern)));
        finish(x, DateFn::Format)
    }))
}

/// `current timestamp`, `X to timestamp`, `X to date`, `X as iso8601`.
fn timestamps(toks: &[Token]) -> Option<Result<Expr, Error>> {
    if toks.len() == 2 && is_word(toks.first(), "current") && is_word(toks.get(1), "timestamp") {
        return Some(finish(vec![], DateFn::CurrentTimestamp));
    }
    let n = toks.len();
    let connector = |at: usize| {
        ["to", "as", "in", "into"]
            .iter()
            .any(|w| is_word(toks.get(at), w))
    };
    let (cut, func) = if n >= 3 && connector(n - 2) && is_word(toks.get(n - 1), "timestamp") {
        (n - 2, DateFn::ToTimestamp)
    } else if n >= 3 && connector(n - 2) && is_word(toks.get(n - 1), "date") {
        (n - 2, DateFn::ToDate)
    } else if n >= 4
        && connector(n - 3)
        && is_word(toks.get(n - 2), "iso")
        && matches!(&toks[n - 1].tok, Tok::Num(v) if v.numer().to_string() == "8601" && v.is_integer())
    {
        (n - 3, DateFn::AsIso)
    } else {
        return None;
    };
    Some(rpn_of(&toks[..cut]).and_then(|r| finish(r, func)))
}

fn single_value(t: &Token) -> Option<&Value> {
    match &t.tok {
        Tok::Value(v) => Some(v),
        _ => None,
    }
}

/// `7:30am to 8:45pm`, `March 1 to March 5`.
fn interval(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let [a, to, b] = toks else {
        return None;
    };
    let (x, y) = (single_value(a)?, single_value(b)?);
    let same = matches!(
        (x.kind(), y.kind()),
        (Kind::Clock, Kind::Clock) | (Kind::Date, Kind::Date)
    );
    (is_word(Some(to), "to") && same).then(|| {
        finish(
            vec![Node::Value(x.clone()), Node::Value(y.clone())],
            DateFn::Interval,
        )
    })
}

/// `1 hour 30 minutes at 1.5x` and `time saved 5 min at 1.5x`.
fn playback(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let n = toks.len();
    if n < 3 || !is_word(toks.get(n - 2), "at") {
        return None;
    }
    let speed = single_value(&toks[n - 1]).filter(|v| v.kind() == Kind::Multiplier)?;
    let (func, from) = if starts_with_words(toks, &["time", "saved"]) {
        (DateFn::TimeSaved, 2)
    } else {
        (DateFn::Playback, 0)
    };
    if from >= n - 2 {
        return None;
    }
    Some(rpn_of(&toks[from..n - 2]).and_then(|mut r| {
        r.push(Node::Value(speed.clone()));
        finish(r, func)
    }))
}

/// `X as timecode`, `X as frames`, each with an optional `at 30 fps` before the connector.
fn timecodes(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let n = toks.len();
    if n < 3 {
        return None;
    }
    let connector = ["to", "as", "in", "into"]
        .iter()
        .any(|w| is_word(toks.get(n - 2), w));
    let (plain, at_rate) = if is_word(toks.get(n - 1), "timecode") && connector {
        (DateFn::AsTimecode, DateFn::AsTimecodeAt)
    } else if (is_word(toks.get(n - 1), "frames") || is_word(toks.get(n - 1), "frame")) && connector
    {
        (DateFn::AsFrames, DateFn::AsFramesAt)
    } else {
        return None;
    };
    let head = &toks[..n - 2];
    let fps = units::lookup("fps");
    let rate_at = head.len().checked_sub(3).filter(|k| {
        is_word(head.get(*k), "at")
            && matches!(&head[k + 1].tok, Tok::Num(_))
            && matches!((&head[k + 2].tok, &fps), (Tok::Unit(u), Some(f)) if u == f)
    });
    Some(match rate_at {
        Some(k) => {
            let Tok::Num(rate) = &head[k + 1].tok else {
                return None;
            };
            rpn_of(&head[..k]).and_then(|mut r| {
                r.push(Node::Value(Value::plain(rate.clone())));
                finish(r, at_rate)
            })
        }
        None => rpn_of(head).and_then(|r| finish(r, plain)),
    })
}
