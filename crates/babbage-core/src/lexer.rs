use num_bigint::BigInt;
use num_rational::BigRational;

use crate::error::Error;
use crate::number::Number;
use crate::parser::BinOp;
use crate::units::{UnitExpr, currency_prefix, lookup, superscript_digit};
use crate::value::Value;

pub const MAX_INPUT_BYTES: usize = 65_536;
pub const MAX_EXPONENT: i64 = 100_000;

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Num(Number),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Comma,
    /// `01:02:03:04`, which only becomes a length of time once the line's frame rate is known.
    Timecode(i64, i64, i64, i64),
    /// The rest of a line after `as` when it is written like a date format: `EEEE, MMM d, yyyy`.
    Format(String),
    /// A comparison, logical or bitwise operator spelled with symbols.
    Op(BinOp),
    /// The word `mod`.
    Mod,
    LParen,
    RParen,
    Word(String),
    Unit(UnitExpr),
    /// A value already worked out elsewhere: a variable or a reference to another line.
    Value(Value),
    /// A parenthesised phrase form such as `(20 as %)`, already parsed: it stands where one operand would.
    Group(crate::parser::Expr),
    /// `attached` is true when `%` directly follows a number or `)`, as in `10%`.
    Percent {
        attached: bool,
    },
}

impl Tok {
    pub fn describe(&self) -> String {
        match self {
            Tok::Num(_) => "number".into(),
            Tok::Plus => "'+'".into(),
            Tok::Minus => "'-'".into(),
            Tok::Star => "'*'".into(),
            Tok::Slash => "'/'".into(),
            Tok::Caret => "'^'".into(),
            Tok::Comma => "','".into(),
            Tok::Format(_) => "date format".into(),
            Tok::Timecode(..) => "timecode".into(),
            Tok::Op(op) => format!("'{op}'"),
            Tok::Mod => "'mod'".into(),
            Tok::LParen => "'('".into(),
            Tok::RParen => "')'".into(),
            Tok::Word(w) => format!("word {w:?}"),
            Tok::Unit(u) => format!("unit {:?}", u.symbol()),
            Tok::Value(_) | Tok::Group(_) => "value".into(),
            Tok::Percent { .. } => "'%'".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub at: usize,
}

/// Tokens plus, for each, the byte offset where its text ends (trailing spaces excluded).
pub fn lex_with_ends(src: &str) -> Result<(Vec<Token>, Vec<usize>), Error> {
    let toks = lex(src)?;
    let mut ends = vec![0; toks.len()];
    let mut i = 0;
    while i < toks.len() {
        let mut j = i;
        while j + 1 < toks.len() && toks[j + 1].at == toks[i].at {
            j += 1;
        }
        let next = toks.get(j + 1).map_or(src.len(), |t| t.at);
        let end = toks[i].at + src[toks[i].at..next].trim_end().len();
        for e in &mut ends[i..=j] {
            *e = end;
        }
        i = j + 1;
    }
    Ok((toks, ends))
}

/// Pairs of units that make one quantity when joined by `and`: `5 feet and 3 inches`, `2 lb and 4 oz`. Other
/// pairs (`between 1 km and 500 m`) are items in a list.
const COMPOUND_QUANTITIES: &[(&str, &str)] = &[
    ("mi", "ft"),
    ("mi", "yd"),
    ("yd", "ft"),
    ("yd", "in"),
    ("ft", "in"),
    ("st", "lb"),
    ("lb", "oz"),
    ("gal", "qt"),
    ("gal", "pint"),
    ("qt", "pint"),
    ("pint", "floz"),
    ("cup", "floz"),
    ("h", "min"),
    ("min", "s"),
    ("day", "h"),
    ("wk", "day"),
];

/// `plus`, `minus`, `times`, `multiplied by`, `divided by`, `to the power of` and `mod` are operators.
fn operator_words(toks: Vec<Token>) -> Vec<Token> {
    let word = |i: usize, w: &str| matches!(toks.get(i).map(|t| &t.tok), Some(Tok::Word(x)) if x.eq_ignore_ascii_case(w));
    let mut out = Vec::with_capacity(toks.len());
    let mut i = 0;
    while i < toks.len() {
        let at = toks[i].at;
        let prev_ends_operand = out.last().is_some_and(|t: &Token| {
            matches!(
                t.tok,
                Tok::Num(_) | Tok::Value(_) | Tok::Unit(_) | Tok::RParen | Tok::Percent { .. }
            )
        });
        let prev_is_for = out.last().is_some_and(
            |t: &Token| matches!(&t.tok, Tok::Word(w) if w.eq_ignore_ascii_case("for")),
        );
        let next_is_unit = matches!(toks.get(i + 1).map(|t| &t.tok), Some(Tok::Unit(_)));
        // `5 feet and 3 inches` is `5 feet 3 inches`: between the parts of a compound quantity `and` only joins;
        // `average of 10 m and 60 m` and `midpoint between 1 km and 500 m` are lists.
        let joins_a_smaller_unit = word(i, "and")
            && matches!(
                (out.last().map(|t| &t.tok), toks.get(i + 1).map(|t| &t.tok), toks.get(i + 2).map(|t| &t.tok)),
                (Some(Tok::Unit(a)), Some(Tok::Num(_)), Some(Tok::Unit(b))) if COMPOUND_QUANTITIES.contains(&(a.symbol().as_str(), b.symbol().as_str()))
            );
        if joins_a_smaller_unit {
            i += 1;
            continue;
        }
        // `pounds & ounces`: an ampersand between two units joins them like `and`.
        let ampersand_between_units = matches!(toks[i].tok, Tok::Op(BinOp::BitAnd))
            && matches!(out.last().map(|t| &t.tok), Some(Tok::Unit(_)))
            && next_is_unit;
        let (tok, used) = if ampersand_between_units {
            (Some(Tok::Word("and".to_string())), 1)
        } else if (word(i, "a") || word(i, "an")) && next_is_unit && prev_ends_operand {
            // `$24 a day` is `$24 per day`.
            (Some(Tok::Slash), 1)
        } else if (word(i, "a") || word(i, "an")) && next_is_unit && prev_is_for {
            // `for a year` is one year.
            (Some(Tok::Num(Number::from_i64(1))), 1)
        } else if word(i, "plus") {
            (Some(Tok::Plus), 1)
        } else if word(i, "minus") {
            (Some(Tok::Minus), 1)
        } else if word(i, "times") {
            (Some(Tok::Star), 1)
        } else if word(i, "mod") {
            (Some(Tok::Mod), 1)
        } else if word(i, "multiplied") && word(i + 1, "by") {
            (Some(Tok::Star), 2)
        } else if word(i, "divided") && word(i + 1, "by") {
            (Some(Tok::Slash), 2)
        } else if word(i, "to") && word(i + 1, "the") && word(i + 2, "power") && word(i + 3, "of") {
            (Some(Tok::Caret), 4)
        } else if word(i, "raised")
            && word(i + 1, "to")
            && word(i + 2, "the")
            && word(i + 3, "power")
            && word(i + 4, "of")
            && prev_ends_operand
        {
            (Some(Tok::Caret), 5)
        } else if word(i, "raised") && word(i + 1, "to") && prev_ends_operand {
            (Some(Tok::Caret), 2)
        } else if word(i, "exponent") && prev_ends_operand {
            (Some(Tok::Caret), 1)
        } else {
            (None, 1)
        };
        match tok {
            Some(tok) => out.push(Token { tok, at }),
            None => out.push(toks[i].clone()),
        }
        i += used;
    }
    out
}

/// Two units written one after the other that measure different things are one unit, their product: `3 W s` is
/// three watt seconds. Two that measure the same thing (`5 feet 3 inches`) stay apart, to be added.
fn unit_products(toks: Vec<Token>) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity(toks.len());
    for t in toks {
        if let Tok::Unit(b) = &t.tok
            && let Some(Token {
                tok: Tok::Unit(a), ..
            }) = out.last()
            && out.len() >= 2
            && matches!(out[out.len() - 2].tok, Tok::Num(_))
            && a.dim() != b.dim()
        {
            let product = a.times(b);
            let last = out.len() - 1;
            out[last].tok = Tok::Unit(product);
            continue;
        }
        out.push(t);
    }
    out
}

/// `1 1/2 pounds` is one and a half pounds: a whole number, then a proper fraction, then a unit.
fn mixed_numbers(toks: Vec<Token>) -> Vec<Token> {
    let whole = |t: Option<&Token>| match t.map(|t| &t.tok) {
        Some(Tok::Num(n)) if n.is_integer() && !n.is_negative() => Some(n.clone()),
        _ => None,
    };
    let mut out = Vec::with_capacity(toks.len());
    let mut i = 0;
    while i < toks.len() {
        let mixed = whole(toks.get(i))
            .zip(whole(toks.get(i + 1)))
            .zip(whole(toks.get(i + 3)));
        if let Some(((a, b), c)) = mixed
            && matches!(toks.get(i + 2).map(|t| &t.tok), Some(Tok::Slash))
            && matches!(toks.get(i + 4).map(|t| &t.tok), Some(Tok::Unit(_)))
            && b.is_positive()
            && b < c
            && let Ok(fraction) = b.div(&c)
            && let Ok(total) = a.add(&fraction)
        {
            out.push(Token {
                tok: Tok::Num(total),
                at: toks[i].at,
            });
            i += 4;
            continue;
        }
        out.push(toks[i].clone());
        i += 1;
    }
    out
}

/// A date moved by several lengths of time written one after another moves by each in turn:
/// `June 5 + 2 months 3 days` is `June 5 + 2 months + 3 days`, `April 1, 2019 - 3 months 5 days` is
/// `April 1, 2019 - 3 months - 5 days`. Months and years move the calendar, the rest a number of days.
fn split_date_shifts(toks: Vec<Token>) -> Vec<Token> {
    let seconds = crate::units::lookup("s").map(|s| s.dim());
    let is_length = |t: Option<&Token>, u: Option<&Token>| {
        matches!(t.map(|t| &t.tok), Some(Tok::Num(_)))
            && matches!(u.map(|t| &t.tok), Some(Tok::Unit(unit)) if Some(unit.dim()) == seconds)
    };
    let mut out = Vec::with_capacity(toks.len());
    let mut i = 0;
    while i < toks.len() {
        let is_date = matches!(&toks[i].tok, Tok::Value(v) if v.kind() == crate::value::Kind::Date);
        let op = toks.get(i + 1).map(|t| t.tok.clone());
        if is_date && matches!(op, Some(Tok::Plus | Tok::Minus)) {
            let mut pairs = 0;
            while is_length(toks.get(i + 2 + 2 * pairs), toks.get(i + 3 + 2 * pairs)) {
                pairs += 1;
            }
            if pairs >= 2 {
                out.push(toks[i].clone());
                for p in 0..pairs {
                    // A repeated operator sits where the token before it ends, so positions never go backwards.
                    let mut operator = toks[i + 1].clone();
                    if p > 0 {
                        operator.at = toks[i + 1 + 2 * p].at;
                    }
                    out.push(operator);
                    out.push(toks[i + 2 + 2 * p].clone());
                    out.push(toks[i + 3 + 2 * p].clone());
                }
                i += 2 + 2 * pairs;
                continue;
            }
        }
        out.push(toks[i].clone());
        i += 1;
    }
    out
}

pub fn lex(src: &str) -> Result<Vec<Token>, Error> {
    // `m` is settled first, so that `2m 25s 10 frames at 24 fps` reads its minutes before the frames are gathered.
    lex_raw(src)
        .map(minutes_beside_time)
        .and_then(resolve_timecodes)
        .map(operator_words)
        .map(mixed_numbers)
        .map(unit_products)
        .map(crate::dates::rewrite)
        .map(split_date_shifts)
        .map(power_words)
}

/// `5 squared` is `5^2` and `2 cubed` is `2^3`, after anything that ends an operand (a number, a quantity, a
/// bracket, a value a name stands for). Elsewhere the word is prose.
pub fn power_words(toks: Vec<Token>) -> Vec<Token> {
    let mut out: Vec<Token> = Vec::with_capacity(toks.len());
    for t in toks {
        let power = match &t.tok {
            Tok::Word(w) if w.eq_ignore_ascii_case("squared") => Some(2),
            Tok::Word(w) if w.eq_ignore_ascii_case("cubed") => Some(3),
            _ => None,
        };
        let after_operand = out.last().is_some_and(|p| {
            matches!(
                p.tok,
                Tok::Num(_) | Tok::Value(_) | Tok::Group(_) | Tok::Unit(_) | Tok::RParen
            )
        });
        match power {
            Some(n) if after_operand => {
                out.push(Token {
                    tok: Tok::Caret,
                    at: t.at,
                });
                out.push(Token {
                    tok: Tok::Num(Number::from_i64(n)),
                    at: t.at,
                });
            }
            _ => out.push(t),
        }
    }
    out
}

/// In a run of quantities written one after another (`3h 5m 10s`), `m` next to a unit of time is minutes.
fn minutes_beside_time(mut toks: Vec<Token>) -> Vec<Token> {
    let (metre, minute, second) = (
        crate::units::lookup("m"),
        crate::units::lookup("min"),
        crate::units::lookup("s"),
    );
    let (Some(metre), Some(minute), Some(second)) = (metre, minute, second) else {
        return toks;
    };
    let time = second.dim();
    let is_time =
        |t: Option<&Token>| matches!(t.map(|t| &t.tok), Some(Tok::Unit(u)) if u.dim() == time);
    let is_num = |t: Option<&Token>| matches!(t.map(|t| &t.tok), Some(Tok::Num(_)));
    for i in 1..toks.len() {
        let is_metre = matches!(&toks[i].tok, Tok::Unit(u) if *u == metre);
        if !is_metre || !is_num(toks.get(i - 1)) {
            continue;
        }
        let after_time = i >= 3 && is_time(toks.get(i - 2)) && is_num(toks.get(i - 3));
        let before_time = is_num(toks.get(i + 1)) && is_time(toks.get(i + 2));
        if after_time || before_time {
            toks[i].tok = Tok::Unit(minute.clone());
        }
    }
    toks
}

fn lex_raw(src: &str) -> Result<Vec<Token>, Error> {
    if src.len() > MAX_INPUT_BYTES {
        return Err(Error::InputTooLong);
    }
    let chars: Vec<(usize, char)> = src.char_indices().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let (at, c) = chars[i];
        let simple = |tok: Tok, out: &mut Vec<Token>| out.push(Token { tok, at });
        match c {
            c if c.is_whitespace() => {}
            _ if currency_amount_at(src, &chars, i).is_some() => {
                let (unit, symbol_chars) =
                    currency_amount_at(src, &chars, i).expect("checked by the guard");
                let (n, next) = scan_number(src, &chars, i + symbol_chars, true)?;
                out.push(Token {
                    tok: Tok::Num(n),
                    at,
                });
                out.push(Token {
                    tok: Tok::Unit(unit),
                    at,
                });
                i = next;
                continue;
            }
            '@' => simple(Tok::Word("at".into()), &mut out),
            _ if bare_currency_at(src, &chars, i).is_some() => {
                let (unit, symbol_chars) =
                    bare_currency_at(src, &chars, i).expect("checked by the guard");
                out.push(Token {
                    tok: Tok::Unit(unit),
                    at,
                });
                i += symbol_chars;
                continue;
            }
            '0' if radix_prefix(&chars, i).is_some() => {
                let (radix, digits_from) = radix_prefix(&chars, i).expect("checked by the guard");
                let is_digit = |k: usize| {
                    chars
                        .get(k)
                        .is_some_and(|(_, d)| d.is_digit(u32::from(radix)))
                };
                let mut end = digits_from;
                while is_digit(end)
                    || (chars.get(end).is_some_and(|(_, c)| *c == '_') && is_digit(end + 1))
                {
                    end += 1;
                }
                let text: String = chars[digits_from..end]
                    .iter()
                    .map(|(_, d)| *d)
                    .filter(|d| *d != '_')
                    .collect();
                let value = num_bigint::BigInt::parse_bytes(text.as_bytes(), u32::from(radix))
                    .ok_or(Error::InvalidNumber { text, at })?;
                let n = Number::from_ratio(value, num_bigint::BigInt::from(1))?;
                let v = Value::plain(n).in_radix(radix)?;
                out.push(Token {
                    tok: Tok::Value(v),
                    at,
                });
                i = end;
                continue;
            }
            '0'..='9' if numeric_date_at(&chars, i).is_some() => {
                let (v, next) = numeric_date_at(&chars, i).expect("checked by the guard");
                out.push(Token {
                    tok: Tok::Value(v),
                    at,
                });
                i = next;
                continue;
            }
            // Text shaped like a date that is not one (`2019-02-30`) is plain arithmetic, as in the owner's
            // the reference app, where it is `2019 - 02 - 30`.
            '0'..='9' if iso_shape_at(&chars, i) && iso_at(&chars, i).is_some() => {
                let (v, next) = iso_at(&chars, i).expect("checked by the guard");
                out.push(Token {
                    tok: Tok::Value(v),
                    at,
                });
                i = next;
                continue;
            }
            '0'..='9' if timecode_at(&chars, i).is_some() => {
                let (tok, next) = timecode_at(&chars, i).expect("checked by the guard");
                out.push(Token { tok, at });
                i = next;
                continue;
            }
            '0'..='9' if clock_at(&chars, i, !after_zone_sign(&out)).is_some() => {
                let (v, next) =
                    clock_at(&chars, i, !after_zone_sign(&out)).expect("checked by the guard");
                out.push(Token {
                    tok: Tok::Value(v),
                    at,
                });
                i = next;
                continue;
            }
            '0'..='9' => {
                let (n, next) = scan_number(src, &chars, i, false)?;
                if let Some((v, end)) = feet_inches_at(src, &chars, &n, next)? {
                    out.push(Token {
                        tok: Tok::Value(v),
                        at,
                    });
                    i = end;
                    continue;
                }
                if multiplier_x_at(&chars, next) {
                    out.push(Token {
                        tok: Tok::Value(Value::multiplier(n)),
                        at,
                    });
                    i = next + 1;
                    continue;
                }
                out.push(Token {
                    tok: Tok::Num(n),
                    at,
                });
                i = next;
                continue;
            }
            '.' if chars.get(i + 1).is_some_and(|(_, d)| d.is_ascii_digit()) => {
                let (n, next) = scan_number(src, &chars, i, false)?;
                out.push(Token {
                    tok: Tok::Num(n),
                    at,
                });
                i = next;
                continue;
            }
            // `100²` and `(2+3)³`: superscript digits after a number or a bracket are a power.
            c if crate::units::superscript_digit(c).is_some()
                && matches!(
                    out.last().map(|t| &t.tok),
                    Some(Tok::Num(_) | Tok::RParen | Tok::Value(_))
                ) =>
            {
                let mut exponent = 0i64;
                let mut j = i;
                while let Some(d) = chars
                    .get(j)
                    .and_then(|(_, ch)| crate::units::superscript_digit(*ch))
                {
                    exponent = exponent.saturating_mul(10).saturating_add(i64::from(d));
                    j += 1;
                }
                out.push(Token {
                    tok: Tok::Caret,
                    at,
                });
                out.push(Token {
                    tok: Tok::Num(Number::from_i64(exponent)),
                    at,
                });
                i = j;
                continue;
            }
            '+' => simple(Tok::Plus, &mut out),
            '-' | '\u{2013}' | '\u{2212}' => simple(Tok::Minus, &mut out),
            '*' if chars.get(i + 1).is_some_and(|(_, d)| *d == '*') => {
                simple(Tok::Caret, &mut out);
                i += 2;
                continue;
            }
            '*' | '\u{00D7}' | '\u{00B7}' | '\u{22C5}' => simple(Tok::Star, &mut out),
            'x' | 'X' if is_times_x(&chars, i) => simple(Tok::Star, &mut out),
            '/' | '\u{00F7}' | '\u{2215}' => simple(Tok::Slash, &mut out),
            '=' if chars.get(i + 1).is_some_and(|(_, d)| *d == '=') => {
                simple(Tok::Op(BinOp::Eq), &mut out);
                i += 2;
                continue;
            }
            '!' if chars.get(i + 1).is_some_and(|(_, d)| *d == '=') => {
                simple(Tok::Op(BinOp::Ne), &mut out);
                i += 2;
                continue;
            }
            '<' | '>' => {
                let next = chars.get(i + 1).map(|(_, d)| *d);
                let (op, used) = match (c, next) {
                    ('<', Some('<')) => (BinOp::Shl, 2),
                    ('>', Some('>')) => (BinOp::Shr, 2),
                    ('<', Some('=')) => (BinOp::Le, 2),
                    ('>', Some('=')) => (BinOp::Ge, 2),
                    ('<', _) => (BinOp::Lt, 1),
                    _ => (BinOp::Gt, 1),
                };
                simple(Tok::Op(op), &mut out);
                i += used;
                continue;
            }
            '&' | '|' => {
                let doubled = chars.get(i + 1).is_some_and(|(_, d)| *d == c);
                let op = match (c, doubled) {
                    ('&', true) => BinOp::LogicAnd,
                    ('&', false) => BinOp::BitAnd,
                    (_, true) => BinOp::LogicOr,
                    (_, false) => BinOp::BitOr,
                };
                simple(Tok::Op(op), &mut out);
                i += if doubled { 2 } else { 1 };
                continue;
            }
            '≤' => simple(Tok::Op(BinOp::Le), &mut out),
            '≥' => simple(Tok::Op(BinOp::Ge), &mut out),
            '≠' => simple(Tok::Op(BinOp::Ne), &mut out),
            '^' => simple(Tok::Caret, &mut out),
            ',' | ';' => simple(Tok::Comma, &mut out),
            '√' => simple(Tok::Word("sqrt".into()), &mut out),
            '%' => {
                let attached = i > 0
                    && (chars[i - 1].1.is_ascii_digit() || matches!(chars[i - 1].1, ')' | '%'));
                simple(Tok::Percent { attached }, &mut out)
            }
            '°' | '′' | '″' => {
                let word = word_end(&chars, i + 1);
                let joined: String = chars[i..word].iter().map(|(_, ch)| *ch).collect();
                let (unit, end) = match lookup(&joined) {
                    Some(unit) => (unit, word),
                    None => (
                        lookup(&c.to_string()).ok_or(Error::UnexpectedChar { ch: c, at })?,
                        i + 1,
                    ),
                };
                let (unit, end) = with_exponent_suffix(&chars, end, unit);
                out.push(Token {
                    tok: Tok::Unit(unit),
                    at,
                });
                i = end;
                continue;
            }
            c if c.is_alphabetic() => {
                // A note name (`C2`) before `to midi` or `to hz` is a word, not a unit with an exponent.
                if let Some(end) = note_name_before_conversion(&chars, i) {
                    out.push(Token {
                        tok: Tok::Word(chars[i..end].iter().map(|(_, ch)| *ch).collect()),
                        at,
                    });
                    i = end;
                    continue;
                }
                // A function name, with any digits it ends in (`log10`), wins over a unit of the same spelling (`min`).
                let letters_end = word_end(&chars, i);
                // `Valentine's` and `year's` are words with an apostrophe inside, not a unit and a stray mark.
                // A curly apostrophe (`New Year’s`, as the macOS keyboard types it) is the same mark.
                if chars
                    .get(letters_end)
                    .is_some_and(|(_, ch)| matches!(*ch, '\'' | '\u{2019}'))
                    && chars
                        .get(letters_end + 1)
                        .is_some_and(|(_, ch)| ch.is_alphabetic())
                {
                    let end = word_end(&chars, letters_end + 1);
                    let word: String = chars[i..end]
                        .iter()
                        .map(|(_, ch)| if *ch == '\u{2019}' { '\'' } else { *ch })
                        .collect();
                    out.push(Token {
                        tok: Tok::Word(word),
                        at,
                    });
                    i = end;
                    continue;
                }
                let mut name_end = letters_end;
                while chars
                    .get(name_end)
                    .is_some_and(|(_, ch)| ch.is_ascii_digit())
                {
                    name_end += 1;
                }
                let name: String = chars[i..name_end].iter().map(|(_, ch)| *ch).collect();
                let next_open = chars[name_end..]
                    .iter()
                    .find(|(_, ch)| *ch != ' ')
                    .is_some_and(|(_, ch)| *ch == '(');
                let is_call = crate::functions::lookup(&name).is_some()
                    && (next_open || name_end > letters_end);
                // `in²`, `in^2`, `in2`, `in³`: square and cubic inches, though a bare `in` is the word.
                if letters_end - i == 2
                    && chars[i].1.eq_ignore_ascii_case(&'i')
                    && chars[i + 1].1.eq_ignore_ascii_case(&'n')
                    && let Some((exp, end)) = inch_power(&chars, letters_end)
                    && let Some(inch) = crate::units::lookup("inch")
                {
                    out.push(Token {
                        tok: Tok::Unit(inch.with_exponent(exp)),
                        at,
                    });
                    i = end;
                    continue;
                }
                // `6 in` at the end of a line or before an operator is six inches; `6 in cm` is a conversion.
                if letters_end - i == 2
                    && name_end == letters_end
                    && chars[i].1.eq_ignore_ascii_case(&'i')
                    && chars[i + 1].1.eq_ignore_ascii_case(&'n')
                    && matches!(out.last().map(|t| &t.tok), Some(Tok::Num(_)))
                    && {
                        let rest: Vec<char> = chars[letters_end..]
                            .iter()
                            .map(|(_, ch)| *ch)
                            .skip_while(|ch| *ch == ' ')
                            .collect();
                        let next_word: String =
                            rest.iter().take_while(|ch| ch.is_alphabetic()).collect();
                        // Nothing after it, an operator, `x`, or another conversion word (`6 in in cm`).
                        rest.first().is_none_or(|ch| {
                            matches!(ch, ')' | ',' | '+' | '-' | '*' | '/' | '×' | '÷')
                        }) || ["x", "in", "to", "as", "into"]
                            .iter()
                            .any(|w| next_word.eq_ignore_ascii_case(w))
                    }
                    && let Some(inch) = crate::units::lookup("inch")
                {
                    out.push(Token {
                        tok: Tok::Unit(inch),
                        at,
                    });
                    i = letters_end;
                    continue;
                }
                if !is_call && let Some((unit, end)) = scan_unit(&chars, i) {
                    out.push(Token {
                        tok: Tok::Unit(unit),
                        at,
                    });
                    i = end;
                    continue;
                }
                let end = if is_call { name_end } else { letters_end };
                let word: String = chars[i..end].iter().map(|(_, ch)| *ch).collect();
                let truth = match word.to_lowercase().as_str() {
                    "true" => Some(true),
                    "false" => Some(false),
                    _ => None,
                };
                if let Some(b) = truth {
                    out.push(Token {
                        tok: Tok::Value(Value::boolean(b)),
                        at,
                    });
                    i = end;
                    continue;
                }
                if let Some(rate) = crate::settings::tax_rate_named(&word) {
                    out.push(Token {
                        tok: Tok::Value(Value::tax_rate(rate)),
                        at,
                    });
                    i = end;
                    continue;
                }
                let constant = match word.to_lowercase().as_str() {
                    "pi" | "π" => Some(crate::approx::pi()),
                    "tau" | "τ" => Some(crate::approx::tau()),
                    "phi" | "φ" => Some(crate::approx::phi()),
                    _ => None,
                };
                if let Some(c) = constant {
                    out.push(Token {
                        tok: Tok::Value(Value::plain(c).approximate()),
                        at,
                    });
                } else {
                    // A name straight against a bracket (`foo(45)`) is a call of a function that does not
                    // exist, which the reference app shows no answer for; with a space (`price (5)`) it is prose.
                    let glued_to_bracket = chars.get(end).is_some_and(|(_, c)| *c == '(');
                    let connector = ["and", "for", "the", "per", "off", "from", "with"]
                        .iter()
                        .any(|w| word.eq_ignore_ascii_case(w));
                    if glued_to_bracket && !is_call && word.chars().count() >= 3 && !connector {
                        return Err(Error::UnexpectedWord { word, at });
                    }
                    let is_as = word.eq_ignore_ascii_case("as");
                    out.push(Token {
                        tok: Tok::Word(word),
                        at,
                    });
                    let rest_at = chars.get(end).map_or(src.len(), |c| c.0);
                    let rest = src[rest_at..].trim();
                    if is_as && looks_like_date_format(rest) {
                        out.push(Token {
                            tok: Tok::Format(rest.to_string()),
                            at: src.len() - src[rest_at..].trim_start().len(),
                        });
                        i = chars.len();
                        continue;
                    }
                }
                i = end;
                continue;
            }
            // The full-width brackets of Chinese, Korean and Japanese keyboards.
            '(' | '（' => simple(Tok::LParen, &mut out),
            ')' | '）' => simple(Tok::RParen, &mut out),
            ch => return Err(Error::UnexpectedChar { ch, at }),
        }
        i += 1;
    }
    Ok(out)
}

