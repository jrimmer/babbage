//! The window side of updates: the pill beside the toolbar buttons, the "What's new" panel it opens (the releases
//! newer than this one, newest first, and Install, Later and Skip this version), and the work behind them. The
//! check and the install run off the window's thread; their answers come back through the view.

use futures::StreamExt;
use gpui::{
    AnyElement, Context, FontWeight, MouseButton, SharedString, Window, div, prelude::*, px,
};
use serde_json::{Value, json};

use crate::chrome::{
    PANEL_RIGHT, PANEL_TOP, Panel, Rect, TOOL_GAP, TOOL_H, TOOLBAR_H, toolbar_buttons,
};
use crate::theme::Colors;
use crate::updates::{self, Found, Note};
use crate::view::SheetView;

pub const PANEL_W: f32 = 460.;
pub const PANEL_H: f32 = 420.;
pub const PILL_W: f32 = 132.;
const BUTTON_H: f32 = 32.;
const PAD: f32 = 14.;

/// What the updates are doing now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Checking,
    /// The last check could not be made, with why.
    Failed(String),
    /// Downloading and installing, with the latest thing said about it.
    Installing(String),
    /// The new version is on disk; the version.
    Installed(String),
    InstallFailed(String),
}

impl Phase {
    fn name(&self) -> &'static str {
        match self {
            Phase::Idle => "idle",
            Phase::Checking => "checking",
            Phase::Failed(_) => "failed",
            Phase::Installing(_) => "installing",
            Phase::Installed(_) => "installed",
            Phase::InstallFailed(_) => "install_failed",
        }
    }
}

pub struct Updates {
    /// What the last check found, kept between launches.
    pub found: Option<Found>,
    pub phase: Phase,
}

impl Updates {
    pub fn load() -> Updates {
        Updates {
            found: updates::load(),
            phase: Phase::Idle,
        }
    }
}

/// A message from the install running off the window's thread.
enum Install {
    Said(String),
    Finished(Result<(), String>),
}

/// The pill's place, relative to the right edge of the pane (negative) and the top of the toolbar.
pub fn pill_rect() -> Rect {
    let settings = toolbar_buttons()[0].1;
    Rect {
        x: settings.x - TOOL_GAP - PILL_W,
        y: settings.y,
        w: PILL_W,
        h: TOOL_H,
    }
}

/// The panel's top left in window coordinates, given the pane's right edge and the toolbar's top.
pub fn panel_origin(right: f32, toolbar_top: f32) -> (f32, f32) {
    (
        right - PANEL_RIGHT - PANEL_W,
        toolbar_top + TOOLBAR_H + PANEL_TOP,
    )
}

/// The panel's buttons, relative to its top left: `(id, rect)`.
pub fn buttons(phase: &Phase) -> Vec<(&'static str, Rect)> {
    let y = PANEL_H - PAD - BUTTON_H;
    let row = |items: &[(&'static str, f32)]| {
        let mut x = PAD;
        items
            .iter()
            .map(|(id, w)| {
                let r = Rect {
                    x,
                    y,
                    w: *w,
                    h: BUTTON_H,
                };
                x += w + 8.;
                (*id, r)
            })
            .collect()
    };
    match phase {
        Phase::Installing(_) => Vec::new(),
        Phase::Installed(_) => row(&[("update-restart", 170.), ("update-later", 90.)]),
        _ => row(&[
            ("update-install", 100.),
            ("update-later", 90.),
            ("update-skip", 160.),
        ]),
    }
}

fn label(id: &str) -> &'static str {
    match id {
        "update-install" => "Install",
        "update-later" => "Later",
        "update-skip" => "Skip this version",
        _ => "Restart Babbage",
    }
}

impl SheetView {
    /// The release to offer: the newest one, unless it was skipped or this version already has it.
    pub fn pending_update(&self) -> Option<&updates::Release> {
        updates::pending(self.updates.found.as_ref(), updates::skipped().as_deref())
    }

    /// Checks at launch when a check is due: never when the interval is off, and not more often than the interval
    /// says.
    pub fn launch_check(&mut self, cx: &mut Context<Self>) {
        let Some(every) = updates::interval().every() else {
            return;
        };
        let source = updates::source_text();
        if updates::due(self.updates.found.as_ref(), &source, updates::now(), every) {
            self.run_check(false, cx);
        }
    }

    /// Asks the release source what is new. `manual` is a check the user asked for, which says how it went.
    pub fn run_check(&mut self, manual: bool, cx: &mut Context<Self>) {
        if matches!(self.updates.phase, Phase::Checking | Phase::Installing(_)) {
            return;
        }
        self.updates.phase = Phase::Checking;
        cx.notify();
        let source = updates::source_text();
        cx.spawn(async move |this, cx| {
            let asked = source.clone();
            let result = cx
                .background_executor()
                .spawn(async move { updates::run_check(&asked) })
                .await;
            let _ = this.update(cx, |v, cx| v.finish_check(result, &source, manual, cx));
        })
        .detach();
    }

