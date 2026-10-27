//! Splits a sheet line into colored spans. It only looks at the text and at what the engine reported
//! (ignored words, warnings, label, line kind); it never evaluates anything itself.

use std::ops::Range;

use babbage_core::{LineKind, LineResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Plain,
    Number,
    Unit,
    Currency,
    LineRef,
    /// A name the sheet defined (`rent = $900`), where it is defined and where it is used.
    Variable,
    Comment,
    Heading,
    /// A word the engine skipped.
    Ignored,
    /// A skipped word straight after a number, where a unit would stand: not understood, so it is underlined.
    Unclear,
    /// A skipped word one letter from a unit.
    Warn,
}

impl Class {
    pub fn name(self) -> &'static str {
        match self {
            Class::Plain => "plain",
            Class::Number => "number",
            Class::Unit => "unit",
            Class::Currency => "currency",
            Class::LineRef => "lineref",
            Class::Variable => "variable",
            Class::Comment => "comment",
            Class::Heading => "heading",
            Class::Ignored => "ignored",
            Class::Unclear => "unclear",
            Class::Warn => "warn",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub range: Range<usize>,
    pub class: Class,
    pub bold: bool,
}

fn is_currency(c: char) -> bool {
    matches!(
        c,
        '$' | '€' | '£' | '¥' | '₹' | '₽' | '₩' | '₿' | '₪' | '₺' | '฿'
    )
}

fn is_word_char(c: char) -> bool {
    // Superscript digits such as the ³ of mm³ count as alphanumeric already.
    c.is_alphanumeric() || matches!(c, '_' | 'µ' | '°')
}

/// Words that join numbers instead of naming a unit.
const JOINERS: [&str; 14] = [
    "x", "of", "on", "off", "in", "to", "as", "and", "or", "mod", "xor", "per", "is", "what",
];

fn push(spans: &mut Vec<Span>, range: Range<usize>, class: Class) {
    if range.is_empty() {
        return;
    }
    if let Some(last) = spans.last_mut()
        && last.class == class
        && last.range.end == range.start
        && class == Class::Plain
    {
        last.range.end = range.end;
        return;
    }
    spans.push(Span {
        range,
        class,
        bold: false,
    });
}

/// Carves `range` out of `spans` and gives it `class`, keeping the rest of each span it cuts.
fn carve(spans: &mut Vec<Span>, range: Range<usize>, class: Class) {
    let mut out = Vec::with_capacity(spans.len() + 2);
    for s in spans.drain(..) {
        if s.range.end <= range.start || s.range.start >= range.end {
            out.push(s);
            continue;
        }
        if s.range.start < range.start {
            out.push(Span {
                range: s.range.start..range.start,
                ..s.clone()
            });
        }
        let (a, b) = (s.range.start.max(range.start), s.range.end.min(range.end));
        out.push(Span {
            range: a..b,
            class,
            bold: s.bold,
        });
        if s.range.end > range.end {
            out.push(Span {
                range: range.end..s.range.end,
                ..s.clone()
            });
        }
    }
    *spans = out;
}

/// `spans`, with each use of a name the sheet defined (`names`, longest first is not required) drawn as a variable.
/// A name matches whole, as typed (names are case-sensitive in the engine), and never inside a comment or a heading.
pub fn spans_in(line: &str, result: Option<&LineResult>, names: &[String]) -> Vec<Span> {
    let mut out = spans(line, result);
    if names.is_empty()
        || matches!(
            result.map(|r| &r.kind),
            Some(LineKind::Comment | LineKind::Heading(_) | LineKind::Divider)
        )
    {
        return out;
    }
    let mut ordered: Vec<&String> = names.iter().filter(|n| !n.is_empty()).collect();
    ordered.sort_by_key(|n| std::cmp::Reverse(n.len()));
    let mut taken: Vec<Range<usize>> = Vec::new();
    for name in ordered {
        let mut from = 0;
        while let Some(at) = line[from..].find(name.as_str()) {
            let start = from + at;
            let end = start + name.len();
            from = end;
            let word = |c: char| c.is_alphanumeric() || c == '_';
            let before = line[..start].chars().next_back().is_some_and(word);
            let after = line[end..].chars().next().is_some_and(word);
            if before || after || taken.iter().any(|t| start < t.end && t.start < end) {
                continue;
            }
            taken.push(start..end);
            carve(&mut out, start..end, Class::Variable);
        }
    }
    out
}

/// Spans covering all of `line`, in order, with no gaps.
pub fn spans(line: &str, result: Option<&LineResult>) -> Vec<Span> {
    let kind = result.map(|r| &r.kind);
    let whole = |class: Class, bold: bool| {
        if line.is_empty() {
            Vec::new()
        } else {
            vec![Span {
                range: 0..line.len(),
                class,
                bold,
            }]
        }
    };
    match kind {
        Some(LineKind::Comment) => return whole(Class::Comment, false),
        Some(LineKind::Heading(_)) => return whole(Class::Heading, true),
        Some(LineKind::Divider) => return whole(Class::Comment, false),
        Some(LineKind::Text) => return carve_ignored(whole(Class::Plain, false), result),
        _ => {}
    }
    let mut out = scan(line);
    out = carve_ignored(out, result);
    if let Some(label) = result.and_then(|r| r.label.as_deref())
        && let Some(at) = line.find(label)
    {
        let mut end = at + label.len();
        if line[end..].starts_with(':') {
            end += 1;
        }
        let mut bolded = Vec::new();
        for s in out.drain(..) {
            if s.range.start >= at && s.range.end <= end {
                bolded.push(Span { bold: true, ..s });
            } else if s.range.start < end && s.range.end > at {
                // A span straddling the label's edge is split so only the label is bold.
                let mid = [s.range.start.max(at), s.range.end.min(end)];
                if s.range.start < mid[0] {
                    bolded.push(Span {
                        range: s.range.start..mid[0],
                        ..s.clone()
                    });
                }
                bolded.push(Span {
                    range: mid[0]..mid[1],
                    bold: true,
                    ..s.clone()
                });
                if mid[1] < s.range.end {
                    bolded.push(Span {
                        range: mid[1]..s.range.end,
                        ..s.clone()
                    });
                }
            } else {
                bolded.push(s);
            }
        }
        out = bolded;
    }
    out
}

fn carve_ignored(mut spans: Vec<Span>, result: Option<&LineResult>) -> Vec<Span> {
    let Some(r) = result else { return spans };
    for ig in &r.ignored {
        let class = if ig.unclear {
            Class::Unclear
        } else {
            Class::Ignored
        };
        carve(&mut spans, ig.at..ig.at + ig.text.len(), class);
    }
    for w in &r.warnings {
        carve(&mut spans, w.at..w.at + w.text.len(), Class::Warn);
    }
    spans
}

fn scan(line: &str) -> Vec<Span> {
    let mut spans: Vec<Span> = Vec::new();
    let chars: Vec<(usize, char)> = line.char_indices().collect();
    let at = |k: usize| chars.get(k).map_or(line.len(), |(i, _)| *i);
    let mut k = 0;
    // Whether the last thing scanned was a number or a unit that follows one, and across how many spaces.
    let mut after_number = false;
    let mut after_joiner_word = false;
    while k < chars.len() {
        let (i, c) = chars[k];
        if c == '/' && chars.get(k + 1).is_some_and(|(_, d)| *d == '/') {
            push(&mut spans, i..line.len(), Class::Comment);
            break;
        }
        if c.is_ascii_digit()
            || (c == '.' && chars.get(k + 1).is_some_and(|(_, d)| d.is_ascii_digit()))
        {
            let mut j = k;
            let prefixed = c == '0'
                && chars
                    .get(k + 1)
                    .is_some_and(|(_, d)| matches!(d, 'x' | 'X' | 'b' | 'B' | 'o' | 'O'));
            if prefixed {
                j += 2;
                while chars.get(j).is_some_and(|(_, d)| d.is_ascii_alphanumeric()) {
                    j += 1;
                }
            } else {
                while let Some(&(_, d)) = chars.get(j) {
                    let next_digit = chars.get(j + 1).is_some_and(|(_, e)| e.is_ascii_digit());
                    if d.is_ascii_digit() || ((d == ',' || d == '.') && next_digit) {
                        j += 1;
                    } else {
                        break;
                    }
                }
            }
            // A number glued to a currency symbol before it takes the symbol's color.
            let class = if matches!(spans.last(), Some(s) if s.class == Class::Currency && s.range.end == i)
            {
                Class::Currency
            } else {
                Class::Number
            };
            push(&mut spans, i..at(j), class);
            k = j;
            after_number = true;
            after_joiner_word = false;
            continue;
        }
        if is_currency(c) {
            push(&mut spans, i..at(k + 1), Class::Currency);
            k += 1;
            continue;
        }
        if c.is_alphabetic() || c == '_' || c == 'µ' || c == '°' {
            let mut j = k;
            while chars.get(j).is_some_and(|(_, d)| is_word_char(*d)) {
                j += 1;
            }
            let word = &line[i..at(j)];
            let lower = word.to_lowercase();
            let digits = lower
                .strip_prefix("line")
                .filter(|d| !d.is_empty() && d.chars().all(|x| x.is_ascii_digit()));
            if digits.is_some() {
                push(&mut spans, i..at(j), Class::LineRef);
                k = j;
                after_number = true;
                after_joiner_word = false;
                continue;
            }
            if lower == "line" {
                // "line 4": the word, spaces, then digits make one reference.
                let mut m = j;
                while chars.get(m).is_some_and(|(_, d)| *d == ' ') {
                    m += 1;
                }
                let d0 = m;
                while chars.get(m).is_some_and(|(_, d)| d.is_ascii_digit()) {
                    m += 1;
                }
                if m > d0 && d0 > j {
                    push(&mut spans, i..at(m), Class::LineRef);
                    k = m;
                    after_number = true;
                    after_joiner_word = false;
                    continue;
                }
            }
            let joiner = JOINERS.contains(&lower.as_str());
            let class = if joiner {
                Class::Plain
            } else if after_number || after_joiner_word {
                Class::Unit
            } else {
                Class::Plain
            };
            push(&mut spans, i..at(j), class);
            after_joiner_word = matches!(lower.as_str(), "in" | "to" | "as");
            after_number = class == Class::Unit && after_number;
            k = j;
            continue;
        }
        // Anything else: operators, brackets, spaces. A single space keeps "number then unit" alive.
        if c != ' ' {
            after_number = false;
            if !matches!(c, '%') {
                after_joiner_word = false;
            }
        }
        let mut j = k + 1;
        while let Some(&(_, d)) = chars.get(j) {
            let special = d.is_ascii_digit()
                || d.is_alphabetic()
                || is_currency(d)
                || d == '_'
                || d == '.'
                || d == '/' && chars.get(j + 1).is_some_and(|(_, e)| *e == '/');
            if special {
                break;
            }
            if d != ' ' {
                after_number = false;
                after_joiner_word = false;
            }
            j += 1;
        }
        push(&mut spans, i..at(j), Class::Plain);
        k = j;
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use babbage_core::{SheetOptions, evaluate_sheet};

    fn run(lines: &[&str]) -> Vec<LineResult> {
        evaluate_sheet(lines, &SheetOptions::default())
    }

    fn classes(line: &str) -> Vec<(&str, Class)> {
        let r = run(&[line]);
        spans(line, r.first())
            .into_iter()
            .map(|s| (&line[s.range.clone()], s.class))
            .collect()
    }

    #[test]
    fn spans_always_cover_the_line_without_gaps() {
        for line in [
            "12 + 12 + 15",
            "$700 + $500",
            "(800mm × 800mm × 1,000mm) / 10",
            "line4 / line5",
            "Cost: $999 // note",
            "price of 5 apples",
            "# Heading",
            "",
            "0xFF + 0b101",
            "5 km in mi",
        ] {
            let r = run(&[line]);
            let sp = spans(line, r.first());
            let mut at = 0;
            for s in &sp {
                assert_eq!(s.range.start, at, "{line:?} {sp:?}");
                at = s.range.end;
            }
            assert_eq!(at, line.len(), "{line:?}");
        }
    }

    #[test]
    fn numbers_units_and_currency_get_their_own_classes() {
        let c = classes("800mm × 5 + $700");
        assert!(c.contains(&("800", Class::Number)));
        assert!(c.contains(&("mm", Class::Unit)));
        assert!(
            c.contains(&("$700", Class::Currency))
                || (c.contains(&("$", Class::Currency)) && c.contains(&("700", Class::Currency)))
        );
    }

    #[test]
    fn grouped_and_decimal_numbers_are_one_span() {
        let c = classes("1,024 × .5 + 3.14");
        assert!(c.contains(&("1,024", Class::Number)), "{c:?}");
        assert!(c.contains(&(".5", Class::Number)), "{c:?}");
        assert!(c.contains(&("3.14", Class::Number)), "{c:?}");
    }

    #[test]
    fn line_references_are_chips_in_both_spellings() {
        let c = classes("line4 / line 5");
        assert!(c.contains(&("line4", Class::LineRef)), "{c:?}");
        assert!(c.contains(&("line 5", Class::LineRef)), "{c:?}");
    }

    #[test]
    fn a_trailing_comment_is_dimmed_and_a_comment_line_is_all_comment() {
        let c = classes("1 + 2 // note");
        assert_eq!(c.last(), Some(&("// note", Class::Comment)));
        assert_eq!(classes("// 1 + 2"), vec![("// 1 + 2", Class::Comment)]);
    }

    #[test]
    fn headings_are_bold() {
        let r = run(&["# Title"]);
        let s = spans("# Title", r.first());
        assert!(s.iter().all(|x| x.bold && x.class == Class::Heading));
    }

    #[test]
    fn the_label_is_bold_and_the_calculation_after_it_is_not() {
        let line = "Cost of 128 GB iPhone 16: $999";
        let r = run(&[line]);
        let s = spans(line, r.first());
        let bold: String = s
            .iter()
            .filter(|x| x.bold)
            .map(|x| &line[x.range.clone()])
            .collect();
        assert_eq!(bold, "Cost of 128 GB iPhone 16:");
        let rest: String = s
            .iter()
            .filter(|x| !x.bold)
            .map(|x| &line[x.range.clone()])
            .collect();
        assert_eq!(rest, " $999");
    }

    #[test]
    fn ignored_words_are_marked_from_the_engine_report() {
        let line = "lunch was $55 + 25% tip";
        let r = run(&[line]);
        assert!(
            !r[0].ignored.is_empty(),
            "the engine reported nothing: {:?}",
            r[0]
        );
        let s = spans(line, r.first());
        for ig in &r[0].ignored {
            assert!(
                s.iter().any(
                    |x| matches!(x.class, Class::Ignored | Class::Unclear | Class::Warn)
                        && line[x.range.clone()] == ig.text
                ),
                "{ig:?} in {s:?}"
            );
        }
    }

    #[test]
    fn a_word_one_letter_from_a_unit_is_a_warning_span() {
        let line = "5 kmm";
        let r = run(&[line]);
        assert!(!r[0].warnings.is_empty(), "{:?}", r[0]);
        assert!(
            spans(line, r.first())
                .iter()
                .any(|x| x.class == Class::Warn)
        );
    }

    #[test]
    fn conversion_targets_are_units() {
        let c = classes("5 km in mi");
        assert!(c.contains(&("mi", Class::Unit)), "{c:?}");
    }

    #[test]
    fn a_name_the_sheet_defined_is_a_variable_where_it_stands() {
        let names = vec!["rent".to_string(), "total spent".to_string()];
        let class_of = |line: &str| -> Vec<(String, Class)> {
            spans_in(line, None, &names)
                .into_iter()
                .map(|s| (line[s.range].to_string(), s.class))
                .collect()
        };
        let c = class_of("rent + 5");
        assert!(c.contains(&("rent".to_string(), Class::Variable)), "{c:?}");
        let c = class_of("total spent += $18");
        assert!(
            c.contains(&("total spent".to_string(), Class::Variable)),
            "{c:?}"
        );
        // Whole words only, and exactly as typed.
        for line in ["parent + 1", "rents + 1", "Rent + 1"] {
            assert!(
                class_of(line).iter().all(|(_, k)| *k != Class::Variable),
                "{line}"
            );
        }
        // Nothing changes for a line with no names, or the spans stay a covering of the line.
        let all = spans_in("rent rent", None, &names);
        assert_eq!(all.iter().map(|s| s.range.len()).sum::<usize>(), 9);
        assert_eq!(all.iter().filter(|s| s.class == Class::Variable).count(), 2);
    }
}
