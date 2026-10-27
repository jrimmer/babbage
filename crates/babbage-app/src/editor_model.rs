//! The text of a sheet and its editing rules, with no UI in it so the rules can be tested alone.
//!
//! The text is a list of lines. A position is a line and a byte column on a char boundary. The selection
//! is an anchor and a head; the caret is the head.

use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use crate::linefmt::LineFormat;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

impl Pos {
    pub fn new(line: usize, col: usize) -> Pos {
        Pos { line, col }
    }
}

/// What an edit was, so a run of typing becomes one undo step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Typing,
    Deleting,
    /// Dragging a number with Alt held: every step of one drag is one undo step.
    Scrub,
    Other,
}

#[derive(Clone, Debug)]
struct Snap {
    lines: Vec<String>,
    formats: Vec<LineFormat>,
    anchor: Pos,
    head: Pos,
}

/// A sheet's undo and redo steps, kept while another sheet is open. `lines` is the text they lead from: they
/// come back only if the sheet still reads that way, so an undo never lands on text that changed meanwhile.
pub struct History {
    lines: Vec<String>,
    undo: Vec<Snap>,
    redo: Vec<Snap>,
}

/// Text the input method has shown but not committed: a byte range inside one line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Marked {
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug)]
pub struct Editor {
    lines: Vec<String>,
    /// How each line's answer is written; always as long as `lines`.
    formats: Vec<LineFormat>,
    anchor: Pos,
    head: Pos,
    /// The column (in graphemes) the caret returns to when moving up and down across short lines.
    goal: Option<usize>,
    undo: Vec<Snap>,
    redo: Vec<Snap>,
    last: Kind,
    pub marked: Option<Marked>,
}

#[derive(PartialEq)]
enum Class {
    Space,
    Word,
    Other,
}

fn class(g: &str) -> Class {
    let c = g.chars().next().unwrap_or(' ');
    if c.is_whitespace() {
        Class::Space
    } else if c.is_alphanumeric() || c == '_' {
        Class::Word
    } else {
        Class::Other
    }
}

fn prev_boundary(s: &str, col: usize) -> usize {
    s[..col]
        .grapheme_indices(true)
        .next_back()
        .map_or(0, |(i, _)| i)
}

fn next_boundary(s: &str, col: usize) -> usize {
    s[col..]
        .graphemes(true)
        .next()
        .map_or(s.len(), |g| col + g.len())
}

fn prev_word(s: &str, col: usize) -> usize {
    let mut at = col;
    let mut seen: Option<Class> = None;
    for (i, g) in s[..col].grapheme_indices(true).rev() {
        let c = class(g);
        match &seen {
            None if c == Class::Space => {}
            None => seen = Some(c),
            Some(k) if *k == c => {}
            Some(_) => break,
        }
        at = i;
    }
    at
}

fn next_word(s: &str, col: usize) -> usize {
    let mut at = col;
    let mut seen: Option<Class> = None;
    for (i, g) in s[col..].grapheme_indices(true) {
        let c = class(g);
        match &seen {
            None if c == Class::Space => {}
            None => seen = Some(c),
            Some(k) if *k == c => {}
            Some(_) => return col + i,
        }
        at = col + i + g.len();
    }
    at
}

impl Editor {
    pub fn new<S: AsRef<str>>(lines: &[S]) -> Editor {
        let mut lines: Vec<String> = lines.iter().map(|l| l.as_ref().to_owned()).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        Editor {
            formats: vec![LineFormat::default(); lines.len()],
            lines,
            anchor: Pos::default(),
            head: Pos::default(),
            goal: None,
            undo: Vec::new(),
            redo: Vec::new(),
            last: Kind::Other,
            marked: None,
        }
    }

    /// Takes the undo and redo steps out, to keep for later.
    pub fn take_history(&mut self) -> History {
        History {
            lines: self.lines.clone(),
            undo: std::mem::take(&mut self.undo),
            redo: std::mem::take(&mut self.redo),
        }
    }

