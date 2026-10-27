//! The Settings window: panes down the left, a pane's controls on the right. Every control writes its choice to the
//! settings file at once and applies it (`prefs::set`). Each control has a fixed place (`item_rects`) so the
//! acceptance checks can aim at it with a real pointer.

use gpui::{
    AnyElement, App, Context, FontWeight, MouseButton, MouseDownEvent, SharedString, Window, div,
    prelude::*, px,
};

use crate::chrome::Rect;
use crate::theme::Colors;
use crate::view::SheetView;

pub const WINDOW_W: f32 = 780.;
pub const WINDOW_H: f32 = 660.;
const TAB_X: f32 = 12.;
const TAB_W: f32 = 150.;
const ROW_TOP: f32 = 64.;
const ROW_H: f32 = 52.;
const CONTROL_X: f32 = 396.;

/// What a control sets.
pub enum Control {
    /// On or off.
    Toggle { default: bool },
    /// One of a few values, `(value, label)`.
    Choice {
        options: &'static [(&'static str, &'static str)],
        default: &'static str,
    },
    /// A whole number in a range.
    Stepper {
        min: i64,
        max: i64,
        default: i64,
        unit: &'static str,
        step: i64,
    },
    /// Free text, taken with Enter.
    Text { placeholder: &'static str },
    /// A color from a few, or the default.
    Swatches { palette: &'static [u32] },
    /// A button that does something and sets nothing; its id is the item's key.
    Button { label: &'static str },
}

pub struct Item {
    /// The key written in the settings file.
    pub key: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub control: Control,
}

pub struct Pane {
    pub id: &'static str,
    pub title: &'static str,
    /// A paragraph under the controls, or empty.
    pub note: &'static str,
    pub items: &'static [Item],
}

/// Colors for text.
const TEXT_COLORS: &[u32] = &[
    0xff7a90, 0xffb86c, 0xf1fa8c, 0x62c25a, 0x7dcfff, 0x7aa2f7, 0xbb9af7, 0xf0b4f0, 0xe6e6e6,
];

/// Colors for the answer column.
const PANEL_COLORS: &[u32] = &[
    0x151515, 0x262626, 0x20283a, 0x2d2238, 0x1f3328, 0x3a2a22, 0x101820,
];

pub const PANES: &[Pane] = &[
    Pane {
        id: "general",
        title: "General",
        note: "",
        items: &[
            Item {
                key: "sheet_order",
                label: "Order of sheets",
                hint: "How the sidebar lists them",
                control: Control::Choice {
                    options: &[
                        ("modified", "Changed"),
                        ("created", "Created"),
                        ("title", "Title"),
                    ],
                    default: "modified",
                },
            },
            Item {
                key: "on_launch",
                label: "When the app opens",
                hint: "Pick up the last sheet or start a new one",
                control: Control::Choice {
                    options: &[("last", "Last sheet"), ("new", "New sheet")],
                    default: "last",
                },
            },
        ],
    },
    Pane {
        id: "editing",
        title: "Editing",
        note: "",
        items: &[
            Item {
                key: "group_as_you_type",
                label: "Thousands separators as you type",
                hint: "10000 becomes 10,000 while you type it",
                control: Control::Toggle { default: false },
            },
            Item {
                key: "paste_convert",
                label: "Convert foreign numbers when pasting",
                hint: "1.234,5 pasted in becomes 1,234.5",
                control: Control::Toggle { default: false },
            },
            Item {
                key: "rename_variables",
                label: "When a variable is renamed",
                hint: "Update the lines that use it",
                control: Control::Choice {
                    options: &[("ask", "Ask"), ("always", "Always"), ("never", "Never")],
                    default: "ask",
                },
            },
            Item {
                key: "autocomplete_key",
                label: "Key that finishes a variable name",
                hint: "While a name is half typed",
                control: Control::Choice {
                    options: &[("escape", "Escape"), ("tab", "Tab")],
                    default: "escape",
                },
            },
        ],
    },
    Pane {
        id: "numbers",
        title: "Numbers",
        note: "",
        items: &[
            Item {
                key: "number_format",
                label: "Number format",
                hint: "How numbers are typed and shown",
                control: Control::Choice {
                    options: &[
                        ("system", "System"),
                        ("north-america", "1,234.56"),
                        ("western-europe", "1.234,56"),
                        ("eastern-europe", "1 234,56"),
                    ],
                    default: "system",
                },
            },
            Item {
                key: "decimal_places",
                label: "Decimal places",
                hint: "At most this many after the point",
                control: Control::Stepper {
                    min: 0,
                    max: 10,
                    default: 10,
                    unit: "places",
                    step: 1,
                },
            },
            Item {
                key: "notation",
                label: "Use notation for large numbers",
                hint: "300000 reads 300k; 16777216 reads 16.777216M",
                control: Control::Toggle { default: true },
            },
            Item {
                key: "group_thousands",
                label: "Thousands separators in answers",
                hint: "1,000 and not 1000",
                control: Control::Toggle { default: true },
            },
        ],
    },
    Pane {
        id: "calculator",
        title: "Calculator",
        note: "",
        items: &[
            Item {
                key: "currency_codes",
                label: "Currency codes, not symbols",
                hint: "5.00 EUR and not €5.00",
                control: Control::Toggle { default: false },
            },
            Item {
                key: "money_trim_zeros",
                label: "Hide trailing zeros on money",
                hint: "$5 and not $5.00",
                control: Control::Toggle { default: false },
            },
            Item {
                key: "dollar",
                label: "What $ means",
                hint: "The dollar a bare $ stands for",
                control: Control::Choice {
                    options: &[
                        ("USD", "USD"),
                        ("AUD", "AUD"),
                        ("CAD", "CAD"),
                        ("NZD", "NZD"),
                    ],
                    default: "USD",
                },
            },
            Item {
                key: "custom_zones",
                label: "Your own names for places",
                hint: "Name = Zone; the zone as in Asia/Tokyo",
                control: Control::Text {
                    placeholder: "HQ = Europe/Berlin; Lab = Asia/Tokyo",
                },
            },
            Item {
                key: "tax",
                label: "Sales tax",
                hint: "A name and a rate, like VAT 15%",
                control: Control::Text {
                    placeholder: "VAT 15%",
                },
            },
        ],
    },
    Pane {
        id: "variables",
        title: "Variables",
        note: "",
        items: &[
            Item {
                key: "globals",
                label: "Global variables",
                hint: "In every sheet; separate with ;",
                control: Control::Text {
                    placeholder: "rent = $900; tip = 18%",
                },
            },
            Item {
                key: "dividers_reset",
                label: "Divider lines reset variables",
                hint: "A --- line ends the variables above it",
                control: Control::Toggle { default: true },
            },
        ],
    },
    Pane {
        id: "styling",
        title: "Styling",
        note: "",
        items: &[
            Item {
                key: "color_number",
                label: "Number color",
                hint: "In the sheet's text",
                control: Control::Swatches {
                    palette: TEXT_COLORS,
                },
            },
            Item {
                key: "color_unit",
                label: "Unit color",
                hint: "kg, mm, hours",
                control: Control::Swatches {
                    palette: TEXT_COLORS,
                },
            },
            Item {
                key: "color_currency",
                label: "Currency color",
                hint: "$, €, £",
                control: Control::Swatches {
                    palette: TEXT_COLORS,
                },
            },
            Item {
                key: "color_variable",
                label: "Variable color",
                hint: "Names you defined",
                control: Control::Swatches {
                    palette: TEXT_COLORS,
                },
            },
            Item {
                key: "answer_background",
                label: "Answer column color",
                hint: "Behind the answers",
                control: Control::Swatches {
                    palette: PANEL_COLORS,
                },
            },
            Item {
                key: "answer_align",
                label: "Answers line up on the",
                hint: "Left or right of their column",
                control: Control::Choice {
                    options: &[("left", "Left"), ("right", "Right")],
                    default: "left",
                },
            },
            Item {
                key: "font",
                label: "Font",
                hint: "For the sheet's text and answers",
                control: Control::Choice {
                    options: &[
                        ("Noto Sans", "Noto"),
                        ("JetBrainsMono Nerd Font", "JetBrains"),
                        ("Liberation Sans", "Liberation"),
                        ("Liberation Serif", "Serif"),
                    ],
                    default: "Noto Sans",
                },
            },
            Item {
                key: "font_size",
                label: "Text size",
                hint: "14 to 26 points",
                control: Control::Stepper {
                    min: 14,
                    max: 26,
                    default: 21,
                    unit: "pt",
                    step: 1,
                },
            },
            Item {
                key: "heading_size",
                label: "Heading size",
                hint: "Against ordinary text",
                control: Control::Stepper {
                    min: 100,
                    max: 160,
                    default: 100,
                    unit: "%",
                    step: 10,
                },
            },
            Item {
                key: "mono_digits",
                label: "Digits all the same width",
                hint: "Columns of numbers line up",
                control: Control::Toggle { default: false },
            },
        ],
    },
    Pane {
        id: "dates",
        title: "Dates & Times",
        note: "",
        items: &[
            Item {
                key: "work_hours",
                label: "Hours in a working day",
                hint: "For `work hours` and workdays",
                control: Control::Stepper {
                    min: 1,
                    max: 24,
                    default: 8,
                    unit: "hours",
                    step: 1,
                },
            },
            Item {
                key: "holidays",
                label: "Public holidays",
                hint: "Days off: 2026-12-25 once, 12-25 every year; separate with ;",
                control: Control::Text {
                    placeholder: "12-25; 01-01; 2026-04-03",
                },
            },
            Item {
                key: "day_first",
                label: "Day first in 12/02/1988",
                hint: "12 February and not December 2",
                control: Control::Toggle { default: false },
            },
        ],
    },
    Pane {
        id: "updates",
        title: "Updates",
        note: "Babbage works offline and goes online only for this. A check sends one request to the release source (api.github.com unless you change it) asking for its list of releases. The request carries your IP address, as any request does, and the name of the update library, praeco; nothing from your sheets or settings is sent. By default it asks at most once a week, when Babbage starts, and keeps the answer so the Update pill stays up until you install or skip that version. Choose Off and Babbage makes no requests at all. BABBAGE_UPDATES=off in the environment also turns it off, whatever is chosen here. Write the source as github owner/repo, or forgejo https://host owner/repo.",
        items: &[
            Item {
                key: "update_check",
                label: "Look for a new version",
                hint: "At most this often",
                control: Control::Choice {
                    options: &[
                        ("off", "Off"),
                        ("monthly", "Monthly"),
                        ("weekly", "Weekly"),
                        ("daily", "Daily"),
                    ],
                    default: "weekly",
                },
            },
            Item {
                key: "update_source",
                label: "Where to look",
                hint: "See the note below",
                control: Control::Text {
                    placeholder: "github jrimmer/babbage",
                },
            },
            Item {
                key: "update_now",
                label: "Check now",
                hint: "Asks right away",
                control: Control::Button { label: "Check now" },
            },
        ],
    },
];

/// A control's places, relative to the window's top left: `(id, rect)`. A toggle is `key`; a choice has one
/// `key=value` per option; a stepper has `key-` and `key+`; a text field is `key`. Tabs are `tab-<pane id>`.
pub fn item_rects(pane: usize) -> Vec<(String, Rect)> {
    let r = |x: f32, y: f32, w: f32, h: f32| Rect { x, y, w, h };
    let mut out = Vec::new();
    for (i, p) in PANES.iter().enumerate() {
        out.push((
            format!("tab-{}", p.id),
            r(TAB_X, ROW_TOP + 38. * i as f32, TAB_W, 32.),
        ));
    }
    let Some(pane) = PANES.get(pane) else {
        return out;
    };
    for (row, item) in pane.items.iter().enumerate() {
        let y = ROW_TOP + ROW_H * row as f32 + 10.;
        match &item.control {
            Control::Toggle { .. } => {
                out.push((item.key.to_string(), r(WINDOW_W - 100., y, 52., 28.)))
            }
            Control::Choice { options, .. } => {
                let each = 88.;
                for (k, (value, _)) in options.iter().enumerate() {
                    out.push((
                        format!("{}={value}", item.key),
                        r(CONTROL_X + each * k as f32, y, each - 4., 28.),
                    ));
                }
            }
            Control::Stepper { .. } => {
                out.push((format!("{}-", item.key), r(WINDOW_W - 196., y, 34., 28.)));
                out.push((format!("{}+", item.key), r(WINDOW_W - 64., y, 34., 28.)));
            }
            Control::Text { .. } => out.push((
                item.key.to_string(),
                r(CONTROL_X, y, WINDOW_W - CONTROL_X - 30., 28.),
            )),
            Control::Button { .. } => out.push((item.key.to_string(), r(CONTROL_X, y, 140., 28.))),
            Control::Swatches { palette } => {
                for (k, c) in palette.iter().enumerate() {
                    out.push((
                        format!("{}=#{c:06x}", item.key),
                        r(CONTROL_X + 28. * k as f32, y + 2., 24., 24.),
                    ));
                }
                out.push((
                    format!("{}=default", item.key),
                    r(CONTROL_X + 28. * palette.len() as f32 + 4., y, 74., 28.),
                ));
            }
        }
    }
    out
}

/// Where the window's top left is (window coordinates) for a window `w` by `h`.
pub fn origin(w: f32, h: f32) -> (f32, f32) {
    (((w - WINDOW_W) / 2.).max(8.), ((h - WINDOW_H) / 2.).max(8.))
}

fn on_off(value: &str) -> Option<bool> {
    match value {
        "on" | "yes" | "true" => Some(true),
        "off" | "no" | "false" => Some(false),
        _ => None,
    }
}

/// A click on control `id` of the Settings window.
fn click_for(
    id: String,
    cx: &mut Context<SheetView>,
) -> impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static {
    cx.listener(move |this, _, window, cx| {
        window.focus(this.focus_handle_ref());
        this.settings_click(&id, cx);
    })
}

impl Item {
    /// The value now, as written in the settings file, or the default when it holds none.
    pub fn current(&self) -> String {
        let raw = crate::prefs::raw(self.key);
        match &self.control {
            Control::Toggle { default } => {
                let v = raw.as_deref().and_then(on_off).unwrap_or(*default);
                if v { "on" } else { "off" }.to_string()
            }
            Control::Choice { options, default } => raw
                .filter(|v| options.iter().any(|(o, _)| o.eq_ignore_ascii_case(v)))
                .map(|v| {
                    options
                        .iter()
                        .find(|(o, _)| o.eq_ignore_ascii_case(&v))
                        .map_or(v.clone(), |(o, _)| (*o).to_string())
                })
                .unwrap_or_else(|| (*default).to_string()),
            Control::Stepper {
                min, max, default, ..
            } => raw
                .and_then(|v| v.parse::<i64>().ok())
                .filter(|v| (*min..=*max).contains(v))
                .unwrap_or(*default)
                .to_string(),
            Control::Text { .. } => raw.unwrap_or_default(),
            Control::Swatches { .. } => raw
                .filter(|v| v.starts_with('#') && v.len() == 7)
                .unwrap_or_else(|| "default".to_string()),
            Control::Button { .. } => String::new(),
        }
    }
}

impl SheetView {
    pub fn settings_origin(&self) -> (f32, f32) {
        let (w, h) = self.layout.as_ref().map_or((1280., 800.), |l| {
            (
                f32::from(l.bounds.right()),
                f32::from(l.bounds.top() + l.bounds.size.height),
            )
        });
        origin(w, h)
    }

    /// A click on a control of the Settings window (by `item_rects` id).
    pub fn settings_click(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(tab) = id.strip_prefix("tab-") {
            if let Some(i) = PANES.iter().position(|p| p.id == tab) {
                self.settings_pane = i;
                self.settings_edit = None;
            }
            cx.notify();
            return;
        }
        let Some(pane) = PANES.get(self.settings_pane) else {
            return;
        };
        for item in pane.items {
            match &item.control {
                Control::Toggle { .. } if id == item.key => {
                    let next = if item.current() == "on" { "off" } else { "on" };
                    self.change_setting(item.key, next, cx);
                }
                Control::Choice { .. } if id.starts_with(&format!("{}=", item.key)) => {
                    let value = &id[item.key.len() + 1..];
                    self.change_setting(item.key, value, cx);
                }
                Control::Stepper { min, max, step, .. }
                    if id == format!("{}-", item.key) || id == format!("{}+", item.key) =>
                {
                    let now: i64 = item.current().parse().unwrap_or(*min);
                    let next = if id.ends_with('-') {
                        now - step
                    } else {
                        now + step
                    };
                    self.change_setting(item.key, &next.clamp(*min, *max).to_string(), cx);
                }
                Control::Swatches { .. } if id.starts_with(&format!("{}=", item.key)) => {
                    let value = &id[item.key.len() + 1..];
                    self.change_setting(item.key, value, cx);
                }
                Control::Text { .. } if id == item.key => {
                    self.settings_edit = Some((item.key.to_string(), item.current()));
                    cx.notify();
                }
                Control::Button { .. } if id == item.key && item.key == "update_now" => {
                    self.run_check(true, cx);
                }
                _ => {}
            }
        }
    }

    /// Keys while a text control is being edited. False when none is.
    pub fn settings_edit_key(
        &mut self,
        key: &str,
        ch: Option<&str>,
        ctrl: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some((name, text)) = self.settings_edit.as_mut() else {
            return false;
        };
        match key {
            "enter" => {
                let (name, text) = (name.clone(), text.trim().to_string());
                self.settings_edit = None;
                self.change_setting(&name, &text, cx);
            }
            "escape" => {
                self.settings_edit = None;
            }
            "backspace" => {
                text.pop();
            }
            _ if !ctrl => {
                if let Some(c) = ch
                    && !c.chars().any(char::is_control)
                {
                    text.push_str(c);
                }
            }
            _ => return false,
        }
        cx.notify();
        true
    }

    pub fn settings_window(&self, c: Colors, cx: &mut Context<Self>) -> AnyElement {
        let (ox, oy) = self.settings_origin();
        let rects = item_rects(self.settings_pane);
        let rect_of = |id: &str| {
            rects
                .iter()
                .find(|(i, _)| i == id)
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
        let mut win = div()
            .occlude()
            .absolute()
            .left(px(ox))
            .top(px(oy))
            .w(px(WINDOW_W))
            .h(px(WINDOW_H))
            .bg(c.raised)
            .border_1()
            .border_color(c.line)
            .rounded(px(14.))
            .text_size(px(14.))
            .text_color(c.text)
            .child(
                div()
                    .absolute()
                    .left(px(TAB_X + 6.))
                    .top(px(20.))
                    .text_size(px(16.))
                    .font_weight(FontWeight::MEDIUM)
                    .child("Settings"),
            )
            .child(
                div()
                    .absolute()
                    .left(px(TAB_X + TAB_W + 12.))
                    .top(px(10.))
                    .w(px(1.))
                    .h(px(WINDOW_H - 20.))
                    .bg(c.line),
            );
        for (i, p) in PANES.iter().enumerate() {
            let id = format!("tab-{}", p.id);
            let on = i == self.settings_pane;
            win = win.child(
                at(rect_of(&id))
                    .id(SharedString::from(id.clone()))
                    .px(px(10.))
                    .flex()
                    .items_center()
                    .rounded(px(8.))
                    .when(on, |d| d.bg(c.selection).font_weight(FontWeight::MEDIUM))
                    .hover(|d| d.bg(c.selection))
                    .on_mouse_down(MouseButton::Left, click_for(id, cx))
                    .child(p.title),
            );
        }
        let Some(pane) = PANES.get(self.settings_pane) else {
            return win.into_any_element();
        };
        win = win.child(
            div()
                .absolute()
                .left(px(TAB_X + TAB_W + 30.))
                .top(px(20.))
                .text_size(px(16.))
                .font_weight(FontWeight::MEDIUM)
                .child(pane.title),
        );
        for (row, item) in pane.items.iter().enumerate() {
            let y = ROW_TOP + ROW_H * row as f32;
            win = win.child(
                div()
                    .absolute()
                    .left(px(TAB_X + TAB_W + 30.))
                    .top(px(y + 6.))
                    .w(px(match item.control {
                        // A switch or a stepper sits at the far right, so its label has room to run.
                        Control::Toggle { .. } | Control::Stepper { .. } => WINDOW_W - 330.,
                        _ => CONTROL_X - TAB_X - TAB_W - 40.,
                    }))
                    .child(div().child(item.label))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(c.muted)
                            .child(item.hint),
                    ),
            );
            let now = item.current();
            match &item.control {
                Control::Toggle { .. } => {
                    let on = now == "on";
                    win = win.child(
                        at(rect_of(item.key))
                            .id(item.key)
                            .rounded(px(14.))
                            .bg(if on { c.caret } else { c.line })
                            .on_mouse_down(MouseButton::Left, click_for(item.key.to_string(), cx))
                            .child(
                                div()
                                    .absolute()
                                    .top(px(3.))
                                    .left(px(if on { 27. } else { 3. }))
                                    .w(px(22.))
                                    .h(px(22.))
                                    .rounded(px(11.))
                                    .bg(c.text),
                            ),
                    );
                }
                Control::Choice { options, .. } => {
                    for (value, label) in *options {
                        let id = format!("{}={value}", item.key);
                        let on = now == *value;
                        win = win.child(
                            at(rect_of(&id))
                                .id(SharedString::from(id.clone()))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(7.))
                                .text_size(px(13.))
                                .bg(if on { c.caret } else { c.line })
                                .text_color(if on { c.background } else { c.text })
                                .hover(|d| d.bg(c.selection))
                                .on_mouse_down(MouseButton::Left, click_for(id, cx))
                                .child(*label),
                        );
                    }
                }
                Control::Stepper { unit, .. } => {
                    for (sign, label) in [("-", "−"), ("+", "+")] {
                        let id = format!("{}{sign}", item.key);
                        win = win.child(
                            at(rect_of(&id))
                                .id(SharedString::from(id.clone()))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(7.))
                                .bg(c.line)
                                .hover(|d| d.bg(c.selection))
                                .on_mouse_down(MouseButton::Left, click_for(id, cx))
                                .child(label),
                        );
                    }
                    win = win.child(
                        div()
                            .absolute()
                            .left(px(WINDOW_W - 158.))
                            .top(px(y + 14.))
                            .w(px(90.))
                            .flex()
                            .justify_center()
                            .child(SharedString::from(format!("{now} {unit}"))),
                    );
                }
                Control::Swatches { palette } => {
                    for c_ in *palette {
                        let id = format!("{}=#{c_:06x}", item.key);
                        let on = now.eq_ignore_ascii_case(&format!("#{c_:06x}"));
                        win = win.child(
                            at(rect_of(&id))
                                .id(SharedString::from(id.clone()))
                                .rounded(px(6.))
                                .bg(gpui::Rgba {
                                    r: ((c_ >> 16) & 0xff) as f32 / 255.,
                                    g: ((c_ >> 8) & 0xff) as f32 / 255.,
                                    b: (c_ & 0xff) as f32 / 255.,
                                    a: 1.,
                                })
                                .border_2()
                                .border_color(if on { c.text } else { c.raised })
                                .on_mouse_down(MouseButton::Left, click_for(id, cx)),
                        );
                    }
                    let id = format!("{}=default", item.key);
                    let on = now == "default";
                    win = win.child(
                        at(rect_of(&id))
                            .id(SharedString::from(id.clone()))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(7.))
                            .text_size(px(13.))
                            .bg(if on { c.caret } else { c.line })
                            .text_color(if on { c.background } else { c.text })
                            .hover(|d| d.bg(c.selection))
                            .on_mouse_down(MouseButton::Left, click_for(id, cx))
                            .child("Default"),
                    );
                }
                Control::Button { label } => {
                    win = win.child(
                        at(rect_of(item.key))
                            .id(item.key)
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(7.))
                            .text_size(px(13.))
                            .bg(c.line)
                            .hover(|d| d.bg(c.selection))
                            .on_mouse_down(MouseButton::Left, click_for(item.key.to_string(), cx))
                            .child(*label),
                    );
                }
                Control::Text { placeholder } => {
                    let editing = self.settings_edit.as_ref().filter(|(k, _)| k == item.key);
                    let shown = match editing {
                        Some((_, t)) => format!("{t}|"),
                        None if now.is_empty() => (*placeholder).to_string(),
                        None => now.clone(),
                    };
                    win = win.child(
                        at(rect_of(item.key))
                            .id(item.key)
                            .px(px(10.))
                            .flex()
                            .items_center()
                            .rounded(px(7.))
                            .bg(c.line)
                            .border_1()
                            .border_color(if editing.is_some() { c.caret } else { c.line })
                            .text_color(if editing.is_none() && now.is_empty() {
                                c.muted
                            } else {
                                c.text
                            })
                            .on_mouse_down(MouseButton::Left, click_for(item.key.to_string(), cx))
                            .child(SharedString::from(shown)),
                    );
                }
            }
        }
        if !pane.note.is_empty() {
            win = win.child(
                div()
                    .absolute()
                    .left(px(TAB_X + TAB_W + 30.))
                    .top(px(ROW_TOP + ROW_H * pane.items.len() as f32 + 8.))
                    .w(px(WINDOW_W - (TAB_X + TAB_W + 30.) - 30.))
                    .text_size(px(12.))
                    .text_color(c.muted)
                    .child(pane.note),
            );
        }
        win.into_any_element()
    }
}
