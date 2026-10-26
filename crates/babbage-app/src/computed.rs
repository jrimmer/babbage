//! Runs the engine over a whole sheet and turns each result into the text the answer column shows.

use babbage_core::sheet::floating_total;
use babbage_core::{
    FormatOptions, LineKind, LineResult, SheetOptions, evaluate_sheet, format_value,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Answer {
    /// The formatted value, if the line has one.
    pub text: Option<String>,
    /// False when the shown text is rounded or approximate (it gets a "≈").
    pub exact: bool,
    /// The engine's message when the line failed.
    pub error: Option<String>,
}

pub struct Computed {
    pub results: Vec<LineResult>,
    pub answers: Vec<Answer>,
    pub total: Option<Answer>,
}

pub fn compute(lines: &[String]) -> Computed {
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let opts = SheetOptions::default();
    let results = evaluate_sheet(&refs, &opts);
    let base = FormatOptions::default();
    let answers = results
        .iter()
        .map(|r| match &r.kind {
            LineKind::Value(v) => {
                let f = format_value(v, &r.display_options(&base));
                Answer {
                    text: Some(f.text),
                    exact: f.exact,
                    error: None,
                }
            }
            LineKind::Error(e) => Answer {
                text: None,
                exact: true,
                error: Some(e.to_string()),
            },
            _ => Answer::default(),
        })
        .collect();
    let total = floating_total(&results, &opts).map(|t| match t {
        Ok(v) => {
            let f = format_value(&v, &base);
            Answer {
                text: Some(f.text),
                exact: f.exact,
                error: None,
            }
        }
        Err(e) => Answer {
            text: None,
            exact: true,
            error: Some(e.to_string()),
        },
    });
    Computed {
        results,
        answers,
        total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_sum_has_its_answer_and_exact_flag() {
        let c = compute(&sheet(&["12 + 12 + 15"]));
        assert_eq!(c.answers[0].text.as_deref(), Some("39"));
        assert!(c.answers[0].exact);
    }

    #[test]
    fn the_reference_sheet_prints_the_published_answers() {
        let src = [
            ("12 + 12 + 15", "39"),
            (
                "$700 + $500 + $35 + $1,150 + $300 + $200 + $100",
                "$2,985.00",
            ),
            ("85 × 12", "1,020"),
            ("(800mm × 800mm × 1,000mm) / 10", "64M mm³"),
            ("256mm x 256mm x 256mm", "16.777216M mm³"),
            ("line4 / line5", "3.8146972656"),
            ("$185 × 4", "$740.00"),
            ("137 × 1.82", "249.34"),
            ("108 × 1.36", "146.88"),
            ("19/29", "0.6551724138"),
            ("76 × line10", "49.7931034483"),
            ("$55 × 168", "$9,240.00"),
            ("$50 × 8", "$400.00"),
            ("55,368 / 24 / 365", "6.3205479452"),
            ("1,024 × 16", "16,384"),
            ("1,024 × 12", "12,288"),
            ("0.92 – .05", "0.87"),
            (".002 ×6", "0.012"),
            ("895,000 / (895,000 + 102,500)", "0.8972431078"),
        ];
        let lines: Vec<String> = src.iter().map(|(l, _)| (*l).to_owned()).collect();
        let c = compute(&lines);
        for (i, (l, want)) in src.iter().enumerate() {
            assert_eq!(
                c.answers[i].text.as_deref(),
                Some(*want),
                "line {}: {l}",
                i + 1
            );
        }
    }

    #[test]
    fn a_rounded_answer_is_not_exact() {
        let c = compute(&sheet(&["19/29"]));
        assert!(!c.answers[0].exact);
    }

    #[test]
    fn an_error_line_has_a_message_and_leaves_its_neighbours_alone() {
        let c = compute(&sheet(&["1 + 1", "1 / 0", "2 + 2"]));
        assert_eq!(c.answers[0].text.as_deref(), Some("2"));
        assert!(c.answers[1].error.is_some() && c.answers[1].text.is_none());
        assert_eq!(c.answers[2].text.as_deref(), Some("4"));
    }

    #[test]
    fn text_comments_and_blank_lines_have_no_answer() {
        let c = compute(&sheet(&["", "// note", "# Heading"]));
        assert!(c.answers.iter().all(|a| *a == Answer::default()));
    }

    #[test]
    fn the_total_follows_the_sheet() {
        let c = compute(&sheet(&["10", "20", "5"]));
        assert_eq!(c.total.and_then(|t| t.text).as_deref(), Some("35"));
        assert!(compute(&sheet(&[""])).total.is_none());
    }
}