    /// Puts kept steps back when the text is the text they were kept for. Returns whether it did.
    pub fn restore_history(&mut self, h: History) -> bool {
        if h.lines != self.lines {
            return false;
        }
        self.undo = h.undo;
        self.redo = h.redo;
        true
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// How line `i`'s answer is written.
    pub fn format(&self, i: usize) -> LineFormat {
        self.formats.get(i).copied().unwrap_or_default()
    }

    pub fn formats(&self) -> &[LineFormat] {
        &self.formats
    }

    /// Sets how line `i`'s answer is written, as one undo step. Does nothing when it is the same.
    pub fn set_format(&mut self, i: usize, f: LineFormat) {
        if i >= self.formats.len() || self.formats[i] == f {
            return;
        }
        let s = self.snap();
        self.undo.push(s);
        self.redo.clear();
        self.last = Kind::Other;
        self.formats[i] = f;
    }

    /// Gives lines their answers' formats, as read from a file (extra or missing entries are fixed up).
    pub fn with_formats(mut self, formats: &[LineFormat]) -> Editor {
        for (i, f) in formats.iter().enumerate().take(self.lines.len()) {
            self.formats[i] = *f;
        }
        self
    }

    /// Removes line `i` (the last line is emptied instead, as a sheet always has a line), as one undo step.
    pub fn delete_line(&mut self, i: usize) {
        if i >= self.lines.len() {
            return;
        }
        if self.lines.len() == 1 {
            let end = Pos::new(0, self.lines[0].len());
            return self.replace(Pos::new(0, 0), end, "", Kind::Other);
        }
        // The lines below keep their own formats: the deleted line's goes with it.
        let mut formats = self.formats.clone();
        formats.remove(i);
        if i + 1 < self.lines.len() {
            self.replace(Pos::new(i, 0), Pos::new(i + 1, 0), "", Kind::Other);
        } else {
            let prev_end = Pos::new(i - 1, self.lines[i - 1].len());
            let end = Pos::new(i, self.lines[i].len());
            self.replace(prev_end, end, "", Kind::Other);
        }
        self.formats = formats;
    }

    /// Replaces several ranges (each inside one line, none overlapping) as one undo step. The caret stays where it
    /// was, moved along by an edit before it on its own line.
    pub fn replace_many(&mut self, edits: &[(Pos, Pos, String)]) {
        if edits.is_empty() {
            return;
        }
        let s = self.snap();
        self.undo.push(s);
        self.redo.clear();
        self.last = Kind::Other;
        self.marked = None;
        let mut ordered: Vec<&(Pos, Pos, String)> = edits.iter().collect();
        ordered.sort_by_key(|b| std::cmp::Reverse(b.0));
        for (a, b, text) in ordered {
            if a.line != b.line || a.line >= self.lines.len() {
                continue;
            }
            let line = &mut self.lines[a.line];
            if b.col <= line.len() && line.is_char_boundary(a.col) && line.is_char_boundary(b.col) {
                line.replace_range(a.col..b.col, text);
                for p in [&mut self.head, &mut self.anchor] {
                    if p.line == a.line && p.col >= b.col {
                        p.col = p.col + text.len() - (b.col - a.col);
                    }
                }
            }
        }
    }

    /// Puts `text` on a new line after line `i`, as one undo step; the caret goes to its end.
    pub fn insert_line_after(&mut self, i: usize, text: &str) {
        let at = Pos::new(
            i.min(self.lines.len() - 1),
            self.lines[i.min(self.lines.len() - 1)].len(),
        );
        self.replace(at, at, &format!("\n{text}"), Kind::Other);
    }

    pub fn line(&self, i: usize) -> &str {
        self.lines.get(i).map_or("", String::as_str)
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn head(&self) -> Pos {
        self.head
    }

    pub fn anchor(&self) -> Pos {
        self.anchor
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    /// The selection as an ordered pair.
    pub fn selection(&self) -> (Pos, Pos) {
        (self.anchor.min(self.head), self.anchor.max(self.head))
    }

    pub fn has_selection(&self) -> bool {
        self.anchor != self.head
    }

    pub fn selected_text(&self) -> String {
        let (a, b) = self.selection();
        self.text_between(a, b)
    }

    pub fn text_between(&self, a: Pos, b: Pos) -> String {
        if a.line == b.line {
            return self.line(a.line)[a.col..b.col].to_owned();
        }
        let mut out = self.line(a.line)[a.col..].to_owned();
        for l in a.line + 1..b.line {
            out.push('\n');
            out.push_str(&self.lines[l]);
        }
        out.push('\n');
        out.push_str(&self.line(b.line)[..b.col]);
        out
    }

    /// Puts `p` back on a real char boundary inside the text.
    fn clamp(&self, p: Pos) -> Pos {
        let line = p.line.min(self.lines.len() - 1);
        let s = &self.lines[line];
        let mut col = p.col.min(s.len());
        while !s.is_char_boundary(col) {
            col -= 1;
        }
        Pos { line, col }
    }

    fn snap(&self) -> Snap {
        Snap {
            lines: self.lines.clone(),
            formats: self.formats.clone(),
            anchor: self.anchor,
            head: self.head,
        }
    }

    fn restore(&mut self, s: Snap) {
        self.lines = s.lines;
        self.formats = s.formats;
        self.anchor = s.anchor;
        self.head = s.head;
        self.goal = None;
        self.marked = None;
        self.last = Kind::Other;
    }

    pub fn undo(&mut self) -> bool {
        let Some(s) = self.undo.pop() else {
            return false;
        };
        self.redo.push(self.snap());
        self.restore(s);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(s) = self.redo.pop() else {
            return false;
        };
        self.undo.push(self.snap());
        self.restore(s);
        true
    }

    /// The one place text changes: replaces `a..b` with `text` (which may hold line breaks), puts the
    /// caret after it and records an undo step, joining it to the last one when both are the same run of
    /// typing or deleting.
    pub fn replace(&mut self, a: Pos, b: Pos, text: &str, kind: Kind) {
        let (a, b) = (self.clamp(a), self.clamp(b));
        let (a, b) = (a.min(b), a.max(b));
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        if a == b && text.is_empty() {
            return;
        }
        let joins = kind != Kind::Other
            && self.last == kind
            && (kind == Kind::Deleting || !text.chars().any(char::is_whitespace));
        if !joins {
            let s = self.snap();
            self.undo.push(s);
        }
        self.redo.clear();
        self.last = kind;
        self.marked = None;
        let tail = self.lines[b.line][b.col..].to_owned();
        let mut parts = text.split('\n');
        let first = parts.next().unwrap_or("");
        self.lines[a.line].truncate(a.col);
        self.lines[a.line].push_str(first);
        let mut line = a.line;
        let mut col = self.lines[a.line].len();
        let rest: Vec<&str> = parts.collect();
        self.lines.drain(a.line + 1..=b.line);
        self.formats.drain(a.line + 1..=b.line);
        for (i, p) in rest.iter().enumerate() {
            line = a.line + 1 + i;
            self.lines.insert(line, (*p).to_owned());
            self.formats.insert(line, LineFormat::default());
            col = p.len();
        }
        self.lines[line].push_str(&tail);
        self.head = Pos { line, col };
        self.anchor = self.head;
        self.goal = None;
    }

    /// Types `text` over the selection.
    #[cfg(test)]
    pub fn insert(&mut self, text: &str) {
        let (a, b) = self.selection();
        self.replace(a, b, text, Kind::Typing);
    }

    pub fn newline(&mut self) {
        let (a, b) = self.selection();
        self.replace(a, b, "\n", Kind::Other);
    }

    /// Backspace: the selection, else one grapheme (or word) back, joining lines at a line start.
    pub fn backspace(&mut self, word: bool) {
        if self.has_selection() {
            let (a, b) = self.selection();
            return self.replace(a, b, "", Kind::Other);
        }
        let h = self.head;
        let from = if h.col == 0 {
            if h.line == 0 {
                return;
            }
            Pos::new(h.line - 1, self.lines[h.line - 1].len())
        } else if word {
            Pos::new(h.line, prev_word(&self.lines[h.line], h.col))
        } else {
            Pos::new(h.line, prev_boundary(&self.lines[h.line], h.col))
        };
        self.replace(from, h, "", Kind::Deleting);
    }

    /// Delete: the selection, else one grapheme (or word) forward, joining lines at a line end.
    pub fn delete(&mut self, word: bool) {
        if self.has_selection() {
            let (a, b) = self.selection();
            return self.replace(a, b, "", Kind::Other);
        }
        let h = self.head;
        let s = &self.lines[h.line];
        let to = if h.col == s.len() {
            if h.line + 1 >= self.lines.len() {
                return;
            }
            Pos::new(h.line + 1, 0)
        } else if word {
            Pos::new(h.line, next_word(s, h.col))
        } else {
            Pos::new(h.line, next_boundary(s, h.col))
        };
        self.replace(h, to, "", Kind::Deleting);
    }

    /// Removes the selection and returns it, for cut.
    pub fn cut(&mut self) -> Option<String> {
        if !self.has_selection() {
            return None;
        }
        let t = self.selected_text();
        let (a, b) = self.selection();
        self.replace(a, b, "", Kind::Other);
        Some(t)
    }

    // Movement. Each takes `extend`: true keeps the anchor (Shift held).

    fn go(&mut self, to: Pos, extend: bool, keep_goal: bool) {
        self.head = self.clamp(to);
        if !extend {
            self.anchor = self.head;
        }
        if !keep_goal {
            self.goal = None;
        }
        self.last = Kind::Other;
    }

    pub fn set_caret(&mut self, to: Pos, extend: bool) {
        self.go(to, extend, false);
    }

    pub fn left(&mut self, extend: bool, word: bool) {
        if !extend && self.has_selection() {
            return self.go(self.selection().0, false, false);
        }
        let h = self.head;
        let to = if h.col == 0 {
            if h.line == 0 {
                return self.go(h, extend, false);
            }
            Pos::new(h.line - 1, self.lines[h.line - 1].len())
        } else if word {
            Pos::new(h.line, prev_word(&self.lines[h.line], h.col))
        } else {
            Pos::new(h.line, prev_boundary(&self.lines[h.line], h.col))
        };
        self.go(to, extend, false);
    }

    pub fn right(&mut self, extend: bool, word: bool) {
        if !extend && self.has_selection() {
            return self.go(self.selection().1, false, false);
        }
        let h = self.head;
        let s = &self.lines[h.line];
        let to = if h.col == s.len() {
            if h.line + 1 >= self.lines.len() {
                return self.go(h, extend, false);
            }
            Pos::new(h.line + 1, 0)
        } else if word {
            Pos::new(h.line, next_word(s, h.col))
        } else {
            Pos::new(h.line, next_boundary(s, h.col))
        };
        self.go(to, extend, false);
    }

    fn graphemes_before(&self, p: Pos) -> usize {
        self.lines[p.line][..p.col].graphemes(true).count()
    }

    fn col_at_grapheme(&self, line: usize, n: usize) -> usize {
        let s = &self.lines[line];
        s.grapheme_indices(true).nth(n).map_or(s.len(), |(i, _)| i)
    }

    pub fn up(&mut self, extend: bool) {
        let h = self.head;
        if h.line == 0 {
            return self.go(Pos::new(0, 0), extend, false);
        }
        let g = self.goal.unwrap_or_else(|| self.graphemes_before(h));
        let col = self.col_at_grapheme(h.line - 1, g);
        self.go(Pos::new(h.line - 1, col), extend, true);
        self.goal = Some(g);
    }

    pub fn down(&mut self, extend: bool) {
        let h = self.head;
        if h.line + 1 >= self.lines.len() {
            let end = Pos::new(h.line, self.lines[h.line].len());
            return self.go(end, extend, false);
        }
        let g = self.goal.unwrap_or_else(|| self.graphemes_before(h));
        let col = self.col_at_grapheme(h.line + 1, g);
        self.go(Pos::new(h.line + 1, col), extend, true);
        self.goal = Some(g);
    }

    pub fn home(&mut self, extend: bool) {
        self.go(Pos::new(self.head.line, 0), extend, false);
    }

    pub fn end(&mut self, extend: bool) {
        let l = self.head.line;
        self.go(Pos::new(l, self.lines[l].len()), extend, false);
    }

    pub fn doc_start(&mut self, extend: bool) {
        self.go(Pos::new(0, 0), extend, false);
    }

    pub fn doc_end(&mut self, extend: bool) {
        let l = self.lines.len() - 1;
        self.go(Pos::new(l, self.lines[l].len()), extend, false);
    }

    /// Comments the lines of the selection out with `// `, or takes the comment off when every line that has text
    /// already starts with one (the reference app's Command-/). The caret and the selection keep their place in the text.
    pub fn toggle_comment(&mut self) {
        let (a, b) = self.selection();
        let first = a.line;
        let last = if b.col == 0 && b.line > first {
            b.line - 1
        } else {
            b.line
        };
        let commented = |s: &str| s.trim_start().starts_with("//");
        let texts: Vec<&String> = self.lines[first..=last]
            .iter()
            .filter(|l| !l.trim().is_empty())
            .collect();
        if texts.is_empty() {
            return;
        }
        let uncomment = texts.iter().all(|l| commented(l));
        let mut shifts: Vec<isize> = Vec::new();
        let rewritten: Vec<String> = self.lines[first..=last]
            .iter()
            .map(|l| {
                if l.trim().is_empty() {
                    shifts.push(0);
                    l.clone()
                } else if uncomment {
                    let indent = l.len() - l.trim_start().len();
                    let after = &l[indent + 2..];
                    let kept = after.strip_prefix(' ').unwrap_or(after);
                    let removed = l.len() - indent - kept.len();
                    shifts.push(-(removed as isize));
                    format!("{}{}", &l[..indent], kept)
                } else {
                    shifts.push(3);
                    format!("// {l}")
                }
            })
            .collect();
        let (head, anchor) = (self.head, self.anchor);
        let end = Pos::new(last, self.lines[last].len());
        self.replace(Pos::new(first, 0), end, &rewritten.join("\n"), Kind::Other);
        let moved = |p: Pos| {
            if p.line < first || p.line > last {
                return p;
            }
            let shift = shifts[p.line - first];
            let col = (p.col as isize + shift).max(0) as usize;
            Pos::new(p.line, col.min(self.lines[p.line].len()))
        };
        let (anchor, head) = (moved(anchor), moved(head));
        self.set_caret(anchor, false);
        self.set_caret(head, true);
    }

    /// The text being composed by an input method, if one is composing.
    pub fn marked_text(&self) -> Option<String> {
        let m = self.marked?;
        self.lines
            .get(m.line)?
            .get(m.start..m.end)
            .map(str::to_owned)
    }

    /// Selects the run of letters and digits (or of spaces, or of marks) under `p`: a double click.
    pub fn select_word_at(&mut self, p: Pos) {
        let p = self.clamp(p);
        let s = &self.lines[p.line];
        let spans: Vec<(usize, &str)> = s.grapheme_indices(true).collect();
        if spans.is_empty() {
            return self.set_caret(p, false);
        }
        let at = spans.iter().rposition(|(i, _)| *i <= p.col).unwrap_or(0);
        let kind = class(spans[at].1);
        let mut first = at;
        while first > 0 && class(spans[first - 1].1) == kind {
            first -= 1;
        }
        let mut last = at;
        while last + 1 < spans.len() && class(spans[last + 1].1) == kind {
            last += 1;
        }
        let (start, end) = (spans[first].0, spans[last].0 + spans[last].1.len());
        self.set_caret(Pos::new(p.line, start), false);
        self.set_caret(Pos::new(p.line, end), true);
    }

    /// Selects the whole line `line`: a triple click.
    pub fn select_line(&mut self, line: usize) {
        let line = line.min(self.lines.len() - 1);
        self.set_caret(Pos::new(line, 0), false);
        self.set_caret(Pos::new(line, self.lines[line].len()), true);
    }

    pub fn select_all(&mut self) {
        self.doc_start(false);
        self.doc_end(true);
    }

    // Offsets into the text as one string, for the input method.

    pub fn offset_of(&self, p: Pos) -> usize {
        let p = self.clamp(p);
        self.lines[..p.line]
            .iter()
            .map(|l| l.len() + 1)
            .sum::<usize>()
            + p.col
    }

    pub fn pos_of(&self, mut offset: usize) -> Pos {
        for (i, l) in self.lines.iter().enumerate() {
            if offset <= l.len() {
                return self.clamp(Pos::new(i, offset));
            }
            offset -= l.len() + 1;
        }
        Pos::new(self.lines.len() - 1, self.lines[self.lines.len() - 1].len())
    }

    pub fn utf16_of(&self, p: Pos) -> usize {
        let text = self.text();
        let off = self.offset_of(p);
        text[..off].encode_utf16().count()
    }

    pub fn pos_of_utf16(&self, units: usize) -> Pos {
        let text = self.text();
        let mut n = 0;
        for (i, c) in text.char_indices() {
            if n >= units {
                return self.pos_of(i);
            }
            n += c.len_utf16();
        }
        self.pos_of(text.len())
    }

    pub fn range_of_utf16(&self, r: &Range<usize>) -> (Pos, Pos) {
        (self.pos_of_utf16(r.start), self.pos_of_utf16(r.end))
    }

    /// The text of lines `a..=b`, for tests and tidy copies.
    #[cfg(test)]
    fn all(&self) -> Vec<&str> {
        self.lines.iter().map(String::as_str).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ed(text: &[&str]) -> Editor {
        Editor::new(text)
    }

    #[test]
    fn typing_inserts_at_the_caret_and_moves_it() {
        let mut e = ed(&[""]);
        e.insert("12 + 12");
        assert_eq!(e.all(), vec!["12 + 12"]);
        assert_eq!(e.head(), Pos::new(0, 7));
    }

    #[test]
    fn a_new_editor_always_has_one_line() {
        assert_eq!(Editor::new::<&str>(&[]).all(), vec![""]);
    }

    #[test]
    fn enter_splits_the_line_and_backspace_joins_it() {
        let mut e = ed(&["ab cd"]);
        e.set_caret(Pos::new(0, 2), false);
        e.newline();
        assert_eq!(e.all(), vec!["ab", " cd"]);
        assert_eq!(e.head(), Pos::new(1, 0));
        e.backspace(false);
        assert_eq!(e.all(), vec!["ab cd"]);
        assert_eq!(e.head(), Pos::new(0, 2));
    }

    #[test]
    fn a_double_click_selects_the_word_and_a_triple_click_the_line() {
        let mut e = ed(&["12 + 345 apples", "second"]);
        e.select_word_at(Pos::new(0, 5));
        assert_eq!(e.selected_text(), "345");
        e.select_word_at(Pos::new(0, 0));
        assert_eq!(e.selected_text(), "12");
        e.select_word_at(Pos::new(0, 2));
        assert_eq!(e.selected_text(), " ", "a run of spaces is a word too");
        e.select_word_at(Pos::new(0, 15));
        assert_eq!(
            e.selected_text(),
            "apples",
            "the end of a line takes the last word"
        );
        e.select_line(1);
        assert_eq!(e.selected_text(), "second");
        e.select_line(9);
        assert_eq!(
            e.selected_text(),
            "second",
            "a line past the end is the last line"
        );
    }

    #[test]
    fn a_comment_is_toggled_on_the_selected_lines() {
        let mut e = ed(&["1 + 1", "", "2 + 2"]);
        e.set_caret(Pos::new(0, 3), false);
        e.toggle_comment();
        assert_eq!(e.all(), vec!["// 1 + 1", "", "2 + 2"]);
        assert_eq!(
            e.head(),
            Pos::new(0, 6),
            "the caret stays on the same character"
        );
        e.toggle_comment();
        assert_eq!(e.all(), vec!["1 + 1", "", "2 + 2"]);
        assert_eq!(e.head(), Pos::new(0, 3));
        e.select_all();
        e.toggle_comment();
        assert_eq!(
            e.all(),
            vec!["// 1 + 1", "", "// 2 + 2"],
            "blank lines are left alone"
        );
        e.toggle_comment();
        assert_eq!(e.all(), vec!["1 + 1", "", "2 + 2"]);
        // A mixed selection comments everything.
        e.set_caret(Pos::new(0, 0), false);
        e.toggle_comment();
        e.select_all();
        e.toggle_comment();
        assert_eq!(e.all(), vec!["// // 1 + 1", "", "// 2 + 2"]);
        // One undo takes the whole toggle back.
        e.undo();
        assert_eq!(e.all(), vec!["// 1 + 1", "", "2 + 2"]);
    }

    #[test]
    fn delete_at_the_line_end_pulls_the_next_line_up() {
        let mut e = ed(&["a", "b"]);
        e.set_caret(Pos::new(0, 1), false);
        e.delete(false);
        assert_eq!(e.all(), vec!["ab"]);
        e.doc_end(false);
        e.delete(false);
        assert_eq!(e.all(), vec!["ab"], "nothing after the end");
    }

    #[test]
    fn backspace_steps_over_a_whole_grapheme() {
        let mut e = ed(&["e\u{301}x"]);
        e.end(false);
        e.backspace(false);
        e.backspace(false);
        assert_eq!(e.all(), vec![""], "the accent goes with its letter");
    }

    #[test]
    fn word_motion_and_word_delete_work_on_runs() {
        let mut e = ed(&["foo bar+baz"]);
        e.end(false);
        e.backspace(true);
        assert_eq!(e.all(), vec!["foo bar+"]);
        e.backspace(true);
        assert_eq!(e.all(), vec!["foo bar"]);
        e.home(false);
        e.right(false, true);
        assert_eq!(e.head().col, 3);
        e.right(false, true);
        assert_eq!(e.head().col, 7);
    }

    #[test]
    fn selection_typing_replaces_it() {
        let mut e = ed(&["hello world"]);
        e.set_caret(Pos::new(0, 6), false);
        e.end(true);
        assert_eq!(e.selected_text(), "world");
        e.insert("there");
        assert_eq!(e.all(), vec!["hello there"]);
        assert!(!e.has_selection());
    }

    #[test]
    fn a_selection_over_several_lines_is_one_string_and_deletes_whole() {
        let mut e = ed(&["one", "two", "three"]);
        e.set_caret(Pos::new(0, 1), false);
        e.set_caret(Pos::new(2, 2), true);
        assert_eq!(e.selected_text(), "ne\ntwo\nth");
        assert_eq!(e.cut().as_deref(), Some("ne\ntwo\nth"));
        assert_eq!(e.all(), vec!["oree"]);
    }

    #[test]
    fn pasting_text_with_line_breaks_makes_lines() {
        let mut e = ed(&["ad"]);
        e.set_caret(Pos::new(0, 1), false);
        e.insert("b\r\nc\n");
        assert_eq!(e.all(), vec!["ab", "c", "d"]);
        assert_eq!(e.head(), Pos::new(2, 0));
    }

    #[test]
    fn up_and_down_remember_the_column_across_short_lines() {
        let mut e = ed(&["abcdef", "ab", "abcdef"]);
        e.set_caret(Pos::new(0, 5), false);
        e.down(false);
        assert_eq!(e.head(), Pos::new(1, 2));
        e.down(false);
        assert_eq!(e.head(), Pos::new(2, 5));
        e.up(false);
        e.up(false);
        assert_eq!(e.head(), Pos::new(0, 5));
    }

    #[test]
    fn up_on_the_first_line_goes_to_the_start_and_down_on_the_last_to_the_end() {
        let mut e = ed(&["abc"]);
        e.set_caret(Pos::new(0, 2), false);
        e.up(false);
        assert_eq!(e.head(), Pos::new(0, 0));
        e.down(false);
        assert_eq!(e.head(), Pos::new(0, 3));
    }

    #[test]
    fn arrows_collapse_a_selection_to_its_edge() {
        let mut e = ed(&["abcdef"]);
        e.set_caret(Pos::new(0, 1), false);
        e.set_caret(Pos::new(0, 4), true);
        e.left(false, false);
        assert_eq!(e.head().col, 1);
        e.set_caret(Pos::new(0, 4), true);
        e.right(false, false);
        assert_eq!(e.head().col, 4);
    }

    #[test]
    fn undo_and_redo_restore_text_and_caret() {
        let mut e = ed(&["a"]);
        e.end(false);
        e.insert("b");
        e.newline();
        e.insert("c");
        assert_eq!(e.all(), vec!["ab", "c"]);
        assert!(e.undo());
        assert_eq!(e.all(), vec!["ab", ""]);
        assert!(e.undo());
        assert_eq!(e.all(), vec!["ab"]);
        assert_eq!(e.head(), Pos::new(0, 2));
        assert!(e.undo());
        assert_eq!(e.all(), vec!["a"]);
        assert!(!e.undo());
        assert!(e.redo());
        assert_eq!(e.all(), vec!["ab"]);
        e.redo();
        e.redo();
        assert_eq!(e.all(), vec!["ab", "c"]);
        assert!(!e.redo());
    }

    #[test]
    fn a_run_of_typing_without_spaces_is_one_undo_step_and_a_space_starts_another() {
        let mut e = ed(&[""]);
        for c in ["1", "2", " ", "+", " ", "3"] {
            e.insert(c);
        }
        assert_eq!(e.all(), vec!["12 + 3"]);
        e.undo();
        assert_eq!(e.all(), vec!["12 +"]);
        e.undo();
        assert_eq!(e.all(), vec!["12"]);
        e.undo();
        assert_eq!(e.all(), vec![""]);
    }

    #[test]
    fn kept_history_comes_back_only_for_the_same_text() {
        let mut e = Editor::new(&["a"]);
        e.insert("b");
        e.insert(" c");
        let kept = e.take_history();
        assert!(!e.undo(), "taking the history empties the editor's");
        let mut other = Editor::new(&["different"]);
        assert!(!other.restore_history(kept));
        assert!(!other.undo());
        let mut e2 = Editor::new(&["a"]);
        e2.insert("b");
        let lines = e2.lines().to_vec();
        let kept = e2.take_history();
        let mut back = Editor::new(&lines);
        assert!(back.restore_history(kept));
        assert!(back.undo());
        assert_eq!(back.lines(), ["a"]);
    }

    #[test]
    fn a_new_edit_clears_the_redo_stack() {
        let mut e = ed(&[""]);
        e.insert("a");
        e.undo();
        e.insert("b");
        assert!(!e.redo());
    }

    #[test]
    fn moving_the_caret_ends_a_typing_run() {
        let mut e = ed(&[""]);
        e.insert("a");
        e.left(false, false);
        e.insert("b");
        assert_eq!(e.all(), vec!["ba"]);
        e.undo();
        assert_eq!(e.all(), vec!["a"]);
    }

    #[test]
    fn utf16_offsets_count_wide_characters_as_two_units() {
        let e = ed(&["a😀b", "c"]);
        assert_eq!(e.utf16_of(Pos::new(0, 5)), 3);
        assert_eq!(e.pos_of_utf16(3), Pos::new(0, 5));
        assert_eq!(e.pos_of_utf16(5), Pos::new(1, 0));
        assert_eq!(e.pos_of_utf16(99), Pos::new(1, 1));
        assert_eq!(e.range_of_utf16(&(1..3)), (Pos::new(0, 1), Pos::new(0, 5)));
    }

    #[test]
    fn select_all_and_replace_empties_the_sheet_of_old_text() {
        let mut e = ed(&["a", "b"]);
        e.select_all();
        e.insert("z");
        assert_eq!(e.all(), vec!["z"]);
    }

    #[test]
    fn a_position_past_the_end_is_clamped_not_a_panic() {
        let mut e = ed(&["é"]);
        e.set_caret(Pos::new(9, 1), false);
        assert_eq!(
            e.head(),
            Pos::new(0, 0),
            "byte 1 is inside é, so it backs up to a boundary"
        );
        e.replace(Pos::new(0, 0), Pos::new(5, 5), "x", Kind::Other);
        assert_eq!(e.all(), vec!["x"]);
    }

    fn dp(p: u32) -> LineFormat {
        LineFormat {
            places: Some(p),
            ..LineFormat::default()
        }
    }

    fn formatted(e: &Editor) -> Vec<Option<u32>> {
        e.formats().iter().map(|f| f.places).collect()
    }

    #[test]
    fn a_line_keeps_its_format_through_edits_above_and_inside_it() {
        let mut e = Editor::new(&["a", "b", "c"]);
        e.set_format(1, dp(4));
        assert_eq!(formatted(&e), [None, Some(4), None]);
        // A line inserted above pushes it down; text typed inside it leaves it alone.
        e.replace(Pos::new(0, 1), Pos::new(0, 1), "\nnew", Kind::Other);
        assert_eq!(e.all(), vec!["a", "new", "b", "c"]);
        assert_eq!(formatted(&e), [None, None, Some(4), None]);
        e.replace(Pos::new(2, 1), Pos::new(2, 1), "x", Kind::Other);
        assert_eq!(formatted(&e), [None, None, Some(4), None]);
        // Joining the line above into it drops the joined line's format, keeping the first line's.
        e.replace(Pos::new(1, 3), Pos::new(2, 0), "", Kind::Other);
        assert_eq!(e.all(), vec!["a", "newbx", "c"]);
        assert_eq!(formatted(&e), [None, None, None]);
    }

    #[test]
    fn splitting_a_formatted_line_leaves_the_new_line_plain() {
        let mut e = Editor::new(&["abcd"]);
        e.set_format(0, dp(2));
        e.replace(Pos::new(0, 2), Pos::new(0, 2), "\n", Kind::Other);
        assert_eq!(formatted(&e), [Some(2), None]);
    }

    #[test]
    fn undo_and_redo_bring_formats_back_with_the_text() {
        let mut e = Editor::new(&["1/3", "2/3"]);
        e.set_format(0, dp(3));
        e.set_format(1, dp(5));
        assert!(e.undo());
        assert_eq!(formatted(&e), [Some(3), None]);
        assert!(e.undo());
        assert_eq!(formatted(&e), [None, None]);
        assert!(e.redo());
        assert!(e.redo());
        assert_eq!(formatted(&e), [Some(3), Some(5)]);
        // Setting the same format again is no step.
        e.set_format(1, dp(5));
        assert!(e.undo());
        assert_eq!(formatted(&e), [Some(3), None]);
    }

    #[test]
    fn formats_are_read_from_a_file_and_fixed_up() {
        let e = Editor::new(&["a", "b"]).with_formats(&[dp(1), LineFormat::default(), dp(9)]);
        assert_eq!(formatted(&e), [Some(1), None]);
        assert_eq!(Editor::new::<&str>(&[]).formats().len(), 1);
    }

    #[test]
    fn a_line_can_be_deleted_or_added_after_another() {
        let mut e = Editor::new(&["a", "b", "c"]);
        e.set_format(2, dp(2));
        e.delete_line(1);
        assert_eq!(e.all(), vec!["a", "c"]);
        assert_eq!(formatted(&e), [None, Some(2)]);
        e.delete_line(1);
        assert_eq!(e.all(), vec!["a"]);
        e.delete_line(0);
        assert_eq!(e.all(), vec![""], "the last line is emptied, not removed");
        e.insert_line_after(0, "subtotal");
        assert_eq!(e.all(), vec!["", "subtotal"]);
        assert_eq!(e.head(), Pos::new(1, 8));
        assert!(e.undo());
        assert_eq!(e.all(), vec![""]);
    }
}
