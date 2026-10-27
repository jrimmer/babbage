//! The window: sidebar, editor with line numbers, answer column and the floating total.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use babbage_store::{Autosaver, SheetBook, SheetId, SheetInfo};
use gpui::{
    AnyElement, App, Bounds, ClipboardItem, Context, EntityInputHandler, FocusHandle, Focusable,
    FontWeight, Hsla, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    Pixels, Point, ScrollWheelEvent, ShapedLine, SharedString, TextRun, UTF16Selection,
    UnderlineStyle, Window, div, point, prelude::*, px, size,
};

use crate::appprefs;
use crate::chrome::Panel;
use crate::computed::{Answer, Computed, compute_formatted};
use crate::editor_model::{Editor, History, Kind, Marked, Pos};
use crate::linefmt::{self, LineFormat};
use crate::shown::{Shown, shown_for};
use crate::syntax::{Class, Span};
use crate::theme::Colors;
use crate::timefmt::describe_time;
use crate::wrap::Wrap;

pub const ROW_H: f32 = 36.;
pub const GUTTER_W: f32 = 64.;
pub const TOP_PAD: f32 = 14.;
pub const SIDEBAR_W: f32 = 236.;
/// Where the first sheet card starts, how tall a card is and how tall a row of its menu is, in points: fixed so
/// a test can aim at them.
pub const SIDEBAR_TOP: f32 = 56.;
pub const CARD_H: f32 = 54.;
pub const MENU_ROW_H: f32 = 28.;
pub const MENU_ROWS: [&str; 2] = ["Rename", "Move to trash"];
const ANSWER_MIN_W: f32 = 300.;
/// How long a pause in typing is before the sheet is written.
pub const SAVE_DELAY_MS: u64 = 400;

/// What the last paint measured, for the mouse, the input method and the test hook.
pub struct Layout {
    pub bounds: Bounds<Pixels>,
    pub text_x: Pixels,
    pub answer_x: Pixels,
    pub answer_w: Pixels,
    pub top: Pixels,
    pub shaped: Vec<Option<ShapedLine>>,
    pub scale: f32,
    /// How wide a row of text may be before it wraps.
    pub text_w: Pixels,
}

/// What the window opens with, read before the window exists so a broken book is reported on the
/// command line.
pub struct Init {
    pub sheets: Vec<SheetInfo>,
    pub current: SheetId,
    pub lines: Vec<String>,
}

impl Init {
    /// Opens the most recently changed sheet, making one if the book is empty.
    pub fn load(book: &SheetBook) -> anyhow::Result<Init> {
        let mut sheets = book.list()?;
        let current = match sheets.first() {
            Some(s) => s.id.clone(),
            None => {
                let id = book.create::<&str>(&[])?;
                sheets = book.list()?;
                id
            }
        };
        let (current, sheets) =
            if appprefs::new_sheet_on_launch() && sheets.first().is_some_and(|s| s.lines > 0) {
                let id = book.create::<&str>(&[])?;
                (id, book.list()?)
            } else {
                (current, sheets)
            };
        let lines = book.read(&current)?;
        Ok(Init {
            sheets: order_sheets(sheets),
            current,
            lines,
        })
    }
}

/// `sheets` in the order the settings ask for: by name, by when they were made, or (the default) as given, which is
/// most recently changed first.
pub fn order_sheets(mut sheets: Vec<SheetInfo>) -> Vec<SheetInfo> {
    match appprefs::sheet_order() {
        appprefs::SheetOrder::Modified => {}
        appprefs::SheetOrder::Created => {
            sheets.sort_by(|a, b| b.created.cmp(&a.created).then_with(|| a.id.cmp(&b.id)));
        }
        appprefs::SheetOrder::Title => sheets.sort_by(|a, b| {
            a.display_name()
                .to_lowercase()
                .cmp(&b.display_name().to_lowercase())
                .then_with(|| a.id.cmp(&b.id))
        }),
    }
    sheets
}

/// A variable under the pointer: its value as the sheet shows it, and the point (window coordinates) the tip hangs
/// from.
#[derive(Clone, Debug, PartialEq)]
pub struct HoverVar {
    pub name: String,
    pub value: String,
    pub x: f32,
    pub y: f32,
}

/// A number being scrubbed: where it is in the line, how it reads now, and where the pointer started.
struct Scrub {
    line: usize,
    start: usize,
    len: usize,
    number: crate::scrub::Scrubbable,
    origin_x: f32,
}

/// An editor for the lines a sheet's file holds: the text, with how each answer is written read from the trailer.
fn editor_from_file(lines: &[String]) -> Editor {
    let (content, formats) = linefmt::split(lines);
    Editor::new(&content).with_formats(&formats)
}

pub struct Rename {
    pub text: String,
}

pub struct SheetView {
    book: SheetBook,
    saver: Arc<Autosaver>,
    pub sheets: Vec<SheetInfo>,
    pub current: SheetId,
    pub ed: Editor,
    /// Undo steps of the sheets that are not open, by sheet.
    histories: HashMap<SheetId, History>,
    pub computed: Computed,
    pub sidebar_open: bool,
    /// The settings or help panel opened from the toolbar.
    pub panel: Option<Panel>,
    /// The pane of the Settings window that is showing, and a text control being edited (key, text so far).
    pub settings_pane: usize,
    pub settings_edit: Option<(String, String)>,
    /// The line the pointer is over in the answer column (it shows the `⋯` button), and the line whose answer menu
    /// is open.
    pub hover_line: Option<usize>,
    pub answer_menu: Option<usize>,
    /// The list of names Escape offers while a name is half typed.
    pub complete: Option<crate::autocomplete::Completion>,
    /// What a variable the pointer rests on is worth, and where to show it.
    pub hover_var: Option<HoverVar>,
    /// A number being dragged with Alt held.
    scrub: Option<Scrub>,
    /// A renamed variable: the line it is declared on and the name it had, while the rename is being typed; the
    /// offer to rename its uses; and the line the user said to leave alone.
    pub rename_origin: Option<(usize, String)>,
    pub rename_offer: Option<crate::rename::RenameOffer>,
    pub rename_muted: Option<usize>,
    pub answer_focus: Option<usize>,
    pub rename: Option<Rename>,
    /// The sheet whose menu (Rename, Move to trash) is open in the sidebar.
    pub menu: Option<SheetId>,
    pub notice: Option<String>,
    /// What the update checks found and what they are doing.
    pub updates: crate::update_ui::Updates,
    scroll: f32,
    focus: FocusHandle,
    pub layout: Option<Layout>,
    /// Where each line breaks into rows, as of the last paint.
    pub wrap: Wrap,
    /// A change moved the caret: the next paint scrolls it into view.
    reveal: bool,
    selecting: bool,
}