/// A currency symbol standing alone as a unit next to a slash: the `$` of `$/month`.
fn bare_currency_at(src: &str, chars: &[(usize, char)], i: usize) -> Option<(UnitExpr, usize)> {
    let (at, first) = chars[i];
    if first.is_alphanumeric() || first.is_whitespace() {
        return None;
    }
    let (bytes, unit) = currency_prefix(&src[at..])?;
    let symbol_chars = src[at..at + bytes].chars().count();
    let next = chars[i + symbol_chars..]
        .iter()
        .map(|(_, c)| *c)
        .find(|c| !c.is_whitespace());
    let prev = chars[..i]
        .iter()
        .rev()
        .map(|(_, c)| *c)
        .find(|c| !c.is_whitespace());
    (next == Some('/') || prev == Some('/')).then_some((unit, symbol_chars))
}

/// A currency code written before an amount, with or without a space: `USD1K`, `EUR 50`, `CAD3.50`.
fn code_amount_at(chars: &[(usize, char)], i: usize) -> Option<(UnitExpr, usize)> {
    let c = |k: usize| chars.get(k).map(|(_, ch)| *ch);
    if (i..i + 3).any(|k| !c(k).is_some_and(|ch| ch.is_ascii_uppercase()))
        || c(i + 3).is_some_and(char::is_alphabetic)
        || (i > 0 && c(i - 1).is_some_and(char::is_alphanumeric))
    {
        return None;
    }
    let code: String = (i..i + 3).filter_map(c).collect();
    let unit = lookup(&code).filter(|u| u.currency_symbol().is_some())?;
    let mut j = i + 3;
    while c(j) == Some(' ') {
        j += 1;
    }
    let digit_after = c(j).is_some_and(|ch| ch.is_ascii_digit())
        || (c(j) == Some('.') && c(j + 1).is_some_and(|ch| ch.is_ascii_digit()));
    digit_after.then_some((unit, j - i))
}

