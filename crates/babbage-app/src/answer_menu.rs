//! The menu behind the `⋯` on an answer: copy the answer, choose how many decimal places it shows, switch the
//! thousands separators, the `k`/`M`/`G` notation and fixed money digits on or off, show a number that never ends
//! with its repeating part, make a subtotal line, delete the line. Every control has a fixed place (`items`) so the acceptance checks can aim at it with a real pointer.

use gpui::{
    AnyElement, ClipboardItem, Context, FontWeight, MouseButton, SharedString, Window, div,
    prelude::*, px,
};

use crate::chrome::Rect;
use crate::linefmt::LineFormat;
use crate::theme::Colors;
use crate::view::{ROW_H, SheetView};

pub const MENU_W: f32 = 290.;
pub const MENU_H: f32 = 272.;
/// The `⋯` button: its center is this far from the pane's right edge, and it has this radius.
pub const MORE_FROM_RIGHT: f32 = 36.;
pub const MORE_RADIUS: f32 = 12.;
/// The most decimal places the slider offers.
pub const MAX_PLACES: u32 = 10;

/// The menu's controls, relative to its top left.
pub fn items() -> Vec<(String, Rect)> {
    let r = |x: f32, y: f32, w: f32, h: f32| Rect { x, y, w, h };
    let mut v = vec![("copy".to_string(), r(8., 8., MENU_W - 16., 30.))];
    for k in 0..=MAX_PLACES {
        v.push((format!("dp-{k}"), r(18. + 20. * k as f32, 54., 20., 28.)));
    }
    v.push(("group".to_string(), r(18., 94., 40., 28.)));
    v.push(("notation".to_string(), r(66., 94., 40., 28.)));
    v.push(("money".to_string(), r(114., 94., 40., 28.)));
    v.push(("reset".to_string(), r(206., 94., 70., 28.)));
    v.push(("repeating".to_string(), r(8., 164., MENU_W - 16., 30.)));
    v.push(("subtotal".to_string(), r(8., 194., MENU_W - 16., 30.)));
    v.push(("delete".to_string(), r(8., 234., MENU_W - 16., 30.)));
    v
}

/// Where the menu's top left goes (window coordinates) for a row at `row_y`, in a window whose pane ends at
/// `pane_right` and `window_h` high: under the row, or above it when there is no room below.
pub fn origin(pane_right: f32, row_y: f32, window_h: f32) -> (f32, f32) {
    let below = row_y + ROW_H - 4.;
    let y = if below + MENU_H > window_h {
        (row_y - MENU_H + 4.).max(4.)
    } else {
        below
    };
    (pane_right - MENU_W - 10., y)
}

/// What a control does to a line's format. `None` for controls that are not a format change.
pub fn change(id: &str, f: LineFormat) -> Option<LineFormat> {
    let flip = |v: Option<bool>| Some(!v.unwrap_or(true));
    match id {
        "group" => Some(LineFormat {
            group: flip(f.group),
            ..f
        }),
        "notation" => Some(LineFormat {
            notation: flip(f.notation),
            ..f
        }),
        "money" => Some(LineFormat {
            money: flip(f.money),
            ..f
        }),
        // Repeating is off unless chosen, and it replaces a fixed number of places (and the other way round).
        "repeating" => Some(LineFormat {
            repeating: Some(!f.repeating.unwrap_or(false)),
            places: None,
            ..f
        }),
        "reset" => Some(LineFormat::default()),
        _ => id
            .strip_prefix("dp-")
            .and_then(|k| k.parse::<u32>().ok())
            .filter(|k| *k <= MAX_PLACES)
            .map(|k| LineFormat {
                places: Some(k),
                repeating: None,
                ..f
            }),
    }
}

impl SheetView {
    /// The line whose answer the menu is open for.
    pub fn open_answer_menu(&mut self, line: usize) {
        self.answer_menu = Some(line);
        self.panel = None;
    }