impl Focusable for SheetView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl SheetView {
    pub fn new(
        init: Init,
        book: SheetBook,
        saver: Arc<Autosaver>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> SheetView {
        let Init {
            sheets,
            current,
            lines,
        } = init;
        let ed = editor_from_file(&lines);
        let computed = compute_formatted(ed.lines(), ed.formats());
        let focus = cx.focus_handle();
        window.focus(&focus);
        let mut view = SheetView {
            book,
            saver,
            sheets,
            current,
            ed,
            histories: HashMap::new(),
            computed,
            sidebar_open: true,
            panel: None,
            settings_pane: 0,
            settings_edit: None,
            hover_line: None,
            answer_menu: None,
            complete: None,
            hover_var: None,
            scrub: None,
            rename_origin: None,
            rename_offer: None,
            rename_muted: None,
            answer_focus: None,
            rename: None,
            menu: None,
            notice: None,
            updates: crate::update_ui::Updates::load(),
            scroll: 0.,
            focus,
            layout: None,
            wrap: Wrap::default(),
            reveal: false,
            selecting: false,
        };
        // The sidebar's times and counts come from the files, so they are read again once saves land.
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(700))
                    .await;
                if this.update(cx, |v, cx| v.refresh_list(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        view.launch_check(cx);
        view
    }

    /// Writes whatever is waiting to be saved.
    pub fn save_now(&self) {
        self.saver.flush();
    }

    // Sheets.

    /// Reads the book again, keeping the order the sidebar already has (so Ctrl+Tab does not shuffle as
    /// files are saved); new sheets go first, vanished ones go.
    pub fn refresh_list(&mut self, cx: &mut Context<Self>) {
        for e in self.saver.take_errors() {
            self.notice = Some(format!("Could not save: {e}"));
        }
        let Ok(fresh) = self.book.list() else { return };
        // Ordered by name or by creation, the list is sorted afresh; by modification it keeps the order it has, so
        // it does not shuffle while a sheet is being saved.
        if appprefs::sheet_order() != appprefs::SheetOrder::Modified {
            let fresh = order_sheets(fresh);
            if fresh != self.sheets {
                self.sheets = fresh;
                cx.notify();
            }
            return;
        }
        let mut next: Vec<SheetInfo> = Vec::with_capacity(fresh.len());
        for old in &self.sheets {
            if let Some(f) = fresh.iter().find(|f| f.id == old.id) {
                next.push(f.clone());
            }
        }
        let known: Vec<SheetId> = next.iter().map(|s| s.id.clone()).collect();
        let mut added: Vec<SheetInfo> = fresh
            .into_iter()
            .filter(|f| !known.contains(&f.id))
            .collect();
        added.append(&mut next);
        if added != self.sheets {
            self.sheets = added;
            cx.notify();
        }
    }

    fn open_sheet(&mut self, id: SheetId, cx: &mut Context<Self>) {
        self.saver.flush();
        match self.book.read(&id) {
            Ok(lines) => {
                let left = self.ed.take_history();
                self.histories.insert(self.current.clone(), left);
                self.ed = editor_from_file(&lines);
                if let Some(kept) = self.histories.remove(&id) {
                    self.ed.restore_history(kept);
                }
                self.current = id;
                self.scroll = 0.;
                self.answer_focus = None;
                self.rename = None;
                self.menu = None;
                self.notice = None;
                self.recompute();
            }
            Err(e) => self.notice = Some(e.to_string()),
        }
        self.refresh_list(cx);
        cx.notify();
    }

    pub fn new_sheet(&mut self, cx: &mut Context<Self>) {
        self.saver.flush();
        match self.book.create::<&str>(&[]) {
            Ok(id) => {
                // The new sheet is first in the sidebar.
                if let Ok(fresh) = self.book.list()
                    && let Some(info) = fresh.into_iter().find(|s| s.id == id)
                {
                    self.sheets.insert(0, info);
                }
                self.open_sheet(id, cx);
            }
            Err(e) => self.notice = Some(e.to_string()),
        }
        cx.notify();
    }

    fn switch(&mut self, delta: isize, cx: &mut Context<Self>) {
        let n = self.sheets.len() as isize;
        if n < 2 {
            return;
        }
        let at = self
            .sheets
            .iter()
            .position(|s| s.id == self.current)
            .unwrap_or(0) as isize;
        let id = self.sheets[(at + delta).rem_euclid(n) as usize].id.clone();
        self.open_sheet(id, cx);
    }

    fn start_rename(&mut self, cx: &mut Context<Self>) {
        let text = self
            .sheets
            .iter()
            .find(|s| s.id == self.current)
            .map(|s| s.display_name())
            .unwrap_or_default();
        self.rename = Some(Rename { text });
        cx.notify();
    }

    fn finish_rename(&mut self, cx: &mut Context<Self>) {
        let Some(r) = self.rename.take() else { return };
        self.saver.flush();
        match self.book.rename(&self.current, &r.text) {
            Ok(id) => {
                self.current = id;
                self.notice = None;
            }
            Err(e) => self.notice = Some(e.to_string()),
        }
        // The renamed sheet keeps its place, so the list is rebuilt around the new name.
        if let Ok(fresh) = self.book.list() {
            let order: Vec<SheetId> = self.sheets.iter().map(|s| s.id.clone()).collect();
            let mut next: Vec<SheetInfo> = Vec::new();
            for old in &order {
                let id = if *old == self.current || fresh.iter().any(|f| f.id == *old) {
                    old.clone()
                } else {
                    self.current.clone()
                };
                if let Some(f) = fresh.iter().find(|f| f.id == id)
                    && !next.iter().any(|n| n.id == id)
                {
                    next.push(f.clone());
                }
            }
            for f in fresh {
                if !next.iter().any(|n| n.id == f.id) {
                    next.push(f);
                }
            }
            self.sheets = next;
        }
        cx.notify();
    }

    /// Moves the current sheet to the trash and opens a neighbor (or a fresh sheet if it was the last).
    fn delete_current(&mut self, cx: &mut Context<Self>) {
        self.saver.flush();
        let gone = self.current.clone();
        let at = self.sheets.iter().position(|s| s.id == gone).unwrap_or(0);
        if let Err(e) = self.book.delete(&gone) {
            self.notice = Some(e.to_string());
            cx.notify();
            return;
        }
        self.sheets.retain(|s| s.id != gone);
        match self
            .sheets
            .get(at)
            .or_else(|| self.sheets.last())
            .map(|s| s.id.clone())
        {
            Some(id) => self.open_sheet(id, cx),
            None => self.new_sheet(cx),
        }
        self.notice = Some("Moved to the trash".to_owned());
        cx.notify();
    }

    /// Writes the open sheet, with its answers, as text, CSV, HTML and PDF into the sheetbook's `exports` folder
    /// (Ctrl+Shift+E) and says where in the notice.
    fn export_current(&mut self, cx: &mut Context<Self>) {
        use babbage_core::LineKind;
        use babbage_store::export::{Row, RowKind};
        let lines = self.ed.lines().to_vec();
        let rows: Vec<Row> = lines
            .iter()
            .enumerate()
            .map(|(i, text)| {
                let kind = match self.computed.results.get(i).map(|r| &r.kind) {
                    Some(LineKind::Heading(_)) => RowKind::Heading,
                    Some(LineKind::Comment) => RowKind::Comment,
                    Some(LineKind::Divider) => RowKind::Divider,
                    Some(LineKind::Blank) | None => RowKind::Blank,
                    Some(_) => RowKind::Plain,
                };
                let answer = self.computed.answers.get(i).and_then(|a| {
                    a.text.as_ref().map(|t| {
                        if a.exact {
                            t.clone()
                        } else {
                            format!("≈ {t}")
                        }
                    })
                });
                Row {
                    text: text.clone(),
                    answer,
                    kind,
                }
            })
            .collect();
        let name = self
            .sheets
            .iter()
            .find(|s| s.id == self.current)
            .map_or_else(|| "sheet".to_owned(), SheetInfo::display_name);
        let total = self.computed.total.as_ref().and_then(|t| t.text.clone());
        let dir = self.book.dir().join("exports");
        self.notice = Some(
            match babbage_store::export::write_all(&dir, &name, &rows, total.as_deref()) {
                Ok(_) => format!("Exported as text, CSV, HTML and PDF to {}", dir.display()),
                Err(e) => format!("Could not export: {e}"),
            },
        );
        cx.notify();
    }

    // Text.

    /// The lines the sheet's file holds: its text, then the trailer lines that keep how its answers are written.
    pub fn file_lines(&self) -> Vec<String> {
        linefmt::join(self.ed.lines(), self.ed.formats())
    }

    /// Sets how line `i`'s answer is written and saves the sheet.
    pub fn set_line_format(&mut self, i: usize, f: LineFormat, cx: &mut Context<Self>) {
        self.ed.set_format(i, f);
        self.recompute();
        self.saver.submit(&self.current, self.file_lines());
        cx.notify();
    }

    /// After the text changed by something other than typing (a menu action): works it out, saves it, keeps the
    /// caret in view.
    pub fn is_scrubbing(&self) -> bool {
        self.scrub.is_some()
    }

    pub fn after_edit(&mut self, cx: &mut Context<Self>) {
        self.edited(cx);
    }

    pub fn recompute(&mut self) {
        self.computed = compute_formatted(self.ed.lines(), self.ed.formats());
    }

    /// Called after the text changed: evaluates the whole sheet again and queues the save.
    fn edited(&mut self, cx: &mut Context<Self>) {
        self.answer_focus = None;
        self.complete = None;
        let declared: Vec<Option<String>> = self
            .computed
            .results
            .iter()
            .map(|r| r.declares.clone())
            .collect();
        self.recompute();
        self.check_rename(&declared, cx);
        self.saver.submit(&self.current, self.file_lines());
        self.reveal = true;
        self.scroll_to_caret();
        cx.notify();
    }

    fn moved(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.rename_muted.is_some_and(|l| l != self.ed.head().line) {
            self.rename_muted = None;
        }
        self.reveal = true;
        self.scroll_to_caret();
        window.invalidate_character_coordinates();
        cx.notify();
    }

    fn viewport_h(&self) -> f32 {
        self.layout
            .as_ref()
            .map_or(600., |l| f32::from(l.bounds.size.height))
    }

    /// What line `i` shows: its text with references drawn as the answers they point at.
    pub fn shown_line(&self, i: usize) -> Shown {
        shown_for(
            self.ed.line(i),
            self.computed.results.get(i),
            i,
            &self.computed.answers,
        )
    }

    /// The row of the sheet the caret is on.
    fn caret_row(&self) -> usize {
        let head = self.ed.head();
        let col = self.shown_line(head.line).to_shown(head.col);
        self.wrap.top_row(head.line) + self.wrap.row_containing(head.line, col)
    }

    fn scroll_to_caret(&mut self) {
        let view = self.viewport_h();
        let y = self.caret_row() as f32 * ROW_H;
        let top = self.scroll;
        if y < top {
            self.scroll = y;
        } else if y + ROW_H + TOP_PAD * 2. > top + view {
            self.scroll = y + ROW_H + TOP_PAD * 2. - view;
        }
        self.clamp_scroll();
    }

    pub fn clamp_scroll(&mut self) {
        let content = self.wrap.total_rows(self.ed.len()) as f32 * ROW_H + TOP_PAD * 2.;
        let max = (content - self.viewport_h()).max(0.);
        self.scroll = self.scroll.clamp(0., max);
    }

    fn copy_answer(&self, line: usize, cx: &mut Context<Self>) {
        if let Some(t) = self.computed.answers.get(line).and_then(|a| a.text.clone()) {
            cx.write_to_clipboard(ClipboardItem::new_string(t));
        }
    }

    // Keys.

    fn on_key(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let m = &ks.modifiers;
        let (ctrl, shift, alt) = (m.control, m.shift, m.alt);
        let key = ks.key.as_str();
        if (self.panel.is_some() || self.answer_menu.is_some()) && key == "escape" {
            self.panel = None;
            self.answer_menu = None;
            cx.notify();
            cx.stop_propagation();
            return;
        }
        if self.menu.take().is_some() {
            // Any key closes the menu; Escape does nothing else.
            cx.notify();
            if key == "escape" {
                cx.stop_propagation();
                return;
            }
        }
        if self.settings_edit.is_some()
            && self.settings_edit_key(key, ks.key_char.as_deref(), ctrl, cx)
        {
            cx.stop_propagation();
            return;
        }
        if self.complete.is_some() && !ctrl && self.completion_key(key, cx) {
            cx.stop_propagation();
            return;
        }
        if self.rename.is_some() {
            self.rename_key(ks.key.as_str(), ks.key_char.as_deref(), ctrl, cx);
            cx.stop_propagation();
            return;
        }
        if ctrl {
            let handled = match key {
                "n" if !shift => {
                    self.new_sheet(cx);
                    true
                }
                "tab" => {
                    self.switch(if shift { -1 } else { 1 }, cx);
                    true
                }
                "b" => {
                    self.sidebar_open = !self.sidebar_open;
                    cx.notify();
                    true
                }
                "q" => {
                    self.saver.flush();
                    cx.quit();
                    true
                }
                "delete" if shift => {
                    self.delete_current(cx);
                    true
                }
                "e" if shift => {
                    self.export_current(cx);
                    true
                }
                _ => false,
            };
            if handled {
                cx.stop_propagation();
                return;
            }
        }
        if key == "f2" {
            self.start_rename(cx);
            cx.stop_propagation();
            return;
        }
        if let Some(line) = self.answer_focus {
            let used = match key {
                "c" if ctrl => {
                    self.copy_answer(line, cx);
                    true
                }
                "up" | "down" => {
                    if key == "up" {
                        self.ed.up(false);
                    } else {
                        self.ed.down(false);
                    }
                    self.answer_focus = Some(self.ed.head().line);
                    self.moved(window, cx);
                    true
                }
                "escape" | "left" => {
                    self.answer_focus = None;
                    cx.notify();
                    true
                }
                _ => false,
            };
            if used {
                cx.stop_propagation();
                return;
            }
        }
        let before = self.ed.lines().to_vec();
        let used = self.editor_key(key, ctrl, shift, alt, window, cx);
        if used {
            if self.ed.lines() != before.as_slice() {
                self.edited(cx);
            } else {
                self.moved(window, cx);
            }
            cx.stop_propagation();
        } else if ctrl || alt {
            // A shortcut this editor does not know must never reach the text as a character.
            cx.stop_propagation();
        }
    }

    fn editor_key(
        &mut self,
        key: &str,
        ctrl: bool,
        shift: bool,
        alt: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.ed.marked.is_some() {
            // The input method owns the keys while it composes.
            return false;
        }
        if alt && key == "right" {
            self.answer_focus = Some(self.ed.head().line);
            return true;
        }
        if ctrl {
            match key {
                "z" if !shift => {
                    self.ed.undo();
                }
                "z" | "y" => {
                    self.ed.redo();
                }
                "a" => self.ed.select_all(),
                // Comment the lines out, or back in (Ctrl-/).
                "/" => self.ed.toggle_comment(),
                "c" => {
                    if self.ed.has_selection() {
                        cx.write_to_clipboard(ClipboardItem::new_string(self.ed.selected_text()));
                    }
                }
                "x" => {
                    if let Some(t) = self.ed.cut() {
                        cx.write_to_clipboard(ClipboardItem::new_string(t));
                    }
                }
                "v" => {
                    if let Some(t) = cx.read_from_clipboard().and_then(|c| c.text()) {
                        let t = if appprefs::convert_pasted_numbers() {
                            crate::grouping::convert_foreign(
                                &t,
                                crate::prefs::current().number_format,
                            )
                        } else {
                            t
                        };
                        let (a, b) = self.ed.selection();
                        self.ed.replace(a, b, &t, Kind::Other);
                    }
                }
                "home" => self.ed.doc_start(shift),
                "end" => self.ed.doc_end(shift),
                "left" => self.ed.left(shift, true),
                "right" => self.ed.right(shift, true),
                "backspace" => self.ed.backspace(true),
                "delete" => self.ed.delete(true),
                _ => return false,
            }
            return true;
        }
        match key {
            "left" => {
                self.ed.left(shift, false);
                self.snap_off_chip(false, shift);
            }
            "right" => {
                self.ed.right(shift, false);
                self.snap_off_chip(true, shift);
            }
            "backspace" if self.delete_chip(false) => {}
            "delete" if self.delete_chip(true) => {}
            "up" if self.vertical_in_line(-1, shift) => {}
            "down" if self.vertical_in_line(1, shift) => {}
            "up" => self.ed.up(shift),
            "down" => self.ed.down(shift),
            "home" => self.ed.home(shift),
            "end" => self.ed.end(shift),
            "pageup" | "pagedown" => {
                let rows = ((self.viewport_h() / ROW_H) as usize)
                    .saturating_sub(1)
                    .max(1);
                for _ in 0..rows {
                    if key == "pageup" {
                        self.ed.up(shift);
                    } else {
                        self.ed.down(shift);
                    }
                }
            }
            "backspace" => self.ed.backspace(false),
            "delete" => self.ed.delete(false),
            "enter" => self.ed.newline(),
            // Escape finishes the name of a variable being typed; Tab does nothing in a sheet (swallowing it keeps a
            // tab out of the text).
            "escape" if !appprefs::autocomplete_on_tab() => self.start_completion(cx),
            "tab" if appprefs::autocomplete_on_tab() => self.start_completion(cx),
            "escape" | "tab" => {}
            _ => return false,
        }
        true
    }

    /// With thousands separators as you type: the number the caret is in, regrouped, as part of the typing step.
    fn regroup_caret(&mut self) {
        let head = self.ed.head();
        let line = self.ed.line(head.line).to_string();
        let format = crate::prefs::current().number_format;
        if let Some((new, caret)) = crate::grouping::regroup(&line, head.col, format) {
            let (range, text) = crate::grouping::smallest_edit(&line, &new);
            self.ed.replace(
                Pos::new(head.line, range.start),
                Pos::new(head.line, range.end),
                text,
                Kind::Typing,
            );
            self.ed.set_caret(Pos::new(head.line, caret), false);
        }
    }

    /// A reference drawn as a chip is one thing to the caret: after a step into it, the caret goes to the far side.
    fn snap_off_chip(&mut self, forward: bool, extend: bool) {
        let head = self.ed.head();
        if let Some(r) = self.shown_line(head.line).chip_at_src(head.col) {
            let col = if forward { r.end } else { r.start };
            self.ed.set_caret(Pos::new(head.line, col), extend);
        }
    }

    /// Backspace after a chip (or Delete before one) removes the whole reference. False when there is no chip there.
    fn delete_chip(&mut self, forward: bool) -> bool {
        if self.ed.has_selection() {
            return false;
        }
        let head = self.ed.head();
        let sh = self.shown_line(head.line);
        let chip = if forward {
            sh.chip_starting_at(head.col)
        } else {
            sh.chip_ending_at(head.col)
        };
        let Some(r) = chip else { return false };
        self.ed.set_caret(Pos::new(head.line, r.start), false);
        self.ed.set_caret(Pos::new(head.line, r.end), true);
        self.ed.backspace(false);
        true
    }

    /// Up and Down inside a line that wraps move between its rows, keeping the caret's x. Returns false when the
    /// caret is already on the first (or last) row, so the move goes to the neighboring line.
    fn vertical_in_line(&mut self, delta: isize, extend: bool) -> bool {
        let head = self.ed.head();
        let rows = self.wrap.rows_of(head.line);
        let k = self
            .wrap
            .row_containing(head.line, self.shown_line(head.line).to_shown(head.col));
        let to = k as isize + delta;
        if rows < 2 || to < 0 || to >= rows as isize {
            return false;
        }
        let Some(shaped) = self
            .layout
            .as_ref()
            .and_then(|l| l.shaped.get(head.line))
            .and_then(Option::as_ref)
        else {
            return false;
        };
        let sh = self.shown_line(head.line);
        let text = sh.text.as_str();
        let (from, _) = self.wrap.row_span(head.line, k, text.len());
        let (start, end) = self.wrap.row_span(head.line, to as usize, text.len());
        let here = sh.to_shown(head.col);
        let x = shaped.x_for_index(here) - shaped.x_for_index(from) + shaped.x_for_index(start);
        let mut col = shaped.closest_index_for_x(x).clamp(start, end);
        if end < text.len() && col == end {
            col = text[..end]
                .char_indices()
                .last()
                .map_or(start, |(i, _)| i)
                .max(start);
        }
        self.ed
            .set_caret(Pos::new(head.line, sh.to_src(col)), extend);
        true
    }

    fn rename_key(&mut self, key: &str, ch: Option<&str>, ctrl: bool, cx: &mut Context<Self>) {
        match key {
            "enter" => return self.finish_rename(cx),
            "escape" => self.rename = None,
            "backspace" => {
                if let Some(r) = &mut self.rename {
                    r.text.pop();
                }
            }
            _ if !ctrl => {
                if let (Some(r), Some(c)) = (&mut self.rename, ch)
                    && !c.chars().any(char::is_control)
                {
                    r.text.push_str(c);
                }
            }
            _ => {}
        }
        cx.notify();
    }

    // Mouse.

    fn position_at(&self, p: Point<Pixels>) -> Option<(Pos, bool)> {
        let l = self.layout.as_ref()?;
        let y = f32::from(p.y - l.top);
        let row = (y / ROW_H).floor().max(0.) as usize;
        let (line, k) = self.wrap.locate(row, self.ed.len());
        let in_answers = p.x >= l.answer_x;
        let sh = self.shown_line(line);
        let text = sh.text.as_str();
        let (start, end) = self.wrap.row_span(line, k, text.len());
        let col = match l.shaped.get(line).and_then(Option::as_ref) {
            Some(s) if !text.is_empty() => {
                let x0 = s.x_for_index(start);
                let col = s.closest_index_for_x(p.x - l.text_x + x0).clamp(start, end);
                // A click at the end of a row that wraps lands before its last character, not at the start of
                // the next row.
                if end < text.len() && col == end {
                    text[..end]
                        .char_indices()
                        .last()
                        .map_or(start, |(i, _)| i)
                        .max(start)
                } else {
                    col
                }
            }
            _ => start,
        };
        Some((Pos::new(line, sh.to_src(col)), in_answers))
    }

    fn on_mouse_down(&mut self, ev: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus);
        self.menu = None;
        self.panel = None;
        self.answer_menu = None;
        let Some((pos, in_answers)) = self.position_at(ev.position) else {
            return;
        };
        if !in_answers
            && ev.modifiers.alt
            && let Some(scrub) = self.number_at(ev.position, pos)
        {
            self.scrub = Some(scrub);
            return;
        }
        if in_answers && self.on_more_button(ev.position, pos.line) {
            self.open_answer_menu(pos.line);
        } else if in_answers {
            self.ed.set_caret(Pos::new(pos.line, 0), false);
            self.answer_focus = Some(pos.line);
        } else {
            self.answer_focus = None;
            self.selecting = true;
            self.ed.set_caret(pos, ev.modifiers.shift);
            // A double click selects the word, a triple click the line.
            match ev.click_count {
                2 => self.ed.select_word_at(pos),
                3.. => self.ed.select_line(pos.line),
                _ => {}
            }
        }
        self.moved(window, cx);
    }

    /// The variable the pointer is on, with its value: the answer of the line that defines it.
    fn variable_at(&self, p: Point<Pixels>) -> Option<HoverVar> {
        let l = self.layout.as_ref()?;
        let (pos, in_answers) = self.position_at(p)?;
        if in_answers || p.x < l.text_x || self.ed.line(pos.line).is_empty() {
            return None;
        }
        let names: Vec<String> = self
            .computed
            .names
            .get(pos.line)?
            .iter()
            .map(|(n, _)| n.clone())
            .collect();
        let line = self.ed.line(pos.line);
        let span = crate::syntax::spans_in(line, self.computed.results.get(pos.line), &names)
            .into_iter()
            .find(|s| {
                s.class == crate::syntax::Class::Variable
                    && s.range.start <= pos.col
                    && pos.col <= s.range.end
            })?;
        let name = line[span.range.clone()].to_string();
        let (_, at) = self
            .computed
            .names
            .get(pos.line)?
            .iter()
            .find(|(n, _)| *n == name)?;
        let value = self.answer_for(*at)?.text.clone()?;
        let sh = self.shown_line(pos.line);
        let (from, to) = (sh.to_shown(span.range.start), sh.to_shown(span.range.end));
        let k = self.wrap.row_containing(pos.line, from);
        // The pointer has to be over the name itself, not past the end of the line it ends.
        let shaped = l.shaped.get(pos.line).and_then(Option::as_ref)?;
        let row_start = self.wrap.row_span(pos.line, k, sh.text.len()).0;
        let x0 = shaped.x_for_index(row_start);
        let (xa, xb) = (
            l.text_x + shaped.x_for_index(from) - x0,
            l.text_x + shaped.x_for_index(to) - x0,
        );
        if p.x < xa - px(2.) || p.x > xb + px(2.) {
            return None;
        }
        let y = f32::from(l.top) + (self.wrap.top_row(pos.line) + k + 1) as f32 * ROW_H;
        Some(HoverVar {
            name,
            value,
            x: f32::from(p.x) - 30.,
            y,
        })
    }

    /// Whether `p` is on the `⋯` button of `line`, which only a line with an answer has.
    fn on_more_button(&self, p: Point<Pixels>, line: usize) -> bool {
        let Some(l) = self.layout.as_ref() else {
            return false;
        };
        let centre = f32::from(l.bounds.right()) - crate::answer_menu::MORE_FROM_RIGHT;
        let has_answer = self.answer_for(line).is_some_and(|a| a.text.is_some());
        has_answer && (f32::from(p.x) - centre).abs() <= crate::answer_menu::MORE_RADIUS + 4.
    }

    /// The plain number under the pointer, ready to scrub.
    fn number_at(&self, p: Point<Pixels>, pos: Pos) -> Option<Scrub> {
        let l = self.layout.as_ref()?;
        let line = self.ed.line(pos.line);
        let names: Vec<String> = self
            .computed
            .names
            .get(pos.line)
            .map(|v| v.iter().map(|(n, _)| n.clone()).collect())
            .unwrap_or_default();
        let span = crate::syntax::spans_in(line, self.computed.results.get(pos.line), &names)
            .into_iter()
            .find(|s| {
                s.class == crate::syntax::Class::Number
                    && s.range.start <= pos.col
                    && pos.col <= s.range.end
            })?;
        let number = crate::scrub::Scrubbable::parse(&line[span.range.clone()])?;
        let sh = self.shown_line(pos.line);
        let (from, to) = (sh.to_shown(span.range.start), sh.to_shown(span.range.end));
        let shaped = l.shaped.get(pos.line).and_then(Option::as_ref)?;
        let k = self.wrap.row_containing(pos.line, from);
        let x0 = shaped.x_for_index(self.wrap.row_span(pos.line, k, sh.text.len()).0);
        let (xa, xb) = (
            l.text_x + shaped.x_for_index(from) - x0,
            l.text_x + shaped.x_for_index(to) - x0,
        );
        if p.x < xa - px(3.) || p.x > xb + px(3.) {
            return None;
        }
        Some(Scrub {
            line: pos.line,
            start: span.range.start,
            len: span.range.len(),
            number,
            origin_x: f32::from(p.x),
        })
    }

    /// Moves a number being scrubbed to where the pointer is.
    fn scrub_to(&mut self, x: f32, cx: &mut Context<Self>) {
        let Some(s) = &self.scrub else { return };
        let text = s.number.shifted(crate::scrub::steps_for(x - s.origin_x));
        let (line, start, len) = (s.line, s.start, s.len);
        if text.len() == len && self.ed.line(line).get(start..start + len) == Some(text.as_str()) {
            return;
        }
        self.ed.replace(
            Pos::new(line, start),
            Pos::new(line, start + len),
            &text,
            Kind::Scrub,
        );
        if let Some(s) = &mut self.scrub {
            s.len = text.len();
        }
        self.edited(cx);
    }

    fn on_mouse_move(&mut self, ev: &MouseMoveEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.scrub.is_some() {
            if ev.pressed_button == Some(MouseButton::Left) {
                self.scrub_to(f32::from(ev.position.x), cx);
            }
            return;
        }
        let over = self
            .position_at(ev.position)
            .and_then(|(pos, in_answers)| in_answers.then_some(pos.line));
        if over != self.hover_line {
            self.hover_line = over;
            cx.notify();
        }
        let var = self.variable_at(ev.position);
        if var != self.hover_var {
            self.hover_var = var;
            cx.notify();
        }
        if self.selecting
            && ev.pressed_button == Some(MouseButton::Left)
            && let Some((pos, _)) = self.position_at(ev.position)
        {
            self.ed.set_caret(pos, true);
            self.moved(window, cx);
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.selecting = false;
        self.scrub = None;
    }

    fn on_scroll(&mut self, ev: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let dy = f32::from(ev.delta.pixel_delta(px(ROW_H)).y);
        self.scroll -= dy;
        self.clamp_scroll();
        cx.notify();
    }

    // Pieces of the window.

    fn sidebar(&self, c: Colors, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let mut cards: Vec<AnyElement> = Vec::new();
        for (i, s) in self.sheets.iter().enumerate() {
            let selected = s.id == self.current;
            let (name, lines) = if selected {
                // The open sheet shows what is typed now, not what was last saved.
                let lines = self.ed.lines();
                let title = babbage_store::title_of(lines);
                let count = babbage_store::count_lines(lines);
                let name = if s.named {
                    s.id.to_string()
                } else {
                    title.unwrap_or_else(|| "Empty".to_owned())
                };
                (name, count)
            } else {
                (s.display_name(), s.lines)
            };
            let name = match (&self.rename, selected) {
                (Some(r), true) => format!("{}|", r.text),
                _ => name,
            };
            let count = match lines {
                0 => "No lines".to_owned(),
                1 => "1 line".to_owned(),
                n => format!("{n} lines"),
            };
            let card = div()
                .id(("sheet", i))
                .mx(px(8.))
                .px(px(12.))
                .h(px(CARD_H))
                .rounded(px(8.))
                .flex()
                .flex_col()
                .justify_center()
                .gap(px(2.))
                .when(selected, |d| d.bg(c.raised))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, ev: &MouseDownEvent, window, cx| {
                        window.focus(&this.focus);
                        let id = this.sheets[i].id.clone();
                        this.open_sheet(id, cx);
                        // A double click renames the sheet it opened.
                        if ev.click_count >= 2 {
                            this.start_rename(cx);
                        }
                    }),
                )
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, _, window, cx| {
                        window.focus(&this.focus);
                        let id = this.sheets[i].id.clone();
                        this.open_sheet(id.clone(), cx);
                        this.menu = Some(id);
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .text_size(px(16.))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis()
                        .child(SharedString::from(name)),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .justify_between()
                        .text_size(px(12.5))
                        .text_color(c.muted)
                        .child(SharedString::from(describe_time(s.modified)))
                        .child(SharedString::from(count)),
                );
            cards.push(card.into_any_element());
            if self.menu.as_ref() == Some(&s.id) {
                for (row, label) in MENU_ROWS.iter().enumerate() {
                    cards.push(
                        div()
                            .id(("sheet-menu", row))
                            .mx(px(20.))
                            .px(px(12.))
                            .h(px(MENU_ROW_H))
                            .flex()
                            .items_center()
                            .rounded(px(6.))
                            .text_size(px(13.))
                            .bg(c.raised)
                            .hover(|d| d.text_color(c.warn))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, window, cx| {
                                    window.focus(&this.focus);
                                    this.menu = None;
                                    match row {
                                        0 => this.start_rename(cx),
                                        _ => this.delete_current(cx),
                                    }
                                    cx.notify();
                                }),
                            )
                            .child(*label)
                            .into_any_element(),
                    );
                }
            }
        }
        div()
            .w(px(SIDEBAR_W))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .bg(c.sidebar)
            .child(
                div()
                    .mx(px(8.))
                    .mt(px(14.))
                    .mb(px(10.))
                    .h(px(32.))
                    .flex()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .pl(px(8.))
                            .text_size(px(18.))
                            .font_weight(FontWeight::BOLD)
                            .text_color(c.text)
                            .child("Babbage"),
                    )
                    .child(
                        div()
                            .id("new-sheet")
                            .h(px(32.))
                            .px(px(10.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .rounded(px(8.))
                            .text_size(px(14.))
                            .text_color(c.muted)
                            .hover(|d| d.bg(c.raised))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, window, cx| {
                                    window.focus(&this.focus);
                                    this.new_sheet(cx);
                                }),
                            )
                            .child(
                                div()
                                    .w(px(18.))
                                    .h(px(18.))
                                    .rounded(px(9.))
                                    .bg(c.muted)
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_color(c.sidebar)
                                    .text_size(px(15.))
                                    .font_weight(FontWeight::BOLD)
                                    .child("+"),
                            )
                            .child("Sheet"),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .children(cards),
            )
            .child(
                div()
                    .mx(px(8.))
                    .mb(px(10.))
                    .pt(px(10.))
                    .border_t_1()
                    .border_color(c.line)
                    .child(
                        div()
                            .id("folder-pill")
                            .h(px(36.))
                            .px(px(12.))
                            .rounded(px(8.))
                            .bg(c.raised)
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .text_size(px(14.))
                            .child(
                                div()
                                    .w(px(16.))
                                    .h(px(13.))
                                    .rounded(px(3.))
                                    .border_2()
                                    .border_color(c.muted),
                            )
                            .child("General"),
                    ),
            )
            .when_some(self.notice.clone(), |d, n| {
                d.child(
                    div()
                        .px(px(14.))
                        .py(px(10.))
                        .text_size(px(12.))
                        .text_color(c.warn)
                        .child(SharedString::from(n)),
                )
            })
    }

    fn total_chip(&self, c: Colors) -> Option<impl IntoElement + use<>> {
        let t = self.computed.total.as_ref()?;
        let (value, failed) = match (&t.text, &t.error) {
            (Some(v), _) => (format!("{}{v}", if t.exact { "" } else { "≈ " }), false),
            (None, Some(e)) => (e.clone(), true),
            _ => return None,
        };
        Some(
            div()
                .absolute()
                .right(px(22.))
                .bottom(px(16.))
                .px(px(14.))
                .py(px(7.))
                .rounded(px(9.))
                .bg(c.raised)
                .border_1()
                .border_color(c.line)
                .flex()
                .gap(px(8.))
                .items_baseline()
                .text_size(px(15.))
                .child(div().text_color(c.muted).text_size(px(13.)).child("Total"))
                .child(
                    div()
                        .font_weight(FontWeight::MEDIUM)
                        .when(failed, |d| d.text_color(c.warn).text_size(px(13.)))
                        .child(SharedString::from(value)),
                )
                .when(self.computed.left_out > 0, |d| {
                    let n = self.computed.left_out;
                    d.child(
                        div()
                            .text_color(c.warn)
                            .text_size(px(12.))
                            .child(SharedString::from(format!(
                                "{n} {} left out",
                                if n == 1 { "line" } else { "lines" }
                            ))),
                    )
                }),
        )
    }
}

