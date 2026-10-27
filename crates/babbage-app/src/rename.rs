//! Renaming a variable renames its uses: when the name on the left of `name = value` changes, the lines below that
//! use the old name are brought along, always, never, or after asking (the Editing setting).

use std::ops::Range;

use gpui::{AnyElement, Context, MouseButton, SharedString, Window, div, prelude::*, px};

use crate::appprefs::{self, RenamePolicy};
use crate::chrome::Rect;
use crate::editor_model::Pos;
use crate::theme::Colors;
use crate::view::SheetView;

/// Whole-word uses of `name` in `line`.
pub fn uses_of(line: &str, name: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    if name.is_empty() {
        return out;
    }
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut from = 0;
    while let Some(at) = line[from..].find(name) {
        let start = from + at;
        let end = start + name.len();
        from = end;
        let before = line[..start].chars().next_back().is_some_and(word);
        let after = line[end..].chars().next().is_some_and(word);
        if !before && !after {
            out.push(start..end);
        }
    }
    out
}

/// A rename waiting for an answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameOffer {
    pub old: String,
    pub new: String,
    /// Where the old name is used: line, byte range.
    pub edits: Vec<(usize, Range<usize>)>,
}

/// The offer's buttons, relative to the toast's top left.
pub const TOAST_W: f32 = 460.;
pub const TOAST_H: f32 = 48.;
pub fn buttons() -> [(&'static str, Rect); 2] {
    [
        (
            "rename-update",
            Rect {
                x: 290.,
                y: 9.,
                w: 90.,
                h: 30.,
            },
        ),
        (
            "rename-keep",
            Rect {
                x: 388.,
                y: 9.,
                w: 62.,
                h: 30.,
            },
        ),
    ]
}

impl SheetView {
    /// After an edit: the declared names `before` and now, and what to do about a renamed one.
    pub fn check_rename(&mut self, before: &[Option<String>], cx: &mut Context<Self>) {
        let policy = appprefs::rename_policy();
        let after: Vec<Option<String>> = self
            .computed
            .results
            .iter()
            .map(|r| r.declares.clone())
            .collect();
        if policy == RenamePolicy::Never || before.len() != after.len() {
            self.rename_origin = None;
            self.rename_offer = None;
            return;
        }
        // One line whose declared name changed to another name.
        let changed: Vec<usize> = (0..after.len())
            .filter(|i| before[*i].is_some() && after[*i].is_some() && before[*i] != after[*i])
            .collect();
        let (line, old) = match (&self.rename_origin, changed.as_slice()) {
            (Some((l, orig)), _) if after[*l].is_some() => (*l, orig.clone()),
            (_, [i]) => (*i, before[*i].clone().unwrap_or_default()),
            _ => {
                self.rename_offer = None;
                return;
            }
        };
        let Some(new) = after[line].clone() else {
            return;
        };
        if new == old
            || after
                .iter()
                .enumerate()
                .any(|(j, n)| j != line && n.as_deref() == Some(old.as_str()))
        {
            self.rename_origin = None;
            self.rename_offer = None;
            return;
        }
        let edits: Vec<(usize, Range<usize>)> = (line + 1..self.ed.len())
            .filter(|j| {
                !matches!(
                    self.computed.results.get(*j).map(|r| &r.kind),
                    Some(
                        babbage_core::LineKind::Comment
                            | babbage_core::LineKind::Heading(_)
                            | babbage_core::LineKind::Divider
                    )
                )
            })
            .flat_map(|j| {
                uses_of(self.ed.line(j), &old)
                    .into_iter()
                    .map(move |r| (j, r))
            })
            .collect();
        if edits.is_empty() {
            self.rename_origin = None;
            self.rename_offer = None;
            return;
        }
        match policy {
            RenamePolicy::Always => {
                self.rename_origin = None;
                self.rename_offer = None;
                self.apply_rename(&new, &edits, cx);
            }
            _ => {
                if self.rename_muted == Some(line) {
                    return;
                }
                self.rename_origin = Some((line, old.clone()));
                self.rename_offer = Some(RenameOffer { old, new, edits });
            }
        }
    }

    /// Puts `new` in every place `edits` names, as one undo step.
    fn apply_rename(&mut self, new: &str, edits: &[(usize, Range<usize>)], cx: &mut Context<Self>) {
        let edits: Vec<(Pos, Pos, String)> = edits
            .iter()
            .map(|(l, r)| (Pos::new(*l, r.start), Pos::new(*l, r.end), new.to_string()))
            .collect();
        self.ed.replace_many(&edits);
        self.after_edit(cx);
    }

    pub fn rename_answer(&mut self, update: bool, cx: &mut Context<Self>) {
        let Some(offer) = self.rename_offer.take() else {
            return;
        };
        let line = self.rename_origin.take().map(|(l, _)| l);
        if update {
            self.apply_rename(&offer.new, &offer.edits, cx);
        } else {
            self.rename_muted = line;
        }
        cx.notify();
    }

    /// Where the toast is (window coordinates): bottom left of the sheet.
    pub fn rename_toast_origin(&self) -> Option<(f32, f32)> {
        let l = self.layout.as_ref()?;
        Some((
            f32::from(l.bounds.left()) + 20.,
            f32::from(l.bounds.top() + l.bounds.size.height) - TOAST_H - 18.,
        ))
    }

    pub fn rename_toast(&self, c: Colors, cx: &mut Context<Self>) -> Option<AnyElement> {
        let offer = self.rename_offer.as_ref()?;
        let (x, y) = self.rename_toast_origin()?;
        let n = offer.edits.len();
        let mut toast = div()
            .occlude()
            .absolute()
            .left(px(x))
            .top(px(y))
            .w(px(TOAST_W))
            .h(px(TOAST_H))
            .bg(c.raised)
            .border_1()
            .border_color(c.line)
            .rounded(px(12.))
            .text_size(px(14.))
            .child(
                div()
                    .absolute()
                    .left(px(16.))
                    .top(px(13.))
                    .w(px(260.))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(SharedString::from(format!(
                        "Rename {} use{} of “{}”?",
                        n,
                        if n == 1 { "" } else { "s" },
                        offer.old
                    ))),
            );
        for (id, r) in buttons() {
            let update = id == "rename-update";
            toast = toast.child(
                div()
                    .absolute()
                    .left(px(r.x))
                    .top(px(r.y))
                    .w(px(r.w))
                    .h(px(r.h))
                    .id(id)
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(7.))
                    .bg(if update { c.caret } else { c.line })
                    .text_color(if update { c.background } else { c.text })
                    .hover(|d| d.bg(c.selection))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window: &mut Window, cx| {
                            window.focus(this.focus_handle_ref());
                            this.rename_answer(update, cx);
                        }),
                    )
                    .child(if update { "Update" } else { "Keep" }),
            );
        }
        Some(toast.into_any_element())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_found_where_it_stands_alone() {
        assert_eq!(uses_of("rent + 5", "rent"), vec![0..4]);
        assert_eq!(uses_of("rent + rent", "rent"), [0..4, 7..11]);
        assert!(uses_of("parent + rents", "rent").is_empty());
        assert_eq!(uses_of("total spent += $5", "total spent"), vec![0..11]);
        assert!(uses_of("anything", "").is_empty());
        assert!(uses_of("Rent", "rent").is_empty());
    }
}
