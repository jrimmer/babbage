//! Forms written in words: `average of 1, 2 and 3`, `clamp 26 between 5 and 25`, `x to nearest 10`.
//!
//! Each form is matched on the words at the start or end of a line and rewritten into the same
//! reverse-Polish nodes the symbolic syntax produces, so they evaluate through one path.

use crate::error::Error;
use crate::functions::Func;
use crate::lexer::{Tok, Token};
use crate::number::Number;
use crate::parser::{Expr, Node, Phrase, parse};
use crate::value::Value;

fn is_word(t: Option<&Token>, w: &str) -> bool {
    matches!(t.map(|t| &t.tok), Some(Tok::Word(x)) if x.eq_ignore_ascii_case(w))
}

fn words_at(toks: &[Token], start: usize, words: &[&str]) -> bool {
    words
        .iter()
        .enumerate()
        .all(|(i, w)| is_word(toks.get(start + i), w))
}

/// Splits at top-level commas and the word `and`.
fn list_parts(toks: &[Token]) -> Vec<&[Token]> {
    // Items are separated by commas, `and`, or just a space between two numbers or brackets (`5 7`, `(1 + 1) 3`).
    let ends = |t: &Token| matches!(t.tok, Tok::Num(_) | Tok::RParen | Tok::Value(_));
    let starts = |t: &Token| matches!(t.tok, Tok::Num(_) | Tok::LParen | Tok::Value(_));
    let mut parts = Vec::new();
    let (mut depth, mut from) = (0i64, 0);
    for (i, t) in toks.iter().enumerate() {
        match t.tok {
            Tok::LParen => depth += 1,
            Tok::RParen => depth -= 1,
            _ => {}
        }
        let separator = matches!(t.tok, Tok::Comma) || is_word(Some(t), "and");
        let adjacent = depth_before(toks, i) == 0 && i > 0 && ends(&toks[i - 1]) && starts(t);
        if depth == 0 && separator {
            parts.push(&toks[from..i]);
            from = i + 1;
        } else if adjacent {
            parts.push(&toks[from..i]);
            from = i;
        }
    }
    parts.push(&toks[from..]);
    parts
}

/// The bracket depth just before token `i`.
fn depth_before(toks: &[Token], i: usize) -> i64 {
    toks[..i].iter().fold(0, |d, t| match t.tok {
        Tok::LParen => d + 1,
        Tok::RParen => d - 1,
        _ => d,
    })
}

