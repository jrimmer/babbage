//! Soft wrapping: where a long line breaks into rows, and which row of the sheet is which line.
//!
//! A row breaks after the last space that fits; a word wider than a row breaks inside it. Every line has at
//! least one row, and a row is one `ROW_H` tall, so a sheet's height is its row count.

/// A row never gets narrower than this, so a tiny window does not break every letter onto its own row.
pub const MIN_WIDTH: f32 = 80.;

/// Byte offsets where each row of `text` starts (the first is 0). `x_of(i)` is the distance from the start of
/// the line to byte `i`, for `i` on a character boundary.
pub fn breaks(text: &str, x_of: impl Fn(usize) -> f32, max: f32) -> Vec<usize> {
    let max = max.max(MIN_WIDTH);
    let mut starts = vec![0];
    let mut start = 0;
    loop {
        let x0 = x_of(start);
        let mut fit = start;
        let mut space = None;
        let mut first_end = start;
        let mut overflowed = false;
        for (off, c) in text[start..].char_indices() {
            let end = start + off + c.len_utf8();
            if first_end == start {
                first_end = end;
            }
            if x_of(end) - x0 > max {
                overflowed = true;
                break;
            }
            fit = end;
            if c == ' ' {
                space = Some(end);
            }
        }
        if !overflowed {
            return starts;
        }
        let cut = match space {
            Some(s) if s > start => s,
            _ if fit > start => fit,
            _ => first_end,
        };
        starts.push(cut);
        start = cut;
    }
}

struct Entry {
    text: String,
    width: u32,
    starts: Vec<usize>,
}

/// The rows of every line, kept between paints: a line is measured again only when its text or the width changes.
#[derive(Default)]
pub struct Wrap {
    entries: Vec<Entry>,
    /// The number of rows above each line.
    tops: Vec<usize>,
}

impl Wrap {
    /// Brings the rows up to date for `lines` lines at `width`. `measure(i, text, width)` gives the row starts of
    /// a line that has to be measured again.
    pub fn refresh(
        &mut self,
        lines: usize,
        width: f32,
        text_of: impl Fn(usize) -> String,
        mut measure: impl FnMut(usize, &str, f32) -> Vec<usize>,
    ) {
        self.entries.truncate(lines);
        let key = width.to_bits();
        for i in 0..lines {
            let text = text_of(i);
            let text = text.as_str();
            let fresh = self
                .entries
                .get(i)
                .is_some_and(|e| e.width == key && e.text == text);
            if fresh {
                continue;
            }
            let starts = if text.is_empty() {
                vec![0]
            } else {
                measure(i, text, width)
            };
            let entry = Entry {
                text: text.to_string(),
                width: key,
                starts,
            };
            match self.entries.get_mut(i) {
                Some(slot) => *slot = entry,
                None => self.entries.push(entry),
            }
        }
        self.tops.clear();
        let mut rows = 0;
        for e in &self.entries {
            self.tops.push(rows);
            rows += e.starts.len();
        }
    }

    /// How many rows `line` takes (one for a line not measured yet).
    pub fn rows_of(&self, line: usize) -> usize {
        self.entries.get(line).map_or(1, |e| e.starts.len())
    }

    /// The rows above `line`.
    pub fn top_row(&self, line: usize) -> usize {
        match self.tops.get(line) {
            Some(t) => *t,
            None => self.measured_rows() + (line - self.entries.len()),
        }
    }

    fn measured_rows(&self) -> usize {
        self.entries
            .last()
            .map_or(0, |e| self.tops[self.entries.len() - 1] + e.starts.len())
    }

    /// The rows of a sheet of `lines` lines.
    pub fn total_rows(&self, lines: usize) -> usize {
        self.measured_rows() + lines.saturating_sub(self.entries.len())
    }

    /// Where row `k` of `line` starts and ends, for a line `len` bytes long.
    pub fn row_span(&self, line: usize, k: usize, len: usize) -> (usize, usize) {
        match self.entries.get(line) {
            Some(e) => {
                let k = k.min(e.starts.len() - 1);
                (e.starts[k], e.starts.get(k + 1).copied().unwrap_or(len))
            }
            None => (0, len),
        }
    }

    /// The row of `line` that holds byte `col`: the last row that starts at or before it.
    pub fn row_containing(&self, line: usize, col: usize) -> usize {
        self.entries
            .get(line)
            .map_or(0, |e| e.starts.iter().rposition(|s| *s <= col).unwrap_or(0))
    }

