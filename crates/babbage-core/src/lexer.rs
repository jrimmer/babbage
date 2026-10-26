use num_bigint::BigInt;
use num_rational::BigRational;

use crate::error::Error;
use crate::number::Number;

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
    /// `attached` is true when `%` directly follows a number or `)`, as in `10%`.
    Percent { attached: bool },
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
            Tok::Percent { .. } => "'%'".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub at: usize,
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
            '0'..='9' => {
                let (n, next) = scan_number(src, &chars, i)?;
                out.push(Token { tok: Tok::Num(n), at });
                i = next;
                continue;
            }
            '.' if chars.get(i + 1).is_some_and(|(_, d)| d.is_ascii_digit()) => {
                let (n, next) = scan_number(src, &chars, i)?;
                out.push(Token { tok: Tok::Num(n), at });
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
                let attached = i > 0 && (chars[i - 1].1.is_ascii_digit() || matches!(chars[i - 1].1, ')' | '%'));
                simple(Tok::Percent { attached }, &mut out)
            }
            c if c.is_alphabetic() => {
                let mut end = i;
                while chars.get(end).is_some_and(|(_, ch)| ch.is_alphabetic()) {
                    end += 1;
                }
                let word: String = chars[i..end].iter().map(|(_, ch)| *ch).collect();
                out.push(Token { tok: Tok::Word(word.to_lowercase()), at });
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

fn is_times_x(chars: &[(usize, char)], i: usize) -> bool {
    let before = i.checked_sub(1).map(|p| chars[p].1);
    let after = chars.get(i + 1).map(|p| p.1);
    !before.is_some_and(char::is_alphabetic) && !after.is_some_and(char::is_alphabetic)
}

fn digit_at(chars: &[(usize, char)], i: usize) -> bool {
    chars.get(i).is_some_and(|(_, c)| c.is_ascii_digit())
}

fn scan_number(src: &str, chars: &[(usize, char)], start: usize) -> Result<(Number, usize), Error> {
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
        let sign_len = usize::from(chars.get(j + 1).is_some_and(|(_, c)| matches!(c, '+' | '-')));
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
