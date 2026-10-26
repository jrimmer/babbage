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
            Tok::Value(_) => "value".into(),
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
        let (tok, used) = if (word(i, "a") || word(i, "an")) && next_is_unit && prev_ends_operand {
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

pub fn lex(src: &str) -> Result<Vec<Token>, Error> {
    lex_raw(src)
        .and_then(resolve_timecodes)
        .map(minutes_beside_time)
        .map(operator_words)
        .map(crate::dates::rewrite)
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
                    || (chars.get(end).is_some_and(|(_, c)| *c == '_')
                        && is_digit(end - 1)
                        && is_digit(end + 1))
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
            '0'..='9' if iso_shape_at(&chars, i) => {
                let (v, next) = iso_at(&chars, i).ok_or_else(|| Error::InvalidNumber {
                    text: chars[i..i + 10].iter().map(|(_, ch)| *ch).collect(),
                    at,
                })?;
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
            '0'..='9' if clock_at(&chars, i).is_some() => {
                let (v, next) = clock_at(&chars, i).expect("checked by the guard");
                out.push(Token {
                    tok: Tok::Value(v),
                    at,
                });
                i = next;
                continue;
            }
            '0'..='9' => {
                let (n, next) = scan_number(src, &chars, i, false)?;
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
                // A function name, with any digits it ends in (`log10`), wins over a unit of the same spelling (`min`).
                let letters_end = word_end(&chars, i);
                // `Valentine's` and `year's` are words with an apostrophe inside, not a unit and a stray mark.
                if chars.get(letters_end).is_some_and(|(_, ch)| *ch == '\'')
                    && chars
                        .get(letters_end + 1)
                        .is_some_and(|(_, ch)| ch.is_alphabetic())
                {
                    let end = word_end(&chars, letters_end + 1);
                    let word: String = chars[i..end].iter().map(|(_, ch)| *ch).collect();
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
            '(' => simple(Tok::LParen, &mut out),
            ')' => simple(Tok::RParen, &mut out),
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

/// A currency symbol at `i` that is directly followed by a number, as in `$55`, `US$7` or `€.5`.
fn currency_amount_at(src: &str, chars: &[(usize, char)], i: usize) -> Option<(UnitExpr, usize)> {
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
    if i > 0 && c(i - 1).is_some_and(|p| p.is_alphanumeric() || matches!(p, '.' | '-' | ':')) {
        return None;
    }
    let (year, month, day) = (digits(i, 4)?, digits(i + 5, 2)?, digits(i + 8, 2)?);
    if c(i + 4) != Some('-') || c(i + 7) != Some('-') {
        return None;
    }
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
                zone = Some((zh * 3600 + zm * 60) * if sign == '-' { -1 } else { 1 });
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
        .map(|days| (Value::date(days), pos))
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
    for _ in 0..3 {
        if c(pos) != Some(':') || digit_run(pos + 1) != 2 {
            return None;
        }
        fields.push(number(pos + 1, 2));
        pos += 3;
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
    if !toks.iter().any(|t| matches!(t.tok, Tok::Timecode(..))) {
        return Ok(toks);
    }
    let fps = crate::units::lookup("fps");
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
    if !rate.is_integer() || !rate.is_positive() {
        return Err(Error::UnsupportedOperation(
            "a timecode needs a whole number of frames per second",
        ));
    }
    let seconds =
        crate::units::lookup("s").ok_or(Error::UnsupportedOperation("seconds are missing"))?;
    let mut out = Vec::with_capacity(toks.len());
    for (k, t) in toks.into_iter().enumerate() {
        if let Some(a) = skip
            && (a..a + 3).contains(&k)
        {
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

/// A time of day (`17:30`, `5:30 pm`, `5pm`) or a lap time (`03:04:05`, `00:00:01.5`) that starts at `i`.
/// `am` and `pm` after a number of 1 to 12 mean a time of day, not attometres or picometres.
fn clock_at(chars: &[(usize, char)], i: usize) -> Option<(Value, usize)> {
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
    for end in second.into_iter().chain([first]) {
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
        } else if chars.get(j).is_some_and(|(_, c)| *c == '_')
            && j > 0
            && digit_at(chars, j - 1)
            && digit_at(chars, j + 1)
        {
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