/// A currency symbol at `i` that is directly followed by a number, as in `$55`, `US$7` or `€.5`.
fn currency_amount_at(src: &str, chars: &[(usize, char)], i: usize) -> Option<(UnitExpr, usize)> {
    if let Some(found) = code_amount_at(chars, i) {
        return Some(found);
    }
    let (at, first) = chars[i];
    if first.is_ascii_digit() || first.is_whitespace() {
        return None;
    }
    let (bytes, unit) = currency_prefix(&src[at..])?;
    let symbol_chars = src[at..at + bytes].chars().count();
    let letters_first = first.is_alphabetic();
    if letters_first && i > 0 && chars[i - 1].1.is_alphanumeric() {
        return None;
    }
    let after = chars.get(i + symbol_chars).map(|(_, c)| *c)?;
    let digit_after = after.is_ascii_digit()
        || (after == '.'
            && chars
                .get(i + symbol_chars + 1)
                .is_some_and(|(_, c)| c.is_ascii_digit()));
    digit_after.then_some((unit, symbol_chars))
}

/// `0x`, `0b` or `0o` followed by at least one digit of that base: the base and where the digits start.
fn radix_prefix(chars: &[(usize, char)], i: usize) -> Option<(u8, usize)> {
    let radix = match chars.get(i + 1).map(|(_, c)| c.to_ascii_lowercase()) {
        Some('x') => 16,
        Some('b') => 2,
        Some('o') => 8,
        _ => return None,
    };
    let first = if chars.get(i + 2).is_some_and(|(_, c)| *c == '_') {
        i + 3
    } else {
        i + 2
    };
    chars
        .get(first)
        .is_some_and(|(_, d)| d.is_digit(u32::from(radix)))
        .then_some((radix, first))
}

