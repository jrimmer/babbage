//! How one line's answer is written, set from the answer's menu: decimal places, thousands separators, the
//! `k`/`M`/`G` notation, fixed money digits and repeating decimals. A line with none of them set follows the settings.
//!
//! A line's choices are kept in the sheet file as one trailer line, `⟦bbg line 3 dp=4 group=off #a1b2c3d4⟧`, so a
//! sheet keeps how its answers are written. The `#` part is a fingerprint of the line's text: a file edited in
//! another editor may have its lines moved, and the choices follow the text, not the number. A trailer without a
//! fingerprint (older files) is trusted by its number.

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
    /// A number that never ends shown with its repeating part in brackets (`0.(3)`).
    pub repeating: Option<bool>,
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
        if let Some(r) = self.repeating {
            o.repeating = r;
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
        if let Some(r) = self.repeating {
            parts.push(format!("repeat={}", flag(r)));
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
                "repeat" => f.repeating = flag,
                _ => {}
            }
        }
        f
    }
}

/// A fingerprint of a line's text (FNV-1a over the text without its outer white space), so the same text gives
/// the same fingerprint on every machine and in every release.
pub fn fingerprint(text: &str) -> u32 {
    text.trim().bytes().fold(0x811c_9dc5, |h, b| {
        (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
    })
}

/// The trailer line for line `index` (0-based) whose text is `text`, or None when the line has no choices.
pub fn trailer_line(index: usize, text: &str, f: &LineFormat) -> Option<String> {
    (!f.is_default()).then(|| {
        format!(
            "{PREFIX}{} {} #{:08x}{SUFFIX}",
            index + 1,
            f.to_text(),
            fingerprint(text)
        )
    })
}

/// What a trailer line holds.
#[derive(Debug, PartialEq, Eq)]
pub struct Trailer {
    /// The line it was written for (0-based).
    pub index: usize,
    pub format: LineFormat,
    /// The fingerprint of that line's text; None in a file from before fingerprints.
    pub print: Option<u32>,
}

/// The line index (0-based), choices and fingerprint a trailer line holds, or None if it is not one.
pub fn parse_trailer(line: &str) -> Option<Trailer> {
    let inner = line.strip_prefix(PREFIX)?.strip_suffix(SUFFIX)?;
    let (n, rest) = inner.split_once(' ').unwrap_or((inner, ""));
    let index = n.parse::<usize>().ok()?.checked_sub(1)?;
    let print = rest
        .split_whitespace()
        .find_map(|part| part.strip_prefix('#'))
        .filter(|h| h.len() == 8)
        .and_then(|h| u32::from_str_radix(h, 16).ok());
    Some(Trailer {
        index,
        format: LineFormat::from_text(rest),
        print,
    })
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
    let prints: Vec<u32> = content.iter().map(|l| fingerprint(l)).collect();
    let mut claimed = vec![false; formats.len()];
    let mut moved = Vec::new();
    for t in trailer.iter().filter_map(|t| parse_trailer(t)) {
        match t.print {
            // An older file: its number is all there is.
            None if t.index < content.len() => {
                formats[t.index] = t.format;
                claimed[t.index] = true;
            }
            Some(p) if prints.get(t.index) == Some(&p) => {
                formats[t.index] = t.format;
                claimed[t.index] = true;
            }
            Some(p) => moved.push((t.index, p, t.format)),
            None => {}
        }
    }
    // The text moved (a line added or taken away above it): it goes to the nearest unclaimed line with the same
    // text, and is dropped when no line has that text any more.
    for (was, p, format) in moved {
        let to = (0..content.len())
            .filter(|&i| prints[i] == p && !claimed[i])
            .min_by_key(|&i| i.abs_diff(was));
        if let Some(i) = to {
            formats[i] = format;
            claimed[i] = true;
        }
    }
    (content.to_vec(), formats)
}

/// The lines a file holds for a sheet: its text, then a trailer line for each line with choices.
pub fn join(content: &[String], formats: &[LineFormat]) -> Vec<String> {
    let mut out = content.to_vec();
    for (i, f) in formats.iter().enumerate().take(content.len()) {
        out.extend(trailer_line(i, &content[i], f));
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
            repeating: Some(true),
        };
        assert_eq!(
            f.to_text(),
            "dp=4 group=off notation=on money=off repeat=on"
        );
        assert_eq!(LineFormat::from_text(&f.to_text()), f);
        assert_eq!(
            LineFormat::from_text("dp=99 group=maybe junk"),
            LineFormat::default()
        );
        assert_eq!(LineFormat::default().to_text(), "");
    }

    fn dp(p: u32) -> LineFormat {
        LineFormat {
            places: Some(p),
            ..LineFormat::default()
        }
    }

    fn strings(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn the_fingerprint_is_fixed_and_ignores_outer_white_space() {
        assert_eq!(fingerprint("1/3"), 0x7245_2f0e);
        assert_eq!(fingerprint("  1/3\t"), fingerprint("1/3"));
        assert_ne!(fingerprint("1/3"), fingerprint("1/4"));
    }

    #[test]
    fn trailer_lines_round_trip_and_only_formatted_lines_have_one() {
        assert_eq!(trailer_line(0, "1/3", &LineFormat::default()), None);
        let t = trailer_line(2, "1/3", &dp(2)).unwrap();
        assert_eq!(t, "⟦bbg line 3 dp=2 #72452f0e⟧");
        assert_eq!(
            parse_trailer(&t),
            Some(Trailer {
                index: 2,
                format: dp(2),
                print: Some(0x7245_2f0e)
            })
        );
        // Older trailers have no fingerprint; a malformed one has none either.
        assert_eq!(parse_trailer("⟦bbg line 3 dp=2⟧").unwrap().print, None);
        assert_eq!(parse_trailer("⟦bbg line 3 dp=2 #xyz⟧").unwrap().print, None);
        assert_eq!(parse_trailer("⟦bbg line 0 dp=2⟧"), None);
        assert_eq!(parse_trailer("1 + 1"), None);
    }

    #[test]
    fn a_sheet_splits_into_text_and_choices_and_joins_back() {
        let lines = strings(&["1/3", "2/3", "⟦bbg line 2 dp=4⟧", "⟦bbg line 9 dp=1⟧"]);
        let (content, formats) = split(&lines);
        assert_eq!(content, ["1/3", "2/3"]);
        assert_eq!(formats.len(), 2);
        assert!(formats[0].is_default());
        assert_eq!(formats[1].places, Some(4));
        // A choice for a line the sheet does not have is dropped; the rest is written with a fingerprint.
        let joined = join(&content, &formats);
        assert_eq!(joined.len(), 3);
        assert!(joined[2].starts_with("⟦bbg line 2 dp=4 #"));
        assert_eq!(split(&joined), (content, formats));
        // A sheet with no trailer is its own text; a trailer-shaped line in the middle is text.
        let (plain, f) = split(&strings(&["a", "⟦bbg line 1 dp=2⟧", "b"]));
        assert_eq!(plain.len(), 3);
        assert!(f.iter().all(|x| x.is_default()));
        let (none, _) = split(&[]);
        assert!(none.is_empty());
    }

    /// A file written by `join`, then edited by hand: `edit` changes its text lines.
    fn edited(
        text: &[&str],
        with: &[(usize, u32)],
        edit: impl Fn(&mut Vec<String>),
    ) -> Vec<LineFormat> {
        let content = strings(text);
        let mut formats = vec![LineFormat::default(); content.len()];
        for &(i, p) in with {
            formats[i] = dp(p);
        }
        let mut file = join(&content, &formats);
        let trailers = file.split_off(content.len());
        edit(&mut file);
        file.extend(trailers);
        split(&file).1
    }

    #[test]
    fn a_line_added_above_keeps_the_choice_with_its_text() {
        let f = edited(&["a", "b", "c"], &[(2, 4)], |l| l.insert(0, "new".into()));
        assert_eq!(
            f.iter().map(|x| x.places).collect::<Vec<_>>(),
            [None, None, None, Some(4)]
        );
    }

    #[test]
    fn a_line_taken_away_above_keeps_the_choice_with_its_text() {
        let f = edited(&["a", "b", "c"], &[(2, 4)], |l| {
            l.remove(0);
        });
        assert_eq!(
            f.iter().map(|x| x.places).collect::<Vec<_>>(),
            [None, Some(4)]
        );
    }

    #[test]
    fn a_line_whose_text_was_edited_loses_its_choice_rather_than_giving_it_away() {
        let f = edited(&["a", "b", "c"], &[(1, 4)], |l| l[1] = "b + 1".into());
        assert!(f.iter().all(|x| x.is_default()), "{f:?}");
    }

    #[test]
    fn a_choice_does_not_land_on_another_line_when_lines_move() {
        // Two lines swapped places: each choice follows its own text.
        let f = edited(&["a", "b"], &[(0, 1), (1, 2)], |l| l.reverse());
        assert_eq!(
            f.iter().map(|x| x.places).collect::<Vec<_>>(),
            [Some(2), Some(1)]
        );
    }

    #[test]
    fn identical_lines_go_to_the_nearest_free_one() {
        // Three `x` lines, the last one with a choice; a line is added at the top and another taken from the middle.
        let f = edited(&["x", "x", "x"], &[(2, 3)], |l| {
            l.insert(0, "new".into());
            l.remove(2);
        });
        assert_eq!(
            f.iter().map(|x| x.places).collect::<Vec<_>>(),
            [None, None, Some(3)]
        );
        // Two choices on the same text both land, on different lines.
        let f = edited(&["x", "y", "x"], &[(0, 1), (2, 2)], |l| {
            l.insert(0, "new".into())
        });
        assert_eq!(
            f.iter().map(|x| x.places).collect::<Vec<_>>(),
            [None, Some(1), None, Some(2)]
        );
    }

    #[test]
    fn an_older_trailer_without_a_fingerprint_is_trusted_by_its_number() {
        let (_, f) = split(&strings(&["a", "b", "⟦bbg line 2 dp=5⟧"]));
        assert_eq!(f[1].places, Some(5));
    }
}
