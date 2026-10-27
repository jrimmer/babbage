//! What a line shows, as against what it holds: a reference to another line (`line4`, `line 4`) is drawn as a chip
//! holding that line's answer. The sheet's text stays as typed; this maps columns between the two.

use std::ops::Range;

use babbage_core::LineResult;

use crate::computed::Answer;
use crate::syntax::{Class, Span};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Part {
    /// The reference as typed, and the answer drawn in its place.
    src: Range<usize>,
    shown: Range<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shown {
    pub text: String,
    parts: Vec<Part>,
}

/// The line a reference points at, counting from 1: `line4` and `line 4` are 4.
fn target(reference: &str) -> Option<usize> {
    let digits: String = reference.chars().filter(char::is_ascii_digit).collect();
    digits.parse().ok().filter(|n| *n >= 1)
}

/// What line `own` shows, given every line's answer.
pub fn shown_for(line: &str, result: Option<&LineResult>, own: usize, answers: &[Answer]) -> Shown {
    let spans = crate::syntax::spans(line, result);
    Shown::build(line, &spans, own, answers)
}

impl Shown {
    /// `line` with each reference to a line that has an answer replaced by that answer. `spans` are the line's
    /// syntax spans; a reference to a line with no answer (or to itself) stays as typed.
    pub fn build(line: &str, spans: &[Span], own: usize, answers: &[Answer]) -> Shown {
        let mut text = String::with_capacity(line.len());
        let mut parts = Vec::new();
        let mut at = 0;
        for s in spans.iter().filter(|s| s.class == Class::LineRef) {
            let Some(n) = target(&line[s.range.clone()]) else {
                continue;
            };
            let Some(answer) = answers
                .get(n - 1)
                .filter(|_| n - 1 != own)
                .and_then(|a| a.text.as_deref())
            else {
                continue;
            };
            text.push_str(&line[at..s.range.start]);
            let from = text.len();
            text.push_str(answer);
            parts.push(Part {
                src: s.range.clone(),
                shown: from..text.len(),
            });
            at = s.range.end;
        }
        text.push_str(&line[at..]);
        Shown { text, parts }
    }

    /// The column in the shown text for column `col` of the typed text. A column inside a reference goes to the
    /// nearer end of its chip.
    pub fn to_shown(&self, col: usize) -> usize {
        let mut delta: isize = 0;
        for p in &self.parts {
            if col >= p.src.end {
                delta += p.shown.len() as isize - p.src.len() as isize;
            } else if col > p.src.start {
                return if (col - p.src.start) * 2 < p.src.len() {
                    p.shown.start
                } else {
                    p.shown.end
                };
            } else {
                break;
            }
        }
        (col as isize + delta) as usize
    }

    /// The column of the typed text for column `col` of the shown text. A column inside a chip goes to the nearer
    /// end of its reference.
    pub fn to_src(&self, col: usize) -> usize {
        let mut delta: isize = 0;
        for p in &self.parts {
            if col >= p.shown.end {
                delta += p.src.len() as isize - p.shown.len() as isize;
            } else if col > p.shown.start {
                return if (col - p.shown.start) * 2 < p.shown.len() {
                    p.src.start
                } else {
                    p.src.end
                };
            } else {
                break;
            }
        }
        (col as isize + delta) as usize
    }

    /// The typed range of the reference whose chip holds the shown column, if any.
    pub fn chip_at_src(&self, col: usize) -> Option<Range<usize>> {
        self.parts
            .iter()
            .find(|p| p.src.start < col && col < p.src.end)
            .map(|p| p.src.clone())
    }

    /// The typed range of the reference that ends at `col` (to delete a chip whole with Backspace).
    pub fn chip_ending_at(&self, col: usize) -> Option<Range<usize>> {
        self.parts
            .iter()
            .find(|p| p.src.end == col)
            .map(|p| p.src.clone())
    }

    /// The typed range of the reference that starts at `col` (to delete a chip whole with Delete).
    pub fn chip_starting_at(&self, col: usize) -> Option<Range<usize>> {
        self.parts
            .iter()
            .find(|p| p.src.start == col)
            .map(|p| p.src.clone())
    }

    /// `spans` moved to the shown text.
    pub fn spans(&self, spans: &[Span]) -> Vec<Span> {
        spans
            .iter()
            .map(|s| Span {
                range: self.to_shown(s.range.start)..self.to_shown(s.range.end),
                ..s.clone()
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(text: Option<&str>) -> Answer {
        Answer {
            text: text.map(str::to_string),
            exact: true,
            error: None,
        }
    }

    fn line_ref(range: Range<usize>) -> Span {
        Span {
            range,
            class: Class::LineRef,
            bold: false,
        }
    }

    fn shown(line: &str, spans: &[Span], own: usize, answers: &[Option<&str>]) -> Shown {
        let a: Vec<Answer> = answers.iter().map(|t| answer(*t)).collect();
        Shown::build(line, spans, own, &a)
    }

    #[test]
    fn a_reference_shows_the_answer_of_its_line() {
        let s = shown(
            "line1 / line 2",
            &[line_ref(0..5), line_ref(8..14)],
            2,
            &[Some("64M mm³"), Some("16.777216M mm³"), None],
        );
        assert_eq!(s.text, "64M mm³ / 16.777216M mm³");
    }

    #[test]
    fn a_reference_to_a_line_without_an_answer_or_to_itself_stays_as_typed() {
        let s = shown(
            "line2 + line1",
            &[line_ref(0..5), line_ref(8..13)],
            0,
            &[None, None],
        );
        assert_eq!(s.text, "line2 + line1");
        assert_eq!(s.to_shown(3), 3);
        let own = shown("line1", &[line_ref(0..5)], 0, &[Some("5")]);
        assert_eq!(own.text, "line1");
        let missing = shown("line9", &[line_ref(0..5)], 0, &[Some("5")]);
        assert_eq!(missing.text, "line9");
    }

    #[test]
    fn columns_map_both_ways_and_snap_to_the_ends_of_a_chip() {
        // "76 × line10" with line 10 answering 0.6551724138 (12 characters for the 6 typed).
        let line = "76 × line10";
        let at = line.find("line10").unwrap();
        let mut answers = vec![None; 10];
        answers[9] = Some("0.6551724138");
        let s = shown(line, &[line_ref(at..at + 6)], 10, &answers);
        assert_eq!(s.text, "76 × 0.6551724138");
        let end = line.len();
        assert_eq!(s.to_shown(0), 0);
        assert_eq!(s.to_shown(at), at);
        assert_eq!(s.to_shown(end), s.text.len());
        assert_eq!(s.to_src(at), at);
        assert_eq!(s.to_src(s.text.len()), end);
        // Inside the typed reference, a column goes to the near end of the chip; and the other way round.
        assert_eq!(s.to_shown(at + 1), at);
        assert_eq!(s.to_shown(at + 5), s.text.len());
        assert_eq!(s.to_src(at + 2), at);
        assert_eq!(s.to_src(s.text.len() - 1), end);
        for col in [0, at, end] {
            assert_eq!(s.to_src(s.to_shown(col)), col);
        }
    }

    #[test]
    fn chips_are_found_from_the_caret() {
        let line = "1 + line1 + 2";
        let s = shown(line, &[line_ref(4..9)], 1, &[Some("99999")]);
        assert_eq!(s.chip_ending_at(9), Some(4..9));
        assert_eq!(s.chip_starting_at(4), Some(4..9));
        assert_eq!(s.chip_at_src(6), Some(4..9));
        assert_eq!(s.chip_at_src(4), None);
        assert_eq!(s.chip_ending_at(8), None);
    }

    #[test]
    fn spans_follow_the_text() {
        let line = "1 + line1 + 2";
        let spans = [
            Span {
                range: 0..1,
                class: Class::Number,
                bold: false,
            },
            line_ref(4..9),
            Span {
                range: 12..13,
                class: Class::Number,
                bold: false,
            },
        ];
        let s = shown(line, &spans, 1, &[Some("99999999")]);
        let moved = s.spans(&spans);
        assert_eq!(s.text, "1 + 99999999 + 2");
        assert_eq!(moved[1].range, 4..12);
        assert_eq!(moved[2].range, 15..16);
        assert_eq!(&s.text[moved[2].range.clone()], "2");
    }
}
