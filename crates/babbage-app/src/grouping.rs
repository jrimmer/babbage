//! Numbers as typed: putting thousands separators in as the digits go in, and turning a number pasted in another
//! region's format into this one's.

use babbage_core::NumberFormat;

/// The separator between groups of three digits, and the decimal mark, of a format.
pub fn marks(format: NumberFormat) -> (char, char) {
    match format {
        NumberFormat::NorthAmerica => (',', '.'),
        NumberFormat::WesternEurope => ('.', ','),
        NumberFormat::EasternEurope => (' ', ','),
    }
}

fn is_digit_or_mark(c: char, group: char, decimal: char) -> bool {
    c.is_ascii_digit() || c == group || c == decimal
}

/// `1234567` as `1,234,567`: `digits` (no separators) grouped in threes from the right.
fn grouped(digits: &str, group: char) -> String {
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(group);
        }
        out.push(c);
    }
    out
}

/// The line with the number around byte `caret` written with thousands separators, and where the caret goes. None
/// when there is no plain number there or it already reads right. A space as the separator is never inserted while
/// typing (it would end the number).
pub fn regroup(line: &str, caret: usize, format: NumberFormat) -> Option<(String, usize)> {
    let (group, decimal) = marks(format);
    if group == ' ' || !line.is_char_boundary(caret) {
        return None;
    }
    let bytes: Vec<(usize, char)> = line.char_indices().collect();
    let at = bytes
        .iter()
        .position(|(i, _)| *i >= caret)
        .unwrap_or(bytes.len());
    // The token of digits and marks the caret touches.
    let mut start = at;
    while start > 0 && is_digit_or_mark(bytes[start - 1].1, group, decimal) {
        start -= 1;
    }
    let mut end = at;
    while end < bytes.len() && is_digit_or_mark(bytes[end].1, group, decimal) {
        end += 1;
    }
    if start == end {
        return None;
    }
    // A number inside a word or after another mark (`x1234`, `1.2.3`, `0x1F`) is left alone.
    let before = start.checked_sub(1).map(|i| bytes[i].1);
    let after = bytes.get(end).map(|(_, c)| *c);
    if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == decimal || c == group)
        || after.is_some_and(|c| c.is_alphanumeric() || c == '_')
    {
        return None;
    }
    let token: String = bytes[start..end].iter().map(|(_, c)| *c).collect();
    let (whole, frac) = match token.split_once(decimal) {
        Some((w, f)) => (w.to_string(), Some(f.to_string())),
        None => (token.clone(), None),
    };
    if frac
        .as_deref()
        .is_some_and(|f| f.contains(group) || f.contains(decimal))
    {
        return None;
    }
    let digits: String = whole.chars().filter(|c| *c != group).collect();
    if digits.is_empty()
        || digits.len() < 4 && !whole.contains(group)
        || digits.starts_with('0') && digits.len() > 1
    {
        return None;
    }
    let new_whole = grouped(&digits, group);
    let new_token = match &frac {
        Some(f) => format!("{new_whole}{decimal}{f}"),
        None => new_whole,
    };
    if new_token == token {
        return None;
    }
    // The caret keeps its place among the digits.
    let digits_before = bytes[start..at]
        .iter()
        .filter(|(_, c)| c.is_ascii_digit())
        .count();
    let mut seen = 0;
    let mut offset = 0;
    for (i, c) in new_token.char_indices() {
        if seen == digits_before {
            break;
        }
        if c.is_ascii_digit() {
            seen += 1;
            offset = i + 1;
        }
    }
    let from = bytes.get(start).map_or(line.len(), |(i, _)| *i);
    let to = bytes.get(end).map_or(line.len(), |(i, _)| *i);
    let mut out = String::with_capacity(line.len() + 2);
    out.push_str(&line[..from]);
    out.push_str(&new_token);
    out.push_str(&line[to..]);
    Some((out, from + offset))
}

/// The smallest edit that turns `old` into `new`: the byte range of `old` to replace and what to put there.
pub fn smallest_edit<'a>(old: &str, new: &'a str) -> (std::ops::Range<usize>, &'a str) {
    let prefix = old
        .char_indices()
        .zip(new.char_indices())
        .take_while(|((_, a), (_, b))| a == b)
        .last()
        .map_or(0, |((i, c), _)| i + c.len_utf8());
    let max_suffix = old.len().min(new.len()) - prefix;
    let suffix = old
        .bytes()
        .rev()
        .zip(new.bytes().rev())
        .take(max_suffix)
        .take_while(|(a, b)| a == b)
        .count();
    let suffix = (0..=suffix)
        .rev()
        .find(|s| old.is_char_boundary(old.len() - s) && new.is_char_boundary(new.len() - s))
        .unwrap_or(0);
    (prefix..old.len() - suffix, &new[prefix..new.len() - suffix])
}

/// `text` with every number written in another region's format turned into `format`'s, where it can be told which
/// that is: both marks present (the last one is the decimal mark), a mark repeated (that is the group mark), or a
/// single mark followed by other than three digits (the decimal mark). `1,234` and `1.234` could be either, so they
/// stay.
pub fn convert_foreign(text: &str, format: NumberFormat) -> String {
    let (group, decimal) = marks(format);
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let mut j = i;
        while j < chars.len()
            && (chars[j].is_ascii_digit()
                || ((chars[j] == ',' || chars[j] == '.')
                    && chars.get(j + 1).is_some_and(char::is_ascii_digit)))
        {
            j += 1;
        }
        let token: String = chars[i..j].iter().collect();
        let word_before = i > 0 && (chars[i - 1].is_alphabetic() || chars[i - 1] == '_');
        match (word_before, read_foreign(&token)) {
            (false, Some((digits, frac))) => {
                out.push_str(&grouped_if_long(&digits, group));
                if let Some(f) = frac {
                    out.push(decimal);
                    out.push_str(&f);
                }
            }
            _ => out.push_str(&token),
        }
        i = j;
    }
    out
}

