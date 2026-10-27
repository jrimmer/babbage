//! Escape finishes the name of a variable: `a long var` becomes `a long variable name`. One match is put in at once;
//! several make a small list to choose from with Up and Down, taken with Enter, Tab or Escape.

use gpui::{AnyElement, Context, FontWeight, SharedString, div, prelude::*, px};

use crate::theme::Colors;
use crate::view::{ROW_H, SheetView};

/// The list of names a name being typed could be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Completion {
    pub line: usize,
    /// Where the half-typed name starts in the line (a byte column).
    pub from: usize,
    pub candidates: Vec<String>,
    pub selected: usize,
}

fn word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The names `before` (the line up to the caret) could be the start of, and where that start is: the longest ending
/// of `before`, starting at the beginning of a word, that begins some name and is not already all of it. Names are
/// matched without regard to case; the list is in alphabetical order.
pub fn candidates(before: &str, names: &[String]) -> Option<(usize, Vec<String>)> {
    for (start, _) in before.char_indices() {
        if before[..start].chars().next_back().is_some_and(word) {
            continue;
        }
        let typed = before[start..].to_lowercase();
        if typed.trim().is_empty() {
            continue;
        }
        let mut found: Vec<String> = names
            .iter()
            .filter(|n| n.to_lowercase().starts_with(&typed) && n.len() > typed.len())
            .cloned()
            .collect();
        if !found.is_empty() {
            found.sort();
            found.dedup();
            return Some((start, found));
        }
    }
    None
}

impl SheetView {
    /// Escape in the sheet: finish the name being typed, if some name starts that way.
    pub fn start_completion(&mut self, cx: &mut Context<Self>) {
        let head = self.ed.head();
        let line = self.ed.line(head.line);
        let names: Vec<String> = self
            .computed
            .names
            .get(head.line)
            .map(|v| {
                v.iter()
                    .filter(|(_, at)| *at != head.line)
                    .map(|(n, _)| n.clone())
                    .collect()
            })
            .unwrap_or_default();
        let Some((from, found)) = candidates(&line[..head.col], &names) else {
            return;
        };
        let c = Completion {
            line: head.line,
            from,
            candidates: found,
            selected: 0,
        };
        if c.candidates.len() == 1 {
            self.complete = Some(c);
            self.accept_completion(cx);
        } else {
            self.complete = Some(c);
            cx.notify();
        }
    }

    /// Puts the chosen name in place of what was typed.
    pub fn accept_completion(&mut self, cx: &mut Context<Self>) {
        let Some(c) = self.complete.take() else {
            return;
        };
        let name = c.candidates[c.selected.min(c.candidates.len() - 1)].clone();
        let head = self.ed.head();
        if head.line == c.line {
            let from = crate::editor_model::Pos::new(c.line, c.from);
            self.ed
                .replace(from, head, &name, crate::editor_model::Kind::Other);
        }
        self.after_edit(cx);
    }

    /// Up, Down, Enter, Tab and Escape while the list is open. False for any other key (which closes the list).
    pub fn completion_key(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        let Some(c) = self.complete.as_mut() else {
            return false;
        };
        match key {
            "up" => c.selected = c.selected.saturating_sub(1),
            "down" => c.selected = (c.selected + 1).min(c.candidates.len() - 1),
            "enter" | "tab" | "escape" => {
                self.accept_completion(cx);
                return true;
            }
            _ => {
                self.complete = None;
                cx.notify();
                return false;
            }
        }
        cx.notify();
        true
    }

    /// Where the list goes (window coordinates): under the caret.
    pub fn completion_origin(&self) -> Option<(f32, f32)> {
        let c = self.complete.as_ref()?;
        let l = self.layout.as_ref()?;
        let sh = self.shown_line(c.line);
        let col = sh.to_shown(self.ed.head().col);
        let k = self.wrap.row_containing(c.line, col);
        let start = self.wrap.row_span(c.line, k, sh.text.len()).0;
        let x = match l.shaped.get(c.line).and_then(Option::as_ref) {
            Some(s) => f32::from(l.text_x + s.x_for_index(col) - s.x_for_index(start)),
            None => f32::from(l.text_x),
        };
        let y = f32::from(l.top) + (self.wrap.top_row(c.line) + k + 1) as f32 * ROW_H - 4.;
        Some((x, y))
    }

    pub fn completion_view(&self, c: Colors, _cx: &mut Context<Self>) -> Option<AnyElement> {
        let comp = self.complete.as_ref()?;
        let (x, y) = self.completion_origin()?;
        Some(
            div()
                .absolute()
                .left(px(x))
                .top(px(y))
                .min_w(px(180.))
                .py(px(4.))
                .rounded(px(8.))
                .bg(c.caret)
                .text_color(c.background)
                .text_size(px(14.))
                .font_weight(FontWeight::MEDIUM)
                .children(comp.candidates.iter().enumerate().map(|(i, name)| {
                    div()
                        .px(px(12.))
                        .h(px(COMPLETION_ROW_H))
                        .flex()
                        .items_center()
                        .when(i == comp.selected, |d| d.bg(c.selection).text_color(c.text))
                        .child(SharedString::from(name.clone()))
                }))
                .into_any_element(),
        )
    }
}

pub const COMPLETION_ROW_H: f32 = 26.;

#[cfg(test)]
mod tests {
    use super::*;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn a_half_typed_name_finds_the_names_it_starts() {
        let n = names(&["a long variable name", "cost of development", "rent"]);
        assert_eq!(
            candidates("a long var", &n),
            Some((0, names(&["a long variable name"])))
        );
        assert_eq!(
            candidates("20 + cost o", &n),
            Some((5, names(&["cost of development"])))
        );
        assert_eq!(candidates("re", &n), Some((0, names(&["rent"]))));
        assert_eq!(candidates("RE", &n), Some((0, names(&["rent"]))));
    }

    #[test]
    fn several_matches_are_listed_alphabetically_and_a_whole_name_matches_nothing() {
        let n = names(&["total tax", "total spent", "tip"]);
        assert_eq!(
            candidates("total ", &n),
            Some((0, names(&["total spent", "total tax"])))
        );
        assert_eq!(candidates("total spent", &n), None);
        assert_eq!(candidates("", &n), None);
        assert_eq!(candidates("5 + ", &n), None);
        assert_eq!(candidates("zzz", &n), None);
    }

    #[test]
    fn a_name_must_start_at_a_word_and_the_longest_match_wins() {
        let n = names(&["rent", "rent due"]);
        // `parent` is not `rent`: the start must be a word's start.
        assert_eq!(candidates("parent", &n), None);
        assert_eq!(
            candidates("x + rent d", &n),
            Some((4, names(&["rent due"])))
        );
        // `1 + re` could only start at `re`.
        assert_eq!(
            candidates("1 + re", &n),
            Some((4, names(&["rent", "rent due"])))
        );
    }
}