    fn finish_check(
        &mut self,
        result: Result<Found, String>,
        source: &str,
        manual: bool,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok(found) => {
                let _ = updates::save(&found);
                if manual {
                    self.notice = Some(match found.latest() {
                        Some(r) => format!("Babbage {} is available", r.version),
                        None => format!("Babbage {} is up to date", updates::VERSION),
                    });
                }
                self.updates.found = Some(found);
                self.updates.phase = Phase::Idle;
            }
            Err(e) => {
                // The attempt counts as the check, so a source that cannot be reached is not asked again at every
                // launch; what was known before is kept.
                let mut kept = self.updates.found.clone().filter(|f| f.source == source);
                let found = kept.get_or_insert_with(|| Found {
                    checked: 0,
                    running: updates::VERSION.to_string(),
                    source: source.to_string(),
                    releases: Vec::new(),
                });
                found.checked = updates::now();
                found.running = updates::VERSION.to_string();
                let _ = updates::save(found);
                self.updates.found = kept;
                if manual {
                    self.notice = Some(format!("Could not check for updates: {e}"));
                }
                self.updates.phase = Phase::Failed(e);
            }
        }
        cx.notify();
    }

    /// Downloads, verifies and installs the offered release.
    pub fn start_install(&mut self, cx: &mut Context<Self>) {
        let Some(version) = self.pending_update().map(|r| r.version.clone()) else {
            return;
        };
        if matches!(
            self.updates.phase,
            Phase::Installing(_) | Phase::Installed(_)
        ) {
            return;
        }
        self.updates.phase = Phase::Installing("Starting".to_string());
        cx.notify();
        let (tx, mut rx) = futures::channel::mpsc::unbounded::<Install>();
        let (source, asked) = (updates::source_text(), version.clone());
        std::thread::spawn(move || {
            let said = tx.clone();
            let done = updates::install(&source, &asked, &mut |event| {
                if let Some(line) = event.human() {
                    let _ = said.unbounded_send(Install::Said(line));
                }
            });
            let _ = tx.unbounded_send(Install::Finished(done));
        });
        cx.spawn(async move |this, cx| {
            while let Some(message) = rx.next().await {
                let version = version.clone();
                let last = matches!(message, Install::Finished(_));
                if this
                    .update(cx, |v, cx| {
                        v.updates.phase = match message {
                            Install::Said(line) => Phase::Installing(line),
                            Install::Finished(Ok(())) => Phase::Installed(version),
                            Install::Finished(Err(e)) => Phase::InstallFailed(e),
                        };
                        cx.notify();
                    })
                    .is_err()
                    || last
                {
                    break;
                }
            }
        })
        .detach();
    }

    /// Hides the offered release until a newer one appears.
    fn skip_update(&mut self, cx: &mut Context<Self>) {
        if let Some(version) = self.pending_update().map(|r| r.version.clone()) {
            self.change_setting("update_skip", &version, cx);
        }
        self.panel = None;
        cx.notify();
    }

    /// Saves what is open, starts the new version on its own and quits this one.
    fn restart_app(&mut self, cx: &mut Context<Self>) {
        self.save_now();
        match updates::launch(&updates::restart_command()) {
            Ok(()) => cx.quit(),
            Err(e) => {
                self.notice = Some(format!(
                    "The update is installed, but Babbage could not start it: {e}"
                ));
                cx.notify();
            }
        }
    }

    /// A click on a button of the panel (by `buttons` id) or the pill.
    pub fn update_click(&mut self, id: &str, cx: &mut Context<Self>) {
        match id {
            "update-pill" => self.toggle_panel(Panel::Updates),
            "update-install" => self.start_install(cx),
            "update-later" => self.panel = None,
            "update-skip" => {
                self.skip_update(cx);
                return;
            }
            "update-restart" => {
                self.restart_app(cx);
                return;
            }
            _ => {}
        }
        cx.notify();
    }

    /// The pill, when there is something to offer.
    pub fn update_pill(&self, c: Colors, cx: &mut Context<Self>) -> Option<AnyElement> {
        let release = self.pending_update()?;
        let on = self.panel == Some(Panel::Updates);
        Some(
            div()
                .id("update-pill")
                .w(px(PILL_W))
                .h(px(TOOL_H))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(TOOL_H / 2.))
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .bg(if on { c.selection } else { c.caret })
                .text_color(if on { c.text } else { c.background })
                .hover(|d| d.bg(c.selection).text_color(c.text))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, window: &mut Window, cx| {
                        window.focus(this.focus_handle_ref());
                        this.update_click("update-pill", cx);
                    }),
                )
                .child(SharedString::from(format!("Update {}", release.version)))
                .into_any_element(),
        )
    }

    /// The "What's new" panel.
    pub fn updates_panel(&self, c: Colors, cx: &mut Context<Self>) -> AnyElement {
        let phase = &self.updates.phase;
        let found = self.updates.found.as_ref();
        let release = self.pending_update();
        let (title, sub) = match (phase, release) {
            (Phase::Installed(v), _) => (
                format!("Babbage {v} is installed"),
                "Restart to start using it".to_string(),
            ),
            (_, Some(r)) => (
                format!("Babbage {} is available", r.version),
                format!("You have {}", updates::VERSION),
            ),
            _ => (
                "Babbage is up to date".to_string(),
                format!("You have {}", updates::VERSION),
            ),
        };
        let status: Option<(String, bool)> = match phase {
            Phase::Checking => Some(("Checking".to_string(), false)),
            Phase::Failed(e) => Some((format!("Could not check: {e}"), true)),
            Phase::Installing(line) => Some((line.clone(), false)),
            Phase::InstallFailed(e) => Some((e.clone(), true)),
            _ => None,
        };
        let releases: Vec<&updates::Release> = if release.is_some() {
            found
                .map(|f| f.releases.iter().collect())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let notes = releases.iter().fold(
            div()
                .id("updates-notes")
                .absolute()
                .left(px(PAD))
                .top(px(PAD + 50.))
                .w(px(PANEL_W - 2. * PAD))
                .h(px(PANEL_H - PAD - 50. - 26. - BUTTON_H - PAD - 24.))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap(px(10.)),
            |col, r| {
                let lines = updates::note_lines(&r.notes);
                col.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(3.))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .gap(px(8.))
                                .child(
                                    div()
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(SharedString::from(r.version.clone())),
                                )
                                .child(
                                    div()
                                        .text_color(c.muted)
                                        .child(SharedString::from(r.date.clone())),
                                ),
                        )
                        .children(lines.into_iter().map(|n| {
                            match n {
                                Note::Heading(t) => div()
                                    .mt(px(4.))
                                    .text_color(c.muted)
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(SharedString::from(t)),
                                Note::Bullet(t) => div()
                                    .pl(px(10.))
                                    .child(SharedString::from(format!("• {t}"))),
                                Note::Text(t) => div().child(SharedString::from(t)),
                            }
                        })),
                )
            },
        );
        let mut panel = self
            .panel_frame(c, Some(PANEL_H))
            .id("updates-panel")
            .w(px(PANEL_W))
            .child(
                div()
                    .absolute()
                    .left(px(PAD))
                    .top(px(PAD))
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(SharedString::from(title)),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(c.muted)
                            .child(SharedString::from(sub)),
                    ),
            )
            .child(notes);
        if let Some((text, bad)) = status {
            panel = panel.child(
                div()
                    .absolute()
                    .left(px(PAD))
                    .top(px(PANEL_H - PAD - BUTTON_H - 8. - 20.))
                    .w(px(PANEL_W - 2. * PAD))
                    .text_size(px(12.))
                    .text_color(if bad { c.warn } else { c.muted })
                    .child(SharedString::from(text)),
            );
        }
        for (id, r) in buttons(phase) {
            let primary = id == "update-install" || id == "update-restart";
            panel = panel.child(
                div()
                    .id(id)
                    .absolute()
                    .left(px(r.x))
                    .top(px(r.y))
                    .w(px(r.w))
                    .h(px(r.h))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(8.))
                    .text_size(px(13.))
                    .bg(if primary { c.caret } else { c.line })
                    .text_color(if primary { c.background } else { c.text })
                    .hover(|d| d.bg(c.selection).text_color(c.text))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window: &mut Window, cx| {
                            window.focus(this.focus_handle_ref());
                            this.update_click(id, cx);
                        }),
                    )
                    .child(label(id)),
            );
        }
        panel.into_any_element()
    }

    /// What the state report says about updates.
    pub fn updates_report(&self, right: f32, toolbar_top: f32) -> Value {
        let u = &self.updates;
        let pill = self.pending_update().map(|_| {
            let r = pill_rect();
            json!({"x": right + r.x, "y": toolbar_top + r.y, "w": r.w, "h": r.h})
        });
        let (px_, py_) = panel_origin(right, toolbar_top);
        let message = match &u.phase {
            Phase::Failed(m)
            | Phase::Installing(m)
            | Phase::Installed(m)
            | Phase::InstallFailed(m) => Some(m.clone()),
            _ => None,
        };
        json!({
            "phase": u.phase.name(),
            "message": message,
            "running": updates::VERSION,
            "interval": format!("{:?}", updates::interval()).to_lowercase(),
            "source": updates::source_text(),
            "checked": u.found.as_ref().map(|f| f.checked),
            "latest": self.pending_update().map(|r| r.version.clone()),
            "releases": u.found.as_ref().map(|f| f.releases.iter().map(|r| json!({
                "version": r.version, "date": r.date, "notes": r.notes,
            })).collect::<Vec<_>>()),
            "pill": pill,
            "panel": {"x": px_, "y": py_, "w": PANEL_W, "h": PANEL_H},
            "buttons": buttons(&u.phase).iter().map(|(id, r)| json!({
                "id": id, "x": px_ + r.x, "y": py_ + r.y, "w": r.w, "h": r.h,
            })).collect::<Vec<_>>(),
        })
    }
}