fn grouped_if_long(digits: &str, group: char) -> String {
    if digits.len() >= 4 {
        grouped(digits, group)
    } else {
        digits.to_string()
    }
}

/// The digits and the decimal part of a token whose format can be told: its group marks removed.
fn read_foreign(token: &str) -> Option<(String, Option<String>)> {
    let commas = token.matches(',').count();
    let dots = token.matches('.').count();
    let last_mark = token.rfind([',', '.']);
    let (decimal_at, group_mark): (Option<usize>, Option<char>) = match (commas, dots) {
        (0, 0) => return None,
        (c, d) if c > 0 && d > 0 => {
            let at = last_mark?;
            let mark = token[at..].chars().next()?;
            (Some(at), Some(if mark == ',' { '.' } else { ',' }))
        }
        (c, 0) if c > 1 => (None, Some(',')),
        (0, d) if d > 1 => (None, Some('.')),
        (1, 0) | (0, 1) => {
            let at = last_mark?;
            let after = token.len() - at - 1;
            if after == 3 {
                return None;
            }
            (Some(at), None)
        }
        _ => return None,
    };
    let (whole, frac) = match decimal_at {
        Some(at) => (&token[..at], Some(token[at + 1..].to_string())),
        None => (token, None),
    };
    let digits: String = match group_mark {
        Some(g) => whole.chars().filter(|c| *c != g).collect(),
        None => whole.to_string(),
    };
    digits
        .chars()
        .all(|c| c.is_ascii_digit())
        .then_some((digits, frac))
}

#[cfg(test)]
mod tests {
    use super::*;
    use NumberFormat::{EasternEurope, NorthAmerica, WesternEurope};

    fn typed(line: &str, caret: usize, f: NumberFormat) -> Option<(String, usize)> {
        regroup(line, caret, f)
    }

    #[test]
    fn digits_typed_into_a_number_get_their_separators() {
        assert_eq!(typed("1000", 4, NorthAmerica), Some(("1,000".into(), 5)));
        assert_eq!(typed("12345", 5, NorthAmerica), Some(("12,345".into(), 6)));
        assert_eq!(typed("1,2345", 6, NorthAmerica), Some(("12,345".into(), 6)));
        assert_eq!(
            typed("5 + 1234567", 11, NorthAmerica),
            Some(("5 + 1,234,567".into(), 13))
        );
        assert_eq!(
            typed("1000.5", 4, NorthAmerica),
            Some(("1,000.5".into(), 5))
        );
        assert_eq!(typed("1000", 4, WesternEurope), Some(("1.000".into(), 5)));
    }

    #[test]
    fn the_caret_keeps_its_place_among_the_digits() {
        // Typing a digit in the middle of `1,234` makes `12,345`; the caret stays after the typed digit.
        assert_eq!(typed("1,23,4", 4, NorthAmerica), Some(("1,234".into(), 4)));
        assert_eq!(typed("12345", 2, NorthAmerica), Some(("12,345".into(), 2)));
    }

    #[test]
    fn short_numbers_words_and_odd_tokens_are_left_alone() {
        assert_eq!(typed("123", 3, NorthAmerica), None);
        assert_eq!(typed("x1234", 5, NorthAmerica), None);
        assert_eq!(typed("1,000", 5, NorthAmerica), None);
        assert_eq!(typed("0123456", 7, NorthAmerica), None);
        assert_eq!(typed("1.2.3", 5, NorthAmerica), None);
        assert_eq!(typed("abc", 3, NorthAmerica), None);
        assert_eq!(typed("1000", 4, EasternEurope), None);
    }

    #[test]
    fn the_smallest_edit_is_what_changed() {
        assert_eq!(smallest_edit("1000", "1,000"), (1..1, ","));
        assert_eq!(smallest_edit("5 + 12345", "5 + 12,345"), (6..6, ","));
        assert_eq!(smallest_edit("abc", "abc"), (3..3, ""));
        assert_eq!(smallest_edit("a1b", "a22b"), (1..2, "22"));
        assert_eq!(smallest_edit("é1", "é,1"), (2..2, ","));
    }

    #[test]
    fn a_foreign_number_is_turned_into_this_regions_format() {
        let na = |t: &str| convert_foreign(t, NorthAmerica);
        assert_eq!(na("1.234,5"), "1,234.5");
        assert_eq!(na("1.234.567"), "1,234,567");
        assert_eq!(na("12,5"), "12.5");
        assert_eq!(na("price 1.234,50 EUR"), "price 1,234.50 EUR");
        // Already this format, or too alike to tell.
        assert_eq!(na("1,234.5"), "1,234.5");
        assert_eq!(na("1,234"), "1,234");
        assert_eq!(na("1.234"), "1.234");
        assert_eq!(na("12345"), "12345");
        assert_eq!(na("x1,5"), "x1,5");
        let we = |t: &str| convert_foreign(t, WesternEurope);
        assert_eq!(we("1,234.5"), "1.234,5");
        assert_eq!(we("12.5"), "12,5");
        assert_eq!(we("1,234,567"), "1.234.567");
        assert_eq!(we("1.234,5"), "1.234,5");
    }
}
