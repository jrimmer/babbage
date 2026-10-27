//! The test hook (`--control <socket>`): newline-delimited JSON on a unix socket that reports what the
//! window shows. It never edits anything; checks type with a real keyboard and read the result here.
//!
//! Requests: `{"cmd":"state"}` and `{"cmd":"quit"}`.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use babbage_core::LineKind;
use futures::StreamExt;
use gpui::{App, Entity, px};
use serde_json::{Value, json};

use crate::view::{ROW_H, SheetView};

type Job = (String, mpsc::Sender<String>);

/// Binds the socket (mode 0600, replacing one a dead process left) and serves it.
pub fn serve(path: PathBuf, view: Entity<SheetView>, cx: &mut App) -> std::io::Result<()> {
    let listener = bind(&path)?;
    let (tx, mut rx) = futures::channel::mpsc::unbounded::<Job>();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let tx = tx.clone();
            std::thread::spawn(move || connection(stream, tx));
        }
    });
    cx.spawn(async move |cx| {
        while let Some((line, reply)) = rx.next().await {
            let out = cx.update(|cx| handle(&line, &view, cx));
            let _ = reply.send(out.unwrap_or_else(|e| json!({"error": e.to_string()}).to_string()));
        }
    })
    .detach();
    Ok(())
}

fn bind(path: &Path) -> std::io::Result<UnixListener> {
    use std::os::unix::fs::FileTypeExt;
    if let Ok(meta) = path.symlink_metadata() {
        if !meta.file_type().is_socket() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!(
                    "{} exists and is not a socket, so it is left alone",
                    path.display()
                ),
            ));
        }
        std::fs::remove_file(path)?;
    }
    let l = UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(l)
}

fn connection(stream: UnixStream, tx: futures::channel::mpsc::UnboundedSender<Job>) {
    let Ok(mut out) = stream.try_clone() else {
        return;
    };
    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else { return };
        let (reply, wait) = mpsc::channel();
        if tx.unbounded_send((line, reply)).is_err() {
            return;
        }
        let Ok(answer) = wait.recv() else { return };
        if writeln!(out, "{answer}").is_err() {
            return;
        }
    }
}

fn handle(line: &str, view: &Entity<SheetView>, cx: &mut App) -> String {
    let req: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(e) => return json!({"error": format!("bad request: {e}")}).to_string(),
    };
    match req["cmd"].as_str() {
        Some("state") => report(view.read(cx)).to_string(),
        Some("quit") => {
            cx.quit();
            json!({"ok": true}).to_string()
        }
        other => json!({"error": format!("unknown command {other:?}")}).to_string(),
    }
}

fn kind_name(k: &LineKind) -> &'static str {
    match k {
        LineKind::Blank => "blank",
        LineKind::Heading(_) => "heading",
        LineKind::Divider => "divider",
        LineKind::Comment => "comment",
        LineKind::Text => "text",
        LineKind::Value(_) => "value",
        LineKind::Error(_) => "error",
    }
}