impl Render for SheetView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = *cx.global::<Colors>();
        let entity = cx.entity();
        let sidebar = if self.sidebar_open {
            Some(self.sidebar(c, cx))
        } else {
            None
        };
        let chip = self.total_chip(c);
        let toolbar = self.toolbar(c, cx);
        let panel = self.panel_view(c, cx);
        let answer_menu = self.answer_menu_view(c, cx);
        let completion = self.completion_view(c, cx);
        let toast = self.rename_toast(c, cx);
        let tip = self.hover_var.clone().map(|t| {
            div()
                .absolute()
                .left(px(t.x))
                .top(px(t.y))
                .px(px(12.))
                .py(px(6.))
                .rounded(px(10.))
                .bg(c.raised)
                .border_1()
                .border_color(c.line)
                .text_size(px(14.))
                .child(SharedString::from(t.value))
        });
        div()
            .key_context("Babbage")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .size_full()
            .relative()
            .flex()
            .flex_row()
            .bg(c.background)
            .text_color(c.text)
            .font_family(SharedString::from(appprefs::font_family()))
            .text_size(px(15.))
            .children(sidebar)
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(toolbar)
                    .child(
                        div()
                            .flex_1()
                            .min_h(px(0.))
                            .w_full()
                            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
                            .on_mouse_move(cx.listener(Self::on_mouse_move))
                            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
                            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
                            .on_scroll_wheel(cx.listener(Self::on_scroll))
                            .child(crate::element::SheetElement { view: entity }),
                    ),
            )
            .children(chip)
            .children(panel)
            .children(answer_menu)
            .children(completion)
            .children(toast)
            .children(tip)
    }
}