/// An `x` right after a number that makes it a multiplier (`1.5x`) and not a times sign: nothing that
/// starts an operand comes after it.
fn multiplier_x_at(chars: &[(usize, char)], x: usize) -> bool {
    let c = |k: usize| chars.get(k).map(|(_, ch)| *ch);
    if !matches!(c(x), Some('x' | 'X')) || c(x + 1).is_some_and(char::is_alphanumeric) {
        return false;
    }
    let lone_zero = x >= 1
        && c(x - 1) == Some('0')
        && (x < 2 || !c(x - 2).is_some_and(|d| d.is_ascii_digit() || d == '.'));
    if lone_zero {
        return false;
    }
    let rest: Vec<char> = chars[x + 1..].iter().map(|(_, ch)| *ch).collect();
    match rest.iter().find(|ch| !ch.is_whitespace()) {
        None => true,
        Some(ch) => {
            matches!(ch, ')' | ',' | '+' | '-' | '\u{2212}' | '/' | '*' | '%')
                || rest
                    .iter()
                    .collect::<String>()
                    .trim_start()
                    .to_lowercase()
                    .starts_with("at ")
        }
    }
}

/// Ten characters written like `2019-04-01`: a date to read as one, so a bad day or month is an error
/// and not a subtraction.
fn iso_shape_at(chars: &[(usize, char)], i: usize) -> bool {
    let c = |k: usize| chars.get(k).map(|(_, ch)| *ch);
    let digit = |k: usize| c(k).is_some_and(|d| d.is_ascii_digit());
    let after_ok = !c(i + 10).is_some_and(|d| d.is_ascii_digit() || d == '.');
    let before_ok =
        i == 0 || !c(i - 1).is_some_and(|p| p.is_alphanumeric() || matches!(p, '.' | '-' | ':'));
    before_ok
        && after_ok
        && (i..i + 4).all(digit)
        && c(i + 4) == Some('-')
        && digit(i + 5)
        && digit(i + 6)
        && c(i + 7) == Some('-')
        && digit(i + 8)
        && digit(i + 9)
}