fn split_at_depth_zero(toks: &[Token], is_separator: impl Fn(&Token) -> bool) -> Vec<&[Token]> {
    let mut parts = Vec::new();
    let (mut depth, mut from) = (0i64, 0);
    for (i, t) in toks.iter().enumerate() {
        match t.tok {
            Tok::LParen => depth += 1,
            Tok::RParen => depth -= 1,
            _ if depth == 0 && is_separator(t) => {
                parts.push(&toks[from..i]);
                from = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&toks[from..]);
    parts
}

fn depth_zero_word(toks: &[Token], word: &str) -> Option<usize> {
    let mut depth = 0i64;
    for (i, t) in toks.iter().enumerate() {
        match t.tok {
            Tok::LParen => depth += 1,
            Tok::RParen => depth -= 1,
            _ if depth == 0 && is_word(Some(t), word) => return Some(i),
            _ => {}
        }
    }
    None
}

fn rpn_of(parts: &[&[Token]]) -> Result<Vec<Node>, Error> {
    let mut rpn = Vec::new();
    for p in parts {
        // A part may itself be a phrase: `half of half of 30% of 90`.
        let expr = match parse(p) {
            Ok(expr) => expr,
            Err(e) => match parse_words(p) {
                Some(Ok(expr)) => expr,
                _ => return Err(e),
            },
        };
        rpn.extend(expr.nodes().iter().cloned());
    }
    Ok(rpn)
}

fn call(parts: &[&[Token]], func: Func) -> Result<Expr, Error> {
    let mut rpn = rpn_of(parts)?;
    rpn.push(Node::Call(func, parts.len()));
    Ok(Expr::from_nodes(rpn))
}

/// Leading forms: `<name words> of <list>`.
const LIST_FORMS: &[(&[&str], Func)] = &[
    (&["total"], Func::Sum),
    (&["sum"], Func::Sum),
    (&["average"], Func::Average),
    (&["avg"], Func::Average),
    (&["mean"], Func::Average),
    (&["count"], Func::Count),
    (&["median"], Func::Median),
    (&["gcd"], Func::Gcd),
    (&["lcm"], Func::Lcm),
    (&["minimum"], Func::Min),
    (&["max"], Func::Max),
    (&["maximum"], Func::Max),
    (&["larger"], Func::Max),
    (&["greater"], Func::Max),
    (&["smaller"], Func::Min),
    (&["lesser"], Func::Min),
    (&["half"], Func::Half),
    (&["square", "root"], Func::Sqrt),
    (&["cube", "root"], Func::Cbrt),
    (&["standard", "deviation"], Func::StdDev),
];

/// Lists with no `of` after the name: `max 50, 20, 10`.
const BARE_LIST_FORMS: &[(&[&str], Func)] = &[
    (&["min"], Func::Min),
    (&["minimum"], Func::Min),
    (&["max"], Func::Max),
    (&["maximum"], Func::Max),
    (&["gcd"], Func::Gcd),
    (&["lcm"], Func::Lcm),
];

/// `min` lexes as the unit of time, so a list that starts with it is spelled out.
fn min_unit_at_start(toks: &[Token]) -> bool {
    matches!(toks.first().map(|t| &t.tok), Some(Tok::Unit(u))
        if crate::units::lookup("min").is_some_and(|k| &k == u))
}

/// Leading forms with no `of`: `midpoint between a and b`.
const BETWEEN_FORMS: &[(&[&str], Func)] = &[
    (&["midpoint", "between"], Func::Midpoint),
    (&["halfway", "between"], Func::Midpoint),
    (&["random", "number", "between"], Func::Random),
];

/// The expression a line written in words stands for, or None when it is not one of these forms.
pub fn parse_words(toks: &[Token]) -> Option<Result<Expr, Error>> {
    list_form(toks)
        .or_else(|| number_span(toks))
        .or_else(|| is_what_per(toks))
        .or_else(|| pace(toks))
        .or_else(|| crate::dateforms::parse_date_forms(toks))
        .or_else(|| unit_in_quantity(toks))
        .or_else(|| proportion_what(toks))
        .or_else(|| in_what(toks))
        .or_else(|| screen_density(toks))
        .or_else(|| at_density(toks))
        .or_else(|| root_and_log(toks))
        .or_else(|| finance_forms(toks))
        .or_else(|| multiplier_forms(toks))
        .or_else(|| rate_forms(toks))
        .or_else(|| clamp(toks))
        .or_else(|| remainder(toks))
        .or_else(|| count_form(toks))
        .or_else(|| proportion(toks))
        .or_else(|| scientific_form(toks))
        .or_else(|| pitch_forms(toks))
        .or_else(|| as_dms(toks))
        .or_else(|| as_number(toks))
        .or_else(|| to_radix(toks))
        .or_else(|| rounding(toks))
}

/// `$30/day is what per year`: the rate on the left restated per the period on the right.
fn is_what_per(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let at =
        (1..toks.len().saturating_sub(3)).find(|i| words_at(toks, *i, &["is", "what", "per"]))?;
    let period = crate::phrase::parse_unit_run(&toks[at + 3..]).ok()?;
    Some(parse(&toks[..at]).map(|e| {
        let mut rpn = e.nodes().to_vec();
        rpn.push(Node::PerPeriod(period));
        Expr::from_nodes(rpn)
    }))
}

/// `5 km in 25 min`: a distance and the time it took, which is a pace (`05:00/km`).
fn pace(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let at = depth_zero_word(toks, "in")?;
    let (distance, time) = (&toks[..at], &toks[at + 1..]);
    let seconds = crate::units::lookup("s")?;
    let has_unit = |run: &[Token]| run.iter().any(|t| matches!(t.tok, Tok::Unit(_)));
    // The right side is a length of time (`25 min`), which is what makes this a pace and not a conversion.
    let is_time = |run: &[Token]| {
        run.iter().all(|t| match &t.tok {
            Tok::Unit(u) => u.dim() == seconds.dim(),
            _ => true,
        })
    };
    let starts_with_number = matches!(time.first().map(|t| &t.tok), Some(Tok::Num(_)));
    // The distance is a quantity (`5 km`), not a bare unit as in `meters in 10 km`.
    let has_number = |run: &[Token]| {
        run.iter()
            .any(|t| matches!(t.tok, Tok::Num(_) | Tok::Value(_)))
    };
    if at == 0
        || !has_number(distance)
        || !has_unit(distance)
        || !has_unit(time)
        || !is_time(time)
        || !starts_with_number
    {
        return None;
    }
    Some(rpn_of(&[distance, time]).map(|mut rpn| {
        rpn.push(Node::Phrase(Phrase::Pace));
        Expr::from_nodes(rpn)
    }))
}

/// `1978 to 2021` is 43 and `10 to 25` is 15: two plain numbers joined by `to` are how far apart they are, a
/// plain number, as expected.
fn number_span(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let [a, to, b] = toks else {
        return None;
    };
    let number = |t: &Token| match &t.tok {
        Tok::Num(n) => Some(n.clone()),
        _ => None,
    };
    if !is_word(Some(to), "to") {
        return None;
    }
    let (from, until) = (number(a)?, number(b)?);
    Some(until.sub(&from).map(Expr::num))
}

fn list_form(toks: &[Token]) -> Option<Result<Expr, Error>> {
    // `min` lexes as the unit of time, so `min of 3 and 4` is spelled out here.
    if matches!(toks.first().map(|t| &t.tok), Some(Tok::Unit(u))
            if crate::units::lookup("min").is_some_and(|k| &k == u))
        && is_word(toks.get(1), "of")
        && toks.len() > 2
    {
        return Some(call(&list_parts(&toks[2..]), Func::Min));
    }
    for (names, func) in LIST_FORMS {
        if words_at(toks, 0, names)
            && is_word(toks.get(names.len()), "of")
            && toks.len() > names.len() + 1
        {
            return Some(call(&list_parts(&toks[names.len() + 1..]), *func));
        }
    }
    // `max 50, 20, 10`, `gcd 10, 20, 5`, `min 5, 3, 7`, `max (100 * 2) 50 100`: no `of`, and a number or bracket next.
    for (names, func) in BARE_LIST_FORMS {
        let starts_a_list = matches!(
            toks.get(names.len()).map(|t| &t.tok),
            Some(Tok::Num(_) | Tok::LParen | Tok::Value(_))
        );
        if (words_at(toks, 0, names) || (*func == Func::Min && min_unit_at_start(toks)))
            && starts_a_list
            && toks.len() > names.len()
        {
            let parts = list_parts(&toks[names.len()..]);
            // `max(1, 2)` and `max (1 + 2)` are calls of the function, not lists.
            let opens_a_bracket = matches!(toks[names.len()].tok, Tok::LParen);
            if !opens_a_bracket || parts.len() > 1 {
                return Some(call(&parts, *func));
            }
        }
    }
    for (names, func) in BETWEEN_FORMS {
        if words_at(toks, 0, names) && toks.len() > names.len() {
            return Some(call(&list_parts(&toks[names.len()..]), *func));
        }
    }
    None
}

/// `root 5 of 100`, `5 root of 2`, `is 59 prime`, `log 20 base 4`, `81 is 9 to what power`.
fn root_and_log(toks: &[Token]) -> Option<Result<Expr, Error>> {
    // `is 59 prime`
    if toks.len() >= 3 && is_word(toks.first(), "is") && is_word(toks.last(), "prime") {
        return Some(call(&[&toks[1..toks.len() - 1]], Func::IsPrime));
    }
    // `5 root of 2`: the fifth root of 2.
    if !is_word(toks.first(), "root")
        && !is_word(toks.first(), "square")
        && !is_word(toks.first(), "cube")
        && let Some(at) = depth_zero_word(toks, "root")
        && at > 0
        && is_word(toks.get(at + 1), "of")
        && at + 2 < toks.len()
    {
        return Some(call(&[&toks[..at], &toks[at + 2..]], Func::Root));
    }
    if is_word(toks.first(), "root") {
        let of = depth_zero_word(toks, "of")?;
        let (degree, value) = (&toks[1..of], &toks[of + 1..]);
        return Some(if degree.is_empty() || value.is_empty() {
            Err(Error::UnexpectedEnd)
        } else {
            call(&[degree, value], Func::Root)
        });
    }
    if is_word(toks.first(), "log") {
        let base = depth_zero_word(toks, "base")?;
        let (value, b) = (&toks[1..base], &toks[base + 1..]);
        return Some(if value.is_empty() || b.is_empty() {
            Err(Error::UnexpectedEnd)
        } else {
            call(&[value, b], Func::LogBase)
        });
    }
    let is = depth_zero_word(toks, "is")?;
    let to = depth_zero_word(&toks[is..], "to")? + is;
    let tail = &toks[to + 1..];
    let tail = if is_word(tail.first(), "the") {
        &tail[1..]
    } else {
        tail
    };
    if tail.len() == 2 && is_word(tail.first(), "what") && is_word(tail.get(1), "power") {
        let (value, base) = (&toks[..is], &toks[is + 1..to]);
        return Some(if value.is_empty() || base.is_empty() {
            Err(Error::UnexpectedEnd)
        } else {
            call(&[value, base], Func::LogBase)
        });
    }
    None
}

/// `clamp 26 between 5 and 25` and `clamp 4 from 5 to 25`.
fn clamp(toks: &[Token]) -> Option<Result<Expr, Error>> {
    if !is_word(toks.first(), "clamp") {
        return None;
    }
    let rest = &toks[1..];
    let (split, sep) = match (
        depth_zero_word(rest, "between"),
        depth_zero_word(rest, "from"),
    ) {
        (Some(i), _) => (i, "and"),
        (None, Some(i)) => (i, "to"),
        _ => return None,
    };
    let (value, limits) = (&rest[..split], &rest[split + 1..]);
    let bounds = split_at_depth_zero(limits, |t| is_word(Some(t), sep));
    if bounds.len() != 2 || value.is_empty() {
        return Some(Err(Error::UnexpectedEnd));
    }
    Some(call(&[value, bounds[0], bounds[1]], Func::Clamp))
}

/// `remainder of 21 divided by 5`.
fn remainder(toks: &[Token]) -> Option<Result<Expr, Error>> {
    if !words_at(toks, 0, &["remainder", "of"]) {
        return None;
    }
    let rest = &toks[2..];
    let parts = split_at_depth_zero(rest, |t| matches!(t.tok, Tok::Slash));
    if parts.len() != 2 {
        return Some(Err(Error::UnexpectedEnd));
    }
    Some(rpn_of(&parts).map(|mut rpn| {
        rpn.push(Node::Bin(crate::parser::BinOp::Mod));
        Expr::from_nodes(rpn)
    }))
}

/// `10 permutation 3`, `25 combinations 3`, `3 permutations of 10`.
fn count_form(toks: &[Token]) -> Option<Result<Expr, Error>> {
    for (word, plain, of) in [
        ("permutation", Func::Permutations, Func::PermutationsOf),
        ("permutations", Func::Permutations, Func::PermutationsOf),
        ("combination", Func::Combinations, Func::CombinationsOf),
        ("combinations", Func::Combinations, Func::CombinationsOf),
    ] {
        if let Some(i) = depth_zero_word(toks, word) {
            let (left, mut right) = (&toks[..i], &toks[i + 1..]);
            let func = if is_word(right.first(), "of") {
                right = &right[1..];
                of
            } else {
                plain
            };
            if left.is_empty() || right.is_empty() {
                return Some(Err(Error::UnexpectedEnd));
            }
            return Some(call(&[left, right], func));
        }
    }
    None
}

/// `6 is to 60 as 8 is to what` and `5 is to 10 as what is to 80`.
fn proportion(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let first = depth_zero_word(toks, "is")?;
    if !is_word(toks.get(first + 1), "to") {
        return None;
    }
    let a = &toks[..first];
    let after = &toks[first + 2..];
    let as_at = depth_zero_word(after, "as")?;
    let b = &after[..as_at];
    let tail = &after[as_at + 1..];
    let (c_or_what, d_or_what) = {
        let second = depth_zero_word(tail, "is")?;
        if !is_word(tail.get(second + 1), "to") {
            return None;
        }
        (&tail[..second], &tail[second + 2..])
    };
    let unknown = |t: &[Token]| t.len() == 1 && is_word(t.first(), "what");
    // a : b = c : x  gives x = b * c / a;   a : b = x : d  gives x = a * d / b.
    let (num1, num2, den) = match (unknown(c_or_what), unknown(d_or_what)) {
        (false, true) => (b, c_or_what, a),
        (true, false) => (a, d_or_what, b),
        _ => return None,
    };
    // `num1`, `num2` and `den` are drawn from every part of the line that matters, so these three cover it.
    if num1.is_empty() || num2.is_empty() || den.is_empty() {
        return Some(Err(Error::UnexpectedEnd));
    }
    Some(rpn_of(&[num1, num2, den]).map(|mut rpn| {
        use crate::parser::BinOp;
        // The stack holds num1, num2, den: reorder to (num1 * num2) / den.
        rpn.insert(rpn.len() - den_len(den), Node::Bin(BinOp::Mul));
        rpn.push(Node::Bin(BinOp::Div));
        Expr::from_nodes(rpn)
    }))
}

fn den_len(den: &[Token]) -> usize {
    parse(den).map_or(0, |e| e.nodes().len())
}

/// `ppi of 15" screen at 1024x768`: the density of a screen.
fn screen_density(toks: &[Token]) -> Option<Result<Expr, Error>> {
    if !(is_word(toks.first(), "ppi") && is_word(toks.get(1), "of")) {
        return None;
    }
    let screen = depth_zero_word(toks, "screen")?;
    let at = depth_zero_word(toks, "at")?;
    let size = &toks[2..screen];
    let resolution = &toks[at + 1..];
    let cross = resolution
        .iter()
        .position(|t| matches!(t.tok, Tok::Star) || is_word(Some(t), "x"))?;
    let (w, h) = (&resolution[..cross], &resolution[cross + 1..]);
    if size.is_empty() || w.is_empty() || h.is_empty() || screen > at {
        return Some(Err(Error::UnexpectedEnd));
    }
    Some(call(&[size, w, h], Func::ScreenPpi))
}

/// `440 hz as pitch` is `A4`; `C2 to midi` is 36 and `A4 to hz` is 440 Hz.
fn pitch_forms(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let n = toks.len();
    let joiner = n >= 3
        && ["as", "to", "in", "into"]
            .iter()
            .any(|w| is_word(toks.get(n - 2), w));
    if !joiner {
        return None;
    }
    if is_word(toks.last(), "pitch") {
        return Some(call(&[&toks[..n - 2]], Func::AsPitch));
    }
    let [
        Token {
            tok: Tok::Word(note),
            ..
        },
        _,
        target,
    ] = toks
    else {
        return None;
    };
    let midi = note_to_midi(note)?;
    // `hz` lexes as the unit of frequency; `midi`, `hertz` and `frequency` are words.
    let target = match &target.tok {
        Tok::Word(w) => w.to_lowercase(),
        Tok::Unit(u) if crate::units::lookup("Hz").is_some_and(|hz| hz == *u) => "hz".to_string(),
        _ => return None,
    };
    let value = match target.as_str() {
        "midi" => Value::plain(Number::from_i64(midi)),
        "hz" | "hertz" | "frequency" => {
            let ratio = Number::from_ratio(midi.into(), 12.into()).ok()?;
            let two_to = crate::approx::pow_rational(
                &Number::from_i64(2),
                &ratio
                    .sub(&Number::from_ratio(69.into(), 12.into()).ok()?)
                    .ok()?,
            )
            .ok()?;
            let hertz = Value::quantity(
                Number::from_i64(440).mul(&two_to.number).ok()?,
                crate::units::lookup("Hz")?,
            )
            .ok()?;
            if two_to.exact {
                hertz
            } else {
                hertz.approximate()
            }
        }
        _ => return None,
    };
    Some(Ok(Expr::from_nodes(vec![Node::Value(value)])))
}

/// MIDI note number of a name such as `C4` (60), `A4` (69), `F#3` or `Bb2`.
fn note_to_midi(name: &str) -> Option<i64> {
    let mut chars = name.chars();
    let letter = chars.next()?;
    let semitone = match letter {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let rest: String = chars.collect();
    let (accidental, octave) = match rest.chars().next()? {
        '#' => (1, &rest[1..]),
        'b' => (-1, &rest[1..]),
        _ => (0, rest.as_str()),
    };
    let octave: i64 = octave.parse().ok()?;
    Some(12 * (octave + 1) + semitone + accidental)
}

/// `10500 in sci`, `1234.5 as scientific`, `in scientific notation`: the answer in scientific form.
fn scientific_form(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let n = toks.len();
    let tail = if n >= 4 && words_at(toks, n - 2, &["scientific", "notation"]) {
        2
    } else if n >= 3 && (is_word(toks.last(), "sci") || is_word(toks.last(), "scientific")) {
        1
    } else {
        return None;
    };
    let joiner = n > tail + 1
        && ["as", "to", "in", "into"]
            .iter()
            .any(|w| is_word(toks.get(n - tail - 1), w));
    joiner.then(|| {
        rpn_of(&[&toks[..n - tail - 1]]).map(|mut rpn| {
            rpn.push(Node::Show(crate::value::Shown::Scientific));
            Expr::from_nodes(rpn)
        })
    })
}

/// `156.742° as DMS`: degrees, minutes and seconds.
fn as_dms(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let n = toks.len();
    if n < 3 || !is_word(toks.last(), "dms") {
        return None;
    }
    let joiner = ["as", "to", "in", "into"]
        .iter()
        .any(|w| is_word(toks.get(n - 2), w));
    joiner.then(|| call(&[&toks[..n - 2]], Func::AsDms))
}

/// `$100 as number`, `40/day to decimal`, `20% as dec`.
fn as_number(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let n = toks.len();
    if n < 3 {
        return None;
    }
    let target = ["number", "decimal", "dec"]
        .iter()
        .any(|w| is_word(toks.get(n - 1), w));
    let joiner = ["as", "to", "in", "into"]
        .iter()
        .any(|w| is_word(toks.get(n - 2), w));
    (target && joiner).then(|| call(&[&toks[..n - 2]], Func::AsNumber))
}

/// `256 as hex`, `99 in binary`, `0b1000101 to octal`, `0x2D as base 2`.
fn to_radix(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let n = toks.len();
    let joiner = |i: usize| {
        ["as", "to", "in", "into"]
            .iter()
            .any(|w| is_word(toks.get(i), w))
    };
    let named = [
        ("hex", 16),
        ("hexadecimal", 16),
        ("binary", 2),
        ("bin", 2),
        ("octal", 8),
        ("oct", 8),
    ];
    if n >= 3
        && joiner(n - 2)
        && let Some((_, base)) = named.iter().find(|(w, _)| is_word(toks.get(n - 1), w))
    {
        return Some(rpn_of(&[&toks[..n - 2]]).map(|mut rpn| {
            rpn.push(Node::Num(Number::from_i64(*base)));
            rpn.push(Node::Call(Func::ToBase, 2));
            Expr::from_nodes(rpn)
        }));
    }
    if n >= 4
        && joiner(n - 3)
        && is_word(toks.get(n - 2), "base")
        && let Some(Token {
            tok: Tok::Num(base),
            ..
        }) = toks.get(n - 1)
    {
        return Some(rpn_of(&[&toks[..n - 3]]).map(|mut rpn| {
            rpn.push(Node::Num(base.clone()));
            rpn.push(Node::Call(Func::ToBase, 2));
            Expr::from_nodes(rpn)
        }));
    }
    None
}

fn step_word(w: &str) -> Option<i64> {
    match w.to_ascii_lowercase().as_str() {
        "ten" => Some(10),
        "hundred" => Some(100),
        "thousand" => Some(1000),
        "million" => Some(1_000_000),
        _ => None,
    }
}

/// `x to 2 dp`, `x rounded`, `x rounded up`, `x to nearest 10`, `x rounded down to nearest 5`, `x to nearest 16th`.
fn rounding(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let n = toks.len();
    // `to N dp|decimal places|digits`
    for tail in [
        &["dp"][..],
        &["decimal", "places"],
        &["decimals"],
        &["digits"],
        &["decimal", "place"],
    ] {
        let k = tail.len();
        if n > k + 2
            && words_at(toks, n - k, tail)
            && let (Some(Tok::Num(_)), true) = (
                toks.get(n - k - 1).map(|t| &t.tok),
                is_word(toks.get(n - k - 2), "to"),
            )
        {
            let value = &toks[..n - k - 2];
            let places = &toks[n - k - 1..n - k];
            let mut rpn = rpn_of(&[value, places]).ok()?;
            rpn.push(Node::Phrase(Phrase::RoundPlaces));
            return Some(Ok(Expr::from_nodes(rpn)));
        }
    }
    // `... to nearest <step>` with an optional `rounded [up|down]` before it.
    if let Some(i) = depth_zero_word(toks, "nearest")
        && i >= 2
        && is_word(toks.get(i - 1), "to")
        && i + 1 < n
    {
        let step_toks = &toks[i + 1..];
        let mut value_end = i - 1;
        let mut phrase = Phrase::RoundNearest;
        let mut as_fraction = false;
        // `round 21 up to nearest 5` and `round down 20 to nearest 8`: `round`, then `up` or `down` before or
        // after the number.
        let mut start = 0;
        let mut led = None;
        if is_word(toks.first(), "round") && value_end >= 2 {
            start = 1;
            if is_word(toks.get(1), "up") {
                led = Some(Phrase::RoundUp);
                start = 2;
            } else if is_word(toks.get(1), "down") {
                led = Some(Phrase::RoundDown);
                start = 2;
            }
        }
        if value_end > start && is_word(toks.get(value_end - 1), "up") {
            phrase = Phrase::RoundUp;
            value_end -= 1;
        } else if value_end > start && is_word(toks.get(value_end - 1), "down") {
            phrase = Phrase::RoundDown;
            value_end -= 1;
        }
        if start > 0 {
            if phrase == Phrase::RoundNearest {
                phrase = led.unwrap_or(Phrase::RoundNearest);
            }
        } else if value_end >= 1 && is_word(toks.get(value_end - 1), "rounded") {
            value_end -= 1;
        } else if phrase != Phrase::RoundNearest {
            return None;
        }
        if value_end <= start {
            return Some(Err(Error::UnexpectedEnd));
        }
        let step = match step_toks {
            [
                Token {
                    tok: Tok::Word(w), ..
                },
            ] => step_word(w).map(|v| vec![Node::Num(Number::from_i64(v))]),
            [
                Token {
                    tok: Tok::Num(k), ..
                },
                Token {
                    tok: Tok::Word(w), ..
                },
            ] if ["th", "st", "nd", "rd"]
                .iter()
                .any(|s| w.eq_ignore_ascii_case(s)) =>
            {
                // Rounding to a fraction of an inch (`to nearest 16th`) shows the answer as a fraction.
                as_fraction = true;
                Some(vec![
                    Node::Num(Number::from_i64(1)),
                    Node::Num(k.clone()),
                    Node::Bin(crate::parser::BinOp::Div),
                ])
            }
            other => parse(other).ok().map(|e| e.nodes().to_vec()),
        };
        let Some(step) = step else {
            return Some(Err(Error::UnexpectedEnd));
        };
        return Some(parse(&toks[start..value_end]).map(|v| {
            let mut rpn = v.nodes().to_vec();
            rpn.extend(step);
            rpn.push(Node::Phrase(phrase));
            if as_fraction {
                rpn.push(Node::Show(crate::value::Shown::Fraction));
            }
            Expr::from_nodes(rpn)
        }));
    }
    // `x rounded`, `x rounded up`, `x rounded down`
    for (tail, func) in [
        (&["rounded", "up"][..], Func::Ceil),
        (&["rounded", "down"], Func::Floor),
        (&["rounded"], Func::Round),
    ] {
        if n > tail.len() && words_at(toks, n - tail.len(), tail) {
            return Some(call(&[&toks[..n - tail.len()]], func));
        }
    }
    None
}

/// The runs between the given words, which must all appear at the top level, in this order.
fn between<'a>(toks: &'a [Token], words: &[&str]) -> Option<Vec<&'a [Token]>> {
    let mut parts = Vec::new();
    let mut from = 0;
    for w in words {
        let at = from + depth_zero_word(&toks[from..], w)?;
        parts.push(&toks[from..at]);
        from = at + 1;
    }
    parts.push(&toks[from..]);
    parts.iter().all(|p| !p.is_empty()).then_some(parts)
}

fn call_with(parts: &[&[Token]], extra: Vec<Node>, func: Func) -> Result<Expr, Error> {
    let mut rpn = rpn_of(parts)?;
    let count = parts.len() + extra.len();
    rpn.extend(extra);
    rpn.push(Node::Call(func, count));
    Ok(Expr::from_nodes(rpn))
}

fn number_node(n: i64) -> Node {
    Node::Num(Number::from_i64(n))
}

/// A trailing `compounding monthly`, as the part before it and the number of periods a year.
fn compounding(toks: &[Token]) -> (&[Token], i64) {
    let n = toks.len();
    if n >= 3 && (is_word(toks.get(n - 2), "compounding") || is_word(toks.get(n - 2), "compounded"))
    {
        let per_year = [
            ("annually", 1),
            ("yearly", 1),
            ("annual", 1),
            ("semiannually", 2),
            ("semi-annually", 2),
            ("biannually", 2),
            ("quarterly", 4),
            ("monthly", 12),
            ("weekly", 52),
            ("daily", 365),
        ]
        .iter()
        .find(|(w, _)| is_word(toks.get(n - 1), w))
        .map(|(_, p)| *p);
        if let Some(p) = per_year {
            return (&toks[..n - 2], p);
        }
    }
    (toks, 1)
}

/// The three parts of `1,000 after 3 years at 7%` or `1,000 for 3 years at 7%`.
fn after_or_for(body: &[Token]) -> Option<Vec<&[Token]>> {
    between(body, &["after", "at"]).or_else(|| between(body, &["for", "at"]))
}

fn finance_forms(toks: &[Token]) -> Option<Result<Expr, Error>> {
    // `daily interest on $1k for 3 years at 4%`: the interest spread over the days (or hours, weeks, months, years).
    // `daily interest on $1M over 30 years at 6.5%` is the loan form below.
    for (code, names) in [
        (1, &["hourly"][..]),
        (2, &["daily"]),
        (3, &["weekly"]),
        (4, &["monthly"]),
        (5, &["yearly", "annual", "annually"]),
    ] {
        if names.iter().any(|n| is_word(toks.first(), n)) && words_at(toks, 1, &["interest", "on"])
        {
            let (body, per_year) = compounding(&toks[3..]);
            if let Some(parts) = after_or_for(body) {
                return Some(call_with(
                    &parts,
                    vec![number_node(per_year), number_node(code)],
                    Func::InterestEach,
                ));
            }
        }
    }
    if words_at(toks, 0, &["interest", "on"]) {
        let (body, per_year) = compounding(&toks[2..]);
        let parts = after_or_for(body)?;
        return Some(call_with(
            &parts,
            vec![number_node(per_year)],
            Func::InterestOn,
        ));
    }
    if words_at(toks, 0, &["present", "value", "of"]) {
        let (body, per_year) = compounding(&toks[3..]);
        let parts = after_or_for(body)?;
        return Some(call_with(
            &parts,
            vec![number_node(per_year)],
            Func::PresentValue,
        ));
    }
    if words_at(toks, 0, &["annual", "return", "on"]) {
        let parts = between(&toks[3..], &["invested", "returned"])?;
        let time = parts[2].strip_prefix(
            &[Token {
                tok: Tok::Word("after".into()),
                at: 0,
            }][..],
        );
        let after = parts[2]
            .first()
            .filter(|t| matches!(&t.tok, Tok::Word(w) if w.eq_ignore_ascii_case("after")));
        let _ = time;
        return Some(match after {
            Some(_) if parts[2].len() > 1 => {
                call(&[parts[0], parts[1], &parts[2][1..]], Func::AnnualReturn)
            }
            _ => Err(Error::UnexpectedEnd),
        });
    }
    // `investment required for $10k/month at 5%`, `deposit needed for $42k/year at 7.5%`.
    for lead in [
        &["investment", "required", "for"][..],
        &["investment", "needed", "for"],
        &["deposit", "required", "for"],
        &["deposit", "needed", "for"],
    ] {
        if words_at(toks, 0, lead) {
            let parts = between(&toks[3..], &["at"])?;
            return Some(call(&parts, Func::InvestmentRequired));
        }
    }
    for (period, code) in [("daily", 1), ("monthly", 2), ("annual", 3), ("total", 4)] {
        for (nouns, func) in [
            (&["repayment"][..], Func::Repayment),
            (&["payment"], Func::Repayment),
            (&["interest"], Func::LoanInterest),
            (&["interest", "paid"], Func::LoanInterest),
        ] {
            let on = 1 + nouns.len();
            if is_word(toks.first(), period)
                && words_at(toks, 1, nouns)
                && is_word(toks.get(on), "on")
            {
                let parts = between(&toks[on + 1..], &["over", "at"])?;
                return Some(call_with(&parts, vec![number_node(code)], func));
            }
        }
    }
    if words_at(toks, 0, &["time", "from"]) {
        return time_to_reach(&toks[2..]);
    }
    if words_at(toks, 0, &["growth", "per"]) {
        let parts = between(&toks[2..], &["from", "to", "over"])?;
        return Some(call_with(
            &[parts[1], parts[2], parts[0], parts[3]],
            vec![],
            Func::GrowthRate,
        ));
    }
    if let Some(i) = depth_zero_word(toks, "invested") {
        let tail = &toks[i + 1..];
        if depth_zero_word(tail, "returned").is_some_and(|r| r + 1 == tail.len())
            && i > 0
            && tail.len() > 1
        {
            return Some(call(
                &[&toks[..i], &tail[..tail.len() - 1]],
                Func::ReturnMultiple,
            ));
        }
    }
    // `20k after 6 months at 10% per month`, and `$1,000 after 3 years at 7% compounding monthly`.
    let (body, per_year) = compounding(toks);
    let parts = after_or_for(body)?;
    let (start, time, rate) = (parts[0], parts[1], parts[2]);
    if let Some(per) = depth_zero_word(rate, "per") {
        if per == 0 || per + 1 >= rate.len() {
            return Some(Err(Error::UnexpectedEnd));
        }
        return Some(call_with(
            &[start, time, &rate[..per], &rate[per + 1..]],
            vec![],
            Func::GrowPerPeriod,
        ));
    }
    Some(call_with(
        &[start, time, rate],
        vec![number_node(per_year)],
        Func::Compound,
    ))
}

/// `time from $20k to $40k at $2k every month in months`, `time from 20k to 100k at 10% growth per month`.
fn time_to_reach(rest: &[Token]) -> Option<Result<Expr, Error>> {
    let (rest, target) = match depth_zero_word(rest, "in") {
        Some(i) if i + 1 < rest.len() => (
            &rest[..i],
            crate::phrase::parse_unit_run(&rest[i + 1..]).ok(),
        ),
        _ => (rest, None),
    };
    let parts = between(rest, &["to", "at"])?;
    let (from, to, how) = (parts[0], parts[1], parts[2]);
    let expr = if let Some(parts) = between(how, &["every"]) {
        call(&[from, to, parts[0], parts[1]], Func::TimeBySteps)
    } else {
        let growth = depth_zero_word(how, "growth")?;
        let per = growth + 1;
        let (rate, period) = (&how[..growth], &how[per + 1..]);
        if growth == 0 || !is_word(how.get(per), "per") || period.is_empty() {
            return Some(Err(Error::UnexpectedEnd));
        }
        call(&[from, to, rate, period], Func::TimeByGrowth)
    };
    Some(expr.map(|e| match target {
        Some(unit) => e.converted_to(unit),
        None => e,
    }))
}

/// `20/5 as multiplier`, `50 as x of 5`, `100 is what multiple of 50`, `2 as multiplier on 1`, `1 as x off 2`, `50 to 75 is what x`, `20 to 40 as x`.
fn multiplier_forms(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let n = toks.len();
    let is_x = |t: Option<&Token>| {
        is_word(t, "multiplier")
            || is_word(t, "multiple")
            || is_word(t, "x")
            || matches!(t.map(|t| &t.tok), Some(Tok::Star))
    };
    // `<A> to <B> is what x` and `<A> to <B> as x`
    let change = if n >= 6
        && is_x(toks.get(n - 1))
        && is_word(toks.get(n - 2), "what")
        && is_word(toks.get(n - 3), "is")
    {
        Some(&toks[..n - 3])
    } else if n >= 4 && is_x(toks.get(n - 1)) && is_word(toks.get(n - 2), "as") {
        Some(&toks[..n - 2])
    } else {
        None
    };
    if let Some(body) = change
        && let Some(parts) = between(body, &["to"])
    {
        return Some(call_with_phrase(&parts, Phrase::MultiplierChange));
    }
    for (word, phrase) in [
        ("of", Phrase::MultiplierOf),
        ("on", Phrase::MultiplierOn),
        ("off", Phrase::MultiplierOff),
    ] {
        if let Some(at) = depth_zero_word(toks, "as")
            && is_x(toks.get(at + 1))
            && is_word(toks.get(at + 2), word)
            && at > 0
            && at + 3 < n
        {
            return Some(call_with_phrase(&[&toks[..at], &toks[at + 3..]], phrase));
        }
        // `100 is what multiple of 50`: the same as `100 as x of 50`.
        if let Some(at) = depth_zero_word(toks, "is")
            && is_word(toks.get(at + 1), "what")
            && is_x(toks.get(at + 2))
            && is_word(toks.get(at + 3), word)
            && at > 0
            && at + 4 < n
        {
            return Some(call_with_phrase(&[&toks[..at], &toks[at + 4..]], phrase));
        }
    }
    if n >= 3 && is_x(toks.get(n - 1)) && is_word(toks.get(n - 2), "as") {
        return Some(call_with_phrase(&[&toks[..n - 2]], Phrase::AsMultiplier));
    }
    None
}

fn call_with_phrase(parts: &[&[Token]], phrase: Phrase) -> Result<Expr, Error> {
    let mut rpn = rpn_of(parts)?;
    rpn.push(Node::Phrase(phrase));
    Ok(Expr::from_nodes(rpn))
}

/// `30 hours at $30/hour`, `time to upload 3GB at 10 MB/s`, `$24 a day for a year`.
fn rate_forms(toks: &[Token]) -> Option<Result<Expr, Error>> {
    // `$24 a day for a year`: a rate, then how long it runs.
    if let Some(f) = depth_zero_word(toks, "for") {
        let after_a_rate = f >= 2
            && matches!(toks[f - 1].tok, Tok::Unit(_))
            && matches!(toks[f - 2].tok, Tok::Slash)
            && f + 1 < toks.len();
        if after_a_rate {
            return Some(rpn_of(&[&toks[..f], &toks[f + 1..]]).map(|mut rpn| {
                rpn.push(Node::Bin(crate::parser::BinOp::Mul));
                Expr::from_nodes(rpn)
            }));
        }
    }
    let converts = ["in", "to", "as", "into", "cast"];
    let at = depth_zero_word(toks, "at")?;
    if converts
        .iter()
        .any(|w| depth_zero_word(&toks[..at], w).is_some())
        && !words_at(toks, 0, &["time", "to"])
    {
        return None;
    }
    let start = if words_at(toks, 0, &["time", "to"]) {
        // Skip the words that say what is being timed, up to the first number.
        toks[2..at]
            .iter()
            .position(|t| matches!(t.tok, Tok::Num(_) | Tok::Unit(_) | Tok::Value(_)))?
            + 2
    } else {
        0
    };
    if start >= at || at + 1 >= toks.len() {
        return None;
    }
    Some(call(&[&toks[start..at], &toks[at + 1..]], Func::AtRate))
}

fn is_unit_run(toks: &[Token]) -> bool {
    !toks.is_empty()
        && matches!(toks[0].tok, Tok::Unit(_))
        && toks
            .iter()
            .all(|t| matches!(t.tok, Tok::Unit(_) | Tok::Star | Tok::Slash))
}

/// `meters in 10 km`, `days in 3 weeks`, `seconds in a day`: how many of the unit on the left the quantity on
/// the right holds.
fn unit_in_quantity(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let at = depth_zero_word(toks, "in")?;
    let (left, rest) = (&toks[..at], &toks[at + 1..]);
    if !is_unit_run(left) || rest.is_empty() {
        return None;
    }
    let target = crate::phrase::parse_unit_run(left).ok()?;
    fn one_of(run: &[Token]) -> Option<&[Token]> {
        let a_or_one =
            is_word(run.first(), "a") || is_word(run.first(), "an") || is_word(run.first(), "one");
        (a_or_one && is_unit_run(&run[1..])).then(|| &run[1..])
    }
    let rpn = match one_of(rest) {
        Some(unit) => {
            crate::phrase::parse_unit_run(unit).map(|u| vec![number_node(1), Node::Unit(u)])
        }
        None => parse(rest).map(|e| e.nodes().to_vec()),
    };
    Some(rpn.map(|mut rpn| {
        rpn.push(Node::Convert(target));
        Expr::from_nodes(rpn)
    }))
}

/// `20% is 500, what is 750` and `if 20 is 30%, what is 60%`: a stands to b as the answer stands to c.
fn proportion_what(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let start = usize::from(is_word(toks.first(), "if"));
    let toks = &toks[start..];
    let first = depth_zero_word(toks, "is")?;
    let what = depth_zero_word(toks, "what")?;
    // `a is to b as what is to c` is the other proportion form.
    let ratio_form = is_word(toks.get(first + 1), "to") || is_word(toks.get(what + 2), "to");
    if ratio_form
        || what <= first + 1
        || !is_word(toks.get(what + 1), "is")
        || toks.len() <= what + 2
    {
        return None;
    }
    let a = &toks[..first];
    let mut b = &toks[first + 1..what];
    if matches!(b.last().map(|t| &t.tok), Some(Tok::Comma)) {
        b = &b[..b.len() - 1];
    }
    let c = &toks[what + 2..];
    if a.is_empty() || b.is_empty() {
        return None;
    }
    Some(call(&[a, b, c], Func::Proportion))
}

/// `5% is 1 in what`: one in twenty.
fn in_what(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let n = toks.len();
    if n < 5 || !is_word(toks.last(), "what") || !is_word(toks.get(n - 2), "in") {
        return None;
    }
    let first = depth_zero_word(toks, "is")?;
    if first == 0 || first + 1 >= n - 2 {
        return None;
    }
    Some(call(
        &[&toks[..first], &toks[first + 1..n - 2]],
        Func::InWhat,
    ))
}

/// `1 cm in px at 326 ppi`: pixels need a density.
fn at_density(toks: &[Token]) -> Option<Result<Expr, Error>> {
    let n = toks.len();
    if n < 6 || !(is_word(toks.last(), "ppi") || is_word(toks.last(), "dpi")) {
        return None;
    }
    let Tok::Num(density) = &toks[n - 2].tok else {
        return None;
    };
    if !is_word(toks.get(n - 3), "at") {
        return None;
    }
    let head = &toks[..n - 3];
    let in_at = (0..head.len()).rev().find(|k| {
        is_word(head.get(*k), "in") || is_word(head.get(*k), "to") || is_word(head.get(*k), "as")
    })?;
    let (value, target) = (&head[..in_at], &head[in_at + 1..]);
    if value.is_empty() || !is_unit_run(target) {
        return None;
    }
    let unit = crate::phrase::parse_unit_run(target).ok()?;
    Some(parse(value).map(|e| {
        let mut rpn = e.nodes().to_vec();
        rpn.push(Node::Num(density.clone()));
        rpn.push(number_node(1));
        rpn.push(Node::Unit(unit));
        rpn.push(Node::Call(Func::AtDensity, 3));
        Expr::from_nodes(rpn)
    }))
}