// The input method.

impl SheetView {
    fn marked_utf16(&self) -> Option<Range<usize>> {
        let m = self.ed.marked?;
        Some(self.ed.utf16_of(Pos::new(m.line, m.start))..self.ed.utf16_of(Pos::new(m.line, m.end)))
    }
}

impl EntityInputHandler for SheetView {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let (a, b) = self.ed.range_of_utf16(&range);
        adjusted.replace(self.ed.utf16_of(a)..self.ed.utf16_of(b));
        Some(self.ed.text_between(a, b))
    }

    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let (a, b) = self.ed.selection();
        Some(UTF16Selection {
            range: self.ed.utf16_of(a)..self.ed.utf16_of(b),
            reversed: self.ed.head() < self.ed.anchor(),
        })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked_utf16()
    }

    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        // The composition is dropped, not typed, when something else (a click) ends it.
        if let Some(m) = self.ed.marked.take() {
            self.ed.replace(
                Pos::new(m.line, m.start),
                Pos::new(m.line, m.end),
                "",
                Kind::Other,
            );
            self.edited(cx);
        }
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (a, b) = match (range, self.ed.marked) {
            (Some(r), _) => self.ed.range_of_utf16(&r),
            (None, Some(m)) => (Pos::new(m.line, m.start), Pos::new(m.line, m.end)),
            (None, None) => self.ed.selection(),
        };
        self.ed.marked = None;
        self.ed.replace(a, b, text, Kind::Typing);
        if appprefs::group_as_you_type()
            && text
                .chars()
                .all(|c| c.is_ascii_digit() || c == '.' || c == ',')
        {
            self.regroup_caret();
        }
        self.edited(cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (a, b) = match (range, self.ed.marked) {
            (Some(r), _) => self.ed.range_of_utf16(&r),
            (None, Some(m)) => (Pos::new(m.line, m.start), Pos::new(m.line, m.end)),
            (None, None) => self.ed.selection(),
        };
        // A composition is one line of text; a line break in it would end it, so it is cut.
        let text = text.split(['\n', '\r']).next().unwrap_or("");
        self.ed.replace(a, b, text, Kind::Other);
        if !text.is_empty() {
            self.ed.marked = Some(Marked {
                line: a.line,
                start: a.col,
                end: a.col + text.len(),
            });
            if let Some(sel) = selected {
                let base = a.col;
                let to_byte = |u: usize| {
                    let mut units = 0;
                    for (i, ch) in text.char_indices() {
                        if units >= u {
                            return base + i;
                        }
                        units += ch.len_utf16();
                    }
                    base + text.len()
                };
                self.ed
                    .set_caret(Pos::new(a.line, to_byte(sel.start)), false);
                self.ed.set_caret(Pos::new(a.line, to_byte(sel.end)), true);
            }
        }
        self.recompute();
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        _: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let l = self.layout.as_ref()?;
        let (a, b) = self.ed.range_of_utf16(&range);
        let shown = |p: Pos| {
            let sh = self.shown_line(p.line);
            let col = sh.to_shown(p.col);
            (col, sh.text.len())
        };
        let row_of = |p: Pos| self.wrap.row_containing(p.line, shown(p).0);
        let x_of = |p: Pos| match l.shaped.get(p.line).and_then(Option::as_ref) {
            Some(s) if p.col > 0 => {
                let (col, len) = shown(p);
                let start = self.wrap.row_span(p.line, row_of(p), len).0;
                s.x_for_index(col) - s.x_for_index(start)
            }
            _ => px(0.),
        };
        let y = l.top + px((self.wrap.top_row(a.line) + row_of(a)) as f32 * ROW_H);
        let x0 = l.text_x + x_of(a);
        let x1 = if a.line == b.line && row_of(a) == row_of(b) {
            l.text_x + x_of(b)
        } else {
            x0
        };
        Some(Bounds::new(
            point(x0, y),
            size((x1 - x0).max(px(2.)), px(ROW_H)),
        ))
    }

    fn character_index_for_point(
        &mut self,
        p: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        let (pos, _) = self.position_at(p)?;
        Some(self.ed.utf16_of(pos))
    }
}