/// An ISO 8601 date or date and time that starts at `i`: `2019-04-01`, `2019-04-01T15:30:00`,
/// `2019-04-01T15:30:00+11:00`, `2019-04-01T04:30:00Z`.
fn iso_at(chars: &[(usize, char)], i: usize) -> Option<(Value, usize)> {
    use crate::calendar::{self, Civil};
    let c = |k: usize| chars.get(k).map(|(_, ch)| *ch);
    let digits = |from: usize, len: usize| -> Option<i64> {
        (from..from + len).try_fold(0i64, |acc, k| {
            c(k).and_then(|d| d.to_digit(10))
                .map(|d| acc * 10 + i64::from(d))
        })
    };
    // The caller has checked the shape (`iso_shape_at`): four digits, `-`, two, `-`, two, and what surrounds them.
    let (year, month, day) = (digits(i, 4)?, digits(i + 5, 2)?, digits(i + 8, 2)?);
    let civil = Civil {
        year,
        month: month as u32,
        day: day as u32,
    };
    if !calendar::is_valid(civil) {
        return None;
    }
    let mut pos = i + 10;
    let mut seconds = Number::from_i64(0);
    let mut zone = None;
    if matches!(c(pos), Some('T' | 't')) {
        let (h, m) = (digits(pos + 1, 2)?, digits(pos + 4, 2)?);
        if c(pos + 3) != Some(':') || h > 23 || m > 59 {
            return None;
        }
        pos += 6;
        let mut s = 0;
        let mut fraction = None;
        if c(pos) == Some(':') {
            s = digits(pos + 1, 2)?;
            if s > 59 {
                return None;
            }
            pos += 3;
            if c(pos) == Some('.') {
                let len = (pos + 1..)
                    .take_while(|k| c(*k).is_some_and(|d| d.is_ascii_digit()))
                    .count();
                if len > 0 {
                    let text: String = (pos + 1..pos + 1 + len).filter_map(c).collect();
                    fraction = Number::from_decimal_str(&format!("0.{text}")).ok();
                    pos += 1 + len;
                }
            }
        }
        seconds = Number::from_i64(h * 3600 + m * 60 + s);
        if let Some(f) = fraction {
            seconds = seconds.add(&f).ok()?;
        }
        match c(pos) {
            Some('Z' | 'z') => {
                zone = Some(0);
                pos += 1;
            }
            Some(sign @ ('+' | '-')) => {
                let (zh, zm) = (
                    digits(pos + 1, 2)?,
                    if c(pos + 3) == Some(':') {
                        digits(pos + 4, 2)?
                    } else {
                        digits(pos + 3, 2)?
                    },
                );
                let width = if c(pos + 3) == Some(':') { 6 } else { 5 };
                if zh > 23 || zm > 59 {
                    return None;
                }
                let seconds = zh * 3600 + zm * 60;
                zone = Some(if sign == '-' { -seconds } else { seconds });
                pos += width;
            }
            _ => {}
        }
    }
    if c(pos).is_some_and(|ch| ch.is_alphanumeric()) {
        return None;
    }
    let mut local = Number::from_i64(calendar::days_from_civil(civil) * 86_400)
        .add(&seconds)
        .ok()?;
    if let Some(offset) = zone {
        let utc = local.sub(&Number::from_i64(offset)).ok()?;
        local = crate::settings::utc_to_wall(&utc).ok()?;
    }
    local
        .div(&Number::from_i64(86_400))
        .ok()
        .map(|days| (Value::date_at(days, pos - i > 10), pos))
}

/// Where a note name such as `C2`, `F#3` or `Bb4` ends, when `to midi`, `as hz` or the like follows it.
fn note_name_before_conversion(chars: &[(usize, char)], i: usize) -> Option<usize> {
    let c = |k: usize| chars.get(k).map(|(_, ch)| *ch);
    if !c(i).is_some_and(|ch| ('A'..='G').contains(&ch)) {
        return None;
    }
    let mut j = i + 1;
    // A sharp, or a flat when a digit or minus sign follows (`Bb2`).
    let flat = c(j) == Some('b')
        && (c(j + 1) == Some('-') || c(j + 1).is_some_and(|d| d.is_ascii_digit()));
    if c(j) == Some('#') || flat {
        j += 1;
    }
    if c(j) == Some('-') {
        j += 1;
    }
    let digits = (j..)
        .take_while(|k| c(*k).is_some_and(|d| d.is_ascii_digit()))
        .count();
    if !(1..=2).contains(&digits) {
        return None;
    }
    let end = j + digits;
    if c(end).is_some_and(|ch| ch.is_alphanumeric()) {
        return None;
    }
    let word_after = |from: usize| -> (String, usize) {
        let mut k = from;
        while c(k) == Some(' ') {
            k += 1;
        }
        let start = k;
        while c(k).is_some_and(char::is_alphabetic) {
            k += 1;
        }
        (
            (start..k).filter_map(c).collect::<String>().to_lowercase(),
            k,
        )
    };
    let (joiner, after_joiner) = word_after(end);
    let (target, _) = word_after(after_joiner);
    (["to", "as", "in", "into"].contains(&joiner.as_str())
        && ["midi", "hz", "hertz", "frequency"].contains(&target.as_str()))
    .then_some(end)
}

