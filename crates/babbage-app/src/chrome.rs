//! The strip above the sheet (settings, help, sidebar toggle) and the two panels it opens.
//!
//! Every clickable thing has a fixed place (`toolbar_buttons`, `settings_items`) so the acceptance checks can aim
//! at it with a real pointer.

use gpui::{
    AnyElement, Context, FontWeight, MouseButton, SharedString, Window, div, prelude::*, px,
};

use crate::theme::Colors;
use crate::view::SheetView;

pub const TOOLBAR_H: f32 = 48.;
pub const TOOL_W: f32 = 34.;
pub const TOOL_H: f32 = 28.;
const TOOL_GAP: f32 = 6.;
const TOOL_PAD_RIGHT: f32 = 14.;

pub const PANEL_W: f32 = 340.;
const PANEL_RIGHT: f32 = 14.;
const PANEL_TOP: f32 = 4.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    Settings,
    Help,
}

impl Panel {
    pub fn name(self) -> &'static str {
        match self {
            Panel::Settings => "settings",
            Panel::Help => "help",
        }
    }
}

/// A rectangle in the window's pane, in points, relative to the top left of what it sits in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// The toolbar's buttons from left to right, as offsets from the right edge of the pane (negative) and the top of
/// the toolbar.
pub fn toolbar_buttons() -> [(&'static str, Rect); 3] {
    let y = (TOOLBAR_H - TOOL_H) / 2.;
    let at = |from_right: usize| {
        let x = -(TOOL_PAD_RIGHT + TOOL_W * (from_right + 1) as f32 + TOOL_GAP * from_right as f32);
        Rect {
            x,
            y,
            w: TOOL_W,
            h: TOOL_H,
        }
    };
    [("settings", at(2)), ("help", at(1)), ("sidebar", at(0))]
}

/// The settings panel's controls, relative to the panel's top left.
pub fn settings_items() -> Vec<(&'static str, Rect)> {
    let row = |y: f32| Rect {
        x: 14.,
        y,
        w: PANEL_W - 28.,
        h: 30.,
    };
    let mut items = vec![];
    for (i, id) in ["north-america", "western-europe", "eastern-europe"]
        .into_iter()
        .enumerate()
    {
        items.push((id, row(70. + 32. * i as f32)));
    }
    items.push((
        "hours-down",
        Rect {
            x: 14.,
            y: 204.,
            w: 34.,
            h: 30.,
        },
    ));
    items.push((
        "hours-up",
        Rect {
            x: 170.,
            y: 204.,
            w: 34.,
            h: 30.,
        },
    ));
    items
}

pub const SETTINGS_H: f32 = 300.;

/// Where the panel's top left is, from the right edge of the pane and the top of the toolbar's bottom edge.
pub fn panel_origin() -> (f32, f32) {
    (-(PANEL_RIGHT + PANEL_W), PANEL_TOP)
}

/// Keys, as `(keys, what they do)`.
pub const KEYS: &[(&str, &str)] = &[
    ("Ctrl+N", "New sheet"),
    ("Ctrl+Tab / Ctrl+Shift+Tab", "Next / previous sheet"),
    ("Ctrl+B", "Show or hide the sidebar"),
    ("F2", "Rename the open sheet"),
    ("Ctrl+Shift+Delete", "Move the open sheet to the trash"),
    ("Ctrl+Z / Ctrl+Shift+Z", "Undo / redo"),
    ("Ctrl+A, C, X, V", "Select all, copy, cut, paste"),
    ("Ctrl+/", "Comment the lines in or out"),
    ("Ctrl+Shift+E", "Export the sheet (text, CSV, HTML, PDF)"),
    ("Alt+Right", "Focus the answer of the line"),
    ("Double / triple click", "Select a word / a line"),
    ("Ctrl+Q", "Save and quit"),
];

/// Things to type, as `(what to type, what it means)`.
pub const SYNTAX: &[(&str, &str)] = &[
    ("12 + 12 × 15", "Arithmetic; words work too: 12 plus 3"),
    ("20% of 85, 5% on $40", "Percentages"),
    ("5 km in miles, 3 hours to min", "Units and conversions"),
    ("$50 × 8, 10 EUR in USD", "Money and rates you set"),
    ("rent = $900", "A variable; use it by name"),
    ("line3 / line4", "An earlier line's answer (a chip)"),
    ("now + 3 days, June 5 to July 2", "Dates, times and zones"),
    (
        "# Heading, // comment, Label:",
        "Headings, comments, labels",
    ),
    ("total, subtotal, total of #tag", "Totals"),
    ("5 squared, 2 cubed, sqrt(16)", "Powers and functions"),
];

fn icon_sidebar(c: &Colors) -> AnyElement {
    div()
        .relative()
        .w(px(20.))
        .h(px(15.))
        .rounded(px(3.))
        .border_2()
        .border_color(c.muted)
        .child(
            div()
                .absolute()
                .left(px(5.))
                .top_0()
                .bottom_0()
                .w(px(2.))
                .bg(c.muted),
        )
        .into_any_element()
}

fn icon_help(c: &Colors) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .gap(px(1.))
        .children((0..2).map(|_| {
            div()
                .w(px(9.))
                .h(px(14.))
                .rounded(px(2.))
                .border_2()
                .border_color(c.muted)
        }))
        .into_any_element()
}

