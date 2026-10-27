//! Runs the engine over a whole sheet and turns each result into the text the answer column shows.

use crate::linefmt::LineFormat;
use babbage_core::settings::{Settings, scoped};
use babbage_core::sheet::{floating_total, floating_total_left_out};
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
    /// How many lines the Total leaves out because they cannot be added to the others.
    pub left_out: usize,
}

#[cfg(test)]
pub fn compute(lines: &[String]) -> Computed {
    compute_formatted(lines, &[])
}

/// Evaluates a sheet with each line's answer written as that line's menu choices say.
pub fn compute_formatted(lines: &[String], formats: &[LineFormat]) -> Computed {
    compute_with_formats(lines, formats, crate::prefs::current())
}

/// Evaluates and formats inside one settings scope, so dates print in the same zone they were read in.
#[cfg(test)]
pub fn compute_with(lines: &[String], settings: &Settings) -> Computed {
    compute_with_formats(lines, &[], settings)
}

fn compute_with_formats(lines: &[String], formats: &[LineFormat], settings: &Settings) -> Computed {
    scoped(settings, || compute_scoped(lines, formats, settings))
}

fn compute_scoped(lines: &[String], formats: &[LineFormat], settings: &Settings) -> Computed {
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let opts = SheetOptions {
        settings: settings.clone(),
        ..SheetOptions::default()
    };
    let results = evaluate_sheet(&refs, &opts);
    let base: FormatOptions = settings.format_options();
    let answers = results
        .iter()
        .enumerate()
        .map(|(i, r)| match &r.kind {
            LineKind::Value(v) => {
                let own = formats.get(i).copied().unwrap_or_default();
                let f = format_value(v, &own.apply(r.display_options(&base)));
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
    let left_out = floating_total_left_out(&results, &opts);
    Computed {
        results,
        answers,
        total,
        left_out,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn now_and_today_are_read_and_printed_in_the_local_zone() {
        use babbage_core::zones::Zone;
        let at = |zone: &str, now: i64| Settings {
            local_zone: Zone::named(zone),
            now_utc_seconds: Some(now),
            ..Settings::default()
        };
        let text = |s: &Settings| {
            let c = compute_with(&sheet(&["now", "today"]), s);
            (
                c.answers[0].text.clone().unwrap_or_default(),
                c.answers[1].text.clone().unwrap_or_default(),
            )
        };
        let (june, jan) = (1_718_452_800, 1_705_320_000);
        assert_eq!(text(&at("America/New_York", june)).0, "8:00 AM");
        assert_eq!(text(&at("America/New_York", jan)).0, "7:00 AM");
        // 2024-06-16 03:00 UTC is still the evening of June 15 in Los Angeles.
        assert_eq!(text(&at("America/Los_Angeles", 1_718_506_800)).1, "June 15");
        assert_eq!(
            text(&Settings {
                now_utc_seconds: Some(1_718_506_800),
                ..Settings::default()
            })
            .1,
            "June 16"
        );
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