/// What the window shows, in logical pixels of the window (multiply by `scale` for screen pixels).
pub fn report(v: &SheetView) -> Value {
    let layout = v.layout.as_ref();
    let lines: Vec<Value> = (0..v.ed.len())
        .map(|i| {
            let text = v.ed.line(i);
            let result = v.computed.results.get(i);
            let ans = v.computed.answers.get(i);
            let shaped = layout.and_then(|l| l.shaped.get(i)).and_then(Option::as_ref);
            let names: Vec<String> = v
                .computed
                .names
                .get(i)
                .map(|n| n.iter().map(|(name, _)| name.clone()).collect())
                .unwrap_or_default();
            let typed = crate::syntax::spans_in(text, result, &names);
            let sh = v.shown_line(i);
            let sp: Vec<Value> = typed
                .into_iter()
                .map(|s| {
                    let (x0, x1) = match (layout, shaped) {
                        (Some(l), Some(line)) => (
                            Some(f32::from(l.text_x + line.x_for_index(sh.to_shown(s.range.start)))),
                            Some(f32::from(l.text_x + line.x_for_index(sh.to_shown(s.range.end)))),
                        ),
                        _ => (None, None),
                    };
                    json!({
                        "text": &text[s.range.clone()],
                        "class": s.class.name(),
                        "bold": s.bold,
                        "x0": x0,
                        "x1": x1,
                    })
                })
                .collect();
            json!({
                "text": text,
                "shown": sh.text,
                "format": v.ed.format(i).to_text(),
                "subtotal": result.is_some_and(|r| r.subtotal),
                "kind": result.map_or("blank", |r| kind_name(&r.kind)),
                "answer": ans.and_then(|a| a.text.clone()),
                "exact": ans.is_none_or(|a| a.exact),
                "error": ans.and_then(|a| a.error.clone()),
                "ignored": result.map(|r| r.ignored.iter().map(|x| x.text.clone()).collect::<Vec<_>>()).unwrap_or_default(),
                "warnings": result.map(|r| r.warnings.iter().map(|x| x.text.clone()).collect::<Vec<_>>()).unwrap_or_default(),
                "y": layout.map(|l| f32::from(l.top + px(v.wrap.top_row(i) as f32 * ROW_H))),
                "h": v.wrap.rows_of(i) as f32 * ROW_H,
                "rows": v.wrap.rows_of(i),
                "row_h": ROW_H,
                "row_starts": (0..v.wrap.rows_of(i)).map(|k| v.wrap.row_span(i, k, sh.text.len()).0).collect::<Vec<_>>(),
                "spans": sp,
            })
        })
        .collect();
    let sheets: Vec<Value> = v
        .sheets
        .iter()
        .map(|s| {
            let current = s.id == v.current;
            // The open sheet reports what is typed, like the card shows.
            let (title, count) = if current {
                (babbage_store::title_of(v.ed.lines()), babbage_store::count_lines(v.ed.lines()))
            } else {
                (s.title.clone(), s.lines)
            };
            json!({
                "id": s.id.as_str(),
                "name": if s.named { s.id.to_string() } else { title.clone().unwrap_or_else(|| "Empty".to_owned()) },
                "title": title,
                "named": s.named,
                "lines": count,
                "time": crate::timefmt::describe_time(s.modified),
                "current": current,
            })
        })
        .collect();
    let (head, sel) = (v.ed.head(), v.ed.selected_text());
    json!({
        "current": v.current.as_str(),
        "sheets": sheets,
        "lines": lines,
        "cursor": {"line": head.line, "col": head.col},
        "selection": sel,
        "composition": v.ed.marked_text(),
        "total": v.computed.total.as_ref().map(|t| json!({"text": t.text, "exact": t.exact, "error": t.error, "left_out": v.computed.left_out})),
        "tzdb": babbage_core::settings::scoped(crate::prefs::current(), babbage_core::zones::current_release),
        "scroll": v.scroll(),
        "panel": v.panel.map(crate::chrome::Panel::name),
        "toolbar": layout.map(|l| {
            let right = f32::from(l.bounds.right());
            let top = f32::from(l.bounds.top()) - crate::chrome::TOOLBAR_H;
            let buttons: Vec<Value> = crate::chrome::toolbar_buttons()
                .iter()
                .map(|(name, r)| json!({"name": name, "x": right + r.x, "y": top + r.y, "w": r.w, "h": r.h}))
                .collect();
            let (px_, py) = crate::chrome::panel_origin();
            let items: Vec<Value> = crate::chrome::settings_items()
                .iter()
                .map(|(id, r)| json!({"id": id, "x": right + px_ + r.x, "y": top + crate::chrome::TOOLBAR_H + py + r.y, "w": r.w, "h": r.h}))
                .collect();
            json!({"h": crate::chrome::TOOLBAR_H, "buttons": buttons, "settings_items": items,
                   "panel_x": right + px_, "panel_y": top + crate::chrome::TOOLBAR_H + py})
        }),
        "answer_menu": v.answer_menu.and_then(|line| {
            let l = layout?;
            let row_y = f32::from(l.top) + v.wrap.top_row(line) as f32 * ROW_H;
            let (mx, my) = crate::answer_menu::origin(
                f32::from(l.bounds.right()),
                row_y,
                f32::from(l.bounds.top() + l.bounds.size.height),
            );
            let items: Vec<Value> = crate::answer_menu::items()
                .iter()
                .map(|(id, r)| json!({"id": id, "x": mx + r.x, "y": my + r.y, "w": r.w, "h": r.h}))
                .collect();
            Some(json!({"line": line, "x": mx, "y": my, "w": crate::answer_menu::MENU_W, "h": crate::answer_menu::MENU_H, "items": items}))
        }),
        "more_button": layout.map(|l| json!({"from_right": crate::answer_menu::MORE_FROM_RIGHT, "radius": crate::answer_menu::MORE_RADIUS, "right": f32::from(l.bounds.right())})),
        "hover_line": v.hover_line,
        "scrubbing": v.is_scrubbing(),
        "tooltip": v.hover_var.as_ref().map(|t| json!({"name": t.name, "value": t.value, "x": t.x, "y": t.y})),
        "completion": v.complete.as_ref().map(|c| {
            let at = v.completion_origin();
            json!({"line": c.line, "candidates": c.candidates, "selected": c.selected,
                   "x": at.map(|a| a.0), "y": at.map(|a| a.1), "row_h": crate::autocomplete::COMPLETION_ROW_H})
        }),
        "number_format": format!("{:?}", crate::prefs::current().number_format),
        "work_hours": crate::prefs::current().work_hours_per_day,
        "settings_path": crate::prefs::settings_path().map(|p| p.display().to_string()),
        "total_rows": v.wrap.total_rows(v.ed.len()),
        "text_w": layout.map(|l| f32::from(l.text_w)),
        "sidebar_open": v.sidebar_open,
        "answer_focus": v.answer_focus,
        "rename": v.rename.as_ref().map(|r| r.text.clone()),
        "menu": v.menu.as_ref().map(|m| m.as_str()),
        "sidebar": {
            "top": crate::view::SIDEBAR_TOP,
            "card_h": crate::view::CARD_H,
            "menu_row_h": crate::view::MENU_ROW_H,
            "menu_rows": crate::view::MENU_ROWS,
            "width": crate::view::SIDEBAR_W,
        },
        "notice": v.notice,
        "laid_out": layout.is_some(),
        "scale": layout.map(|l| l.scale),
        "text_x": layout.map(|l| f32::from(l.text_x)),
        "answer_x": layout.map(|l| f32::from(l.answer_x)),
        "answer_w": layout.map(|l| f32::from(l.answer_w)),
        "window": layout.map(|l| json!({
            "x": f32::from(l.bounds.origin.x), "y": f32::from(l.bounds.origin.y),
            "w": f32::from(l.bounds.size.width), "h": f32::from(l.bounds.size.height)})),
    })
}