fn icon_settings(c: &Colors) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(3.))
        .children((0..2).map(|i| {
            div()
                .relative()
                .w(px(20.))
                .h(px(8.))
                .rounded(px(4.))
                .border_2()
                .border_color(c.muted)
                .child(
                    div()
                        .absolute()
                        .top(px(1.))
                        .left(px(if i == 0 { 2. } else { 9. }))
                        .w(px(4.))
                        .h(px(4.))
                        .rounded(px(2.))
                        .bg(c.muted),
                )
        }))
        .into_any_element()
}

impl SheetView {
    fn tool_button(
        &self,
        id: &'static str,
        active: bool,
        c: Colors,
        icon: AnyElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id(id)
            .w(px(TOOL_W))
            .h(px(TOOL_H))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(6.))
            .when(active, |d| d.bg(c.raised))
            .hover(|d| d.bg(c.raised))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, window: &mut Window, cx| {
                    window.focus(this.focus_handle_ref());
                    match id {
                        "settings" => this.toggle_panel(Panel::Settings),
                        "help" => this.toggle_panel(Panel::Help),
                        _ => this.sidebar_open = !this.sidebar_open,
                    }
                    cx.notify();
                }),
            )
            .child(icon)
            .into_any_element()
    }

    pub fn toggle_panel(&mut self, panel: Panel) {
        self.panel = if self.panel == Some(panel) {
            None
        } else {
            Some(panel)
        };
    }

    /// The strip above the sheet: empty over the text, the three buttons over the answers.
    pub fn toolbar(&self, c: Colors, cx: &mut Context<Self>) -> AnyElement {
        let answer_w = self.layout.as_ref().map_or(300., |l| f32::from(l.answer_w));
        div()
            .h(px(TOOLBAR_H))
            .flex_none()
            .w_full()
            .flex()
            .flex_row()
            .bg(c.background)
            .child(div().flex_1())
            .child(
                div()
                    .w(px(answer_w))
                    .h_full()
                    .bg(c.answers)
                    .border_l_1()
                    .border_color(c.line)
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_end()
                    .gap(px(TOOL_GAP))
                    .pr(px(TOOL_PAD_RIGHT))
                    .child(self.tool_button(
                        "settings",
                        self.panel == Some(Panel::Settings),
                        c,
                        icon_settings(&c),
                        cx,
                    ))
                    .child(self.tool_button(
                        "help",
                        self.panel == Some(Panel::Help),
                        c,
                        icon_help(&c),
                        cx,
                    ))
                    .child(self.tool_button("sidebar", self.sidebar_open, c, icon_sidebar(&c), cx)),
            )
            .into_any_element()
    }

    fn panel_frame(&self, c: Colors, height: Option<f32>) -> gpui::Div {
        div()
            .occlude()
            .absolute()
            .top(px(TOOLBAR_H + PANEL_TOP))
            .right(px(PANEL_RIGHT))
            .w(px(PANEL_W))
            .when_some(height, |d, h| d.h(px(h)))
            .bg(c.raised)
            .border_1()
            .border_color(c.line)
            .rounded(px(12.))
            .text_size(px(13.))
            .text_color(c.text)
    }

    /// The open panel, drawn over the sheet.
    pub fn panel_view(&self, c: Colors, cx: &mut Context<Self>) -> Option<AnyElement> {
        match self.panel? {
            Panel::Help => Some(self.help_panel(c)),
            Panel::Settings => Some(self.settings_panel(c, cx)),
        }
    }

    fn help_panel(&self, c: Colors) -> AnyElement {
        let heading = |t: &'static str| {
            div()
                .mt(px(10.))
                .mb(px(4.))
                .text_size(px(12.))
                .text_color(c.muted)
                .font_weight(FontWeight::MEDIUM)
                .child(t)
        };
        let row = |a: &'static str, b: &'static str| {
            div()
                .flex()
                .flex_col()
                .py(px(3.))
                .child(div().child(a))
                .child(div().text_size(px(12.)).text_color(c.muted).child(b))
        };
        self.panel_frame(c, None)
            .id("help-panel")
            .max_h(px(560.))
            .overflow_y_scroll()
            .p(px(14.))
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight::MEDIUM)
                    .child("Help"),
            )
            .child(heading("Keys"))
            .children(KEYS.iter().map(|(k, d)| row(k, d)))
            .child(heading("Things to type"))
            .children(SYNTAX.iter().map(|(k, d)| row(k, d)))
            .into_any_element()
    }

    fn settings_panel(&self, c: Colors, cx: &mut Context<Self>) -> AnyElement {
        let settings = crate::prefs::current();
        let hours = settings.work_hours_per_day.unwrap_or(8);
        let formats = [
            ("north-america", "1,234.56", "North America"),
            ("western-europe", "1.234,56", "Western Europe"),
            ("eastern-europe", "1 234,56", "Eastern Europe"),
        ];
        let current = match settings.number_format {
            babbage_core::NumberFormat::NorthAmerica => "north-america",
            babbage_core::NumberFormat::WesternEurope => "western-europe",
            babbage_core::NumberFormat::EasternEurope => "eastern-europe",
        };
        let items = settings_items();
        let rect = |id: &str| {
            items
                .iter()
                .find(|(i, _)| *i == id)
                .map(|(_, r)| *r)
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
        let mut panel = self
            .panel_frame(c, Some(SETTINGS_H))
            .child(
                div()
                    .absolute()
                    .left(px(14.))
                    .top(px(14.))
                    .text_size(px(15.))
                    .font_weight(FontWeight::MEDIUM)
                    .child("Settings"),
            )
            .child(
                div()
                    .absolute()
                    .left(px(14.))
                    .top(px(48.))
                    .text_size(px(12.))
                    .text_color(c.muted)
                    .child("Number format"),
            );
        for (id, sample, name) in formats {
            let selected = id == current;
            panel = panel.child(
                at(rect(id))
                    .id(id)
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded(px(6.))
                    .when(selected, |d| d.bg(c.selection))
                    .hover(|d| d.bg(c.selection))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.change_setting("number_format", id, cx);
                        }),
                    )
                    .child(SharedString::from(format!("{sample}  {name}")))
                    .child(if selected { "✓" } else { "" }),
            );
        }
        panel = panel
            .child(
                div()
                    .absolute()
                    .left(px(14.))
                    .top(px(182.))
                    .text_size(px(12.))
                    .text_color(c.muted)
                    .child("Working day"),
            )
            .child(
                at(rect("hours-down"))
                    .id("hours-down")
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .bg(c.line)
                    .hover(|d| d.bg(c.selection))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            let h = crate::prefs::current().work_hours_per_day.unwrap_or(8);
                            this.change_setting(
                                "work_hours",
                                &h.saturating_sub(1).max(1).to_string(),
                                cx,
                            );
                        }),
                    )
                    .child("−"),
            )
            .child(
                div()
                    .absolute()
                    .left(px(56.))
                    .top(px(204.))
                    .w(px(110.))
                    .h(px(30.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(SharedString::from(format!("{hours} hours"))),
            )
            .child(
                at(rect("hours-up"))
                    .id("hours-up")
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(6.))
                    .bg(c.line)
                    .hover(|d| d.bg(c.selection))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            let h = crate::prefs::current().work_hours_per_day.unwrap_or(8);
                            this.change_setting("work_hours", &(h + 1).min(24).to_string(), cx);
                        }),
                    )
                    .child("+"),
            );
        let path = crate::prefs::settings_path().map_or_else(
            || "no settings file".to_owned(),
            |p| p.display().to_string(),
        );
        panel
            .child(
                div()
                    .absolute()
                    .left(px(14.))
                    .top(px(250.))
                    .w(px(PANEL_W - 28.))
                    .text_size(px(11.))
                    .text_color(c.muted)
                    .child(SharedString::from(format!("Saved in {path}"))),
            )
            .into_any_element()
    }

    /// Writes a setting to the file, applies it and works every line out again.
    pub fn change_setting(&mut self, key: &str, value: &str, cx: &mut Context<Self>) {
        match crate::prefs::set(key, value) {
            Ok(()) => {
                self.notice = None;
                self.recompute();
            }
            Err(e) => self.notice = Some(format!("Could not save the setting: {e}")),
        }
        cx.notify();
    }
}
