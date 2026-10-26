//! A sheet with its answers as plain text, CSV, HTML or PDF.
//!
//! The store knows nothing of the engine: the caller says what each line shows. The plain-text form writes a
//! line, then ` | ` and its answer, the way the reference's examples are laid out; the CSV form has one
//! `line,answer` row per line and a last `Total` row; the HTML form is one self-contained page; the PDF form
//! is a plain document in a monospaced font, one page after another, with the answers in a column on the right.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::{IoContext, Result, write_atomic};

/// What kind of line a row is, for the forms that style them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    Plain,
    Heading,
    Comment,
    Divider,
    Blank,
}

/// One line of the sheet and what it shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub text: String,
    pub answer: Option<String>,
    pub kind: RowKind,
}

impl Row {
    pub fn new(text: &str, answer: Option<&str>, kind: RowKind) -> Row {
        Row {
            text: text.to_owned(),
            answer: answer.map(str::to_owned),
            kind,
        }
    }
}

fn width(s: &str) -> usize {
    s.chars().count()
}

/// `1 + 2      | 3`: each line, and for the lines with an answer the answer after a bar, aligned.
pub fn text(rows: &[Row], total: Option<&str>) -> String {
    let wide = rows
        .iter()
        .filter(|r| r.answer.is_some())
        .map(|r| width(&r.text))
        .chain(total.map(|_| "Total".len()))
        .max()
        .unwrap_or(0);
    let mut out = String::new();
    for r in rows {
        match &r.answer {
            Some(a) => {
                let pad = " ".repeat(wide - width(&r.text));
                let _ = writeln!(out, "{}{pad} | {a}", r.text);
            }
            None => {
                let _ = writeln!(out, "{}", r.text);
            }
        }
    }
    if let Some(t) = total {
        let _ = writeln!(out, "\nTotal{} | {t}", " ".repeat(wide - "Total".len()));
    }
    out
}

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}