/// `5'` and `5' 6"`: feet, with inches after them. The marks are feet (`'`, `’`) and inches (`"`, `”`); the primes
/// `′` and `″` are the arcminute and arcsecond.
fn feet_inches_at(
    src: &str,
    chars: &[(usize, char)],
    feet: &Number,
    next: usize,
) -> Result<Option<(Value, usize)>, Error> {
    let c = |k: usize| chars.get(k).map(|(_, ch)| *ch);
    // `6"` on its own is six inches.
    if matches!(c(next), Some('"' | '”')) {
        return Ok(Some((Value::inches_shown(feet.clone())?, next + 1)));
    }
    if !matches!(c(next), Some('\'' | '’')) {
        return Ok(None);
    }
    let mut j = next + 1;
    while c(j) == Some(' ') {
        j += 1;
    }
    if c(j).is_some_and(|d| d.is_ascii_digit()) {
        let (inches, after) = scan_number(src, chars, j, false)?;
        if matches!(c(after), Some('"' | '”')) {
            return Ok(Some((Value::feet_inches(feet.clone(), inches)?, after + 1)));
        }
    }
    Ok(Some((
        Value::feet_inches(feet.clone(), Number::from_i64(0))?,
        next + 1,
    )))
}

/// A date written with a two-digit month and day and a four-digit year, joined by `/` or `-`: `04-05-2020`,
/// `4/5/2020`. The order is month first unless the day-first setting says otherwise. A date that does not exist
/// (`13/13/2020`) is left to be arithmetic.
fn numeric_date_at(chars: &[(usize, char)], i: usize) -> Option<(Value, usize)> {
    use crate::calendar::{self, Civil};
    let c = |k: usize| chars.get(k).map(|(_, ch)| *ch);
    if i > 0 && c(i - 1).is_some_and(|p| p.is_alphanumeric() || matches!(p, '.' | '-' | '/' | ':'))
    {
        return None;
    }
    let run = |from: usize| {
        (from..)
            .take_while(|k| c(*k).is_some_and(|d| d.is_ascii_digit()))
            .count()
    };
    let first = run(i);
    let sep = c(i + first).filter(|s| matches!(s, '/' | '-'))?;
    let second = run(i + first + 1);
    if !(1..=2).contains(&first)
        || !(1..=2).contains(&second)
        || c(i + first + 1 + second) != Some(sep)
    {
        return None;
    }
    let year_at = i + first + second + 2;
    if run(year_at) != 4 || c(year_at + 4).is_some_and(|d| d.is_alphanumeric() || d == '.') {
        return None;
    }
    let number = |from: usize, len: usize| -> Option<i64> {
        (from..from + len).try_fold(0, |acc, k| {
            c(k).and_then(|d| d.to_digit(10))
                .map(|d| acc * 10 + i64::from(d))
        })
    };
    let (a, b) = (number(i, first)?, number(i + first + 1, second)?);
    let (month, day) = if crate::settings::day_first() {
        (b, a)
    } else {
        (a, b)
    };
    let civil = Civil {
        year: number(year_at, 4)?,
        month: u32::try_from(month).ok()?,
        day: u32::try_from(day).ok()?,
    };
    calendar::is_valid(civil).then(|| {
        (
            Value::date(Number::from_i64(calendar::days_from_civil(civil))),
            year_at + 4,
        )
    })
}

/// The exponent written straight after `in` (`in²`, `in³`, `in^2`, `in^3`, `in2`, `in3`) and where it ends.
fn inch_power(chars: &[(usize, char)], at: usize) -> Option<(i8, usize)> {
    let c = |k: usize| chars.get(k).map(|(_, ch)| *ch);
    let (exp, len) = match c(at)? {
        '²' => (2, 1),
        '³' => (3, 1),
        '^' => match (c(at + 1)?, c(at + 2)) {
            ('2', next) if !next.is_some_and(|n| n.is_ascii_digit()) => (2, 2),
            ('3', next) if !next.is_some_and(|n| n.is_ascii_digit()) => (3, 2),
            _ => return None,
        },
        d @ ('2' | '3') => (d.to_digit(10)? as i8, 1),
        _ => return None,
    };
    let end = at + len;
    if c(end).is_some_and(|ch| ch.is_alphanumeric()) {
        return None;
    }
    Some((exp, end))
}

/// A video timecode, `01:02:03:04`: hours, minutes, seconds and frames.
fn timecode_at(chars: &[(usize, char)], i: usize) -> Option<(Tok, usize)> {
    let c = |k: usize| chars.get(k).map(|(_, ch)| *ch);
    let digit_run = |from: usize| {
        (from..)
            .take_while(|k| c(*k).is_some_and(|d| d.is_ascii_digit()))
            .count()
    };
    let number = |from: usize, len: usize| -> i64 {
        (from..from + len)
            .filter_map(|k| c(k).and_then(|d| d.to_digit(10)))
            .fold(0i64, |acc, d| acc * 10 + i64::from(d))
    };
    if i > 0 && c(i - 1).is_some_and(|p| p.is_alphanumeric() || matches!(p, '.' | ':' | ',')) {
        return None;
    }
    let hour_len = digit_run(i);
    if !(1..=3).contains(&hour_len) {
        return None;
    }
    let mut pos = i + hour_len;
    let mut fields = vec![number(i, hour_len)];
    for field in 0..3 {
        // Minutes and seconds are two digits; the frames may be written with three (`00:00:00:010`).
        let run = digit_run(pos + 1);
        let width_ok = run == 2 || (field == 2 && run == 3);
        if c(pos) != Some(':') || !width_ok {
            return None;
        }
        fields.push(number(pos + 1, run));
        pos += 1 + run;
    }
    if c(pos).is_some_and(|ch| ch.is_alphanumeric() || ch == '.' || ch == ':')
        || fields[1] > 59
        || fields[2] > 59
    {
        return None;
    }
    Some((
        Tok::Timecode(fields[0], fields[1], fields[2], fields[3]),
        pos,
    ))
}

/// Gives every timecode on the line its frame rate: the `at 30 fps` written anywhere on it, else 24.
fn resolve_timecodes(toks: Vec<Token>) -> Result<Vec<Token>, Error> {
    let fps = crate::units::lookup("fps");
    let frame = crate::units::lookup("frame");
    let is_frame = |t: &Token| matches!((&t.tok, &frame), (Tok::Unit(u), Some(f)) if u == f);
    let is_at = |t: &Token| matches!(&t.tok, Tok::Word(w) if w.eq_ignore_ascii_case("at"));
    let mut rate = Number::from_i64(24);
    let mut skip: Option<usize> = None;
    for k in 0..toks.len().saturating_sub(2) {
        if is_at(&toks[k])
            && let Tok::Num(n) = &toks[k + 1].tok
            && matches!((&toks[k + 2].tok, &fps), (Tok::Unit(u), Some(f)) if u == f)
        {
            rate = n.clone();
            skip = Some(k);
            break;
        }
    }
    // A count of frames written with a rate (`43,440 frames @ 24 fps`, `3h 2m 25s 10 frames at 24 fps`) is a
    // timecode too.
    let counts_frames = toks
        .windows(2)
        .any(|w| matches!(w[0].tok, Tok::Num(_)) && is_frame(&w[1]));
    let frames_with_rate = skip.is_some() && counts_frames;
    if !frames_with_rate && !toks.iter().any(|t| matches!(t.tok, Tok::Timecode(..))) {
        return Ok(toks);
    }
    if !rate.is_integer() || !rate.is_positive() {
        return Err(Error::UnsupportedOperation(
            "a timecode needs a whole number of frames per second",
        ));
    }
    let seconds =
        crate::units::lookup("s").ok_or(Error::UnsupportedOperation("seconds are missing"))?;
    let time_dim = seconds.dim();
    let frame_dim = frame.as_ref().map(|f| f.dim());
    // The run of `number unit` pairs from `k` whose units are time or frames, when it holds frames.
    let frames_run = |k: usize| -> Option<(usize, Number)> {
        let mut total = Number::from_i64(0);
        let (mut at, mut has_frames) = (k, false);
        while let (Some(n), Some(u)) = (toks.get(at), toks.get(at + 1)) {
            let (Tok::Num(n), Tok::Unit(u)) = (&n.tok, &u.tok) else {
                break;
            };
            if is_frame(&toks[at + 1]) {
                has_frames = true;
                total = total.add(&n.div(&rate).ok()?).ok()?;
            } else if u.dim() == time_dim {
                total = total.add(&n.mul(&u.factor().ok()?).ok()?).ok()?;
            } else {
                break;
            }
            at += 2;
        }
        let _ = &frame_dim;
        (has_frames && at > k).then_some((at - k, total))
    };
    let mut out = Vec::with_capacity(toks.len());
    let mut run_left = 0usize;
    for (k, t) in toks.iter().cloned().enumerate() {
        if run_left > 0 {
            run_left -= 1;
            continue;
        }
        if let Some(a) = skip
            && (a..a + 3).contains(&k)
        {
            continue;
        }
        if frames_with_rate
            && matches!(t.tok, Tok::Num(_))
            && let Some((len, total)) = frames_run(k)
        {
            out.push(Token {
                tok: Tok::Value(Value::quantity(total, seconds.clone())?.as_timecode(rate.clone())),
                at: t.at,
            });
            run_left = len - 1;
            continue;
        }
        match t.tok {
            Tok::Timecode(h, m, s, f) => {
                if Number::from_i64(f).cmp(&rate) != std::cmp::Ordering::Less {
                    return Err(Error::InvalidNumber {
                        text: format!("frame {f} at {} frames per second", rate.numer()),
                        at: t.at,
                    });
                }
                let total = Number::from_i64(h * 3600 + m * 60 + s)
                    .add(&Number::from_i64(f).div(&rate)?)?;
                out.push(Token {
                    tok: Tok::Value(
                        Value::quantity(total, seconds.clone())?.as_timecode(rate.clone()),
                    ),
                    at: t.at,
                });
            }
            _ => out.push(t),
        }
    }
    Ok(out)
}

