//! How one line's answer is written, set from the answer's menu: decimal places, thousands separators, the
//! `k`/`M`/`G` notation and fixed money digits. A line with none of them set follows the settings.
//!
//! A line's choices are kept in the sheet file as one trailer line, `⟦bbg line 3 dp=4 group=off⟧`, so a sheet
//! keeps how its answers are written.

use babbage_core::FormatOptions;

/// The marker that starts a trailer line.
pub const PREFIX: &str = "⟦bbg line ";
const SUFFIX: char = '⟧';

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LineFormat {
    /// Decimal places, 0 to 10.
    pub places: Option<u32>,
    pub group: Option<bool>,
    pub notation: Option<bool>,
    /// Money with its currency's fixed digits (`$5.00`).
    pub money: Option<bool>,
}

impl LineFormat {
    pub fn is_default(self) -> bool {
        self == LineFormat::default()
    }

    /// `base` with this line's choices laid over it.
    pub fn apply(self, base: FormatOptions) -> FormatOptions {
        let mut o = match self.places {
            // Fixing the places also stops large-number notation, unless the line asks for it below.
            Some(p) => base.clone().with_places(p.min(10)).unwrap_or(base),
            None => base,
        };
        if let Some(g) = self.group {
            o.group_thousands = g;
        }
        if let Some(n) = self.notation {
            o.abbreviate_large = n;
        }
        if let Some(m) = self.money {
            o.fixed_money_digits = m;
        }
        o
    }

    /// `dp=4 group=off notation=on money=off`, only what is set.
    pub fn to_text(self) -> String {
        let flag = |b: bool| if b { "on" } else { "off" };
        let mut parts = Vec::new();
        if let Some(p) = self.places {
            parts.push(format!("dp={p}"));
        }
        if let Some(g) = self.group {
            parts.push(format!("group={}", flag(g)));
        }
        if let Some(n) = self.notation {
            parts.push(format!("notation={}", flag(n)));
        }
        if let Some(m) = self.money {
            parts.push(format!("money={}", flag(m)));
        }
        parts.join(" ")
    }

    /// The choices written by `to_text`; anything else is ignored.
    pub fn from_text(text: &str) -> LineFormat {
        let mut f = LineFormat::default();
        for part in text.split_whitespace() {
            let Some((key, value)) = part.split_once('=') else {
                continue;
            };
            let flag = match value {
                "on" => Some(true),
                "off" => Some(false),
                _ => None,
            };
            match key {
                "dp" => f.places = value.parse().ok().filter(|p| *p <= 10),
                "group" => f.group = flag,
                "notation" => f.notation = flag,
                "money" => f.money = flag,
                _ => {}
            }
        }
        f
    }
}

/// The trailer line for line `index` (0-based), or None when the line has no choices.
pub fn trailer_line(index: usize, f: &LineFormat) -> Option<String> {
    (!f.is_default()).then(|| format!("{PREFIX}{} {}{SUFFIX}", index + 1, f.to_text()))
}

/// The line index (0-based) and choices a trailer line holds, or None if it is not one.
pub fn parse_trailer(line: &str) -> Option<(usize, LineFormat)> {
    let inner = line.strip_prefix(PREFIX)?.strip_suffix(SUFFIX)?;
    let (n, rest) = inner.split_once(' ').unwrap_or((inner, ""));
    let index = n.parse::<usize>().ok()?.checked_sub(1)?;
    Some((index, LineFormat::from_text(rest)))
}

/// A sheet's text lines and its per-line choices from the lines a file holds: the trailer lines at the end are
/// taken out.
pub fn split(lines: &[String]) -> (Vec<String>, Vec<LineFormat>) {
    let split_at = lines
        .iter()
        .rposition(|l| !l.starts_with(PREFIX) || parse_trailer(l).is_none())
        .map_or(0, |i| i + 1);
    let (content, trailer) = lines.split_at(split_at);
    let mut formats = vec![LineFormat::default(); content.len().max(1)];
    for t in trailer {
        if let Some((i, f)) = parse_trailer(t)
            && i < formats.len()
        {
            formats[i] = f;
        }
    }
    (content.to_vec(), formats)
}