    /// The line and the row within it of row `row` of the sheet, for a sheet of `lines` lines; past the end it is
    /// the last row of the last line.
    pub fn locate(&self, row: usize, lines: usize) -> (usize, usize) {
        if lines == 0 {
            return (0, 0);
        }
        if row >= self.total_rows(lines) {
            let last = lines - 1;
            return (last, self.rows_of(last) - 1);
        }
        let known = self.tops.partition_point(|t| *t <= row);
        if known < self.entries.len() || (known == self.entries.len() && row < self.measured_rows())
        {
            let line = known - 1;
            return (line, row - self.tops[line]);
        }
        (self.entries.len() + (row - self.measured_rows()), 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ten units a character.
    fn x(i: usize) -> f32 {
        i as f32 * 10.
    }

    fn wrapped(text: &str, max: f32) -> Vec<String> {
        let starts = breaks(text, x, max);
        starts
            .iter()
            .enumerate()
            .map(|(k, s)| text[*s..starts.get(k + 1).copied().unwrap_or(text.len())].to_string())
            .collect()
    }

    #[test]
    fn a_line_that_fits_is_one_row() {
        assert_eq!(breaks("short", x, 200.), vec![0]);
        assert_eq!(breaks("", x, 200.), vec![0]);
        assert_eq!(wrapped("exactly ten", 110.), vec!["exactly ten"]);
    }

    #[test]
    fn rows_break_after_the_last_space_that_fits() {
        assert_eq!(
            wrapped("one two three four five", 120.),
            vec!["one two ", "three four ", "five"]
        );
    }

    #[test]
    fn a_word_wider_than_a_row_breaks_inside_it() {
        assert_eq!(
            wrapped("abcdefghijklmnopqrstuvwxyz", 100.),
            vec!["abcdefghij", "klmnopqrst", "uvwxyz"]
        );
        assert_eq!(
            wrapped("ab cdefghijklmnopqrstuv", 100.),
            vec!["ab ", "cdefghijkl", "mnopqrstuv"]
        );
    }

    #[test]
    fn a_row_has_at_least_the_minimum_width_and_always_makes_progress() {
        let rows = wrapped("abcdefghijklmnopqrst", 1.);
        assert_eq!(rows.concat(), "abcdefghijklmnopqrst");
        assert!(rows.iter().all(|r| !r.is_empty()));
        assert!(rows.iter().all(|r| r.chars().count() <= 8));
    }

    #[test]
    fn breaks_land_on_character_boundaries() {
        let text = "héllo wörld ünïcode tëxt ❤ ❤ ❤ ❤ ❤ ❤";
        let chars = |i: usize| text[..i].chars().count() as f32 * 10.;
        for s in breaks(text, chars, 120.) {
            assert!(text.is_char_boundary(s), "{s}");
        }
    }

    fn sheet(lines: &[&str], max: f32) -> Wrap {
        let mut w = Wrap::default();
        w.refresh(
            lines.len(),
            max,
            |i| lines[i].to_string(),
            |_, t, m| breaks(t, x, m),
        );
        w
    }

    #[test]
    fn rows_stack_up_and_a_row_of_the_sheet_finds_its_line() {
        let w = sheet(&["a", "one two three four five", "", "b"], 120.);
        assert_eq!(w.rows_of(1), 3);
        assert_eq!(w.top_row(0), 0);
        assert_eq!(w.top_row(1), 1);
        assert_eq!(w.top_row(2), 4);
        assert_eq!(w.top_row(3), 5);
        assert_eq!(w.total_rows(4), 6);
        let located: Vec<_> = (0..6).map(|r| w.locate(r, 4)).collect();
        assert_eq!(located, [(0, 0), (1, 0), (1, 1), (1, 2), (2, 0), (3, 0)]);
        assert_eq!(w.locate(99, 4), (3, 0));
    }

    #[test]
    fn a_row_knows_where_it_starts_and_which_row_holds_a_column() {
        let w = sheet(&["one two three four five"], 120.);
        let len = 23;
        assert_eq!(w.row_span(0, 0, len), (0, 8));
        assert_eq!(w.row_span(0, 1, len), (8, 19));
        assert_eq!(w.row_span(0, 2, len), (19, 23));
        assert_eq!(w.row_span(0, 9, len), (19, 23));
        assert_eq!(w.row_containing(0, 0), 0);
        assert_eq!(w.row_containing(0, 7), 0);
        assert_eq!(w.row_containing(0, 8), 1);
        assert_eq!(w.row_containing(0, 23), 2);
    }

    #[test]
    fn lines_not_measured_yet_count_as_one_row_each() {
        let w = sheet(&["one two three four five"], 120.);
        assert_eq!(w.rows_of(5), 1);
        assert_eq!(w.top_row(1), 3);
        assert_eq!(w.top_row(4), 6);
        assert_eq!(w.total_rows(3), 5);
        assert_eq!(w.locate(3, 3), (1, 0));
        assert_eq!(w.locate(4, 3), (2, 0));
        assert_eq!(w.locate(9, 3), (2, 0));
        assert_eq!(Wrap::default().locate(0, 0), (0, 0));
        assert_eq!(Wrap::default().locate(4, 7), (4, 0));
    }

    #[test]
    fn only_changed_lines_are_measured_again() {
        let mut lines = vec!["one two three four five".to_string(), "x".into()];
        let mut w = Wrap::default();
        let mut measured = Vec::new();
        let go = |w: &mut Wrap, lines: &[String], width: f32, log: &mut Vec<usize>| {
            w.refresh(
                lines.len(),
                width,
                |i| lines[i].clone(),
                |i, t, m| {
                    log.push(i);
                    breaks(t, x, m)
                },
            )
        };
        go(&mut w, &lines, 120., &mut measured);
        assert_eq!(measured, [0, 1]);
        measured.clear();
        go(&mut w, &lines, 120., &mut measured);
        assert!(measured.is_empty());
        lines[1] = "y".into();
        go(&mut w, &lines, 120., &mut measured);
        assert_eq!(measured, [1]);
        measured.clear();
        go(&mut w, &lines, 300., &mut measured);
        assert_eq!(measured, [0, 1]);
        assert_eq!(w.rows_of(0), 1);
        lines.pop();
        go(&mut w, &lines, 300., &mut measured);
        assert_eq!(w.total_rows(1), 1);
    }
}
