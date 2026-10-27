//! babbage: a notepad calculator.
//!
//! ```text
//! babbage [--control <socket>]
//! babbage calc [options] EXPRESSION...      (also run as `bbg`; see `cli.rs`)
//! ```
//!
//! Sheets live in `$BABBAGE_DATA_DIR` (default `~/.local/share/babbage/sheets`). `--control` serves the
//! test hook described in `control.rs`.

mod answer_menu;
mod appprefs;
mod autocomplete;
mod chrome;
mod cli;
mod computed;
mod control;
mod editor_model;
mod element;
mod grouping;
mod linefmt;
mod prefs;
mod rename;
mod scrub;
mod settings_ui;
mod shown;
mod syntax;
mod theme;
mod timefmt;
mod update_ui;
mod updates;
mod view;
mod wrap;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, bail};
use babbage_store::{Autosaver, SheetBook};
use gpui::{AppContext, Bounds, TitlebarOptions, WindowBounds, WindowOptions, px, size};

use view::{Init, SAVE_DELAY_MS, SheetView};

const USAGE: &str = "usage: babbage [--control <socket>]\n       babbage calc [options] EXPRESSION... (see babbage calc --help)";

fn main() -> anyhow::Result<()> {
    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if let Some(rest) = cli::command_args(&argv) {
        let rest: Vec<String> = rest
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        std::process::exit(cli::run(&rest));
    }
    let mut control = None;
    let mut args = argv.into_iter().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--control") => {
                control = Some(PathBuf::from(
                    args.next().context("--control needs a path")?,
                ));
            }
            Some("-h" | "--help") => {
                println!("{USAGE}");
                return Ok(());
            }
            Some("-V" | "--version") => {
                println!("babbage {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            _ => bail!("unexpected argument {}\n{USAGE}", arg.to_string_lossy()),
        }
    }

    let book = SheetBook::open_default().context("open the sheetbook")?;
    let init = Init::load(&book).context("read the sheets")?;
    let saver = Arc::new(Autosaver::start(
        SheetBook::open(book.dir()).context("open the sheetbook for saving")?,
        Duration::from_millis(SAVE_DELAY_MS),
    ));
    let flush = saver.clone();

    gpui::Application::new().run(move |cx| {
        theme::install(cx);
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(1200.), px(800.)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("Babbage".into()),
                ..Default::default()
            }),
            app_id: Some("babbage".into()),
            ..Default::default()
        };
        let opened = cx.open_window(options, |window, cx| {
            cx.new(|cx| SheetView::new(init, book, saver, window, cx))
        });
        match opened {
            Ok(handle) => {
                if let Some(path) = control {
                    let view = handle.entity(cx);
                    match view {
                        Ok(view) => {
                            if let Err(e) = control::serve(path.clone(), view, cx) {
                                eprintln!("cannot serve {}: {e}", path.display());
                                std::process::exit(1);
                            }
                        }
                        Err(e) => {
                            eprintln!("cannot reach the window: {e}");
                            cx.quit();
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("cannot open a window: {e}");
                cx.quit();
            }
        }
    });
    // Whatever was typed in the last moments is written before the process ends.
    flush.flush();
    Ok(())
}