// Painting helpers shared with the element.

/// A text run for `len` bytes.
fn run(
    len: usize,
    font: &gpui::Font,
    color: Hsla,
    bold: bool,
    underline: Option<UnderlineStyle>,
) -> TextRun {
    let mut f = font.clone();
    if bold {
        f.weight = FontWeight::BOLD;
    }
    TextRun {
        len,
        font: f,
        color,
        background_color: None,
        underline,
        strikethrough: None,
    }
}

pub fn class_color(c: &Colors, class: Class) -> Hsla {
    match class {
        Class::Plain => c.text,
        Class::Number => c.number,
        Class::Unit => c.unit,
        Class::Currency => c.currency,
        Class::LineRef => c.chip_text,
        Class::Variable => c.variable,
        Class::Comment => c.comment,
        Class::Heading => c.text,
        Class::Ignored => c.ignored,
        Class::Unclear => c.ignored,
        Class::Warn => c.ignored,
    }
}

/// The runs that draw `line`, split further at the input method's composition so it is underlined.
pub fn runs_for(
    line: &str,
    sp: &[Span],
    marked: Option<(usize, usize)>,
    font: &gpui::Font,
    c: &Colors,
) -> Vec<TextRun> {
    let mut cuts: Vec<usize> = vec![0, line.len()];
    for s in sp {
        cuts.push(s.range.start);
        cuts.push(s.range.end);
    }
    if let Some((a, b)) = marked {
        cuts.push(a.min(line.len()));
        cuts.push(b.min(line.len()));
    }
    cuts.sort_unstable();
    cuts.dedup();
    let mut out = Vec::new();
    for w in cuts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let span = sp.iter().find(|s| s.range.start <= a && b <= s.range.end);
        let (color, bold, class) = match span {
            Some(s) => (class_color(c, s.class), s.bold, s.class),
            None => (c.text, false, Class::Plain),
        };
        let in_marked = marked.is_some_and(|(m0, m1)| a >= m0 && b <= m1);
        let underline = if in_marked {
            Some(UnderlineStyle {
                thickness: px(1.),
                color: Some(color),
                wavy: false,
            })
        } else if class == Class::Unclear {
            // Faint and plain: the word was skipped, and the line may not mean what it says.
            Some(UnderlineStyle {
                thickness: px(1.),
                color: Some(c.ignored),
                wavy: false,
            })
        } else if class == Class::Warn {
            Some(UnderlineStyle {
                thickness: px(1.5),
                color: Some(c.warn),
                wavy: true,
            })
        } else {
            None
        };
        out.push(run(b - a, font, color, bold, underline));
    }
    out
}

pub fn answer_min_w() -> f32 {
    ANSWER_MIN_W
}

impl SheetView {
    pub fn answer_for(&self, line: usize) -> Option<&Answer> {
        self.computed.answers.get(line)
    }

    /// Called by the paint once the rows are known: reveals the caret if a change asked for it, and keeps the
    /// scroll inside the sheet.
    pub fn settle_scroll(&mut self) {
        if std::mem::take(&mut self.reveal) {
            self.scroll_to_caret();
        }
        self.clamp_scroll();
    }

    pub fn scroll(&self) -> f32 {
        self.scroll
    }

    pub fn focus_handle_ref(&self) -> &FocusHandle {
        &self.focus
    }

    pub fn set_layout(&mut self, l: Layout) {
        self.layout = Some(l);
    }
}