    fn menu_do(&mut self, id: &str, line: usize, cx: &mut Context<Self>) {
        if let Some(next) = change(id, self.ed.format(line)) {
            self.set_line_format(line, next, cx);
            return;
        }
        match id {
            "copy" => {
                if let Some(text) = self.answer_for(line).and_then(|a| a.text.clone()) {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }
            "subtotal" => {
                if self.ed.line(line).trim().is_empty() {
                    let end = crate::editor_model::Pos::new(line, self.ed.line(line).len());
                    self.ed.replace(
                        crate::editor_model::Pos::new(line, 0),
                        end,
                        "subtotal",
                        crate::editor_model::Kind::Other,
                    );
                } else {
                    self.ed.insert_line_after(line, "subtotal");
                }
                self.after_edit(cx);
            }
            "delete" => {
                self.ed.delete_line(line);
                self.after_edit(cx);
            }
            _ => {}
        }
        self.answer_menu = None;
        cx.notify();
    }

    pub fn answer_menu_view(&self, c: Colors, cx: &mut Context<Self>) -> Option<AnyElement> {
        let line = self.answer_menu?;
        let l = self.layout.as_ref()?;
        let row_y = f32::from(l.top) + (self.wrap.top_row(line)) as f32 * ROW_H;
        let (x, y) = origin(
            f32::from(l.bounds.right()),
            row_y,
            f32::from(l.bounds.top() + l.bounds.size.height),
        );
        let f = self.ed.format(line);
        let places = f.places.unwrap_or(MAX_PLACES);
        let on = |v: Option<bool>| v.unwrap_or(true);
        let rect_of = |id: &str| {
            items()
                .into_iter()
                .find(|(i, _)| i == id)
                .map(|(_, r)| r)
                .unwrap_or(Rect {
                    x: 0.,
                    y: 0.,
                    w: 0.,
                    h: 0.,
                })
        };
        let at = |r: Rect| {
            div()
                .absolute()
                .left(px(r.x))
                .top(px(r.y))
                .w(px(r.w))
                .h(px(r.h))
        };
        let mut menu = div()
            .occlude()
            .absolute()
            .left(px(x))
            .top(px(y))
            .w(px(MENU_W))
            .h(px(MENU_H))
            .bg(c.raised)
            .border_1()
            .border_color(c.line)
            .rounded(px(12.))
            .text_size(px(14.))
            .text_color(c.text);
        let action =
            |id: &'static str, label: &'static str, danger: bool, cx: &mut Context<Self>| {
                at(rect_of(id))
                    .id(id)
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .rounded(px(6.))
                    .hover(|d| d.bg(c.selection))
                    .when(danger, |d| d.text_color(c.warn))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window: &mut Window, cx| {
                            window.focus(this.focus_handle_ref());
                            if let Some(line) = this.answer_menu {
                                this.menu_do(id, line, cx);
                            }
                        }),
                    )
                    .child(label)
            };
        menu = menu.child(action("copy", "Copy Answer", false, cx));
        // A divider under the copy row, one above the repeating row and one above delete.
        for y in [42., 158., 228.] {
            menu = menu.child(
                div()
                    .absolute()
                    .left(px(8.))
                    .top(px(y))
                    .w(px(MENU_W - 16.))
                    .h(px(1.))
                    .bg(c.line),
            );
        }
        // The decimal places: eleven steps, the chosen one and those before it filled.
        for k in 0..=MAX_PLACES {
            let id: &'static str = [
                "dp-0", "dp-1", "dp-2", "dp-3", "dp-4", "dp-5", "dp-6", "dp-7", "dp-8", "dp-9",
                "dp-10",
            ][k as usize];
            let r = rect_of(id);
            menu = menu.child(
                at(r)
                    .id(id)
                    .flex()
                    .items_center()
                    .justify_center()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window: &mut Window, cx| {
                            window.focus(this.focus_handle_ref());
                            if let Some(line) = this.answer_menu {
                                this.menu_do(id, line, cx);
                            }
                        }),
                    )
                    .child(
                        div()
                            .w(px(if k == places { 14. } else { 6. }))
                            .h(px(if k == places { 14. } else { 6. }))
                            .rounded(px(7.))
                            .bg(if k <= places { c.caret } else { c.gutter }),
                    ),
            );
        }
        menu = menu.child(
            div()
                .absolute()
                .left(px(244.))
                .top(px(58.))
                .text_size(px(13.))
                .text_color(c.muted)
                .child(SharedString::from(format!("{places} dp"))),
        );
        for (id, label, active) in [
            ("group", ",", on(f.group)),
            ("notation", "M", on(f.notation)),
            ("money", "$", on(f.money)),
        ] {
            let id: &'static str = id;
            menu = menu.child(
                at(rect_of(id))
                    .id(id)
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .font_weight(FontWeight::MEDIUM)
                    .bg(if active { c.caret } else { c.line })
                    .text_color(if active { c.background } else { c.muted })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window: &mut Window, cx| {
                            window.focus(this.focus_handle_ref());
                            if let Some(line) = this.answer_menu {
                                this.menu_do(id, line, cx);
                            }
                        }),
                    )
                    .child(label),
            );
        }
        menu = menu
            .child(
                at(rect_of("reset"))
                    .id("reset")
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .bg(c.line)
                    .hover(|d| d.bg(c.selection))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, window: &mut Window, cx| {
                            window.focus(this.focus_handle_ref());
                            if let Some(line) = this.answer_menu {
                                this.menu_do("reset", line, cx);
                            }
                        }),
                    )
                    .child("Reset"),
            )
            .child(
                div()
                    .absolute()
                    .left(px(18.))
                    .top(px(132.))
                    .text_size(px(12.))
                    .text_color(c.muted)
                    .child(SharedString::from(format!(
                        "Display up to {places} decimal places"
                    ))),
            )
            .child(action(
                "repeating",
                if f.repeating == Some(true) {
                    "✓ Show as Repeating"
                } else {
                    "Show as Repeating"
                },
                false,
                cx,
            ))
            .child(action("subtotal", "Make Subtotal Line", false, cx))
            .child(action("delete", "Delete Line", true, cx));
        Some(menu.into_any_element())
    }
}
