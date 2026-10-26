//! The text of a sheet and its editing rules, with no UI in it so the rules can be tested alone.
//!
//! The text is a list of lines. A position is a line and a byte column on a char boundary. The selection
//! is an anchor and a head; the caret is the head.

use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

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
    Other,
}

#[derive(Clone, Debug)]
struct Snap {
    lines: Vec<String>,
    anchor: Pos,
    head: Pos,
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

    pub fn lines(&self) -> &[String] {
        &self.lines
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
            anchor: self.anchor,
            head: self.head,
        }
    }

    fn restore(&mut self, s: Snap) {
        self.lines = s.lines;
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
        for (i, p) in rest.iter().enumerate() {
            line = a.line + 1 + i;
            self.lines.insert(line, (*p).to_owned());
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
}