/// The lines a file holds for a sheet: its text, then a trailer line for each line with choices.
pub fn join(content: &[String], formats: &[LineFormat]) -> Vec<String> {
    let mut out = content.to_vec();
    for (i, f) in formats.iter().enumerate().take(content.len()) {
        out.extend(trailer_line(i, f));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use babbage_core::format_value;

    fn show(f: &LineFormat, text: &str) -> String {
        let v = babbage_core::evaluate(text).expect("a value");
        format_value(&v, &f.apply(FormatOptions::default())).text
    }

    #[test]
    fn a_line_with_nothing_set_follows_the_settings() {
        assert!(LineFormat::default().is_default());
        assert_eq!(show(&LineFormat::default(), "1/3"), "0.3333333333");
        assert_eq!(show(&LineFormat::default(), "12345 * 3"), "37,035");
    }

    #[test]
    fn decimal_places_cut_or_pad_the_answer() {
        let dp = |p| LineFormat {
            places: Some(p),
            ..LineFormat::default()
        };
        assert_eq!(show(&dp(4), "1/3"), "0.3333");
        assert_eq!(show(&dp(0), "7/2"), "4");
        assert_eq!(show(&dp(2), "1/8"), "0.13");
        assert_eq!(show(&dp(10), "1/3"), "0.3333333333");
    }

    #[test]
    fn thousands_separators_and_notation_and_money_digits_switch_off() {
        let no_group = LineFormat {
            group: Some(false),
            ..LineFormat::default()
        };
        assert_eq!(show(&no_group, "12345 * 3"), "37035");
        let no_notation = LineFormat {
            notation: Some(false),
            ..LineFormat::default()
        };
        assert_eq!(show(&no_notation, "300000 * 3"), "900,000");
        assert_eq!(show(&LineFormat::default(), "300000 * 3"), "900k");
        let no_money = LineFormat {
            money: Some(false),
            ..LineFormat::default()
        };
        assert_eq!(show(&no_money, "$1.495"), "$1.495");
        assert_eq!(show(&LineFormat::default(), "$1.495"), "$1.50");
    }

    #[test]
    fn choices_write_and_read_back() {
        let f = LineFormat {
            places: Some(4),
            group: Some(false),
            notation: Some(true),
            money: Some(false),
        };
        assert_eq!(f.to_text(), "dp=4 group=off notation=on money=off");
        assert_eq!(LineFormat::from_text(&f.to_text()), f);
        assert_eq!(
            LineFormat::from_text("dp=99 group=maybe junk"),
            LineFormat::default()
        );
        assert_eq!(LineFormat::default().to_text(), "");
    }

    #[test]
    fn trailer_lines_round_trip_and_only_formatted_lines_have_one() {
        let f = LineFormat {
            places: Some(2),
            ..LineFormat::default()
        };
        assert_eq!(trailer_line(0, &LineFormat::default()), None);
        let t = trailer_line(2, &f).unwrap();
        assert_eq!(t, "⟦bbg line 3 dp=2⟧");
        assert_eq!(parse_trailer(&t), Some((2, f)));
        assert_eq!(parse_trailer("⟦bbg line 0 dp=2⟧"), None);
        assert_eq!(parse_trailer("1 + 1"), None);
    }

    #[test]
    fn a_sheet_splits_into_text_and_choices_and_joins_back() {
        let lines: Vec<String> = ["1/3", "2/3", "⟦bbg line 2 dp=4⟧", "⟦bbg line 9 dp=1⟧"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let (content, formats) = split(&lines);
        assert_eq!(content, ["1/3", "2/3"]);
        assert_eq!(formats.len(), 2);
        assert!(formats[0].is_default());
        assert_eq!(formats[1].places, Some(4));
        // A choice for a line the sheet does not have is dropped.
        assert_eq!(
            join(&content, &formats),
            ["1/3", "2/3", "⟦bbg line 2 dp=4⟧"]
        );
        // A sheet with no trailer is its own text; a trailer-shaped line in the middle is text.
        let (plain, f) = split(&[
            "a".to_string(),
            "⟦bbg line 1 dp=2⟧".to_string(),
            "b".to_string(),
        ]);
        assert_eq!(plain.len(), 3);
        assert!(f.iter().all(|x| x.is_default()));
        let (none, _) = split(&[]);
        assert!(none.is_empty());
    }
}