/// `text,answer` per line, ended by `\r\n`, and a `Total` row when the sheet has a total.
pub fn csv(rows: &[Row], total: Option<&str>) -> String {
    let mut out = String::new();
    for r in rows {
        let _ = write!(
            out,
            "{},{}\r\n",
            csv_field(&r.text),
            csv_field(r.answer.as_deref().unwrap_or(""))
        );
    }
    if let Some(t) = total {
        let _ = write!(out, "Total,{}\r\n", csv_field(t));
    }
    out
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// One self-contained page: a table of lines and answers, headings and comments styled, the total last.
pub fn html(title: &str, rows: &[Row], total: Option<&str>) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n<style>\n\
         body {{ font: 15px/1.5 ui-monospace, Menlo, Consolas, monospace; margin: 2rem; color: #222; }}\n\
         table {{ border-collapse: collapse; }}\n\
         td {{ padding: 0 1.5rem 0 0; white-space: pre-wrap; vertical-align: top; }}\n\
         td.answer {{ text-align: right; padding-right: 0; border-left: 1px solid #ccc; padding-left: 1rem; }}\n\
         tr.heading td {{ font-weight: bold; font-size: 1.15em; padding-top: .75rem; }}\n\
         tr.comment td {{ color: #888; }}\n\
         tr.divider td {{ border-bottom: 1px solid #ccc; }}\n\
         tr.total td {{ font-weight: bold; border-top: 1px solid #222; padding-top: .5rem; }}\n\
         </style>\n</head>\n<body>\n<table>\n",
        escape(title)
    );
    for r in rows {
        let class = match r.kind {
            RowKind::Plain => "line",
            RowKind::Heading => "heading",
            RowKind::Comment => "comment",
            RowKind::Divider => "divider",
            RowKind::Blank => "blank",
        };
        let shown = match r.kind {
            RowKind::Heading => r.text.trim_start_matches('#').trim_start().to_owned(),
            RowKind::Divider => String::new(),
            _ => r.text.clone(),
        };
        let _ = writeln!(
            out,
            "<tr class=\"{class}\"><td>{}</td><td class=\"answer\">{}</td></tr>",
            escape(&shown),
            escape(r.answer.as_deref().unwrap_or(""))
        );
    }
    if let Some(t) = total {
        let _ = writeln!(
            out,
            "<tr class=\"total\"><td>Total</td><td class=\"answer\">{}</td></tr>",
            escape(t)
        );
    }
    out.push_str("</table>\n</body>\n</html>\n");
    out
}

// The PDF form.

const PAGE_W: f32 = 595.0;
const PAGE_H: f32 = 842.0;
const MARGIN: f32 = 50.0;
const FONT_SIZE: f32 = 10.0;
const LEADING: f32 = 13.0;
/// A Courier character is six tenths of the font size wide.
const CHAR_W: f32 = 6.0;
/// Characters of a line that fit before the answer column.
const TEXT_COLUMNS: usize = 62;

/// A string as the bytes of a PDF literal: WinAnsi for the standard fonts, `?` for what they cannot show.
fn pdf_string(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '(' | ')' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            ' '..='~' => out.push(c),
            '\u{20ac}' => out.push_str("\\200"),
            '\u{2018}' => out.push_str("\\221"),
            '\u{2019}' => out.push_str("\\222"),
            '\u{201c}' => out.push_str("\\223"),
            '\u{201d}' => out.push_str("\\224"),
            '\u{2013}' => out.push_str("\\226"),
            '\u{2014}' => out.push_str("\\227"),
            '\u{00a0}'..='\u{00ff}' => {
                let _ = write!(out, "\\{:03o}", c as u32);
            }
            _ => out.push('?'),
        }
    }
    out
}

/// The text lines of one row: a long line goes on to further lines, and the answer sits on the first.
fn wrapped(r: &Row) -> Vec<(String, Option<String>)> {
    let chars: Vec<char> = r.text.chars().collect();
    let mut parts: Vec<String> = chars
        .chunks(TEXT_COLUMNS)
        .map(|c| c.iter().collect())
        .collect();
    if parts.is_empty() {
        parts.push(String::new());
    }
    parts
        .into_iter()
        .enumerate()
        .map(|(i, p)| (p, if i == 0 { r.answer.clone() } else { None }))
        .collect()
}

struct Placed {
    text: String,
    answer: Option<String>,
    bold: bool,
}

/// A PDF of the sheet: the title, the lines with their answers, the total, on as many A4 pages as it takes.
pub fn pdf(title: &str, rows: &[Row], total: Option<&str>) -> Vec<u8> {
    let mut lines: Vec<Placed> = vec![
        Placed {
            text: title.chars().take(TEXT_COLUMNS).collect(),
            answer: None,
            bold: true,
        },
        Placed {
            text: String::new(),
            answer: None,
            bold: false,
        },
    ];
    for r in rows {
        let bold = r.kind == RowKind::Heading;
        for (text, answer) in wrapped(r) {
            lines.push(Placed { text, answer, bold });
        }
    }
    if let Some(t) = total {
        lines.push(Placed {
            text: String::new(),
            answer: None,
            bold: false,
        });
        lines.push(Placed {
            text: "Total".to_owned(),
            answer: Some(t.to_owned()),
            bold: true,
        });
    }
    let per_page = (((PAGE_H - 2.0 * MARGIN) / LEADING) as usize).max(1);
    let pages: Vec<&[Placed]> = lines.chunks(per_page).collect();

    // Objects: 1 catalog, 2 page tree, 3 and 4 the fonts, then a page and its content for each page.
    let mut objects: Vec<String> = Vec::new();
    let kids: Vec<String> = (0..pages.len())
        .map(|i| format!("{} 0 R", 5 + 2 * i))
        .collect();
    objects.push("<< /Type /Catalog /Pages 2 0 R >>".to_owned());
    objects.push(format!(
        "<< /Type /Pages /Kids [{}] /Count {} >>",
        kids.join(" "),
        pages.len()
    ));
    objects.push(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Courier /Encoding /WinAnsiEncoding >>"
            .to_owned(),
    );
    objects.push(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Courier-Bold /Encoding /WinAnsiEncoding >>"
            .to_owned(),
    );
    for (i, page) in pages.iter().enumerate() {
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_W} {PAGE_H}] \
             /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {} 0 R >>",
            6 + 2 * i
        ));
        let mut content = String::new();
        let answer_right = PAGE_W - MARGIN;
        for (n, p) in page.iter().enumerate() {
            let y = PAGE_H - MARGIN - LEADING * (n as f32 + 1.0) + 3.0;
            let font = if p.bold { "F2" } else { "F1" };
            if !p.text.is_empty() {
                let _ = writeln!(
                    content,
                    "BT /{font} {FONT_SIZE} Tf 1 0 0 1 {MARGIN} {y:.2} Tm ({}) Tj ET",
                    pdf_string(&p.text)
                );
            }
            if let Some(a) = &p.answer {
                let x = answer_right - CHAR_W * width(a) as f32;
                let _ = writeln!(
                    content,
                    "BT /{font} {FONT_SIZE} Tf 1 0 0 1 {x:.2} {y:.2} Tm ({}) Tj ET",
                    pdf_string(a)
                );
            }
        }
        objects.push(format!(
            "<< /Length {} >>\nstream\n{content}endstream",
            content.len()
        ));
    }

    let mut out: Vec<u8> = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// Writes `<name>.txt`, `.csv`, `.html` and `.pdf` into `dir` (made if it is missing) and returns their paths.
pub fn write_all(
    dir: &Path,
    name: &str,
    rows: &[Row],
    total: Option<&str>,
) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(dir).ctx(|| format!("making {}", dir.display()))?;
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || " -_.".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe = safe.trim().trim_matches('.');
    let safe = if safe.is_empty() { "sheet" } else { safe };
    let files: [(&str, Vec<u8>); 4] = [
        ("txt", text(rows, total).into_bytes()),
        ("csv", csv(rows, total).into_bytes()),
        ("html", html(name, rows, total).into_bytes()),
        ("pdf", pdf(name, rows, total)),
    ];
    let mut paths = Vec::new();
    for (ext, bytes) in files {
        let path = dir.join(format!("{safe}.{ext}"));
        write_atomic(&path, &bytes)?;
        paths.push(path);
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Row> {
        vec![
            Row::new("# Trip", None, RowKind::Heading),
            Row::new("1 + 2", Some("3"), RowKind::Plain),
            Row::new("hotel, 3 nights × $120", Some("$360.00"), RowKind::Plain),
            Row::new("// a note", None, RowKind::Comment),
            Row::new("", None, RowKind::Blank),
        ]
    }

    #[test]
    fn the_text_form_aligns_the_answers_after_a_bar() {
        let t = text(&sample(), Some("$363.00"));
        assert_eq!(
            t,
            "# Trip\n1 + 2                  | 3\nhotel, 3 nights × $120 | $360.00\n// a note\n\n\nTotal                  | $363.00\n"
        );
    }

    #[test]
    fn the_text_form_with_no_answers_is_the_lines() {
        let rows = vec![Row::new("hello", None, RowKind::Plain)];
        assert_eq!(text(&rows, None), "hello\n");
    }

    #[test]
    fn the_csv_form_quotes_what_needs_it() {
        let c = csv(&sample(), Some("$363.00"));
        assert_eq!(
            c,
            "# Trip,\r\n1 + 2,3\r\n\"hotel, 3 nights × $120\",$360.00\r\n// a note,\r\n,\r\nTotal,$363.00\r\n"
        );
        let quoted = csv(&[Row::new("say \"hi\"", Some("1"), RowKind::Plain)], None);
        assert_eq!(quoted, "\"say \"\"hi\"\"\",1\r\n");
    }

    #[test]
    fn the_html_form_escapes_and_styles() {
        let h = html("A <b> & c", &sample(), Some("$363.00"));
        assert!(h.contains("<title>A &lt;b&gt; &amp; c</title>"));
        assert!(
            h.contains("<tr class=\"heading\"><td>Trip</td>"),
            "the # is not shown"
        );
        assert!(h.contains("<tr class=\"comment\"><td>// a note</td>"));
        assert!(h.contains("<td class=\"answer\">$360.00</td>"));
        assert!(h.contains("<tr class=\"total\"><td>Total</td><td class=\"answer\">$363.00</td>"));
        assert!(h.starts_with("<!doctype html>") && h.ends_with("</html>\n"));
        let tricky = html("t", &[Row::new("<script>", None, RowKind::Plain)], None);
        assert!(!tricky.contains("<script>"), "markup in a line is escaped");
    }

    /// Every cross-reference offset points at the object it names, and the trailer says where the table is.
    fn check_structure(pdf: &[u8]) -> usize {
        let text = String::from_utf8_lossy(pdf).into_owned();
        assert!(text.starts_with("%PDF-1.4\n") && text.ends_with("%%EOF\n"));
        let start: usize = text
            .rsplit("startxref\n")
            .next()
            .and_then(|t| t.lines().next())
            .and_then(|n| n.parse().ok())
            .expect("a startxref offset");
        assert!(text[start..].starts_with("xref\n"));
        let table: Vec<&str> = text[start..].lines().skip(1).collect();
        let count: usize = table[0]
            .split_whitespace()
            .nth(1)
            .and_then(|n| n.parse().ok())
            .expect("the size");
        for n in 1..count {
            let entry = table[1 + n];
            let offset: usize = entry[..10].parse().expect("an offset");
            assert!(
                text[offset..].starts_with(&format!("{n} 0 obj\n")),
                "object {n} is not at {offset}"
            );
        }
        count - 1
    }

    #[test]
    fn the_pdf_form_is_well_formed_and_carries_the_text() {
        let p = pdf("Trip", &sample(), Some("$363.00"));
        let objects = check_structure(&p);
        assert_eq!(
            objects, 6,
            "catalog, tree, two fonts, one page and its content"
        );
        let text = String::from_utf8_lossy(&p);
        assert!(text.contains("/Count 1"));
        assert!(text.contains("(1 + 2)"), "the line");
        assert!(text.contains("($360.00)"), "its answer");
        assert!(
            text.contains("(hotel, 3 nights \\327 $120)"),
            "× is a Latin-1 character in octal"
        );
        assert!(text.contains("(Total)") && text.contains("($363.00)"));
    }

    #[test]
    fn a_long_sheet_runs_onto_more_pages_and_a_long_line_wraps() {
        let rows: Vec<Row> = (0..200)
            .map(|i| {
                Row::new(
                    &format!("{i} + 1"),
                    Some(&format!("{}", i + 1)),
                    RowKind::Plain,
                )
            })
            .collect();
        let p = pdf("Long", &rows, None);
        check_structure(&p);
        let text = String::from_utf8_lossy(&p);
        assert!(
            text.contains("/Count 4"),
            "{}",
            &text[..text.find("/Count").map_or(0, |i| i + 12)]
        );
        let long = Row::new(&"x".repeat(130), Some("1"), RowKind::Plain);
        let wrapped = wrapped(&long);
        assert_eq!(wrapped.len(), 3);
        assert_eq!(
            wrapped[0].1.as_deref(),
            Some("1"),
            "the answer is on the first part"
        );
        assert!(wrapped[1].1.is_none());
    }

    #[test]
    fn characters_the_font_cannot_show_become_question_marks() {
        assert_eq!(pdf_string("π (a) \\ € é"), "? \\(a\\) \\\\ \\200 \\351");
    }

    #[test]
    fn writing_makes_the_four_files_and_a_safe_name() {
        let dir = std::env::temp_dir().join(format!("babbage-export-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let paths = write_all(&dir, "a/b: c", &sample(), Some("$363.00")).unwrap();
        let names: Vec<String> = paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            ["a_b_ c.txt", "a_b_ c.csv", "a_b_ c.html", "a_b_ c.pdf"]
        );
        assert!(std::fs::read_to_string(&paths[0]).unwrap().contains("| 3"));
        assert!(std::fs::read(&paths[3]).unwrap().starts_with(b"%PDF-1.4"));
        let empty = write_all(&dir, "  ", &[], None).unwrap();
        assert!(empty[0].ends_with("sheet.txt"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