/// True right after `GMT+` or `UTC-`: an offset there is hours and minutes (`GMT+5:30`), so `GMT+5pm` is not
/// the clock time 17:00.
fn after_zone_sign(out: &[Token]) -> bool {
    matches!(
        out,
        [.., Token { tok: Tok::Word(w), .. }, Token { tok: Tok::Plus | Tok::Minus, .. }]
            if w.eq_ignore_ascii_case("gmt") || w.eq_ignore_ascii_case("utc")
    )
}

/// A time of day (`17:30`, `5:30 pm`, `5pm`) or a lap time (`03:04:05`, `00:00:01.5`) that starts at `i`.
/// `am` and `pm` after a number of 1 to 12 mean a time of day, not attometres or picometres.
fn clock_at(chars: &[(usize, char)], i: usize, allow_meridiem: bool) -> Option<(Value, usize)> {
    let c = |k: usize| chars.get(k).map(|(_, ch)| *ch);
    let digit_run = |from: usize| {
        (from..)
            .take_while(|k| c(*k).is_some_and(|d| d.is_ascii_digit()))
            .count()
    };
    let number = |from: usize, len: usize| -> i64 {
        (from..from + len)
            .filter_map(|k| c(k).and_then(|d| d.to_digit(10)))
            .fold(0i64, |acc, d| acc * 10 + i64::from(d))
    };
    if i > 0 && c(i - 1).is_some_and(|p| p.is_alphanumeric() || matches!(p, '.' | ':' | ',')) {
        return None;
    }
    let hour_len = digit_run(i);
    if hour_len == 0 || hour_len > 6 {
        return None;
    }
    let hour = number(i, hour_len);
    let mut pos = i + hour_len;
    let two_digits_below_60 = |at: usize| digit_run(at) == 2 && number(at, 2) < 60;
    let mut minute = None;
    let mut second = None;
    let mut fraction = None;
    if c(pos) == Some(':') && two_digits_below_60(pos + 1) {
        minute = Some(number(pos + 1, 2));
        pos += 3;
        if c(pos) == Some(':') && two_digits_below_60(pos + 1) {
            second = Some(number(pos + 1, 2));
            pos += 3;
            if c(pos) == Some('.') && digit_run(pos + 1) > 0 {
                let len = digit_run(pos + 1);
                fraction = Some((pos + 1, len));
                pos += 1 + len;
            }
        }
    }
    let meridiem_at = if c(pos) == Some(' ') { pos + 1 } else { pos };
    let meridiem = match (c(meridiem_at), c(meridiem_at + 1)) {
        (Some(a), Some(m)) if m.eq_ignore_ascii_case(&'m') && a.eq_ignore_ascii_case(&'a') => {
            Some(false)
        }
        (Some(p), Some(m)) if m.eq_ignore_ascii_case(&'m') && p.eq_ignore_ascii_case(&'p') => {
            Some(true)
        }
        _ => None,
    };
    let ends_word = |at: usize| !c(at).is_some_and(|ch| ch.is_alphanumeric());
    if let Some(pm) = meridiem
        && allow_meridiem
        && ends_word(meridiem_at + 2)
        && hour_len <= 2
        && (1..=12).contains(&hour)
        && fraction.is_none()
    {
        let seconds = (hour % 12 + if pm { 12 } else { 0 }) * 3600
            + minute.unwrap_or(0) * 60
            + second.unwrap_or(0);
        return Some((Value::clock(Number::from_i64(seconds)), meridiem_at + 2));
    }
    if !ends_word(pos) || minute.is_none() {
        return None;
    }
    if second.is_none() {
        return (hour_len <= 2 && hour < 24).then(|| {
            (
                Value::clock(Number::from_i64(hour * 3600 + minute.unwrap_or(0) * 60)),
                pos,
            )
        });
    }
    let mut total = Number::from_i64(hour * 3600 + minute.unwrap_or(0) * 60 + second.unwrap_or(0));
    if let Some((from, len)) = fraction {
        let digits: String = (from..from + len).filter_map(c).collect();
        let part = Number::from_decimal_str(&format!("0.{digits}")).ok()?;
        total = total.add(&part).ok()?;
    }
    let seconds = crate::units::lookup("s")?;
    Value::quantity(total, seconds)
        .ok()
        .map(|v| (v.as_laptime(), pos))
}

/// True for text such as `EEEE, MMM d, yyyy` or `yyyy-MM-dd HH:mm`.
fn looks_like_date_format(text: &str) -> bool {
    let letters_only = text
        .chars()
        .all(|c| c.is_ascii_alphabetic() || " ,.:/-'".contains(c));
    let clock_pattern = text.contains(':') && text.chars().any(|c| "hHms".contains(c));
    letters_only
        && (["yy", "MM", "dd", "EEE", "HH", "hh"]
            .iter()
            .any(|p| text.contains(p))
            || clock_pattern)
}

fn word_end(chars: &[(usize, char)], from: usize) -> usize {
    let mut end = from;
    while chars.get(end).is_some_and(|(_, ch)| ch.is_alphabetic()) {
        end += 1;
    }
    end
}

/// A unit name of one word, or of two words joined by a space or hyphen (`fl oz`, `acre-foot`).
fn scan_unit(chars: &[(usize, char)], start: usize) -> Option<(UnitExpr, usize)> {
    let first = word_end(chars, start);
    let joined = matches!(chars.get(first).map(|(_, c)| *c), Some(' ' | '-'))
        && chars.get(first + 1).is_some_and(|(_, c)| c.is_alphabetic());
    let second = if joined {
        Some(word_end(chars, first + 1))
    } else {
        None
    };
    // A third word, for names like `imperial fluid ounces`.
    let third = second.filter(|&e| {
        matches!(chars.get(e).map(|(_, c)| *c), Some(' ' | '-'))
            && chars.get(e + 1).is_some_and(|(_, c)| c.is_alphabetic())
    });
    let third = third.map(|e| word_end(chars, e + 1));
    for end in third.into_iter().chain(second).chain([first]) {
        let name: String = chars[start..end].iter().map(|(_, c)| *c).collect();
        if let Some(unit) = lookup(&name) {
            return Some(with_exponent_suffix(chars, end, unit));
        }
    }
    None
}

