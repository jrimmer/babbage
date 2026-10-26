use num_bigint::BigInt;
use num_rational::BigRational;

use crate::error::Error;
use crate::number::Number;
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

pub fn lex(src: &str) -> Result<Vec<Token>, Error> {
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
            '0'..='9' => {
                let (n, next) = scan_number(src, &chars, i, false)?;
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
            '^' => simple(Tok::Caret, &mut out),
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
                if let Some((unit, end)) = scan_unit(&chars, i) {
                    out.push(Token {
                        tok: Tok::Unit(unit),
                        at,
                    });
                    i = end;
                    continue;
                }
                let end = word_end(&chars, i);
                let word: String = chars[i..end].iter().map(|(_, ch)| *ch).collect();
                out.push(Token {
                    tok: Tok::Word(word),
                    at,
                });
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

    let mut j = start;
    let mut mantissa = String::new();
    while digit_at(chars, j) {
        mantissa.push(chars[j].1);
        j += 1;
    }
    let first_group = mantissa.len();
    while chars.get(j).is_some_and(|(_, c)| *c == ',')
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

    let mut frac_len: i64 = 0;
    if chars.get(j).is_some_and(|(_, c)| *c == '.') && digit_at(chars, j + 1) {
        j += 1;
        while digit_at(chars, j) {
            mantissa.push(chars[j].1);
            frac_len += 1;
            j += 1;
        }
        if chars.get(j).is_some_and(|(_, c)| *c == '.') && digit_at(chars, j + 1) {
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
