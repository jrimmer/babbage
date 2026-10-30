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
mod completions;
mod computed;
mod control;
mod editor_model;
mod element;
mod grouping;
mod linefmt;
mod omarchy_setup;
mod prefs;
mod quick;
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

/// The app's own pictures, built into the program.
struct Assets;

impl gpui::AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        Ok(match path {
            "gear.svg" => Some(include_bytes!("../assets/gear.svg").as_slice().into()),
            _ => None,
        })
    }

    fn list(&self, _path: &str) -> anyhow::Result<Vec<gpui::SharedString>> {
        Ok(vec!["gear.svg".into()])
    }
}

const USAGE: &str = "usage: babbage [--quick] [--control <socket>]\n       babbage calc [options] EXPRESSION... (see babbage calc --help)\n       babbage setup omarchy [--remove] [--key KEY]\n       babbage --print-icon\n\n  --quick       a small window for one calculation; brings the open window forward on Hyprland\n  setup omarchy add a quick-calculation key and a menu entry to Omarchy (--remove undoes it)\n  --print-icon  print the app icon (SVG)";

/// The app icon, which the installer saves for the launcher.
const ICON: &str = include_str!("../../../packaging/babbage.svg");

fn main() -> anyhow::Result<()> {
    let argv: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if let Some(rest) = cli::command_args(&argv) {
        let rest: Vec<String> = rest
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        std::process::exit(cli::run(&rest));
    }
    if argv.get(1).and_then(|a| a.to_str()) == Some("setup") {
        let rest: Vec<String> = argv[2..]
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        std::process::exit(omarchy_setup::main(&rest));
    }
    let mut control = None;
    let mut quick_window = false;
    let mut args = argv.into_iter().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--control") => {
                control = Some(PathBuf::from(
                    args.next().context("--control needs a path")?,
                ));
            }
            Some("--quick") => quick_window = true,
            Some("--print-icon") => {
                print!("{ICON}");
                return Ok(());
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

    if quick_window && quick::focus_running("babbage") {
        return Ok(());
    }
    let book = SheetBook::open_default().context("open the sheetbook")?;
    let init = Init::load(&book).context("read the sheets")?;
    let saver = Arc::new(Autosaver::start(
        SheetBook::open(book.dir()).context("open the sheetbook for saving")?,
        Duration::from_millis(SAVE_DELAY_MS),
    ));
    let flush = saver.clone();

    gpui::Application::new().with_assets(Assets).run(move |cx| {
        theme::install(cx);
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let (title, width, height) = if quick_window {
            (quick::TITLE, quick::WIDTH, quick::HEIGHT)
        } else {
            ("Babbage", 1200., 800.)
        };
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(width), px(height)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some(title.into()),
                ..Default::default()
            }),
            app_id: Some("babbage".into()),
            ..Default::default()
        };
        let opened = cx.open_window(options, |window, cx| {
            // The titlebar option is not sent to a Wayland compositor, and a window rule matches on the title.
            window.set_window_title(title);
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