/// Reads an exponent written straight after a unit: `mm³`, `mm^3`, `mm3`, `s^-1`, `s⁻¹`.
fn with_exponent_suffix(chars: &[(usize, char)], at: usize, unit: UnitExpr) -> (UnitExpr, usize) {
    let c = |i: usize| chars.get(i).map(|(_, ch)| *ch);
    let mut i = at;
    let negative_caret = c(i) == Some('^') && c(i + 1) == Some('-');
    if c(i) == Some('^') {
        i += if negative_caret { 2 } else { 1 };
        let start = i;
        while c(i).is_some_and(|d| d.is_ascii_digit()) {
            i += 1;
        }
        if i == start {
            return (unit, at);
        }
        let n: i8 = chars[start..i]
            .iter()
            .map(|(_, d)| *d)
            .collect::<String>()
            .parse()
            .unwrap_or(i8::MAX);
        return (unit.with_exponent(if negative_caret { -n } else { n }), i);
    }
    if c(i) == Some('⁻') || c(i).and_then(superscript_digit).is_some() {
        let negative = c(i) == Some('⁻');
        if negative {
            i += 1;
        }
        let mut n: i8 = 0;
        let start = i;
        while let Some(d) = c(i).and_then(superscript_digit) {
            n = n.saturating_mul(10).saturating_add(d);
            i += 1;
        }
        if i == start {
            return (unit, at);
        }
        return (unit.with_exponent(if negative { -n } else { n }), i);
    }
    if c(i).is_some_and(|d| d.is_ascii_digit()) {
        let start = i;
        while c(i).is_some_and(|d| d.is_ascii_digit()) {
            i += 1;
        }
        let n: i8 = chars[start..i]
            .iter()
            .map(|(_, d)| *d)
            .collect::<String>()
            .parse()
            .unwrap_or(i8::MAX);
        return (unit.with_exponent(n), i);
    }
    (unit, at)
}

fn is_times_x(chars: &[(usize, char)], i: usize) -> bool {
    let before = i.checked_sub(1).map(|p| chars[p].1);
    let after = chars.get(i + 1).map(|p| p.1);
    !before.is_some_and(char::is_alphabetic) && !after.is_some_and(char::is_alphabetic)
}

fn digit_at(chars: &[(usize, char)], i: usize) -> bool {
    chars.get(i).is_some_and(|(_, c)| c.is_ascii_digit())
}

/// Appends the digits starting at `j`, allowing one underscore between two digits (`1_000`), and returns
/// where they end.
fn take_digits(chars: &[(usize, char)], mut j: usize, out: &mut String) -> usize {
    loop {
        if digit_at(chars, j) {
            out.push(chars[j].1);
        } else if chars.get(j).is_some_and(|(_, c)| *c == '_') && digit_at(chars, j + 1) {
        } else {
            return j;
        }
        j += 1;
    }
}

fn scan_number(
    src: &str,
    chars: &[(usize, char)],
    start: usize,
    currency: bool,
) -> Result<(Number, usize), Error> {
    let at = chars[start].0;
    let bad = |end: usize| Error::InvalidNumber {
        text: src[at..chars.get(end).map_or(src.len(), |c| c.0)].to_string(),
        at,
    };

    let format = crate::settings::number_format();
    let mut j = start;
    let mut mantissa = String::new();
    j = take_digits(chars, j, &mut mantissa);
    let first_group = mantissa.len();
    while chars.get(j).is_some_and(|(_, c)| format.reads_as_group(*c))
        && digit_at(chars, j + 1)
        && digit_at(chars, j + 2)
        && digit_at(chars, j + 3)
        && !digit_at(chars, j + 4)
    {
        if first_group == 0 || first_group > 3 {
            return Err(bad(j + 4));
        }
        for k in 1..=3 {
            mantissa.push(chars[j + k].1);
        }
        j += 4;
    }
    if format == crate::format::NumberFormat::WesternEurope
        && chars.get(j).is_some_and(|(_, c)| *c == '.')
        && digit_at(chars, j + 1)
    {
        let mut end = j + 1;
        while digit_at(chars, end) {
            end += 1;
        }
        return Err(bad(end));
    }

    let mut frac_len: i64 = 0;
    if chars
        .get(j)
        .is_some_and(|(_, c)| format.reads_as_decimal(*c))
        && digit_at(chars, j + 1)
    {
        j += 1;
        let mut fraction = String::new();
        j = take_digits(chars, j, &mut fraction);
        frac_len = fraction.len() as i64;
        mantissa.push_str(&fraction);
        if chars
            .get(j)
            .is_some_and(|(_, c)| format.reads_as_decimal(*c))
            && digit_at(chars, j + 1)
        {
            let mut end = j + 1;
            while digit_at(chars, end) {
                end += 1;
            }
            return Err(bad(end));
        }
    }

    let mut exp: i64 = 0;
    if chars.get(j).is_some_and(|(_, c)| matches!(c, 'e' | 'E')) {
        let sign_len = usize::from(
            chars
                .get(j + 1)
                .is_some_and(|(_, c)| matches!(c, '+' | '-')),
        );
        if digit_at(chars, j + 1 + sign_len) {
            let negative = sign_len == 1 && chars[j + 1].1 == '-';
            let mut k = j + 1 + sign_len;
            let mut value: i64 = 0;
            while digit_at(chars, k) {
                value = value
                    .saturating_mul(10)
                    .saturating_add(i64::from(chars[k].1.to_digit(10).unwrap_or(0)));
                if value > MAX_EXPONENT {
                    return Err(Error::Overflow);
                }
                k += 1;
            }
            exp = if negative { -value } else { value };
            j = k;
        }
    }

    let mut magnitude: i64 = 0;
    let suffix = |c: char, next: Option<char>, after_next: Option<char>| -> Option<(i64, usize)> {
        let ends = |x: Option<char>| !x.is_some_and(char::is_alphanumeric);
        match c {
            'k' | 'K' if currency || c == 'k' => ends(next).then_some((3, 1)),
            'M' | 'm' if currency && next == Some('n') && ends(after_next) => Some((6, 2)),
            'M' => ends(next).then_some((6, 1)),
            'm' if currency => ends(next).then_some((6, 1)),
            'G' if !currency => ends(next).then_some((9, 1)),
            'B' | 'b' if currency => {
                if next == Some('n') && ends(after_next) {
                    Some((9, 2))
                } else {
                    ends(next).then_some((9, 1))
                }
            }
            'T' if !currency => ends(next).then_some((12, 1)),
            'T' | 't' if currency => {
                if next == Some('n') && ends(after_next) {
                    Some((12, 2))
                } else {
                    ends(next).then_some((12, 1))
                }
            }
            _ => None,
        }
    };
    let here = chars.get(j).map(|(_, c)| *c);
    if let Some((m, used)) = here.and_then(|c| {
        suffix(
            c,
            chars.get(j + 1).map(|(_, c)| *c),
            chars.get(j + 2).map(|(_, c)| *c),
        )
    }) {
        magnitude = m;
        j += used;
    } else {
        let mut k = j;
        while chars.get(k).is_some_and(|(_, c)| *c == ' ') {
            k += 1;
        }
        let end = word_end(chars, k);
        let word: String = chars[k..end].iter().map(|(_, c)| *c).collect();
        let found = match word.to_lowercase().as_str() {
            "thousand" => Some(3),
            "million" => Some(6),
            "billion" => Some(9),
            "trillion" => Some(12),
            _ => None,
        };
        if let Some(m) = found {
            magnitude = m;
            j = end;
        }
    }
    let exp = exp + magnitude;
    let scale = exp - frac_len;
    let int: BigInt = mantissa.parse().map_err(|_| bad(j))?;
    let ten_pow = BigInt::from(10).pow(scale.unsigned_abs() as u32);
    let ratio = if scale >= 0 {
        BigRational::from_integer(int * ten_pow)
    } else {
        BigRational::new(int, ten_pow)
    };
    Ok((Number::new(ratio)?, j))
}
