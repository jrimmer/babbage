//! Forms written in words: `average of 1, 2 and 3`, `clamp 26 between 5 and 25`, `x to nearest 10`.
//!
//! Each form is matched on the words at the start or end of a line and rewritten into the same
//! reverse-Polish nodes the symbolic syntax produces, so they evaluate through one path.

use crate::error::Error;
use crate::functions::Func;
use crate::lexer::{Tok, Token};
use crate::number::Number;
use crate::parser::{Expr, Node, Phrase, parse};

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
    split_at_depth_zero(toks, |t| {
        matches!(t.tok, Tok::Comma) || is_word(Some(t), "and")
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
        rpn.extend(parse(p)?.nodes().iter().cloned());
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
    (&["larger"], Func::Max),
    (&["greater"], Func::Max),
    (&["smaller"], Func::Min),
    (&["lesser"], Func::Min),
    (&["half"], Func::Half),
    (&["square", "root"], Func::Sqrt),
    (&["cube", "root"], Func::Cbrt),
    (&["standard", "deviation"], Func::StdDev),
];

/// Leading forms with no `of`: `midpoint between a and b`.
const BETWEEN_FORMS: &[(&[&str], Func)] = &[
    (&["midpoint", "between"], Func::Midpoint),
    (&["random", "number", "between"], Func::Random),
];

/// Words that begin a form, so the sheet layer must not treat them as skippable prose.
pub fn is_form_word(word: &str) -> bool {
    const WORDS: &[&str] = &[
        "total",
        "sum",
        "average",
        "avg",
        "mean",
        "count",
        "median",
        "gcd",
        "lcm",
        "larger",
        "greater",
        "smaller",
        "lesser",
        "half",
        "midpoint",
        "between",
        "random",
        "number",
        "clamp",
        "from",
        "remainder",
        "permutation",
        "permutations",
        "combination",
        "combinations",
        "nearest",
        "rounded",
        "up",
        "down",
        "dp",
        "decimal",
        "decimals",
        "places",
        "digits",
        "dec",
        "th",
        "st",
        "nd",
        "rd",
        "hundred",
        "thousand",
        "million",
        "ten",
        "base",
        "power",
        "root",
        "square",
        "cube",
        "standard",
        "deviation",
        "what",
        "the",
    ];
    WORDS.iter().any(|w| w.eq_ignore_ascii_case(word))
}

/// The expression a line written in words stands for, or None when it is not one of these forms.
pub fn parse_words(toks: &[Token]) -> Option<Result<Expr, Error>> {
    list_form(toks)
        .or_else(|| root_and_log(toks))
        .or_else(|| clamp(toks))
        .or_else(|| remainder(toks))
        .or_else(|| count_form(toks))
        .or_else(|| proportion(toks))
        .or_else(|| as_number(toks))
        .or_else(|| rounding(toks))
}

fn list_form(toks: &[Token]) -> Option<Result<Expr, Error>> {
    for (names, func) in LIST_FORMS {
        if words_at(toks, 0, names)
            && is_word(toks.get(names.len()), "of")
            && toks.len() > names.len() + 1
        {
            return Some(call(&list_parts(&toks[names.len() + 1..]), *func));
        }
    }
    for (names, func) in BETWEEN_FORMS {
        if words_at(toks, 0, names) && toks.len() > names.len() {
            return Some(call(&list_parts(&toks[names.len()..]), *func));
        }
    }
    None
}

/// `root 5 of 100`, `log 20 base 4`, `81 is 9 to what power`.
fn root_and_log(toks: &[Token]) -> Option<Result<Expr, Error>> {
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
    if a.is_empty() || b.is_empty() || num1.is_empty() || num2.is_empty() || den.is_empty() {
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
        if value_end >= 1 && is_word(toks.get(value_end - 1), "up") {
            phrase = Phrase::RoundUp;
            value_end -= 1;
        } else if value_end >= 1 && is_word(toks.get(value_end - 1), "down") {
            phrase = Phrase::RoundDown;
            value_end -= 1;
        }
        if value_end >= 1 && is_word(toks.get(value_end - 1), "rounded") {
            value_end -= 1;
        } else if phrase != Phrase::RoundNearest {
            return None;
        }
        if value_end == 0 {
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
        return Some(parse(&toks[..value_end]).map(|v| {
            let mut rpn = v.nodes().to_vec();
            rpn.extend(step);
            rpn.push(Node::Phrase(phrase));
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
