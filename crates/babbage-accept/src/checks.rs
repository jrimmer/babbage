//! The acceptance checks. Each one starts the real binary on a temporary data folder, types real
//! keystrokes into its window, and reads the result back from the screen and from the app's state hook.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, bail, ensure};
use serde_json::Value;

use crate::harness::{App, Harness, Shot, luminance};

pub type Check = fn(&mut Harness) -> anyhow::Result<()>;

pub const CHECKS: &[(&str, Check)] = &[
    // Command mode and the installer: no window, so they need no compositor.
    ("cli-expression", cli_expression),
    ("cli-errors", cli_errors),
    ("cli-sheet-input", cli_sheet_input),
    ("cli-answers", cli_answers),
    ("cli-total", cli_total),
    ("cli-json", cli_json),
    ("cli-saved", cli_saved),
    ("cli-repeating", cli_repeating),
    ("cli-bbg-name", cli_bbg_name),
    ("cli-completions", cli_completions),
    ("install-script", install_script),
    ("sum", sum_shows_39),
    ("sheet-overview", sheet_overview),
    ("error-line", error_line),
    ("ignored-dim", ignored_dim),
    ("unclear-words", unclear_words),
    ("typo-hint", typo_hint),
    ("total", total_chip),
    ("sheets", sheets_lifecycle),
    ("sidebar-mouse", sidebar_mouse),
    ("undo-per-sheet", undo_per_sheet),
    ("autosave-sigkill", autosave_sigkill),
    ("keys", keyboard_features),
    ("quit-saves", quit_saves),
    ("local-time", local_time),
    ("tz-database", tz_database),
    ("sales-tax", sales_tax),
    ("sheet-functions", sheet_functions),
    ("everyday-lines", everyday_lines),
    ("number-format", number_format),
    ("number-format-system", number_format_system),
    ("scrolling", scrolling),
    ("mouse-editing", mouse_editing),
    ("drag-select", drag_select),
    ("wheel-scroll", wheel_scroll),
    ("comment-toggle", comment_toggle),
    ("export", export_sheet),
    ("resize", resize_window),
    ("ime-composition", ime_composition),
    ("clipboard-lines", clipboard_lines),
    ("wrapping", wrapping),
    ("line-chips", line_chips),
    ("toolbar", toolbar),
    ("toolbar-follows-resize", toolbar_follows_resize),
    ("settings-window", settings_window),
    ("settings-editing", settings_editing),
    ("settings-styling", settings_styling),
    ("answer-menu", answer_menu),
    ("answer-repeating", answer_repeating),
    ("answer-formats-follow-text", answer_formats_follow_text),
    ("showcase", showcase),
    ("guide-sheets", guide_sheets),
    ("guide-screens", guide_screens),
    ("updates-interval", updates_interval),
    ("updates-install", updates_install),
    ("variables", variables),
    ("subtotal-style", subtotal_style),
    ("number-scrub", number_scrub),
    ("light-theme", light_theme),
    ("no-real-desktop", no_real_desktop),
];

const T: Duration = Duration::from_secs(15);

/// Maps window coordinates reported by the app to pixels of the screenshot.
struct Screen {
    ox: f32,
    oy: f32,
    scale: f32,
}

impl Screen {
    fn of(h: &Harness, app: &App, s: &Value) -> anyhow::Result<Screen> {
        let (ox, oy) = h.window_origin(app)?;
        let scale = s["scale"].as_f64().context("the app has not painted yet")? as f32;
        Ok(Screen { ox, oy, scale })
    }

    /// The brightest pixel in a rectangle given in window coordinates.
    fn brightest(&self, shot: &Shot, x0: f32, y0: f32, x1: f32, y1: f32) -> Option<[u8; 3]> {
        shot.brightest(
            self.ox + x0 * self.scale,
            self.oy + y0 * self.scale,
            self.ox + x1 * self.scale,
            self.oy + y1 * self.scale,
        )
    }

    /// The brightest pixel of the answer cell of line `i`.
    fn answer_cell(&self, shot: &Shot, s: &Value, i: usize) -> anyhow::Result<[u8; 3]> {
        let ax = f(&s["answer_x"])?;
        let (y, h) = (f(&s["lines"][i]["y"])?, f(&s["lines"][i]["h"])?);
        let w = f(&s["window"]["w"])?;
        // Skips the column's left edge line and the margin that holds the "≈".
        Ok(self
            .brightest(shot, ax + 28., y + 2., (ax + 300.).min(w - 4.), y + h - 2.)
            .unwrap_or([0, 0, 0]))
    }
}

fn f(v: &Value) -> anyhow::Result<f32> {
    v.as_f64()
        .map(|x| x as f32)
        .with_context(|| format!("not a number: {v}"))
}

fn answer(s: &Value, i: usize) -> Option<&str> {
    s["lines"][i]["answer"].as_str()
}

/// Gives the app a moment to paint what it was last told, so a screenshot shows it.
fn settle() {
    std::thread::sleep(Duration::from_millis(250));
}

fn read_sheet(dir: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(dir.join(format!("{name}.bbg"))).ok()
}

/// Waits until the sheet file on disk has exactly `want`.
fn wait_file(dir: &Path, name: &str, want: &str) -> anyhow::Result<()> {
    let start = std::time::Instant::now();
    loop {
        let got = read_sheet(dir, name);
        if got.as_deref() == Some(want) {
            return Ok(());
        }
        ensure!(
            start.elapsed() < Duration::from_secs(10),
            "{name}.bbg never became {want:?}; it holds {got:?}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn sheet_names(s: &Value) -> Vec<String> {
    s["sheets"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|x| x["name"].as_str().unwrap_or("").to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// The clock the zone checks run on: Saturday 2024-06-15 12:00 UTC.
const FIXED_NOW: &str = "1718452800";

/// Types one expression per line and waits for every answer, returning the final state.
fn answers_for(h: &Harness, app: &mut App, lines: &[(&str, &str)]) -> anyhow::Result<Value> {
    for (i, (text, _)) in lines.iter().enumerate() {
        if i + 1 == lines.len() {
            h.type_text(text)?;
        } else {
            h.type_line(text)?;
        }
    }
    let want: Vec<String> = lines.iter().map(|(_, a)| (*a).to_owned()).collect();
    app.wait_for("every answer", T, |s| {
        want.iter()
            .enumerate()
            .all(|(i, w)| answer(s, i) == Some(w.as_str()))
    })
}

/// The vendored zoneinfo files (IANA 2026e), so zone checks give the same answers on any machine.
fn tzdir(h: &Harness) -> String {
    h.repo
        .join("crates/babbage-core/tests/fixtures/zoneinfo")
        .to_string_lossy()
        .into_owned()
}

fn local_time(h: &mut Harness) -> anyhow::Result<()> {
    let dir = tzdir(h);
    // Los Angeles is on daylight time in June and standard time in January: one fixed offset cannot do both.
    let mut app = h.launch(
        "local-la",
        None,
        &[
            ("TZ", "America/Los_Angeles"),
            ("BABBAGE_NOW", FIXED_NOW),
            ("BABBAGE_TZDIR", &dir),
        ],
    )?;
    let s = answers_for(
        h,
        &mut app,
        &[
            ("now", "5:00 AM"),
            ("3pm in Tokyo", "Tomorrow at 7:00 AM"),
            ("now to timestamp", "1718452800"),
            (
                "January 15, 2024 3pm as iso8601",
                "2024-01-15T15:00:00-08:00",
            ),
            ("June 15, 2024 3pm as iso8601", "2024-06-15T15:00:00-07:00"),
        ],
    )?;
    ensure!(
        answer(&s, 0) != Some("12:00 PM"),
        "the app is showing UTC, not the local zone"
    );
    settle();
    let s = app.state()?;
    let shot = h.shot()?;
    let scr = Screen::of(h, &app, &s)?;
    ensure!(
        luminance(scr.answer_cell(&shot, &s, 0)?) > 150.,
        "no answer text on screen for the first line"
    );
    h.save_shot(&h.logs.join("local-time.png"))?;
    app.finish()?;

    // A zone with no daylight saving and a half-hour offset.
    let mut app = h.launch(
        "local-kolkata",
        None,
        &[
            ("TZ", "Asia/Kolkata"),
            ("BABBAGE_NOW", FIXED_NOW),
            ("BABBAGE_TZDIR", &dir),
        ],
    )?;
    answers_for(h, &mut app, &[("now", "5:30 PM"), ("today", "June 15")])?;
    app.finish()?;

    // Late in the evening in Los Angeles it is already the next day in UTC: `today` must be the local day.
    let mut app = h.launch(
        "local-late",
        None,
        &[
            ("TZ", "America/Los_Angeles"),
            ("BABBAGE_NOW", "1718506800"),
            ("BABBAGE_TZDIR", &dir),
        ],
    )?;
    answers_for(h, &mut app, &[("today", "June 15"), ("now", "8:00 PM")])?;
    app.finish()
}

/// 2026-12-01 12:00 UTC, after the rule changes that came in tzdata releases newer than the bundled 2025b.
const AFTER_RULE_CHANGES: &str = "1796126400";

fn tz_database(h: &mut Harness) -> anyhow::Result<()> {
    let dir = tzdir(h);
    // The machine's files say Vancouver stays on UTC-7 from November 2026; the bundled 2025b says UTC-8.
    let mut app = h.launch(
        "tz-system",
        None,
        &[
            ("TZ", "America/Vancouver"),
            ("BABBAGE_NOW", AFTER_RULE_CHANGES),
            ("BABBAGE_TZDIR", &dir),
        ],
    )?;
    let s = answers_for(
        h,
        &mut app,
        &[
            ("now", "5:00 AM"),
            ("noon in Casablanca", "7:00 PM"),
            ("time in Paris", "1:00 PM"),
        ],
    )?;
    ensure!(
        s["tzdb"].as_str() == Some("2026e"),
        "the app should report the machine's release, got {:?}",
        s["tzdb"]
    );
    app.finish()?;

    let config = h.logs.join("bundled-zones.settings");
    std::fs::write(&config, "tz_source = bundled\n")?;
    let config = config.to_string_lossy().into_owned();
    let mut app = h.launch(
        "tz-bundled",
        None,
        &[
            ("TZ", "America/Vancouver"),
            ("BABBAGE_NOW", AFTER_RULE_CHANGES),
            ("BABBAGE_TZDIR", &dir),
            ("BABBAGE_CONFIG_FILE", &config),
        ],
    )?;
    let s = answers_for(h, &mut app, &[("now", "4:00 AM")])?;
    ensure!(
        s["tzdb"].as_str() == Some("2025b"),
        "the bundled source should report 2025b, got {:?}",
        s["tzdb"]
    );
    app.finish()
}

fn sales_tax(h: &mut Harness) -> anyhow::Result<()> {
    let config = h.logs.join("vat.settings");
    std::fs::write(&config, "tax = VAT 15%\n")?;
    let config = config.to_string_lossy().into_owned();
    let mut app = h.launch("tax", None, &[("BABBAGE_CONFIG_FILE", &config)])?;
    answers_for(
        h,
        &mut app,
        &[
            ("$300 + VAT", "$345.00"),
            ("VAT on $300", "$45.00"),
            ("$300 - VAT", "$260.87"),
            ("VAT in $300", "$39.13"),
        ],
    )?;
    app.finish()?;
    // With no tax set, the same line is an error and not a guess.
    let mut app = h.launch("no-tax", None, &[])?;
    h.type_text("$300 + VAT")?;
    app.wait_for("an error for VAT", T, |s| {
        s["lines"][0]["error"].is_string()
    })?;
    app.finish()
}

/// Function calls and layouts must reach the engine: in a sheet, unknown words are dropped, and a function
/// name once went with them (`sqrt(16)` answered 16).
fn sheet_functions(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("functions", None, &[])?;
    let s = answers_for(
        h,
        &mut app,
        &[
            ("sqrt(16)", "4"),
            ("cbrt(343)", "7"),
            ("max(3, 9, 4)", "9"),
            ("round(2.567)", "3"),
            ("2.567 to 1 dp", "2.6"),
            ("5.5 minutes as timespan", "5 min 30 s"),
            ("5.5 minutes as laptime", "00:05:30"),
        ],
    );
    app.finish()?;
    s.map(|_| ())
}

/// Dates, times, zones, forms and money typed into the real app on a fixed clock (Saturday 2024-06-15 12:00 UTC).
fn everyday_lines(h: &mut Harness) -> anyhow::Result<()> {
    let dir = tzdir(h);
    let mut app = h.launch(
        "everyday",
        None,
        &[
            ("TZ", "UTC"),
            ("BABBAGE_NOW", FIXED_NOW),
            ("BABBAGE_TZDIR", &dir),
        ],
    )?;
    let lines = [
        ("June 3 + 3 weeks", "June 24"),
        ("days until Christmas", "193 days"),
        ("Easter 2027", "March 28, 2027"),
        ("workdays in 3 weeks", "15 workdays"),
        ("7:30am to 8:45pm", "13 hours 15 min"),
        ("00:10:00:00 + 00:00:30:00", "00:10:30:00"),
        ("15% of 240", "36"),
        ("3/8 as fraction", "3/8"),
        ("meters in 5 km", "5,000 m"),
        ("9am London in Tokyo", "5:00 PM"),
        ("$300/workday * 3 weeks", "$4,500.00"),
        ("day of the week on March 3, 2025", "Monday"),
        ("Christmas 2026", "December 25, 2026"),
    ];
    let s = answers_for(h, &mut app, &lines);
    app.finish()?;
    s.map(|_| ())
}

/// "System" is opt-in: with no choice made a sheet reads `1.5` as one and a half whatever the language of the session,
/// and a file that says `number_format = system` follows the language.
fn number_format_system(h: &mut Harness) -> anyhow::Result<()> {
    let german = [("LC_ALL", "de_DE.UTF-8")];
    let mut app = h.launch("number-format-default", None, &german)?;
    h.type_text("1.5 + 1")?;
    let s = app.wait_for("the answer", T, |s| answer(s, 0) == Some("2.5"))?;
    ensure!(
        s["number_format"] == "NorthAmerica",
        "{}",
        s["number_format"]
    );
    app.finish()?;

    let config = h.logs.join("number-format-system.settings");
    std::fs::write(&config, "number_format = system\n")?;
    let config = config.to_string_lossy().into_owned();
    let mut app = h.launch(
        "number-format-system",
        None,
        &[("LC_ALL", "de_DE.UTF-8"), ("BABBAGE_CONFIG_FILE", &config)],
    )?;
    h.type_text("1,5 + 1")?;
    let s = app.wait_for("the answer", T, |s| answer(s, 0) == Some("2,5"))?;
    ensure!(
        s["number_format"] == "WesternEurope",
        "{}",
        s["number_format"]
    );
    app.finish()
}

fn number_format(h: &mut Harness) -> anyhow::Result<()> {
    let config = h.logs.join("western-europe.settings");
    std::fs::write(
        &config,
        "number_format = western-europe\nwork_hours = 7\ndollar = AUD\n",
    )?;
    let config = config.to_string_lossy().into_owned();
    let mut app = h.launch(
        "region",
        None,
        &[("BABBAGE_CONFIG_FILE", &config), ("TZ", "UTC")],
    )?;
    let s = answers_for(
        h,
        &mut app,
        &[
            ("1.234,5 + 1", "1.235,5"),
            ("$1.234,5 + $1", "$1.235,50"),
            ("21 h in workdays", "3 workdays"),
            ("1_000 + 1", "1.001"),
            ("$5 + A$5", "$10,00"),
            ("US$5", "US$5,00"),
        ],
    )?;
    ensure!(
        answer(&s, 0) != Some("1,235.5"),
        "the settings file was ignored"
    );
    app.finish()
}

fn sum_shows_39(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("sum", None, &[])?;
    h.type_text("12 + 12 + 15")?;
    app.wait_for("39 on line 1", T, |s| answer(s, 0) == Some("39"))?;
    settle();
    // The screen agrees with the state: the answer cell of line 1 has bright text, the empty line 3 does not.
    let s = app.state()?;
    let shot = h.shot()?;
    let scr = Screen::of(h, &app, &s)?;
    let on = scr.answer_cell(&shot, &s, 0)?;
    ensure!(
        luminance(on) > 150.,
        "no answer text on screen for line 1: {on:?}"
    );
    h.save_shot(&h.logs.join("sum.png"))?;
    app.finish()
}

const SHEET: [(&str, &str); 19] = [
    ("18 + 24 + 36", "78"),
    ("$120 + $85 + $42 + $310 + $64", "$621.00"),
    ("48 × 25", "1,200"),
    ("(600mm × 400mm × 1,200mm) / 8", "36M mm³"),
    ("300mm x 300mm x 300mm", "27M mm³"),
    ("line4 / line5", "1.3333333333"),
    ("$95 × 6", "$570.00"),
    ("72 × 1.45", "104.4"),
    ("64 × 2.25", "144"),
    ("7/16", "0.4375"),
    ("40 × line10", "17.5"),
    ("$28 × 96", "$2,688.00"),
    ("$35 × 12", "$420.00"),
    ("30,000 / 12 / 52", "48.0769230769"),
    ("512 × 8", "4,096"),
    ("512 × 6", "3,072"),
    ("0.81 – .04", "0.77"),
    (".003 ×5", "0.015"),
    ("600,000 / (600,000 + 150,000)", "0.8"),
];

fn sheet_overview(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("sheet-overview", None, &[])?;
    for (i, (line, _)) in SHEET.iter().enumerate() {
        if i + 1 < SHEET.len() {
            h.type_line(line)?;
        } else {
            h.type_text(line)?;
        }
    }
    let s = app.wait_for("all 19 answers", Duration::from_secs(30), |s| {
        SHEET
            .iter()
            .enumerate()
            .all(|(i, (_, want))| answer(s, i) == Some(*want))
    })?;
    for (i, (line, want)) in SHEET.iter().enumerate() {
        ensure!(
            s["lines"][i]["text"] == *line,
            "line {} typed as {}",
            i + 1,
            s["lines"][i]["text"]
        );
        ensure!(answer(&s, i) == Some(*want), "line {}: {line}", i + 1);
    }
    // Line references are chips.
    let spans = s["lines"][5]["spans"].as_array().context("spans")?;
    let chips: Vec<&str> = spans
        .iter()
        .filter(|x| x["class"] == "lineref")
        .filter_map(|x| x["text"].as_str())
        .collect();
    ensure!(chips == ["line4", "line5"], "chips on line 6: {chips:?}");
    // The total chip shows the sheet's total.
    // The sheet mixes dollars and cubic millimetres, so the engine has no single total for it; the chip
    // still shows, with the engine's message. A sheet of one kind shows a value (see `total`).
    ensure!(s["total"].is_object(), "no total chip: {}", s["total"]);
    settle();
    let s = app.state()?;
    let shot = h.shot()?;
    let scr = Screen::of(h, &app, &s)?;
    for i in [0, 9, 18] {
        let p = scr.answer_cell(&shot, &s, i)?;
        ensure!(
            luminance(p) > 150.,
            "no answer text on screen for line {}",
            i + 1
        );
    }
    h.save_shot(&h.doc_shot("sheet-overview.png"))?;
    app.finish()
}

fn error_line(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("error", None, &[])?;
    h.type_line("1 + 1")?;
    h.type_line("1 / 0")?;
    h.type_text("2 + 2")?;
    let s = app.wait_for("the three lines", T, |s| {
        answer(s, 0) == Some("2") && answer(s, 2) == Some("4") && s["lines"][1]["error"].is_string()
    })?;
    ensure!(
        answer(&s, 1).is_none(),
        "the failing line must have no answer"
    );
    let msg = s["lines"][1]["error"].as_str().unwrap_or("");
    ensure!(!msg.is_empty() && msg.len() < 80, "message {msg:?}");
    settle();
    let s = app.state()?;
    let shot = h.shot()?;
    let scr = Screen::of(h, &app, &s)?;
    // The message is in the warning color: more red than blue, unlike the white answers.
    let p = scr.answer_cell(&shot, &s, 1)?;
    ensure!(
        p[0] as i32 > p[2] as i32 + 40,
        "the error text is not in the warning color: {p:?}"
    );
    let ok = scr.answer_cell(&shot, &s, 0)?;
    ensure!(
        (ok[0] as i32 - ok[2] as i32).abs() < 30,
        "the answer beside it is not neutral: {ok:?}"
    );
    h.save_shot(&h.doc_shot("error-line.png"))?;
    // Fixing the line leaves the others as they were.
    h.press(&[], "Up")?;
    h.press(&[], "End")?;
    h.press(&[], "BackSpace")?;
    h.type_text("2")?;
    app.wait_for("the fixed line", T, |s| {
        answer(s, 1) == Some("0.5") && answer(s, 0) == Some("2") && answer(s, 2) == Some("4")
    })?;
    app.finish()
}

fn ignored_dim(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("ignored", None, &[])?;
    h.type_text("lunch was $55 + 25% tip")?;
    let s = app.wait_for("the ignored words", T, |s| answer(s, 0) == Some("$68.75"))?;
    let ig: Vec<&str> = s["lines"][0]["ignored"]
        .as_array()
        .context("ignored")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    ensure!(ig.contains(&"lunch"), "ignored words: {ig:?}");
    settle();
    let s = app.state()?;
    let spans = s["lines"][0]["spans"].as_array().context("spans")?;
    let find = |text: &str| spans.iter().find(|x| x["text"] == text);
    let lunch = find("lunch").context("no span for lunch")?;
    ensure!(lunch["class"] == "ignored", "lunch is {}", lunch["class"]);
    let pink = spans
        .iter()
        .find(|x| x["class"] == "currency")
        .context("no currency span")?;
    let (y, hgt) = (f(&s["lines"][0]["y"])?, f(&s["lines"][0]["h"])?);
    let shot = h.shot()?;
    let scr = Screen::of(h, &app, &s)?;
    let light = |sp: &Value| -> anyhow::Result<f32> {
        let p = scr
            .brightest(&shot, f(&sp["x0"])?, y + 2., f(&sp["x1"])?, y + hgt - 2.)
            .context("off screen")?;
        Ok(luminance(p))
    };
    let (dim, normal) = (light(lunch)?, light(pink)?);
    ensure!(
        dim < normal * 0.75,
        "the ignored word is not dimmer than the number beside it: {dim} against {normal}"
    );
    h.save_shot(&h.doc_shot("ignored-words.png"))?;
    app.finish()
}

/// A word one letter from a unit, straight after a number, gets a tip with the unit it may have meant.
fn typo_hint(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("typo-hint", None, &[])?;
    h.type_text("5 kmm")?;
    let s = app.wait_for("the answer", T, |s| answer(s, 0) == Some("5"))?;
    settle();
    let s2 = app.state()?;
    let word = s2["lines"][0]["spans"]
        .as_array()
        .context("spans")?
        .iter()
        .find(|x| x["text"] == "kmm")
        .context("a span for kmm")?
        .clone();
    let (x0, x1) = (f(&word["x0"])?, f(&word["x1"])?);
    let y = f(&s["lines"][0]["y"])? + f(&s["lines"][0]["row_h"])? / 2.;
    h.hover(&app, (x0 + x1) / 2., y)?;
    let tip = app.wait_for("the hint", T, |s| {
        s["tooltip"]["value"] == "Did you mean km?"
    })?;
    ensure!(tip["tooltip"]["name"] == "kmm", "{}", tip["tooltip"]);
    h.hover(&app, f(&s["text_x"])? + 700., y)?;
    app.wait_for("the hint gone", T, |s| s["tooltip"].is_null())?;
    app.finish()
}

/// A word the sheet skipped right after a number may be a mistake, so it is marked "unclear"; one in plain prose
/// is not.
fn unclear_words(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("unclear", None, &[])?;
    h.type_line("3 widgets × 4")?;
    h.type_text("I spent $45 on clothes")?;
    let s = app.wait_for("the answers", T, |s| {
        answer(s, 0) == Some("12") && answer(s, 1) == Some("$45.00")
    })?;
    let unclear = |s: &Value, i: usize| -> Vec<String> {
        s["lines"][i]["unclear"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    let class_of = |s: &Value, i: usize, text: &str| -> Option<String> {
        s["lines"][i]["spans"]
            .as_array()?
            .iter()
            .find(|x| x["text"] == text)
            .and_then(|x| x["class"].as_str().map(str::to_owned))
    };
    ensure!(
        unclear(&s, 0) == ["widgets"],
        "unclear words in the first line: {:?}",
        unclear(&s, 0)
    );
    ensure!(
        class_of(&s, 0, "widgets").as_deref() == Some("unclear"),
        "widgets is {:?}",
        class_of(&s, 0, "widgets")
    );
    ensure!(
        unclear(&s, 1).is_empty(),
        "plain prose is marked unclear: {:?}",
        unclear(&s, 1)
    );
    ensure!(
        class_of(&s, 1, "clothes").as_deref() == Some("ignored"),
        "clothes is {:?}",
        class_of(&s, 1, "clothes")
    );
    app.finish()
}

fn total_chip(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("total", None, &[])?;
    h.type_line("10")?;
    h.type_line("20")?;
    h.type_text("5")?;
    let s = app.wait_for("the total of 35", T, |s| s["total"]["text"] == "35")?;
    settle();
    let shot = h.shot()?;
    let scr = Screen::of(h, &app, &s)?;
    let (w, hh) = (
        f(&s["window"]["x"])? + f(&s["window"]["w"])?,
        f(&s["window"]["y"])? + f(&s["window"]["h"])?,
    );
    // The chip sits in the bottom right corner and its text is bright against the dark column.
    let p = scr
        .brightest(&shot, w - 160., hh - 56., w - 24., hh - 14.)
        .context("off screen")?;
    ensure!(
        luminance(p) > 150.,
        "no chip text at the bottom right: {p:?}"
    );
    // The total follows the text.
    h.press(&["ctrl"], "a")?;
    h.type_text("7")?;
    app.wait_for("the total of 7", T, |s| s["total"]["text"] == "7")?;
    h.press(&[], "BackSpace")?;
    app.wait_for("no total for an empty sheet", T, |s| s["total"].is_null())?;
    app.finish()
}

fn sheets_lifecycle(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("sheets", None, &[])?;
    h.type_text("1 + 1")?;
    let s = app.wait_for("the first sheet", T, |s| answer(s, 0) == Some("2"))?;
    let first = s["current"].as_str().context("current")?.to_owned();
    ensure!(sheet_names(&s) == ["1 + 1"], "{:?}", sheet_names(&s));
    let dir = app.data_dir.clone();
    wait_file(&dir, &first, "1 + 1\n")?;

    // New sheet.
    h.press(&["ctrl"], "n")?;
    let s = app.wait_for("a second sheet", T, |s| {
        s["sheets"].as_array().is_some_and(|a| a.len() == 2)
    })?;
    let second = s["current"].as_str().context("current")?.to_owned();
    ensure!(second != first, "the new sheet is the open one");
    ensure!(
        s["lines"].as_array().is_some_and(|l| l.len() == 1) && s["lines"][0]["text"] == "",
        "not empty"
    );
    ensure!(sheet_names(&s)[0] == "Empty", "{:?}", sheet_names(&s));
    h.type_text("7 + 7")?;
    let s = app.wait_for("the second sheet's text", T, |s| answer(s, 0) == Some("14"))?;
    ensure!(
        sheet_names(&s)[0] == "7 + 7",
        "the sidebar shows what is typed: {:?}",
        sheet_names(&s)
    );
    ensure!(s["sheets"][0]["lines"] == 1);

    // Switch away and back; nothing is lost.
    h.press(&["ctrl"], "Tab")?;
    let s = app.wait_for("the first sheet again", T, |s| {
        s["current"] == first.as_str()
    })?;
    ensure!(s["lines"][0]["text"] == "1 + 1");
    h.press(&["ctrl", "shift"], "Tab")?;
    let s = app.wait_for("the second sheet again", T, |s| {
        s["current"] == second.as_str()
    })?;
    ensure!(s["lines"][0]["text"] == "7 + 7");
    settle();
    h.save_shot(&h.doc_shot("sidebar.png"))?;

    // Rename.
    h.press(&[], "F2")?;
    app.wait_for("rename mode", T, |s| s["rename"].is_string())?;
    h.press(&["ctrl"], "a")?;
    for _ in 0..8 {
        h.press(&[], "BackSpace")?;
    }
    h.type_text("Budget")?;
    h.press(&[], "Return")?;
    let s = app.wait_for("the new name", T, |s| s["current"] == "Budget")?;
    ensure!(
        sheet_names(&s).contains(&"Budget".to_owned()),
        "{:?}",
        sheet_names(&s)
    );
    ensure!(
        s["lines"][0]["text"] == "7 + 7",
        "the text survives a rename"
    );
    ensure!(dir.join("Budget.bbg").is_file() && !dir.join(format!("{second}.bbg")).exists());

    // Delete moves the file to the trash.
    h.press(&["ctrl", "shift"], "Delete")?;
    let s = app.wait_for("the sheet gone", T, |s| s["current"] == first.as_str())?;
    ensure!(sheet_names(&s) == ["1 + 1"], "{:?}", sheet_names(&s));
    ensure!(!dir.join("Budget.bbg").exists());
    let trashed: Vec<_> = std::fs::read_dir(dir.join(".trash"))?.flatten().collect();
    ensure!(trashed.len() == 1, "{} files in the trash", trashed.len());
    let name = trashed[0].file_name().to_string_lossy().into_owned();
    ensure!(name.starts_with("Budget."), "{name}");
    ensure!(std::fs::read_to_string(trashed[0].path())? == "7 + 7\n");
    app.finish()
}

/// Where a sidebar card or one row of an open menu sits, in points from the window's top-left corner.
struct Sidebar {
    top: f32,
    card_h: f32,
    row_h: f32,
}

impl Sidebar {
    fn of(s: &Value) -> anyhow::Result<Sidebar> {
        Ok(Sidebar {
            top: f(&s["sidebar"]["top"])?,
            card_h: f(&s["sidebar"]["card_h"])?,
            row_h: f(&s["sidebar"]["menu_row_h"])?,
        })
    }

    /// The middle of the card at `index`.
    fn card(&self, index: usize) -> (f32, f32) {
        (120., self.top + self.card_h * (index as f32 + 0.5))
    }

    /// The middle of menu row `row` of the menu under the card at `index`.
    fn menu_row(&self, index: usize, row: usize) -> (f32, f32) {
        (
            120.,
            self.top + self.card_h * (index as f32 + 1.) + self.row_h * (row as f32 + 0.5),
        )
    }
}

fn sheet_index(s: &Value, id: &str) -> anyhow::Result<usize> {
    s["sheets"]
        .as_array()
        .and_then(|a| a.iter().position(|x| x["id"] == id))
        .with_context(|| format!("no sheet {id} in the sidebar"))
}

/// The sidebar with a real mouse: click opens, right-click shows Rename and Move to trash, double click
/// renames, and Escape or a click elsewhere closes the menu.
fn sidebar_mouse(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("sidebar-mouse", None, &[])?;
    let dir = app.data_dir.clone();
    h.type_text("1 + 1")?;
    let s = app.wait_for("the first sheet", T, |s| answer(s, 0) == Some("2"))?;
    let first = s["current"].as_str().context("current")?.to_owned();
    h.press(&["ctrl"], "n")?;
    app.wait_for("a second sheet", T, |s| {
        s["sheets"].as_array().is_some_and(|a| a.len() == 2)
    })?;
    h.type_text("7 + 7")?;
    let s = app.wait_for("the second sheet's text", T, |s| answer(s, 0) == Some("14"))?;
    let second = s["current"].as_str().context("current")?.to_owned();
    let bar = Sidebar::of(&s)?;
    ensure!(s["menu"].is_null(), "a menu is open at the start");

    // A left click opens a sheet.
    let (x, y) = bar.card(sheet_index(&s, &first)?);
    h.click(&app, x, y, "left", 1)?;
    let s = app.wait_for("the first sheet by click", T, |s| {
        s["current"] == first.as_str()
    })?;
    ensure!(
        s["lines"][0]["text"] == "1 + 1",
        "the text of the clicked sheet"
    );
    ensure!(s["menu"].is_null() && s["rename"].is_null());

    // A right click opens that sheet's menu.
    let (x, y) = bar.card(sheet_index(&s, &second)?);
    h.click(&app, x, y, "right", 1)?;
    let s = app.wait_for("the menu", T, |s| s["menu"] == second.as_str())?;
    ensure!(
        s["current"] == second.as_str(),
        "right-click opens the sheet"
    );
    ensure!(s["lines"][0]["text"] == "7 + 7");
    ensure!(
        s["sidebar"]["menu_rows"] == serde_json::json!(["Rename", "Move to trash"]),
        "{:?}",
        s["sidebar"]["menu_rows"]
    );
    settle();
    h.save_shot(&h.doc_shot("sidebar-menu.png"))?;

    // Escape closes it and does nothing else.
    h.press(&[], "Escape")?;
    let s = app.wait_for("the menu closed by Escape", T, |s| s["menu"].is_null())?;
    ensure!(s["current"] == second.as_str() && s["rename"].is_null());
    ensure!(sheet_names(&s).len() == 2);

    // A click in the editor closes it too.
    let (x, y) = bar.card(sheet_index(&s, &second)?);
    h.click(&app, x, y, "right", 1)?;
    app.wait_for("the menu again", T, |s| s["menu"] == second.as_str())?;
    h.click(
        &app,
        f(&s["text_x"])? + 300.,
        f(&s["lines"][0]["y"])? + 8.,
        "left",
        1,
    )?;
    app.wait_for("the menu closed by a click", T, |s| s["menu"].is_null())?;

    // Rename from the menu.
    let (x, y) = bar.card(sheet_index(&s, &second)?);
    h.click(&app, x, y, "right", 1)?;
    app.wait_for("the menu for rename", T, |s| s["menu"] == second.as_str())?;
    let (x, y) = bar.menu_row(sheet_index(&s, &second)?, 0);
    h.click(&app, x, y, "left", 1)?;
    app.wait_for("rename mode", T, |s| {
        s["rename"].is_string() && s["menu"].is_null()
    })?;
    h.press(&["ctrl"], "a")?;
    for _ in 0..8 {
        h.press(&[], "BackSpace")?;
    }
    h.type_text("Mouse")?;
    h.press(&[], "Return")?;
    let s = app.wait_for("the new name", T, |s| s["current"] == "Mouse")?;
    ensure!(
        s["lines"][0]["text"] == "7 + 7",
        "the text survives a rename"
    );
    ensure!(dir.join("Mouse.bbg").is_file());

    // A double click renames the sheet under it, and Escape leaves the name alone.
    let (x, y) = bar.card(sheet_index(&s, &first)?);
    h.click(&app, x, y, "left", 2)?;
    let s = app.wait_for("rename by double click", T, |s| {
        s["current"] == first.as_str() && s["rename"].is_string()
    })?;
    ensure!(s["lines"][0]["text"] == "1 + 1");
    h.press(&[], "Escape")?;
    let s = app.wait_for("the rename canceled", T, |s| s["rename"].is_null())?;
    ensure!(s["current"] == first.as_str() && sheet_names(&s).len() == 2);
    ensure!(dir.join(format!("{first}.bbg")).is_file());

    // Move to trash from the menu.
    let (x, y) = bar.card(sheet_index(&s, "Mouse")?);
    h.click(&app, x, y, "right", 1)?;
    app.wait_for("the menu for trash", T, |s| s["menu"] == "Mouse")?;
    let (x, y) = bar.menu_row(sheet_index(&s, "Mouse")?, 1);
    h.click(&app, x, y, "left", 1)?;
    let s = app.wait_for("the sheet in the trash", T, |s| {
        s["menu"].is_null() && s["sheets"].as_array().is_some_and(|a| a.len() == 1)
    })?;
    ensure!(s["current"] == first.as_str(), "{:?}", s["current"]);
    ensure!(!dir.join("Mouse.bbg").exists());
    let trashed: Vec<_> = std::fs::read_dir(dir.join(".trash"))?.flatten().collect();
    ensure!(trashed.len() == 1, "{} files in the trash", trashed.len());
    let name = trashed[0].file_name().to_string_lossy().into_owned();
    ensure!(name.starts_with("Mouse."), "{name}");
    ensure!(std::fs::read_to_string(trashed[0].path())? == "7 + 7\n");

    // Ctrl+Shift+T brings it back, and a second one finds the trash empty.
    h.press(&["ctrl", "shift"], "t")?;
    let s = app.wait_for("the sheet restored", T, |s| {
        s["current"] == "Mouse" && s["sheets"].as_array().is_some_and(|a| a.len() == 2)
    })?;
    ensure!(s["lines"][0]["text"] == "7 + 7", "the restored text");
    ensure!(dir.join("Mouse.bbg").is_file());
    ensure!(std::fs::read_dir(dir.join(".trash"))?.flatten().count() == 0);
    h.press(&["ctrl", "shift"], "t")?;
    app.wait_for("the empty trash said so", T, |s| {
        s["notice"] == "The trash is empty"
    })?;
    app.finish()
}

/// Each sheet keeps its own undo steps while another is open.
fn undo_per_sheet(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("undo-per-sheet", None, &[])?;
    h.type_text("1 + 1")?;
    let s = app.wait_for("the first sheet", T, |s| answer(s, 0) == Some("2"))?;
    let first = s["current"].as_str().context("current")?.to_owned();
    // Let the autosave land so the sheet reads the same when it is opened again.
    wait_file(&app.data_dir.clone(), &first, "1 + 1\n")?;
    h.press(&["ctrl"], "n")?;
    app.wait_for("a second sheet", T, |s| {
        s["sheets"].as_array().is_some_and(|a| a.len() == 2)
    })?;
    h.type_text("7 + 7")?;
    let s = app.wait_for("the second sheet's text", T, |s| answer(s, 0) == Some("14"))?;
    let second = s["current"].as_str().context("current")?.to_owned();
    wait_file(&app.data_dir.clone(), &second, "7 + 7\n")?;

    // Back to the first: its own typing can be undone, the second sheet's cannot leak in.
    h.press(&["ctrl"], "Tab")?;
    app.wait_for("the first sheet again", T, |s| {
        s["current"] == first.as_str()
    })?;
    h.press(&["ctrl"], "z")?;
    let s = app.wait_for("one step undone in the first sheet", T, |s| {
        s["current"] == first.as_str() && s["lines"][0]["text"] != "1 + 1"
    })?;
    let shorter = s["lines"][0]["text"].as_str().context("text")?.to_owned();
    ensure!(
        "1 + 1".starts_with(&shorter) && shorter != "7 + 7",
        "undo gave {shorter:?}"
    );

    // The second sheet is as it was left, and its own undo works.
    h.press(&["ctrl", "shift"], "Tab")?;
    let s = app.wait_for("the second sheet again", T, |s| {
        s["current"] == second.as_str()
    })?;
    ensure!(
        s["lines"][0]["text"] == "7 + 7",
        "{:?}",
        s["lines"][0]["text"]
    );
    h.press(&["ctrl"], "z")?;
    let s = app.wait_for("one step undone in the second sheet", T, |s| {
        s["current"] == second.as_str() && s["lines"][0]["text"] != "7 + 7"
    })?;
    let shorter = s["lines"][0]["text"].as_str().context("text")?.to_owned();
    ensure!("7 + 7".starts_with(&shorter), "undo gave {shorter:?}");

    // Redo in the first sheet still has its step after all the switching.
    h.press(&["ctrl"], "Tab")?;
    app.wait_for("the first sheet a third time", T, |s| {
        s["current"] == first.as_str()
    })?;
    h.press(&["ctrl", "shift"], "z")?;
    app.wait_for("the first sheet redone", T, |s| {
        s["lines"][0]["text"] == "1 + 1"
    })?;
    app.finish()
}

fn autosave_sigkill(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("autosave1", None, &[])?;
    let dir = app.data_dir.clone();
    h.type_line("12 + 12 + 15")?;
    h.type_text("85 × 12")?;
    app.wait_for("both answers", T, |s| answer(s, 1) == Some("1,020"))?;
    let id = app.state()?["current"]
        .as_str()
        .context("current")?
        .to_owned();
    // The debounce writes the sheet shortly after the last key.
    wait_file(&dir, &id, "12 + 12 + 15\n85 × 12\n")?;
    app.kill_hard()?;
    ensure!(dir.join(format!("{id}.bbg")).is_file());

    let mut again = h.launch("autosave2", Some(&dir), &[])?;
    let s = again.wait_for("the saved sheet", T, |s| answer(s, 1) == Some("1,020"))?;
    ensure!(s["current"] == id.as_str());
    ensure!(s["lines"][0]["text"] == "12 + 12 + 15" && answer(&s, 0) == Some("39"));
    ensure!(sheet_names(&s) == ["12 + 12 + 15"], "{:?}", sheet_names(&s));
    again.finish()
}

fn keyboard_features(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("keys", None, &[])?;
    // Undo and redo.
    h.type_line("5")?;
    h.type_text("6")?;
    app.wait_for("two lines", T, |s| answer(s, 1) == Some("6"))?;
    h.press(&["ctrl"], "z")?;
    app.wait_for("undo of the 6", T, |s| {
        s["lines"][1]["text"] == "" && s["lines"][0]["text"] == "5"
    })?;
    h.press(&["ctrl"], "z")?;
    app.wait_for("undo of the line break", T, |s| {
        s["lines"].as_array().is_some_and(|l| l.len() == 1)
    })?;
    h.press(&["ctrl", "shift"], "z")?;
    app.wait_for("redo", T, |s| {
        s["lines"].as_array().is_some_and(|l| l.len() == 2)
    })?;

    // The sidebar toggles.
    h.press(&["ctrl"], "b")?;
    app.wait_for("the sidebar hidden", T, |s| s["sidebar_open"] == false)?;
    settle();
    let s = app.state()?;
    ensure!(
        f(&s["window"]["x"])? < 1.0,
        "the editor starts at the window edge: {}",
        s["window"]
    );
    h.press(&["ctrl"], "b")?;
    app.wait_for("the sidebar back", T, |s| s["sidebar_open"] == true)?;

    // Select, replace, then copy an answer with the keyboard and paste it into the sheet.
    h.press(&["ctrl"], "a")?;
    h.type_text("12 + 12 + 15")?;
    app.wait_for("the replaced text", T, |s| {
        s["lines"].as_array().is_some_and(|l| l.len() == 1) && answer(s, 0) == Some("39")
    })?;
    h.press(&["alt"], "Right")?;
    app.wait_for("the answer focused", T, |s| s["answer_focus"] == 0)?;
    h.press(&["ctrl"], "c")?;
    h.press(&[], "Escape")?;
    app.wait_for("the focus returned", T, |s| s["answer_focus"].is_null())?;
    h.press(&[], "End")?;
    h.press(&[], "Return")?;
    h.press(&["ctrl"], "v")?;
    let s = app.wait_for("the pasted answer", T, |s| s["lines"][1]["text"] == "39")?;
    ensure!(answer(&s, 1) == Some("39"));

    // Shift+arrows select; typing replaces the selection.
    h.press(&[], "Home")?;
    h.press(&["shift"], "End")?;
    app.wait_for("a selection", T, |s| s["selection"] == "39")?;
    h.type_text("8")?;
    app.wait_for("the replaced selection", T, |s| {
        s["lines"][1]["text"] == "8"
    })?;
    app.finish()
}

/// The row of line `line` lies inside the window's editor.
fn row_visible(s: &Value, line: usize) -> anyhow::Result<()> {
    let (y, h) = (f(&s["lines"][line]["y"])?, f(&s["lines"][line]["h"])?);
    // The pane starts below the toolbar: rows are placed in window coordinates.
    let (top, window_h) = (f(&s["window"]["y"])?, f(&s["window"]["h"])?);
    ensure!(
        y >= top - 1. && y + h <= top + window_h + 1.,
        "line {} is off the pane: y {y}, row {h}, pane {top} to {}",
        line + 1,
        top + window_h
    );
    Ok(())
}

/// A long sheet: the window scrolls to keep the caret in view, Ctrl+Home and Ctrl+End jump to the ends, Page Down
/// moves a screen, and a click puts the caret on the line it lands on.
fn scrolling(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("scrolling", None, &[])?;
    for i in 1..=80 {
        h.type_line(&format!("{i} + 1"))?;
    }
    let s = app.wait_for("eighty answered lines", T, |s| {
        s["lines"].as_array().is_some_and(|l| l.len() == 81) && answer(s, 79) == Some("81")
    })?;
    ensure!(
        f(&s["scroll"])? > 0.,
        "the window did not scroll with the caret: {}",
        s["scroll"]
    );
    ensure!(s["cursor"]["line"] == 80, "the caret is on the last line");
    row_visible(&s, 80)?;

    h.press(&["ctrl"], "Home")?;
    let s = app.wait_for("the top of the sheet", T, |s| {
        s["cursor"]["line"] == 0 && s["scroll"].as_f64() == Some(0.)
    })?;
    row_visible(&s, 0)?;

    h.press(&[], "Page_Down")?;
    let s = app.wait_for("a screen down", T, |s| {
        s["cursor"]["line"].as_u64().is_some_and(|l| l > 5)
    })?;
    let line = s["cursor"]["line"].as_u64().context("line")? as usize;
    row_visible(&s, line)?;
    h.press(&[], "Page_Up")?;
    app.wait_for("a screen back up", T, |s| s["cursor"]["line"] == 0)?;

    h.press(&["ctrl"], "End")?;
    let s = app.wait_for("the end of the sheet", T, |s| {
        s["cursor"]["line"] == 80 && s["scroll"].as_f64().is_some_and(|v| v > 0.)
    })?;
    row_visible(&s, 80)?;

    // Clicking a visible line puts the caret there, at the end when the click is past its text.
    h.press(&["ctrl"], "Home")?;
    let s = app.wait_for("the top again", T, |s| s["scroll"].as_f64() == Some(0.))?;
    let (y, rh) = (f(&s["lines"][4]["y"])?, f(&s["lines"][4]["h"])?);
    h.click(&app, f(&s["text_x"])? + 120., y + rh / 2., "left", 1)?;
    let s = app.wait_for("the caret moved by the click", T, |s| {
        s["cursor"]["line"] == 4
    })?;
    ensure!(
        s["cursor"]["col"] == s["lines"][4]["text"].as_str().unwrap_or("").len(),
        "a click past the text is at its end: {}",
        s["cursor"]
    );
    app.finish()
}

/// The mouse in the text: a double click selects a word, a triple click a line, a click moves the caret.
fn mouse_editing(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("mouse-editing", None, &[])?;
    h.type_line("12 + 345 apples")?;
    h.type_text("second")?;
    let s = app.wait_for("both lines", T, |s| {
        answer(s, 0) == Some("357") && s["lines"][1]["text"] == "second"
    })?;
    let spans = s["lines"][0]["spans"].as_array().context("spans")?;
    let number = spans
        .iter()
        .find(|x| x["text"] == "345")
        .context("a span for 345")?;
    let x = (f(&number["x0"])? + f(&number["x1"])?) / 2.;
    let (y, rh) = (f(&s["lines"][0]["y"])?, f(&s["lines"][0]["h"])?);

    h.click(&app, x, y + rh / 2., "left", 2)?;
    app.wait_for("the word selected by a double click", T, |s| {
        s["selection"] == "345"
    })?;
    h.click(&app, x, y + rh / 2., "left", 3)?;
    app.wait_for("the line selected by a triple click", T, |s| {
        s["selection"] == "12 + 345 apples"
    })?;
    // A single click clears the selection and moves the caret to the second line.
    let (y2, rh2) = (f(&s["lines"][1]["y"])?, f(&s["lines"][1]["h"])?);
    h.click(&app, f(&s["text_x"])? + 10., y2 + rh2 / 2., "left", 1)?;
    let s = app.wait_for("the caret on the second line", T, |s| {
        s["cursor"]["line"] == 1 && s["selection"] == ""
    })?;
    ensure!(s["cursor"]["col"].as_u64().is_some_and(|c| c <= 3));
    // Typing goes where the caret was put.
    h.type_text("X")?;
    app.wait_for("the typed letter", T, |s| {
        s["lines"][1]["text"]
            .as_str()
            .is_some_and(|t| t.contains('X') && t.len() == 7)
    })?;
    app.finish()
}

/// Dragging with the left button selects the text between the press and the release, and dragging the other way
/// selects backwards.
fn drag_select(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("drag-select", None, &[])?;
    h.type_text("12 + 345 apples")?;
    let s = app.wait_for("the line", T, |s| answer(s, 0) == Some("357"))?;
    let spans = s["lines"][0]["spans"].as_array().context("spans")?;
    let number = spans
        .iter()
        .find(|x| x["text"] == "345")
        .context("a span for 345")?;
    let (y, rh) = (f(&s["lines"][0]["y"])?, f(&s["lines"][0]["h"])?);
    let mid = y + rh / 2.;
    let start = f(&s["text_x"])? + 1.;
    let end_of_345 = f(&number["x1"])? - 1.;
    h.drag(&app, (start, mid), (end_of_345, mid))?;
    app.wait_for("the dragged text selected", T, |s| {
        s["selection"] == "12 + 345"
    })?;
    // Backwards: from the end of the line to the start of 345.
    let begin_of_345 = f(&number["x0"])? + 1.;
    h.drag(&app, (f(&s["text_x"])? + 400., mid), (begin_of_345, mid))?;
    app.wait_for("the backward drag", T, |s| s["selection"] == "345 apples")?;
    app.finish()
}

/// The mouse wheel scrolls the sheet, and scrolling back returns to the top.
fn wheel_scroll(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("wheel-scroll", None, &[])?;
    for i in 1..=80 {
        h.type_line(&format!("{i}"))?;
    }
    let s = app.wait_for("a long sheet", T, |s| {
        s["lines"].as_array().is_some_and(|l| l.len() == 81)
    })?;
    h.press(&["ctrl"], "Home")?;
    let s2 = app.wait_for("the top", T, |s| s["scroll"].as_f64() == Some(0.))?;
    let (x, y) = (f(&s2["text_x"])? + 50., 200.);
    h.scroll(&app, x, y, 5)?;
    let down = app.wait_for("scrolled down by the wheel", T, |s| {
        s["scroll"].as_f64().is_some_and(|v| v > 0.)
    })?;
    let amount = f(&down["scroll"])?;
    ensure!(
        amount > 20.,
        "five notches moved the sheet {amount} points: {}",
        s["window"]
    );
    h.scroll(&app, x, y, -5)?;
    app.wait_for("scrolled back up", T, |s| s["scroll"].as_f64() == Some(0.))?;
    // The caret did not move with the wheel.
    let top = app.state()?;
    ensure!(top["cursor"]["line"] == 0);
    app.finish()
}

/// Ctrl+/ comments a line out and back in, and works on a whole selection, and one undo takes it back.
fn comment_toggle(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("comment-toggle", None, &[])?;
    h.type_line("1 + 1")?;
    h.type_text("2 + 2")?;
    app.wait_for("two answers", T, |s| {
        answer(s, 0) == Some("2") && answer(s, 1) == Some("4")
    })?;
    h.press(&["ctrl"], "slash")?;
    let s = app.wait_for("the second line commented out", T, |s| {
        s["lines"][1]["text"] == "// 2 + 2"
    })?;
    ensure!(s["lines"][1]["kind"] == "comment", "{}", s["lines"][1]);
    ensure!(answer(&s, 1).is_none(), "a comment has no answer");
    ensure!(answer(&s, 0) == Some("2"), "the other line is untouched");
    h.press(&["ctrl"], "slash")?;
    app.wait_for("the comment taken off", T, |s| {
        s["lines"][1]["text"] == "2 + 2" && answer(s, 1) == Some("4")
    })?;
    h.press(&["ctrl"], "a")?;
    h.press(&["ctrl"], "slash")?;
    app.wait_for("both lines commented out", T, |s| {
        s["lines"][0]["text"] == "// 1 + 1" && s["lines"][1]["text"] == "// 2 + 2"
    })?;
    h.press(&["ctrl"], "z")?;
    app.wait_for("one undo takes both back", T, |s| {
        s["lines"][0]["text"] == "1 + 1" && s["lines"][1]["text"] == "2 + 2"
    })?;
    app.finish()
}

/// The window is made smaller and larger while it runs: the layout follows, the answer column stays inside the
/// window and keeps showing its text, and the text column never runs under it.
fn resize_window(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("resize", None, &[])?;
    h.type_line("12 + 12 + 15")?;
    h.type_text("lunch with the whole team 3 + 4 then some more words here")?;
    app.wait_for("both answers", T, |s| {
        answer(s, 0) == Some("39") && answer(s, 1) == Some("7")
    })?;
    for (w, hh) in [(900u32, 600u32), (620, 500), (1000, 700)] {
        h.resize(&app, w, hh)?;
        // The app reports the size it last painted: give it a few seconds, and say what it reported if it never
        // takes the size.
        let mut s = serde_json::Value::Null;
        let mut fitted = false;
        for _ in 0..40 {
            s = app.state()?;
            let near = |k: &str, want: u32| {
                s["window"][k]
                    .as_f64()
                    .is_some_and(|x| (x - f64::from(want)).abs() <= 2.)
            };
            // The app's `window` is its editor pane: the right edge is the window's, the left edge is where the
            // sidebar ends.
            let right =
                s["window"]["x"].as_f64().unwrap_or(0.) + s["window"]["w"].as_f64().unwrap_or(0.);
            let bar = s["toolbar"]["h"].as_f64().unwrap_or(0.) as u32;
            fitted = (right - f64::from(w)).abs() <= 2. && near("h", hh - bar);
            if fitted {
                break;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        ensure!(
            fitted,
            "after resizing to {w}x{hh} the app reports its editor pane as {}, scale {}",
            s["window"],
            s["scale"]
        );
        settle();
        let s = app.state().unwrap_or(s);
        let (text_x, answer_x, answer_w) =
            (f(&s["text_x"])?, f(&s["answer_x"])?, f(&s["answer_w"])?);
        ensure!(
            answer_x + answer_w <= f64::from(w) as f32 + 1.,
            "the answer column leaves the {w} wide window: x {answer_x}, width {answer_w}, pane {}",
            s["window"]
        );
        ensure!(
            text_x < answer_x,
            "the text column starts after the answers at {w}: {text_x} against {answer_x}"
        );
        row_visible(&s, 0)?;
        let shot = h.shot()?;
        let scr = Screen::of(h, &app, &s)?;
        ensure!(
            luminance(scr.answer_cell(&shot, &s, 0)?) > 150.,
            "no answer text on screen for line 1 at {w} wide"
        );
    }
    app.finish()
}

/// Opens the Settings window with the toolbar button.
fn open_settings(h: &mut Harness, app: &mut App) -> anyhow::Result<Value> {
    let s = app.state()?;
    let b = s["toolbar"]["buttons"]
        .as_array()
        .context("buttons")?
        .iter()
        .find(|b| b["name"] == "settings")
        .context("the settings button")?
        .clone();
    h.click(
        app,
        f(&b["x"])? + f(&b["w"])? / 2.,
        f(&b["y"])? + f(&b["h"])? / 2.,
        "left",
        1,
    )?;
    app.wait_for("the Settings window", T, |s| s["panel"] == "settings")
}

/// Clicks a control of the open Settings window by its id (`tab-numbers`, `notation`, `on_launch=new`, ...).
fn settings_click(h: &mut Harness, app: &mut App, id: &str) -> anyhow::Result<()> {
    let st = app.state()?;
    let b = st["toolbar"]["settings_items"]
        .as_array()
        .context("items")?
        .iter()
        .find(|b| b["id"] == id)
        .with_context(|| format!("a {id} control"))?
        .clone();
    h.click(
        app,
        f(&b["x"])? + f(&b["w"])? / 2.,
        f(&b["y"])? + f(&b["h"])? / 2.,
        "left",
        1,
    )
}

/// Closes the Settings window with Escape.
fn close_settings(h: &mut Harness, app: &mut App) -> anyhow::Result<()> {
    h.press(&[], "Escape")?;
    app.wait_for("the window closed", T, |s| s["panel"].is_null())?;
    Ok(())
}

/// The names of the sheets in the sidebar, in order.
fn sheet_order(s: &Value) -> Vec<String> {
    sheet_names(s)
}

/// General, Editing and Variables settings, with a real pointer and keyboard.
fn settings_editing(h: &mut Harness) -> anyhow::Result<()> {
    // The settings file outlives the first run, so the second run reads what the first one chose.
    let config = h.logs.join("settings-editing.settings");
    let _ = std::fs::remove_file(&config);
    let config = config.to_string_lossy().into_owned();
    let mut app = h.launch(
        "settings-editing",
        None,
        &[("BABBAGE_CONFIG_FILE", &config)],
    )?;
    let dir = app.data_dir.clone();

    // Editing: thousands separators as you type; a pasted foreign number converts; Tab finishes a name.
    open_settings(h, &mut app)?;
    settings_click(h, &mut app, "tab-editing")?;
    settings_click(h, &mut app, "group_as_you_type")?;
    settings_click(h, &mut app, "paste_convert")?;
    settings_click(h, &mut app, "autocomplete_key=tab")?;
    close_settings(h, &mut app)?;
    h.type_text("1234567")?;
    let s = app.wait_for("separators put in", T, |s| {
        s["lines"][0]["text"] == "1,234,567"
    })?;
    ensure!(
        s["cursor"]["col"] == 9,
        "the caret stays at the end: {}",
        s["cursor"]
    );
    h.press(&["ctrl"], "a")?;
    h.type_text("1.234,5")?;
    h.press(&["ctrl"], "a")?;
    h.press(&["ctrl"], "c")?;
    h.press(&[], "End")?;
    h.press(&[], "Return")?;
    h.press(&["ctrl"], "v")?;
    app.wait_for("the pasted number converted", T, |s| {
        s["lines"][1]["text"] == "1,234.5"
    })?;
    // Tab finishes a variable's name now, and Escape does not.
    h.press(&[], "Return")?;
    h.type_line("rent = 5")?;
    h.type_text("re")?;
    h.press(&[], "Escape")?;
    std::thread::sleep(Duration::from_millis(600));
    ensure!(
        app.state()?["lines"][3]["text"] == "re",
        "Escape still finishes the name"
    );
    h.press(&[], "Tab")?;
    app.wait_for("Tab finished the name", T, |s| {
        s["lines"][3]["text"] == "rent"
    })?;

    // Renaming a variable: ask, then update; keep; always.
    h.press(&["ctrl"], "a")?;
    h.type_line("rent = 5")?;
    h.type_text("rent + 1")?;
    app.wait_for("both lines", T, |s| answer(s, 1) == Some("6"))?;
    h.press(&["ctrl"], "Home")?;
    h.press(&["ctrl", "shift"], "Right")?;
    h.type_text("pay")?;
    let s = app.wait_for("the offer", T, |s| s["rename_offer"]["old"] == "rent")?;
    ensure!(
        s["rename_offer"]["uses"] == 1 && s["lines"][1]["text"] == "rent + 1",
        "offer {}",
        s["rename_offer"]
    );
    let b = s["rename_offer"]["buttons"]
        .as_array()
        .context("buttons")?
        .iter()
        .find(|b| b["id"] == "rename-update")
        .context("update")?
        .clone();
    h.click(
        &app,
        f(&b["x"])? + f(&b["w"])? / 2.,
        f(&b["y"])? + f(&b["h"])? / 2.,
        "left",
        1,
    )?;
    app.wait_for("the use renamed", T, |s| {
        s["lines"][1]["text"] == "pay + 1" && s["rename_offer"].is_null()
    })?;
    ensure!(answer(&app.state()?, 1) == Some("6"));
    // One undo takes the update back, a second the typing.
    h.press(&["ctrl"], "z")?;
    app.wait_for("the update undone", T, |s| {
        s["lines"][1]["text"] == "rent + 1"
    })?;
    h.press(&["ctrl"], "z")?;
    app.wait_for("the rename undone", T, |s| {
        s["lines"][0]["text"] == "rent = 5"
    })?;
    // Keep leaves the use alone and does not ask again on this line.
    h.press(&["ctrl"], "Home")?;
    h.press(&["ctrl", "shift"], "Right")?;
    h.type_text("cost")?;
    let s = app.wait_for("the offer again", T, |s| s["rename_offer"]["old"] == "rent")?;
    let b = s["rename_offer"]["buttons"]
        .as_array()
        .context("buttons")?
        .iter()
        .find(|b| b["id"] == "rename-keep")
        .context("keep")?
        .clone();
    h.click(
        &app,
        f(&b["x"])? + f(&b["w"])? / 2.,
        f(&b["y"])? + f(&b["h"])? / 2.,
        "left",
        1,
    )?;
    app.wait_for("the offer gone", T, |s| s["rename_offer"].is_null())?;
    h.press(&["ctrl"], "Home")?;
    h.press(&[], "End")?;
    h.type_text("s")?;
    std::thread::sleep(Duration::from_millis(500));
    let s = app.state()?;
    ensure!(
        s["rename_offer"].is_null() && s["lines"][1]["text"] == "rent + 1",
        "Keep was ignored: {}",
        s["rename_offer"]
    );
    // Always: no question.
    open_settings(h, &mut app)?;
    settings_click(h, &mut app, "tab-editing")?;
    settings_click(h, &mut app, "rename_variables=always")?;
    close_settings(h, &mut app)?;
    h.press(&["ctrl"], "a")?;
    h.type_line("tip = 5")?;
    h.type_text("tip + 1")?;
    app.wait_for("both lines", T, |s| {
        s["lines"][1]["text"] == "tip + 1" && answer(s, 1) == Some("6")
    })?;
    h.press(&["ctrl"], "Home")?;
    h.press(&["ctrl", "shift"], "Right")?;
    h.type_text("fee")?;
    app.wait_for("renamed with no question", T, |s| {
        s["lines"][1]["text"] == "fee + 1" && s["rename_offer"].is_null()
    })?;

    // Variables: a global variable is in every sheet; dividers keep the globals and may keep the rest.
    open_settings(h, &mut app)?;
    settings_click(h, &mut app, "tab-variables")?;
    settings_click(h, &mut app, "globals")?;
    h.type_text("pay = $900; tips = 3")?;
    h.press(&[], "Return")?;
    close_settings(h, &mut app)?;
    h.press(&["ctrl"], "a")?;
    h.type_line("pay * 2")?;
    h.type_line("mine = 5")?;
    h.type_line("---")?;
    h.type_line("pay + tips")?;
    h.type_text("mine")?;
    let s = app.wait_for("the global variables", T, |s| {
        answer(s, 0) == Some("$1,800.00") && answer(s, 3) == Some("$903.00")
    })?;
    ensure!(
        answer(&s, 4) != Some("5"),
        "the sheet's own name survived the divider"
    );
    open_settings(h, &mut app)?;
    settings_click(h, &mut app, "tab-variables")?;
    settings_click(h, &mut app, "dividers_reset")?;
    close_settings(h, &mut app)?;
    app.wait_for("dividers keep the names", T, |s| answer(s, 4) == Some("5"))?;

    // General: the order of sheets, and a new sheet when the app opens.
    h.press(&["ctrl"], "a")?;
    h.type_text("zebra")?;
    h.press(&["ctrl"], "n")?;
    h.type_text("apple")?;
    let s = app.wait_for("two sheets", T, |s| {
        s["sheets"].as_array().is_some_and(|l| l.len() == 2)
    })?;
    ensure!(
        sheet_order(&s) == ["apple", "zebra"],
        "changed order: {:?}",
        sheet_order(&s)
    );
    open_settings(h, &mut app)?;
    settings_click(h, &mut app, "tab-general")?;
    settings_click(h, &mut app, "sheet_order=title")?;
    // Newest first would be `apple, zebra` already; the title order is the same, so make it differ.
    close_settings(h, &mut app)?;
    h.press(&["ctrl"], "Tab")?;
    h.press(&["ctrl"], "a")?;
    h.type_text("mango")?;
    app.wait_for("alphabetical", T, |s| sheet_order(s) == ["apple", "mango"])?;
    open_settings(h, &mut app)?;
    settings_click(h, &mut app, "tab-general")?;
    settings_click(h, &mut app, "sheet_order=modified")?;
    settings_click(h, &mut app, "on_launch=new")?;
    close_settings(h, &mut app)?;
    let id = app.state()?["current"]
        .as_str()
        .context("current")?
        .to_owned();
    wait_file(&dir, &id, "mango\n")?;
    app.kill_hard()?;
    let mut again = h.launch(
        "settings-editing-again",
        Some(&dir),
        &[("BABBAGE_CONFIG_FILE", &config)],
    )?;
    let s = again.wait_for("a new sheet", T, |s| {
        s["lines"][0]["text"] == "" && s["sheets"].as_array().is_some_and(|l| l.len() == 3)
    })?;
    ensure!(
        s["current"] != id.as_str(),
        "the app opened the last sheet, not a new one"
    );
    again.finish()
}

/// The Settings window, panes and controls, with a real pointer: each control changes the setting, writes it to the
/// settings file and applies it to the sheet at once.
fn settings_window(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("settings-window", None, &[])?;
    for line in [
        "1/3",
        "300000 * 3",
        "12345 * 3",
        "$5",
        "$100 + VAT",
        "12/02/1988 + 1 day",
        "21 h in workdays",
        "US$5",
        "noon HQ in UTC",
    ] {
        h.type_line(line)?;
    }
    h.type_text("December 24, 2026 + 3 workdays")?;
    let s = app.wait_for("the answers", T, |s| {
        answer(s, 0) == Some("0.3333333333")
            && answer(s, 1) == Some("900k")
            && answer(s, 2) == Some("37,035")
            && answer(s, 3) == Some("$5.00")
            && answer(s, 5) == Some("December 3, 1988")
            && answer(s, 6) == Some("2.625 workdays")
            && answer(s, 7) == Some("$5.00")
            && answer(s, 9).is_some_and(|a| a.starts_with("December 29"))
    })?;
    let path = s["settings_path"]
        .as_str()
        .context("settings path")?
        .to_string();
    let button = |s: &Value, name: &str| -> anyhow::Result<(f32, f32)> {
        let b = s["toolbar"]["buttons"]
            .as_array()
            .context("buttons")?
            .iter()
            .find(|b| b["name"] == name)
            .context("button")?;
        Ok((
            f(&b["x"])? + f(&b["w"])? / 2.,
            f(&b["y"])? + f(&b["h"])? / 2.,
        ))
    };
    let (x, y) = button(&s, "settings")?;
    h.click(&app, x, y, "left", 1)?;
    let mut s = app.wait_for("the window", T, |s| s["panel"] == "settings")?;
    let shot = h.shot()?;
    let scr = Screen::of(h, &app, &s)?;
    let (wx, wy) = (f(&s["toolbar"]["panel_x"])?, f(&s["toolbar"]["panel_y"])?);
    let ink = scr
        .brightest(&shot, wx + 190., wy + 14., wx + 330., wy + 40.)
        .unwrap_or([0, 0, 0]);
    ensure!(
        luminance(ink) > 150.,
        "no title in the Settings window: {ink:?}"
    );
    h.save_shot(&h.doc_shot("settings-window.png"))?;

    // Click a control by its id, wait for the state to show it, and read the settings file.
    let click = |h: &mut Harness, app: &mut App, id: &str| -> anyhow::Result<Value> {
        let st = app.state()?;
        let b = st["toolbar"]["settings_items"]
            .as_array()
            .context("items")?
            .iter()
            .find(|b| b["id"] == id)
            .with_context(|| format!("a {id} control"))?
            .clone();
        h.click(
            app,
            f(&b["x"])? + f(&b["w"])? / 2.,
            f(&b["y"])? + f(&b["h"])? / 2.,
            "left",
            1,
        )?;
        Ok(st)
    };
    let file = || std::fs::read_to_string(&path).unwrap_or_default();

    // Numbers.
    click(h, &mut app, "tab-numbers")?;
    for _ in 0..3 {
        click(h, &mut app, "decimal_places-")?;
    }
    app.wait_for("seven places", T, |s| answer(s, 0) == Some("0.3333333"))?;
    ensure!(file().contains("decimal_places = 7"), "file {:?}", file());
    click(h, &mut app, "notation")?;
    app.wait_for("no notation", T, |s| answer(s, 1) == Some("900,000"))?;
    click(h, &mut app, "group_thousands")?;
    app.wait_for("no separators", T, |s| {
        answer(s, 1) == Some("900000") && answer(s, 2) == Some("37035")
    })?;
    ensure!(file().contains("notation = off") && file().contains("group_thousands = off"));
    click(h, &mut app, "group_thousands")?;
    click(h, &mut app, "notation")?;
    app.wait_for("separators and notation back", T, |s| {
        answer(s, 1) == Some("900k") && answer(s, 2) == Some("37,035")
    })?;

    // Calculator.
    click(h, &mut app, "tab-calculator")?;
    click(h, &mut app, "currency_codes")?;
    app.wait_for("money with codes", T, |s| answer(s, 3) == Some("5.00 USD"))?;
    click(h, &mut app, "money_trim_zeros")?;
    app.wait_for("money without zeros", T, |s| answer(s, 3) == Some("5 USD"))?;
    click(h, &mut app, "currency_codes")?;
    app.wait_for("symbols again", T, |s| answer(s, 3) == Some("$5"))?;
    click(h, &mut app, "money_trim_zeros")?;
    app.wait_for("zeros again", T, |s| answer(s, 3) == Some("$5.00"))?;
    // Text: click the field, type a tax, press Return.
    click(h, &mut app, "tax")?;
    h.type_text("VAT 15%")?;
    h.press(&[], "Return")?;
    s = app.wait_for("the tax", T, |s| answer(s, 4) == Some("$115.00"))?;
    ensure!(file().contains("tax = VAT 15%"), "file {:?}", file());
    ensure!(s["panel"] == "settings", "the window stays open");
    click(h, &mut app, "dollar=AUD")?;
    // In an Australian region `$` is the Australian dollar and the US dollar needs its prefix.
    app.wait_for("Australian dollars", T, |s| answer(s, 7) == Some("US$5.00"))?;
    ensure!(file().contains("dollar = AUD"), "file {:?}", file());
    click(h, &mut app, "dollar=USD")?;
    app.wait_for("US dollars again", T, |s| answer(s, 7) == Some("$5.00"))?;

    click(h, &mut app, "custom_zones")?;
    h.type_text("HQ = Asia/Tokyo")?;
    h.press(&[], "Return")?;
    app.wait_for("the own name for a place", T, |s| {
        answer(s, 8) == Some("03:00")
    })?;
    ensure!(
        file().contains("custom_zones = HQ = Asia/Tokyo"),
        "file {:?}",
        file()
    );

    // Dates & Times.
    click(h, &mut app, "tab-dates")?;
    click(h, &mut app, "holidays")?;
    h.type_text("12-25")?;
    h.press(&[], "Return")?;
    app.wait_for("the holiday skipped", T, |s| {
        answer(s, 9).is_some_and(|a| a.starts_with("December 30"))
    })?;
    ensure!(file().contains("holidays = 12-25"), "file {:?}", file());
    click(h, &mut app, "work_hours+")?;
    app.wait_for("nine hours", T, |s| {
        answer(s, 6) == Some("2.3333333 workdays")
    })?;
    click(h, &mut app, "day_first")?;
    app.wait_for("day first", T, |s| {
        answer(s, 5) == Some("February 13, 1988")
    })?;
    ensure!(
        file().contains("work_hours = 9") && file().contains("day_first = on"),
        "file {:?}",
        file()
    );
    click(h, &mut app, "day_first")?;
    click(h, &mut app, "work_hours-")?;
    app.wait_for("back to the defaults", T, |s| {
        answer(s, 5) == Some("December 3, 1988") && answer(s, 6) == Some("2.625 workdays")
    })?;
    h.press(&[], "Escape")?;
    app.wait_for("the window closed", T, |s| s["panel"].is_null())?;
    app.finish()
}

/// The leftmost and rightmost columns, in window points, holding ink brighter than `floor` in a rectangle.
fn ink_extent(
    scr: &Screen,
    shot: &Shot,
    (x0, y0, x1, y1): (f32, f32, f32, f32),
    floor: f32,
) -> Option<(f32, f32)> {
    let (ya, yb) = (scr.oy + y0 * scr.scale, scr.oy + y1 * scr.scale);
    let (first, last) = (
        (scr.ox + x0 * scr.scale).ceil() as usize,
        (scr.ox + x1 * scr.scale).floor() as usize,
    );
    let inked = |px: usize| {
        shot.brightest(px as f32, ya, px as f32 + 1., yb)
            .is_some_and(|p| luminance(p) > floor)
    };
    let lo = (first..last).find(|&px| inked(px))?;
    let hi = (first..last).rev().find(|&px| inked(px))?;
    let back = |px: usize| (px as f32 - scr.ox) / scr.scale;
    Some((back(lo), back(hi)))
}

/// Where the editor's right edge is, in window points.
fn right_edge(s: &Value) -> anyhow::Result<f32> {
    Ok(f(&s["window"]["x"])? + f(&s["window"]["w"])?)
}

/// Is `p` within `tol` of the 0xRRGGBB color on every channel?
fn near(p: [u8; 3], rgb: u32, tol: i32) -> bool {
    [(rgb >> 16) & 255, (rgb >> 8) & 255, rgb & 255]
        .iter()
        .zip(p)
        .all(|(&want, got)| (want as i32 - got as i32).abs() <= tol)
}

/// Styling settings, painted: colors, the answer column, alignment, text and heading size, and digit widths.
fn settings_styling(h: &mut Harness) -> anyhow::Result<()> {
    let config = h.logs.join("settings-styling.settings");
    let _ = std::fs::remove_file(&config);
    let config = config.to_string_lossy().into_owned();
    let mut app = h.launch(
        "settings-styling",
        None,
        &[("BABBAGE_CONFIG_FILE", &config)],
    )?;
    h.type_line("# Heading")?;
    h.type_line("8888888")?;
    h.type_line("1111111")?;
    h.type_line("5 + 5")?;
    h.type_text("5 kg")?;
    app.wait_for("the answers", T, |s| {
        answer(s, 3) == Some("10") && answer(s, 4) == Some("5 kg")
    })?;
    settle();
    let file = || std::fs::read_to_string(&config).unwrap_or_default();
    let region = |s: &Value, i: usize, answer_side: bool| -> anyhow::Result<(f32, f32, f32, f32)> {
        let (y, rh) = (f(&s["lines"][i]["y"])?, f(&s["lines"][i]["h"])?);
        let (tx, ax, w) = (f(&s["text_x"])?, f(&s["answer_x"])?, right_edge(s)?);
        Ok(if answer_side {
            (ax + 28., y + 2., w - 4., y + rh - 2.)
        } else {
            (tx, y + 2., ax - 12., y + rh - 2.)
        })
    };
    let paint = |h: &mut Harness, app: &mut App| -> anyhow::Result<(Value, Shot, Screen)> {
        settle();
        let s = app.state()?;
        let shot = h.shot()?;
        let scr = Screen::of(h, app, &s)?;
        Ok((s, shot, scr))
    };
    // The window covers the sheet, so look with it closed and come back to the Styling pane.
    let look = |h: &mut Harness, app: &mut App| -> anyhow::Result<(Value, Shot, Screen)> {
        close_settings(h, app)?;
        let seen = paint(h, app)?;
        open_settings(h, app)?;
        settings_click(h, app, "tab-styling")?;
        Ok(seen)
    };

    // Text size, heading size and digit widths, all measured as the width of painted ink.
    let (s, shot, scr) = paint(h, &mut app)?;
    let width_of = |s: &Value, shot: &Shot, scr: &Screen, i: usize| -> anyhow::Result<f32> {
        let (lo, hi) = ink_extent(scr, shot, region(s, i, false)?, 90.).context("ink")?;
        Ok(hi - lo)
    };
    let (heading0, eights0, ones0) = (
        width_of(&s, &shot, &scr, 0)?,
        width_of(&s, &shot, &scr, 1)?,
        width_of(&s, &shot, &scr, 2)?,
    );
    open_settings(h, &mut app)?;
    settings_click(h, &mut app, "tab-styling")?;
    settings_click(h, &mut app, "mono_digits")?;
    let (s, shot, scr) = look(h, &mut app)?;
    let (eights1, ones1) = (width_of(&s, &shot, &scr, 1)?, width_of(&s, &shot, &scr, 2)?);
    // Every font installed here already has digits of one width, so the setting cannot change a line's look on
    // this machine: it must save, and leave the layout alone.
    ensure!(
        (eights1 - eights0).abs() < 2.
            && (ones1 - ones0).abs() < 2.
            && file().contains("mono_digits = on"),
        "the digits setting disturbed the text: {ones0} to {ones1}, {eights0} to {eights1}; file {:?}",
        file()
    );
    settings_click(h, &mut app, "mono_digits")?;
    for _ in 0..3 {
        settings_click(h, &mut app, "heading_size+")?;
    }
    let (s, shot, scr) = look(h, &mut app)?;
    let heading1 = width_of(&s, &shot, &scr, 0)?;
    ensure!(
        (1.2..1.4).contains(&(heading1 / heading0)) && file().contains("heading_size = 130"),
        "a heading at 130% is {heading1} wide against {heading0}; file {:?}",
        file()
    );
    ensure!(
        (width_of(&s, &shot, &scr, 1)? - eights0).abs() < 3.,
        "ordinary text must not change with the heading size"
    );
    for _ in 0..5 {
        settings_click(h, &mut app, "font_size+")?;
    }
    let (s, shot, scr) = look(h, &mut app)?;
    let eights2 = width_of(&s, &shot, &scr, 1)?;
    ensure!(
        (1.15..1.32).contains(&(eights2 / eights0)) && file().contains("font_size = 26"),
        "text at 26pt is {eights2} wide against {eights0}; file {:?}",
        file()
    );
    for _ in 0..5 {
        settings_click(h, &mut app, "font_size-")?;
    }
    settings_click(h, &mut app, "font=Liberation Serif")?;
    let (s, shot, scr) = look(h, &mut app)?;
    let serif = width_of(&s, &shot, &scr, 1)?;
    ensure!(
        (serif - eights0).abs() > 4. && file().contains("font = Liberation Serif"),
        "the font did not change the text: {serif} against {eights0}; file {:?}",
        file()
    );
    settings_click(h, &mut app, "font=Noto Sans")?;
    for _ in 0..3 {
        settings_click(h, &mut app, "heading_size-")?;
    }

    // Colors: the number, the unit, and the variable color paint the chosen color.
    settings_click(h, &mut app, "color_number=#ff7a90")?;
    settings_click(h, &mut app, "color_unit=#62c25a")?;
    let (s, shot, scr) = look(h, &mut app)?;
    let num = scr.brightest(
        &shot,
        region(&s, 1, false)?.0,
        region(&s, 1, false)?.1,
        region(&s, 1, false)?.2,
        region(&s, 1, false)?.3,
    );
    ensure!(
        num.is_some_and(|p| near(p, 0xff7a90, 24)),
        "numbers are not the chosen color: {num:?}"
    );
    let (lx, ly, lx1, ly1) = region(&s, 4, false)?;
    let unit = ink_extent(&scr, &shot, (lx, ly, lx1, ly1), 90.).context("the unit line")?;
    let kg = scr.brightest(&shot, unit.1 - 30., ly, unit.1 + 2., ly1);
    ensure!(
        kg.is_some_and(|p| near(p, 0x62c25a, 24)),
        "units are not the chosen color: {kg:?}"
    );
    ensure!(
        file().contains("color_number = #ff7a90") && file().contains("color_unit = #62c25a"),
        "file {:?}",
        file()
    );
    settings_click(h, &mut app, "color_number=default")?;
    let (s, shot, scr) = look(h, &mut app)?;
    let r = region(&s, 1, false)?;
    let back = scr.brightest(&shot, r.0, r.1, r.2, r.3);
    ensure!(
        back.is_some_and(|p| !near(p, 0xff7a90, 40)),
        "numbers kept the old color: {back:?}"
    );

    // The answer column: its color and which side its text sits on.
    settings_click(h, &mut app, "answer_background=#20283a")?;
    let (s, shot, scr) = look(h, &mut app)?;
    let (ax, aw, wh) = (
        f(&s["answer_x"])?,
        f(&s["answer_w"])?,
        f(&s["window"]["h"])?,
    );
    let at = |x: f32, y: f32| {
        shot.pixel(
            (scr.ox + x * scr.scale) as usize,
            (scr.oy + y * scr.scale) as usize,
        )
    };
    let bg = at(ax + aw - 24., wh - 90.);
    ensure!(near(bg, 0x20283a, 6), "the answer column is {bg:?}");
    let left = ink_extent(&scr, &shot, region(&s, 3, true)?, 90.).with_context(|| {
        format!(
            "the answer, region {:?} scale {} window {} {}",
            region(&s, 3, true),
            scr.scale,
            scr.ox,
            scr.oy
        )
    })?;
    settings_click(h, &mut app, "answer_align=right")?;
    let (s, shot, scr) = look(h, &mut app)?;
    let right = ink_extent(&scr, &shot, region(&s, 3, true)?, 90.).with_context(|| {
        format!(
            "right: region {:?}, scale {} origin {} {}",
            region(&s, 3, true),
            scr.scale,
            scr.ox,
            scr.oy
        )
    })?;
    let w = right_edge(&s)?;
    ensure!(
        right.1 > w - 40. && right.0 > left.0 + 60. && file().contains("answer_align = right"),
        "answers did not move to the right: {left:?} then {right:?}; file {:?}",
        file()
    );
    // The picture for the settings notes: the pane open over the sheet it has restyled (pink numbers, green units, a
    // blue answer column with its answers on the right).
    settings_click(h, &mut app, "color_number=#ff7a90")?;
    settle();
    h.save_shot(&h.doc_shot("settings-styling.png"))?;
    close_settings(h, &mut app)?;
    app.finish()
}

/// A small sheet of the kind the project page shows: variables, a percentage, conversions, a date and a time span.
/// The picture is kept in docs/shots, and the answers are checked so the page never shows a wrong one.
fn showcase(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("showcase", None, &[])?;
    let lines = [
        "# Weekend trip",
        "flights = $412 × 2",
        "hotel = 3 × $138",
        "meals = 4 × $65",
        "fuel = 14 × $3.89",
        "fees = 8% of hotel",
        "total = flights + hotel + meals + fuel + fees",
        "total / 4",
        "",
        "// units, dates and times",
        "150 lb in kg",
        "5 km in miles",
        "20% of 85",
        "1/3 + 1/6",
        "June 3, 2027 + 3 weeks",
        "7:30am to 8:45pm",
    ];
    for line in &lines[..lines.len() - 1] {
        h.type_line(line)?;
    }
    h.type_text(lines[lines.len() - 1])?;
    let s = app.wait_for("the answers", T, |s| {
        answer(s, 6) == Some("$1,585.58")
            && answer(s, 10) == Some("68.0388555 kg")
            && answer(s, 13) == Some("0.5")
            && answer(s, 14) == Some("June 24, 2027")
            && answer(s, 15) == Some("13 hours 15 min")
    })?;
    ensure!(
        answer(&s, 7) == Some("$396.40") && answer(&s, 12) == Some("17"),
        "the totals: {:?}",
        (0..16).map(|i| answer(&s, i)).collect::<Vec<_>>()
    );
    h.resize(&app, 1060, 740)?;
    h.hover(&app, 1050., 730.)?;
    settle();
    h.save_window_shot(&app, &h.doc_shot("showcase.png"))?;
    app.finish()
}

/// One picture per sheet in docs/guide/sheets: the lines of the file are typed into a fresh sheet on the fixed clock,
/// in a window wide enough that no line wraps and as tall as the sheet. An optional first line `@width WIDTH` or
/// `@size WIDTH HEIGHT` (points) overrides that. A line that has an answer must show one, so the guide never pictures a sheet that does
/// not work.
fn guide_sheets(h: &mut Harness) -> anyhow::Result<()> {
    let dir = h.repo.join("docs/guide/sheets");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .with_context(|| format!("read {}", dir.display()))?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "txt"))
        .collect();
    files.sort();
    ensure!(!files.is_empty(), "no sheets in {}", dir.display());
    let zones = tzdir(h);
    for file in files {
        let name = file
            .file_stem()
            .and_then(|s| s.to_str())
            .context("a sheet name")?
            .to_owned();
        let text = std::fs::read_to_string(&file)?;
        let mut lines: Vec<&str> = text.lines().collect();
        let mut dims = vec![1060, 0];
        if let Some(width) = lines.first().and_then(|l| l.strip_prefix("@width ")) {
            dims[0] = width
                .trim()
                .parse()
                .with_context(|| format!("{name}: bad @width"))?;
            lines.remove(0);
            dims[1] = (36 * (lines.len() as u32 + 1) + 130).max(330);
        } else if let Some(size) = lines.first().and_then(|l| l.strip_prefix("@size ")) {
            dims = size
                .split_whitespace()
                .filter_map(|n| n.parse().ok())
                .collect();
            ensure!(dims.len() == 2, "{name}: bad @size line");
            lines.remove(0);
        } else {
            dims[1] = (36 * (lines.len() as u32 + 1) + 130).max(330);
        }
        let mut app = h.launch(
            &name,
            None,
            &[
                ("TZ", "UTC"),
                ("BABBAGE_NOW", FIXED_NOW),
                ("BABBAGE_TZDIR", &zones),
            ],
        )?;
        for line in &lines {
            h.type_line(line)?;
        }
        let s = app.wait_for("every line", T, |s| {
            s["lines"].as_array().is_some_and(|l| l.len() > lines.len())
        })?;
        // A picture of an error, named `...-error`, is meant to show one.
        for (i, line) in lines.iter().enumerate().filter(|_| !name.contains("error")) {
            let shown = &s["lines"][i];
            ensure!(
                shown["error"].is_null(),
                "{name}: line {} ({line:?}) is an error: {}",
                i + 1,
                shown["error"]
            );
        }
        h.resize(&app, dims[0], dims[1])?;
        h.hover(&app, dims[0] as f32 - 6., dims[1] as f32 - 6.)?;
        settle();
        h.save_window_shot(&app, &h.guide_shot(&name))?;
        app.finish()?;
    }
    Ok(())
}

/// Puts the window at `width` by `height` points, moves the pointer out of the way and keeps a picture of the window
/// for the user guide.
fn guide_picture(
    h: &mut Harness,
    app: &App,
    name: &str,
    width: u32,
    height: u32,
) -> anyhow::Result<()> {
    h.resize(app, width, height)?;
    h.hover(app, width as f32 - 6., height as f32 - 6.)?;
    guide_snap(h, app, name)
}

/// A picture of the window as it is, after the paint has settled.
fn guide_snap(h: &mut Harness, app: &App, name: &str) -> anyhow::Result<()> {
    settle();
    h.save_window_shot(app, &h.guide_shot(name))
}

/// The pictures of the guide that are not just a sheet: the window and its sidebar, the answer menu, the name list,
/// the Settings panes. Each is taken from a fresh app on the fixed clock.
fn guide_screens(h: &mut Harness) -> anyhow::Result<()> {
    let zones = tzdir(h);
    let config = h.logs.join("guide-screens.settings");
    std::fs::write(
        &config,
        "custom_zones = HQ = Europe/Berlin; Lab = Asia/Tokyo\nglobals = rent = $900; tip = 18%\ntax = VAT 15%\n",
    )?;
    let config = config.to_string_lossy().into_owned();
    let plain = [
        ("TZ", "UTC"),
        ("BABBAGE_NOW", FIXED_NOW),
        ("BABBAGE_TZDIR", zones.as_str()),
    ];
    let configured = [
        ("TZ", "UTC"),
        ("BABBAGE_NOW", FIXED_NOW),
        ("BABBAGE_TZDIR", zones.as_str()),
        ("BABBAGE_CONFIG_FILE", config.as_str()),
    ];
    let sheet_count =
        |n: usize| move |s: &Value| s["sheets"].as_array().is_some_and(|a| a.len() == n);

    // The window with two sheets, and the sidebar with four.
    let mut app = h.launch("guide-window", None, &plain)?;
    for line in [
        "# Weekend trip",
        "Hotel: $180 + $20",
        "Dinner for two: 2 * $35",
        "lunch was $55 + 25% tip",
        "rent = $900",
        "rent * 12",
    ] {
        h.type_line(line)?;
    }
    let s = app.wait_for("the first sheet", T, |s| answer(s, 5) == Some("$10,800.00"))?;
    let first = s["current"].as_str().context("current")?.to_owned();
    let mut second = String::new();
    for (n, lines) in [
        (2, &["# Groceries", "milk = $4.50", "3 * milk"][..]),
        (3, &["# Budget", "salary = $4,200", "salary * 12"][..]),
        (4, &["15% of 240"][..]),
    ] {
        h.press(&["ctrl"], "n")?;
        app.wait_for("a new sheet", T, sheet_count(n))?;
        for line in lines {
            h.type_line(line)?;
        }
        if n == 2 {
            second = app.state()?["current"]
                .as_str()
                .context("current")?
                .to_owned();
        }
    }
    app.wait_for("the last sheet", T, |s| answer(s, 0) == Some("36"))?;
    guide_picture(h, &app, "sidebar", 900, 520)?;
    let s = app.state()?;
    let bar = Sidebar::of(&s)?;
    let (x, y) = bar.card(sheet_index(&s, &first)?);
    h.click(&app, x, y, "left", 1)?;
    app.wait_for("the first sheet again", T, |s| {
        s["current"] == first.as_str()
    })?;
    guide_picture(h, &app, "window-parts", 1000, 560)?;

    // The card menu, the name being typed, and the notice after an export.
    let s = app.state()?;
    let (x, y) = bar.card(sheet_index(&s, &second)?);
    h.click(&app, x, y, "right", 1)?;
    app.wait_for("the card menu", T, |s| s["menu"] == second.as_str())?;
    guide_picture(h, &app, "sheet-menu", 900, 520)?;
    h.press(&[], "Escape")?;
    app.wait_for("the menu closed", T, |s| s["menu"].is_null())?;
    h.press(&[], "F2")?;
    app.wait_for("rename mode", T, |s| s["rename"].is_string())?;
    h.press(&["ctrl"], "a")?;
    h.type_text("Budget")?;
    guide_picture(h, &app, "sheet-rename", 900, 520)?;
    h.press(&[], "Escape")?;
    app.wait_for("rename canceled", T, |s| s["rename"].is_null())?;
    h.press(&["ctrl", "shift"], "e")?;
    app.wait_for("the export notice", T, |s| !s["notice"].is_null())?;
    guide_picture(h, &app, "export-notice", 900, 520)?;
    app.finish()?;

    // The name list under the caret.
    let mut app = h.launch("guide-names", None, &plain)?;
    h.resize(&app, 900, 340)?;
    for line in ["monthly rent = $900", "monthly food = $400"] {
        h.type_line(line)?;
    }
    h.type_text("mon")?;
    h.press(&[], "Escape")?;
    app.wait_for("the name list", T, |s| !s["completion"].is_null())?;
    guide_snap(h, &app, "completion-list")?;
    app.finish()?;

    // The answer menu on a line.
    let mut app = h.launch("guide-answer-menu", None, &plain)?;
    h.resize(&app, 900, 520)?;
    for line in ["1/3", "1234567", "$1.495"] {
        h.type_line(line)?;
    }
    let s = app.wait_for("the answers", T, |s| answer(s, 1) == Some("1.234567M"))?;
    let right = f(&s["more_button"]["right"])?;
    let from_right = f(&s["more_button"]["from_right"])?;
    let y = f(&s["lines"][1]["y"])? + f(&s["lines"][1]["row_h"])? / 2.;
    h.hover(&app, right - from_right - 80., y)?;
    app.wait_for("the line hovered", T, |s| s["hover_line"] == 1)?;
    h.click(&app, right - from_right, y, "left", 1)?;
    app.wait_for("the menu", T, |s| s["answer_menu"]["line"] == 1)?;
    guide_snap(h, &app, "answer-menu")?;
    app.finish()?;

    // The Settings panes.
    for (name, env) in [
        ("settings", &plain[..]),
        ("settings-with-values", &configured[..]),
    ] {
        let mut app = h.launch(&format!("guide-{name}"), None, env)?;
        h.resize(&app, 1200, 800)?;
        // The toolbar moves with the window; let it paint before aiming at it.
        settle();
        open_settings(h, &mut app)?;
        let panes: &[&str] = if name == "settings" {
            &["general", "editing", "numbers", "styling", "updates"]
        } else {
            &["calculator", "variables", "dates"]
        };
        for pane in panes {
            settings_click(h, &mut app, &format!("tab-{pane}"))?;
            h.hover(&app, 500., 780.)?;
            guide_snap(h, &app, &format!("settings-{pane}"))?;
        }
        app.finish()?;
    }
    Ok(())
}

/// The center of a rectangle the state reports (`x`, `y`, `w`, `h`).
fn centre(r: &Value) -> anyhow::Result<(f32, f32)> {
    Ok((
        f(&r["x"])? + f(&r["w"])? / 2.,
        f(&r["y"])? + f(&r["h"])? / 2.,
    ))
}

fn updates_of(s: &Value) -> &Value {
    &s["updates"]
}

/// The update check is infrequent and can be turned off: off asks nothing, the default (weekly) asks once and
/// then not again within the week while the pill stays up, Check now asks whatever the interval says, and the
/// Updates pane says plainly what the check does. The release source is a server on loopback.
fn updates_interval(h: &mut Harness) -> anyhow::Result<()> {
    use crate::releases::{Listed, Releases};
    let server = Releases::start(
        "jrimmer/babbage",
        &[
            Listed {
                version: "99.1.0",
                notes: "- Faster totals",
            },
            Listed {
                version: "99.0.0",
                notes: "- A first release",
            },
        ],
        b"not a program",
    )?;
    let config = h.logs.join("updates-interval.settings");
    let state = h.logs.join("updates-interval-state");
    let _ = std::fs::remove_dir_all(&state);
    let source = format!("update_source = forgejo {} jrimmer/babbage\n", server.base);
    let (config_s, state_s) = (
        config.to_string_lossy().into_owned(),
        state.to_string_lossy().into_owned(),
    );
    // An empty BABBAGE_UPDATES leaves it to the settings (the harness otherwise switches updates off).
    let env = [
        ("BABBAGE_CONFIG_FILE", config_s.as_str()),
        ("XDG_STATE_HOME", state_s.as_str()),
        ("BABBAGE_UPDATES", ""),
    ];

    // Off: nothing is asked, however long it waits.
    std::fs::write(&config, format!("{source}update_check = off\n"))?;
    let mut app = h.launch("updates-off", None, &env)?;
    std::thread::sleep(Duration::from_millis(2500));
    let s = app.state()?;
    ensure!(
        server.requests().is_empty()
            && updates_of(&s)["interval"] == "off"
            && updates_of(&s)["pill"].is_null(),
        "an off check still asked: {:?} {}",
        server.requests(),
        updates_of(&s)
    );
    // The Updates pane says what the check does, and the choice is saved.
    open_settings(h, &mut app)?;
    settings_click(h, &mut app, "tab-updates")?;
    settings_click(h, &mut app, "update_check=weekly")?;
    app.wait_for("weekly saved", T, |s| updates_of(s)["interval"] == "weekly")?;
    let saved = std::fs::read_to_string(&config)?;
    ensure!(saved.contains("update_check = weekly"), "file {saved:?}");
    settle();
    h.save_shot(&h.doc_shot("settings-updates.png"))?;
    ensure!(
        server.requests().is_empty(),
        "choosing an interval asked the server: {:?}",
        server.requests()
    );
    app.finish()?;

    // The default interval: the first launch asks once and a pill offers the newest release.
    std::fs::write(&config, &source)?;
    let mut app = h.launch("updates-weekly", None, &env)?;
    let s = app.wait_for("the pill", T, |s| !updates_of(s)["pill"].is_null())?;
    ensure!(
        updates_of(&s)["interval"] == "weekly"
            && updates_of(&s)["latest"] == "99.1.0"
            && server.list_requests() == 1,
        "first launch: {} after {:?}",
        updates_of(&s),
        server.requests()
    );
    ensure!(
        state.join("babbage/updates.json").is_file(),
        "the answer was not kept"
    );
    app.finish()?;

    // The next launch is inside the week: no request, and the pill is still up from the kept answer.
    let mut app = h.launch("updates-weekly-again", None, &env)?;
    app.wait_for("the pill from the kept answer", T, |s| {
        !updates_of(s)["pill"].is_null()
    })?;
    std::thread::sleep(Duration::from_millis(1500));
    ensure!(
        server.list_requests() == 1,
        "asked again within the week: {:?}",
        server.requests()
    );

    // Check now asks whatever the interval says, and says how it went.
    open_settings(h, &mut app)?;
    settings_click(h, &mut app, "tab-updates")?;
    settings_click(h, &mut app, "update_check=off")?;
    app.wait_for("off saved", T, |s| updates_of(s)["interval"] == "off")?;
    settings_click(h, &mut app, "update_now")?;
    let s = app.wait_for("the answer", T, |s| {
        s["notice"]
            .as_str()
            .is_some_and(|n| n.contains("99.1.0 is available"))
    })?;
    ensure!(
        server.list_requests() == 2 && updates_of(&s)["phase"] == "idle",
        "check now: {} after {:?}",
        updates_of(&s),
        server.requests()
    );
    app.finish()
}

/// What an update looks like from the window: the pill opens the panel with the releases newest first and the
/// install boilerplate cut off; Later closes it and the pill stays; Skip this version hides the pill and keeps
/// hiding it at the next launch; Install downloads, checks the checksum and replaces the AppImage; Restart starts
/// the new file and quits the old process.
fn updates_install(h: &mut Harness) -> anyhow::Result<()> {
    use crate::releases::{Listed, Releases};
    let marker = h.logs.join("updates-install.started");
    let _ = std::fs::remove_file(&marker);
    let bin = h.repo.join("target/release/babbage");
    // The "AppImage" the server hands out: a script that notes it was started and runs the real program.
    let new_file = format!(
        "#!/bin/sh\necho started >> '{}'\nexec '{}' \"$@\"\n",
        marker.display(),
        bin.display()
    );
    let server = Releases::start(
        "jrimmer/babbage",
        &[
            Listed {
                version: "99.1.0",
                notes: "## Faster\n\n- **Totals** restart at a line that does not fit",
            },
            Listed {
                version: "99.0.0",
                notes: "- A first release",
            },
        ],
        new_file.as_bytes(),
    )?;
    let dir = h.logs.join("updates-install");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    let image = dir.join("Babbage.AppImage");
    std::fs::write(&image, "#!/bin/sh\necho old\n")?;
    std::fs::set_permissions(&image, std::fs::Permissions::from_mode(0o755))?;
    let config = dir.join("settings");
    let state = dir.join("state");
    let text = format!("update_source = forgejo {} jrimmer/babbage\n", server.base);
    std::fs::write(&config, &text)?;
    let (config_s, state_s, image_s, appdir_s) = (
        config.to_string_lossy().into_owned(),
        state.to_string_lossy().into_owned(),
        image.to_string_lossy().into_owned(),
        bin.parent()
            .context("the program's folder")?
            .to_string_lossy()
            .into_owned(),
    );
    let env = [
        ("BABBAGE_CONFIG_FILE", config_s.as_str()),
        ("XDG_STATE_HOME", state_s.as_str()),
        ("BABBAGE_UPDATES", ""),
        ("APPIMAGE", image_s.as_str()),
        ("APPDIR", appdir_s.as_str()),
    ];
    let click = |h: &mut Harness, app: &App, r: &Value| -> anyhow::Result<()> {
        let (x, y) = centre(r)?;
        h.click(app, x, y, "left", 1)
    };
    let button = |s: &Value, id: &str| -> anyhow::Result<Value> {
        updates_of(s)["buttons"]
            .as_array()
            .context("buttons")?
            .iter()
            .find(|b| b["id"] == id)
            .cloned()
            .with_context(|| format!("a {id} button"))
    };

    // The pill, the panel, Later and Skip.
    let mut app = h.launch("updates-panel", None, &env)?;
    let s = app.wait_for("the pill", T, |s| !updates_of(s)["pill"].is_null())?;
    click(h, &app, &updates_of(&s)["pill"])?;
    let s = app.wait_for("the panel", T, |s| s["panel"] == "updates")?;
    let releases = updates_of(&s)["releases"].as_array().context("releases")?;
    let versions: Vec<&str> = releases
        .iter()
        .filter_map(|r| r["version"].as_str())
        .collect();
    ensure!(versions == ["99.1.0", "99.0.0"], "releases {versions:?}");
    let all_notes: String = releases
        .iter()
        .filter_map(|r| r["notes"].as_str())
        .collect();
    ensure!(
        all_notes.contains("Totals") && !all_notes.contains("BOILERPLATE"),
        "the notes {all_notes:?}"
    );
    settle();
    h.save_shot(&h.doc_shot("update-panel.png"))?;
    click(h, &app, &button(&s, "update-later")?)?;
    let s = app.wait_for("Later closes the panel", T, |s| s["panel"].is_null())?;
    ensure!(
        !updates_of(&s)["pill"].is_null(),
        "the pill stays after Later"
    );
    click(h, &app, &updates_of(&s)["pill"])?;
    let s = app.wait_for("the panel again", T, |s| s["panel"] == "updates")?;
    click(h, &app, &button(&s, "update-skip")?)?;
    let s = app.wait_for("the pill gone", T, |s| {
        updates_of(s)["pill"].is_null() && s["panel"].is_null()
    })?;
    ensure!(
        std::fs::read_to_string(&config)?.contains("update_skip = 99.1.0"),
        "the skip was not saved: {:?}",
        std::fs::read_to_string(&config)
    );
    ensure!(updates_of(&s)["latest"].is_null());
    app.finish()?;
    // Skipped stays skipped at the next launch.
    let mut app = h.launch("updates-skipped", None, &env)?;
    std::thread::sleep(Duration::from_millis(1500));
    ensure!(
        updates_of(&app.state()?)["pill"].is_null(),
        "a skipped version came back"
    );
    app.finish()?;
    ensure!(server.list_requests() == 1, "asked {:?}", server.requests());

    // Install: the file on disk is replaced, after its checksum was checked.
    std::fs::write(&config, &text)?;
    let mut app = h.launch("updates-install", None, &env)?;
    let s = app.wait_for("the pill", T, |s| !updates_of(s)["pill"].is_null())?;
    click(h, &app, &updates_of(&s)["pill"])?;
    let s = app.wait_for("the panel", T, |s| s["panel"] == "updates")?;
    click(h, &app, &button(&s, "update-install")?)?;
    let s = app.wait_for("installed", Duration::from_secs(60), |s| {
        updates_of(s)["phase"] == "installed" || updates_of(s)["phase"] == "install_failed"
    })?;
    ensure!(
        updates_of(&s)["phase"] == "installed",
        "the install failed: {}",
        updates_of(&s)
    );
    ensure!(
        std::fs::read_to_string(&image)? == new_file,
        "the AppImage was not replaced: {:?}",
        std::fs::read_to_string(&image)
    );
    ensure!(
        std::fs::metadata(&image)?.permissions().mode() & 0o100 != 0,
        "the new file is not executable"
    );
    ensure!(
        button(&s, "update-restart").is_ok() && button(&s, "update-install").is_err(),
        "buttons {}",
        updates_of(&s)["buttons"]
    );

    // Restart: the new file starts, and this process goes.
    let old = app.pid();
    let sock = app.sock_path().to_string_lossy().into_owned();
    click(h, &app, &button(&s, "update-restart")?)?;
    let start = Instant::now();
    while Path::new(&format!("/proc/{old}")).exists() {
        ensure!(
            start.elapsed() < Duration::from_secs(20),
            "the old process did not quit"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    app.reconnect(Duration::from_secs(30))?;
    let started = std::fs::read_to_string(&marker).unwrap_or_default();
    ensure!(
        started.trim() == "started",
        "the new file did not start: {started:?}"
    );
    let fresh: Vec<u32> = std::fs::read_dir("/proc")?
        .flatten()
        .filter_map(|e| e.file_name().to_string_lossy().parse::<u32>().ok())
        .filter(|pid| {
            std::fs::read(format!("/proc/{pid}/cmdline"))
                .is_ok_and(|c| String::from_utf8_lossy(&c).contains(&sock))
        })
        .collect();
    ensure!(
        fresh.len() == 1 && fresh[0] != old,
        "one new process for this run's socket, found {fresh:?} (old {old})"
    );
    // The process this check started by restarting is stopped by its own socket's quit command.
    let result = app.finish();
    let start = Instant::now();
    while Path::new(&format!("/proc/{}", fresh[0])).exists()
        && start.elapsed() < Duration::from_secs(8)
    {
        std::thread::sleep(Duration::from_millis(50));
    }
    ensure!(
        !Path::new(&format!("/proc/{}", fresh[0])).exists(),
        "the restarted app did not quit"
    );
    result
}

/// The strip above the sheet shades the same share of the width as the answers column below it, also after the window
/// changes size (it once kept the width of the earlier window, so its shading began too far left).
fn toolbar_follows_resize(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("toolbar-resize", None, &[])?;
    h.type_text("1 + 1")?;
    let first = app.wait_for("the answer", T, |s| answer(s, 0) == Some("2"))?;
    for (w, hgt) in [(900u32, 500u32), (1300, 600), (700, 450)] {
        h.resize(&app, w, hgt)?;
        h.hover(&app, w as f32 - 6., hgt as f32 - 6.)?;
        settle();
        // No request for the state in between: asking for it makes the app draw again, which hid the fault.
        let shot = h.shot()?;
        let scr = Screen::of(h, &app, &first)?;
        let at = |x: f32, y: f32| {
            scr.brightest(&shot, x, y, x + 2., y + 2.)
                .unwrap_or([0, 0, 0])
        };
        // From the middle of the window to its right edge, a point in an empty part of the strip and one straight
        // below it (the buttons sit lower than the strip's first few points).
        let mut x = w as f32 * 0.5;
        while x < w as f32 - 160. {
            let (strip, below) = (at(x, 3.), at(x, hgt as f32 / 2.));
            ensure!(
                strip.iter().zip(below).all(|(a, b)| a.abs_diff(b) <= 3),
                "at {w}x{hgt} the strip over x={x} is {strip:?} but the column below it is {below:?}"
            );
            x += 20.;
        }
    }
    app.finish()
}

/// The strip above the sheet: its three buttons work with a real pointer, the settings panel changes the number
/// format and the working day (saved in the settings file and applied at once), the help panel opens and Escape
/// closes either, and a Total that leaves lines out says so.
fn toolbar(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("toolbar", None, &[])?;
    h.type_line("1000 + 234")?;
    h.type_line("$10")?;
    h.type_line("5")?;
    h.type_text("3 gallons")?;
    let s = app.wait_for("the lines worked out", T, |s| {
        answer(s, 0) == Some("1,234") && s["total"]["left_out"] == 3
    })?;
    ensure!(
        s["total"]["text"] == "3 gal",
        "a Total of clashing kinds follows the last line with a unit: {}",
        s["total"]
    );
    let button = |s: &Value, name: &str| -> anyhow::Result<(f32, f32)> {
        let b = s["toolbar"]["buttons"]
            .as_array()
            .context("toolbar buttons")?
            .iter()
            .find(|b| b["name"] == name)
            .with_context(|| format!("a {name} button"))?;
        Ok((
            f(&b["x"])? + f(&b["w"])? / 2.,
            f(&b["y"])? + f(&b["h"])? / 2.,
        ))
    };
    // Something is painted on each button.
    let shot = h.shot()?;
    let scr = Screen::of(h, &app, &s)?;
    for name in ["settings", "help", "sidebar"] {
        let (x, y) = button(&s, name)?;
        let ink = scr
            .brightest(&shot, x - 12., y - 10., x + 12., y + 10.)
            .unwrap_or([0, 0, 0]);
        ensure!(
            luminance(ink) > 60.,
            "nothing is drawn on the {name} button: {ink:?}"
        );
    }

    // The sidebar button hides and shows the sidebar.
    ensure!(s["sidebar_open"] == true, "the sidebar starts open");
    let (x, y) = button(&s, "sidebar")?;
    h.click(&app, x, y, "left", 1)?;
    app.wait_for("the sidebar hidden", T, |s| s["sidebar_open"] == false)?;
    h.click(&app, x, y, "left", 1)?;
    app.wait_for("the sidebar shown", T, |s| s["sidebar_open"] == true)?;

    // Help opens and Escape closes it.
    let (x, y) = button(&s, "help")?;
    h.click(&app, x, y, "left", 1)?;
    app.wait_for("the help panel", T, |s| s["panel"] == "help")?;
    h.press(&[], "Escape")?;
    app.wait_for("the panel closed", T, |s| s["panel"].is_null())?;

    // Settings: a number format and the working day, both saved to the file and applied.
    let (x, y) = button(&s, "settings")?;
    h.click(&app, x, y, "left", 1)?;
    let s = app.wait_for("the settings window", T, |s| s["panel"] == "settings")?;
    let item = |s: &Value, id: &str| -> anyhow::Result<(f32, f32)> {
        let b = s["toolbar"]["settings_items"]
            .as_array()
            .context("items")?
            .iter()
            .find(|b| b["id"] == id)
            .with_context(|| format!("a {id} control"))?;
        Ok((
            f(&b["x"])? + f(&b["w"])? / 2.,
            f(&b["y"])? + f(&b["h"])? / 2.,
        ))
    };
    let (x, y) = item(&s, "tab-numbers")?;
    h.click(&app, x, y, "left", 1)?;
    let s = app.wait_for("the Numbers pane", T, |s| {
        s["toolbar"]["settings_pane"] == "numbers"
    })?;
    let (x, y) = item(&s, "number_format=western-europe")?;
    h.click(&app, x, y, "left", 1)?;
    let s = app.wait_for("the European format", T, |s| {
        answer(s, 0) == Some("1.234") && s["number_format"] == "WesternEurope"
    })?;
    let path = s["settings_path"]
        .as_str()
        .context("settings path")?
        .to_string();
    let saved = std::fs::read_to_string(&path)?;
    ensure!(
        saved.contains("number_format = western-europe"),
        "the file {path} holds {saved:?}"
    );
    let (x, y) = item(&s, "tab-dates")?;
    h.click(&app, x, y, "left", 1)?;
    let s = app.wait_for("the Dates & Times pane", T, |s| {
        s["toolbar"]["settings_pane"] == "dates"
    })?;
    let (x, y) = item(&s, "work_hours+")?;
    h.click(&app, x, y, "left", 1)?;
    let s = app.wait_for("nine working hours", T, |s| s["work_hours"] == 9)?;
    ensure!(
        std::fs::read_to_string(&path)?.contains("work_hours = 9"),
        "the working day was not saved"
    );
    let (x, y) = item(&s, "tab-numbers")?;
    h.click(&app, x, y, "left", 1)?;
    let s = app.wait_for("the Numbers pane", T, |s| {
        s["toolbar"]["settings_pane"] == "numbers"
    })?;
    let (x, y) = item(&s, "number_format=north-america")?;
    h.click(&app, x, y, "left", 1)?;
    app.wait_for("the American format again", T, |s| {
        answer(s, 0) == Some("1,234") && s["number_format"] == "NorthAmerica"
    })?;
    // A click in the sheet closes the panel.
    let s = app.state()?;
    let (ty, rh) = (f(&s["lines"][0]["y"])?, f(&s["lines"][0]["row_h"])?);
    h.click(&app, f(&s["text_x"])? + 200., ty + rh / 2., "left", 1)?;
    app.wait_for("the panel closed by a click", T, |s| s["panel"].is_null())?;
    app.finish()
}

/// Holding Alt and dragging across a number counts it by its last digit (one step every few pixels), as one undo
/// step; the same drag without Alt selects text and changes nothing.
fn number_scrub(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("number-scrub", None, &[])?;
    h.type_line("12.5 + 3")?;
    h.type_text("1,000")?;
    let s = app.wait_for("both answers", T, |s| {
        answer(s, 0) == Some("15.5") && answer(s, 1) == Some("1,000")
    })?;
    let number = |s: &Value, line: usize, text: &str| -> anyhow::Result<(f32, f32)> {
        let span = s["lines"][line]["spans"]
            .as_array()
            .context("spans")?
            .iter()
            .find(|x| x["class"] == "number" && x["text"] == text)
            .with_context(|| format!("a number span {text:?}"))?;
        Ok((
            (f(&span["x0"])? + f(&span["x1"])?) / 2.,
            f(&s["lines"][line]["y"])? + f(&s["lines"][line]["row_h"])? / 2.,
        ))
    };

    // Without Alt a drag selects and changes no digit.
    let (x, y) = number(&s, 0, "12.5")?;
    h.drag(&app, (x - 12., y), (x + 40., y))?;
    let plain = app.wait_for("a selection", T, |s| {
        s["selection"].as_str().is_some_and(|t| !t.is_empty())
    })?;
    ensure!(
        plain["lines"][0]["text"] == "12.5 + 3",
        "a plain drag edited the line: {}",
        plain["lines"][0]["text"]
    );

    // With Alt it counts: six steps to the right.
    let mut alt = h.hold("alt", 4000)?;
    std::thread::sleep(Duration::from_millis(400));
    h.drag(&app, (x, y), (x + 36., y))?;
    let s = app.wait_for("12.5 scrubbed up", T, |s| {
        s["lines"][0]["text"] == "13.1 + 3"
    })?;
    ensure!(
        answer(&s, 0) == Some("16.1"),
        "the answer follows: {:?}",
        answer(&s, 0)
    );
    alt.kill().ok();
    alt.wait().ok();

    // One undo takes the whole drag back.
    h.press(&["ctrl"], "z")?;
    app.wait_for("the drag undone in one step", T, |s| {
        s["lines"][0]["text"] == "12.5 + 3"
    })?;

    // Thousands commas stay, and a drag to the left counts down.
    let s = app.state()?;
    let (x, y) = number(&s, 1, "1,000")?;
    let mut alt = h.hold("alt", 4000)?;
    std::thread::sleep(Duration::from_millis(400));
    h.drag(&app, (x, y), (x - 12., y))?;
    app.wait_for("1,000 scrubbed down", T, |s| s["lines"][1]["text"] == "998")?;
    alt.kill().ok();
    alt.wait().ok();
    app.finish()
}

/// A subtotal line's answer is bold under a rule across the answer column; a heading is bold; other answers have
/// neither.
fn subtotal_style(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("subtotal-style", None, &[])?;
    for line in ["# Costs", "$10", "$20", "subtotal", "$5"] {
        h.type_line(line)?;
    }
    h.type_text("subtotal")?;
    let s = app.wait_for("the subtotals", T, |s| {
        answer(s, 3) == Some("$30.00") && answer(s, 5) == Some("$5.00")
    })?;
    ensure!(s["lines"][3]["subtotal"] == true && s["lines"][5]["subtotal"] == true);
    ensure!(s["lines"][1]["subtotal"] == false);
    let shot = h.shot()?;
    let scr = Screen::of(h, &app, &s)?;
    let (ax, aw) = (f(&s["answer_x"])?, f(&s["answer_w"])?);
    let band = |line: usize| -> anyhow::Result<f32> {
        let y = f(&s["lines"][line]["y"])?;
        Ok(luminance(
            scr.brightest(&shot, ax + 20., y, ax + aw - 20., y + 3.)
                .unwrap_or([0, 0, 0]),
        ))
    };
    // The rule sits at the top of a subtotal's row and nowhere else; the answers' own text is lower in the row.
    ensure!(
        band(3)? > 70.,
        "no rule above the subtotal at line 4: {}",
        band(3)?
    );
    ensure!(
        band(5)? > 70.,
        "no rule above the subtotal at line 6: {}",
        band(5)?
    );
    for line in [1, 2, 4] {
        ensure!(
            band(line)? < 60.,
            "a rule over a plain answer at line {}: {}",
            line + 1,
            band(line)?
        );
    }
    // Bold ink is heavier: more bright pixels in the answer's text than in a plain answer of the same text.
    let heavy = |line: usize| -> anyhow::Result<usize> {
        let y = f(&s["lines"][line]["y"])?;
        let h = f(&s["lines"][line]["row_h"])?;
        let mut n = 0;
        let scale = f(&s["scale"])?;
        let (ox, oy) = h_origin(&scr);
        for py in ((oy + (y + 6.) * scale) as usize)..((oy + (y + h - 4.) * scale) as usize) {
            for px_ in ((ox + (ax + 30.) * scale) as usize)..((ox + (ax + 130.) * scale) as usize) {
                if luminance(shot.pixel(px_, py)) > 140. {
                    n += 1;
                }
            }
        }
        Ok(n)
    };
    // `$30.00` (bold) against `$20.00` (plain): the same digit count.
    let (plain, bold) = (heavy(2)?, heavy(3)?);
    ensure!(
        bold > plain,
        "the subtotal is not heavier than a plain answer: {bold} against {plain}"
    );
    app.finish()
}

fn h_origin(scr: &Screen) -> (f32, f32) {
    (scr.ox, scr.oy)
}

/// Names the sheet defined are green where they are defined and used, a tip shows the value under the pointer, and
/// Escape finishes a half-typed name: at once when one name fits, from a list (Up, Down, Enter) when several do.
fn variables(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("variables", None, &[])?;
    for line in [
        "cost of development = $9.8k",
        "cost of development",
        "a long variable name = 20",
        "a long variable two = 5",
    ] {
        h.type_line(line)?;
    }
    h.type_text("a long var")?;
    let s = app.wait_for("the names defined", T, |s| {
        answer(s, 0) == Some("$9,800.00")
            && answer(s, 1) == Some("$9,800.00")
            && answer(s, 3) == Some("5")
    })?;
    // Green where defined and where used.
    let green = |line: usize, text: &str| -> anyhow::Result<(f32, f32)> {
        let span = s["lines"][line]["spans"]
            .as_array()
            .context("spans")?
            .iter()
            .find(|x| x["class"] == "variable" && x["text"] == text)
            .with_context(|| format!("line {} has a variable span {text:?}", line + 1))?;
        Ok((f(&span["x0"])?, f(&span["x1"])?))
    };
    let (x0, x1) = green(0, "cost of development")?;
    let (u0, u1) = green(1, "cost of development")?;
    green(2, "a long variable name")?;
    let shot = h.shot()?;
    let scr = Screen::of(h, &app, &s)?;
    let (y, rh) = (f(&s["lines"][1]["y"])?, f(&s["lines"][1]["row_h"])?);
    for (a, b, line_y) in [(u0, u1, y), (x0, x1, f(&s["lines"][0]["y"])?)] {
        let ink = scr
            .brightest(&shot, a, line_y + 6., b, line_y + rh - 6.)
            .unwrap_or([0, 0, 0]);
        ensure!(
            ink[1] > ink[0].saturating_add(25) && ink[1] > ink[2].saturating_add(25),
            "a variable is not green on screen: {ink:?}"
        );
    }

    // The tip under the pointer shows the value, and goes when the pointer leaves the name.
    h.hover(&app, (u0 + u1) / 2., y + rh / 2.)?;
    let s2 = app.wait_for("the tip", T, |s| s["tooltip"]["value"] == "$9,800.00")?;
    ensure!(
        s2["tooltip"]["name"] == "cost of development",
        "{}",
        s2["tooltip"]
    );
    let shot = h.shot()?;
    let (tx, ty) = (f(&s2["tooltip"]["x"])?, f(&s2["tooltip"]["y"])?);
    let ink = scr
        .brightest(&shot, tx + 8., ty + 6., tx + 90., ty + 26.)
        .unwrap_or([0, 0, 0]);
    ensure!(luminance(ink) > 150., "no text in the tip: {ink:?}");
    h.hover(&app, f(&s["text_x"])? + 900., y + rh / 2.)?;
    app.wait_for("the tip gone", T, |s| s["tooltip"].is_null())?;

    // Several names fit `a long var`: a list, chosen with Down and Enter.
    h.press(&[], "Escape")?;
    let s = app.wait_for("the list", T, |s| s["completion"]["candidates"].is_array())?;
    ensure!(
        s["completion"]["candidates"]
            == serde_json::json!(["a long variable name", "a long variable two"])
            && s["completion"]["selected"] == 0,
        "the list: {}",
        s["completion"]
    );
    let shot = h.shot()?;
    let (cx0, cy0) = (f(&s["completion"]["x"])?, f(&s["completion"]["y"])?);
    let blue = scr
        .brightest(&shot, cx0 + 4., cy0 + 4., cx0 + 120., cy0 + 40.)
        .unwrap_or([0, 0, 0]);
    ensure!(luminance(blue) > 60., "the list is not drawn: {blue:?}");
    h.press(&[], "Down")?;
    app.wait_for("the second name", T, |s| s["completion"]["selected"] == 1)?;
    h.press(&[], "Return")?;
    let s = app.wait_for("the name put in", T, |s| {
        s["lines"][4]["text"] == "a long variable two" && s["completion"].is_null()
    })?;
    ensure!(
        answer(&s, 4) == Some("5"),
        "the line is the variable: {:?}",
        answer(&s, 4)
    );

    // One name fits: it is put in at once. Any other key closes a list without taking anything.
    h.press(&[], "Return")?;
    h.type_text("cost of d")?;
    h.press(&[], "Escape")?;
    let s = app.wait_for("the name finished", T, |s| {
        s["lines"][5]["text"] == "cost of development"
    })?;
    ensure!(answer(&s, 5) == Some("$9,800.00"));
    h.press(&[], "Return")?;
    h.type_text("a long var")?;
    h.press(&[], "Escape")?;
    app.wait_for("the list again", T, |s| {
        s["completion"]["candidates"].is_array()
    })?;
    h.type_text("x")?;
    let s = app.wait_for("the list closed by typing", T, |s| {
        s["completion"].is_null()
    })?;
    ensure!(
        s["lines"][6]["text"] == "a long varx",
        "text {}",
        s["lines"][6]["text"]
    );
    app.finish()
}

/// A sheet file edited in another editor: a line added above, and a line's text changed. The per-line choices
/// follow their text (the fingerprint in each trailer line), a choice whose text is gone is dropped, and the file
/// the app writes next carries the new line numbers.
fn answer_formats_follow_text(h: &mut Harness) -> anyhow::Result<()> {
    let dir = h.logs.join("outside-edit-sheets");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir)?;
    // Written as Babbage wrote it (1/3 on line 1 with two places, 2/3 on line 2 with one, $1.495 on line 3 with
    // money digits off), then edited: "new line" was added at the top and the last line's text was changed.
    std::fs::write(
        dir.join("Outside.bbg"),
        "new line\n1/3\n2/3\n$1.496\n⟦bbg line 1 dp=2 #72452f0e⟧\n⟦bbg line 2 dp=1 #411bf787⟧\n⟦bbg line 3 money=off #b93e0d76⟧\n",
    )?;
    let mut app = h.launch("answer-formats-follow-text", Some(&dir), &[])?;
    let s = app.wait_for("the sheet", T, |s| answer(s, 1).is_some())?;
    ensure!(
        answer(&s, 1) == Some("0.33")
            && answer(&s, 2) == Some("0.7")
            && answer(&s, 3) == Some("$1.50")
            && answer(&s, 0).is_none(),
        "the choices did not follow their lines: {:?}",
        (0..4).map(|i| answer(&s, i)).collect::<Vec<_>>()
    );
    ensure!(
        s["lines"][1]["format"] == "dp=2"
            && s["lines"][2]["format"] == "dp=1"
            && s["lines"][3]["format"] == "",
        "formats {:?}",
        (0..4)
            .map(|i| s["lines"][i]["format"].clone())
            .collect::<Vec<_>>()
    );
    ensure!(
        s["sheets"][0]["lines"] == 4,
        "the trailer lines are not counted as lines: {}",
        s["sheets"][0]
    );
    // A line added in the app moves the numbers in the next save.
    h.press(&["ctrl"], "Home")?;
    h.type_text("top")?;
    h.press(&[], "Return")?;
    app.wait_for("the line added", T, |s| answer(s, 2) == Some("0.33"))?;
    wait_file(
        &dir,
        "Outside",
        "top\nnew line\n1/3\n2/3\n$1.496\n⟦bbg line 3 dp=2 #72452f0e⟧\n⟦bbg line 4 dp=1 #411bf787⟧\n",
    )?;
    app.finish()
}

/// `Show as Repeating` in the answer menu: a fraction that never ends shows its repeating part, the choice is kept
/// in the sheet file with the line, a fixed number of places replaces it, and the money digits are left alone.
fn answer_repeating(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("answer-repeating", None, &[])?;
    let dir = app.data_dir.clone();
    h.type_line("1/3")?;
    h.type_line("1/6")?;
    h.type_text("$10 / 3")?;
    let s = app.wait_for("the answers", T, |s| {
        answer(s, 0) == Some("0.3333333333")
            && answer(s, 1) == Some("0.1666666667")
            && answer(s, 2) == Some("$3.33")
    })?;
    let right = f(&s["more_button"]["right"])?;
    let from_right = f(&s["more_button"]["from_right"])?;
    let open = |h: &mut Harness, app: &mut App, s: &Value, line: usize| -> anyhow::Result<Value> {
        let y = f(&s["lines"][line]["y"])? + f(&s["lines"][line]["row_h"])? / 2.;
        h.hover(app, right - from_right - 80., y)?;
        app.wait_for("the line hovered", T, |s| s["hover_line"] == line)?;
        h.click(app, right - from_right, y, "left", 1)?;
        app.wait_for("the menu", T, |s| s["answer_menu"]["line"] == line)
    };
    let item = |s: &Value, id: &str| -> anyhow::Result<(f32, f32)> {
        let b = s["answer_menu"]["items"]
            .as_array()
            .context("an open menu")?
            .iter()
            .find(|b| b["id"] == id)
            .with_context(|| format!("a {id} control"))?;
        Ok((
            f(&b["x"])? + f(&b["w"])? / 2.,
            f(&b["y"])? + f(&b["h"])? / 2.,
        ))
    };

    let s = open(h, &mut app, &s, 0)?;
    let (x, y) = item(&s, "repeating")?;
    h.click(&app, x, y, "left", 1)?;
    let s = app.wait_for("the repeating part", T, |s| {
        answer(s, 0) == Some("0.(3)") && s["lines"][0]["format"] == "repeat=on"
    })?;
    ensure!(
        s["lines"][0]["exact"] == true,
        "a repeating answer is exact: {}",
        s["lines"][0]["exact"]
    );
    let id = s["current"].as_str().context("current")?.to_owned();
    wait_file(
        &dir,
        &id,
        "1/3\n1/6\n$10 / 3\n⟦bbg line 1 repeat=on #72452f0e⟧\n",
    )?;
    ensure!(
        answer(&s, 1) == Some("0.1666666667"),
        "another line is not changed: {:?}",
        answer(&s, 1)
    );

    // A fixed number of places replaces it, and choosing it again brings it back.
    let (x, y) = item(&s, "dp-4")?;
    h.click(&app, x, y, "left", 1)?;
    app.wait_for("four places", T, |s| {
        answer(s, 0) == Some("0.3333") && s["lines"][0]["format"] == "dp=4"
    })?;
    let (x, y) = item(&s, "repeating")?;
    h.click(&app, x, y, "left", 1)?;
    app.wait_for("repeating again", T, |s| {
        answer(s, 0) == Some("0.(3)") && s["lines"][0]["format"] == "repeat=on"
    })?;
    h.click(&app, x, y, "left", 1)?;
    app.wait_for("repeating off", T, |s| {
        answer(s, 0) == Some("0.3333333333") && s["lines"][0]["format"] == "repeat=off"
    })?;

    // Money keeps its cents.
    h.press(&[], "Escape")?;
    app.wait_for("the menu closed", T, |s| s["answer_menu"].is_null())?;
    let s = app.state()?;
    let s = open(h, &mut app, &s, 2)?;
    let (x, y) = item(&s, "repeating")?;
    h.click(&app, x, y, "left", 1)?;
    app.wait_for("money unchanged", T, |s| {
        answer(s, 2) == Some("$3.33") && s["lines"][2]["format"] == "repeat=on"
    })?;
    app.finish()
}

/// The `⋯` on an answer and its menu, with a real pointer: it shows on the line the pointer is over, the menu sets
/// the decimal places and switches the separators, notation and money digits per line, resets them, makes a
/// subtotal line and deletes a line; the choices are saved with the sheet and come back when it is opened again.
fn answer_menu(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("answer-menu", None, &[])?;
    let dir = app.data_dir.clone();
    for line in ["1/3", "$1.495", "12345 * 3", "300000 * 3"] {
        h.type_line(line)?;
    }
    h.type_text("5 + 5")?;
    let s = app.wait_for("the answers", T, |s| {
        answer(s, 0) == Some("0.3333333333")
            && answer(s, 1) == Some("$1.50")
            && answer(s, 2) == Some("37,035")
            && answer(s, 3) == Some("900k")
            && answer(s, 4) == Some("10")
    })?;
    let right = f(&s["more_button"]["right"])?;
    let from_right = f(&s["more_button"]["from_right"])?;
    let row_mid = |s: &Value, i: usize| -> anyhow::Result<f32> {
        Ok(f(&s["lines"][i]["y"])? + f(&s["lines"][i]["row_h"])? / 2.)
    };
    let more = |s: &Value, i: usize| -> anyhow::Result<(f32, f32)> {
        Ok((right - from_right, row_mid(s, i)?))
    };
    let item = |s: &Value, id: &str| -> anyhow::Result<(f32, f32)> {
        let b = s["answer_menu"]["items"]
            .as_array()
            .context("an open menu")?
            .iter()
            .find(|b| b["id"] == id)
            .with_context(|| format!("a {id} control"))?;
        Ok((
            f(&b["x"])? + f(&b["w"])? / 2.,
            f(&b["y"])? + f(&b["h"])? / 2.,
        ))
    };

    // The button shows under the pointer, on that line only.
    let (mx, my) = more(&s, 0)?;
    ensure!(s["hover_line"].is_null(), "no line is hovered at the start");
    h.hover(&app, mx - 80., my)?;
    let s = app.wait_for("line 1 hovered", T, |s| s["hover_line"] == 0)?;
    let shot = h.shot()?;
    let scr = Screen::of(h, &app, &s)?;
    let ink = scr
        .brightest(&shot, mx - 8., my - 3., mx + 8., my + 3.)
        .unwrap_or([0, 0, 0]);
    ensure!(
        luminance(ink) > 60.,
        "no button is drawn on the hovered answer: {ink:?}"
    );
    let off = scr
        .brightest(
            &shot,
            mx - 8.,
            row_mid(&s, 2)? - 3.,
            mx + 8.,
            row_mid(&s, 2)? + 3.,
        )
        .unwrap_or([0, 0, 0]);
    ensure!(
        luminance(off) < 60.,
        "a button is drawn on a line that is not hovered: {off:?}"
    );

    // Decimal places.
    h.click(&app, mx, my, "left", 1)?;
    let s = app.wait_for("the menu", T, |s| s["answer_menu"]["line"] == 0)?;
    let shot = h.shot()?;
    let scr = Screen::of(h, &app, &s)?;
    let (cx0, cy0) = item(&s, "copy")?;
    ensure!(
        luminance(
            scr.brightest(&shot, cx0 - 60., cy0 - 8., cx0 + 20., cy0 + 8.)
                .unwrap_or([0, 0, 0])
        ) > 150.,
        "no text on the Copy Answer row"
    );
    let (x, y) = item(&s, "dp-4")?;
    h.click(&app, x, y, "left", 1)?;
    let s = app.wait_for("four places", T, |s| {
        answer(s, 0) == Some("0.3333") && s["lines"][0]["format"] == "dp=4"
    })?;
    ensure!(
        s["answer_menu"]["line"] == 0,
        "the menu stays open after a choice"
    );
    let (x, y) = item(&s, "dp-0")?;
    h.click(&app, x, y, "left", 1)?;
    app.wait_for("no places", T, |s| answer(s, 0) == Some("0"))?;
    let (x, y) = item(&s, "dp-2")?;
    h.click(&app, x, y, "left", 1)?;
    app.wait_for("two places", T, |s| answer(s, 0) == Some("0.33"))?;
    let id = s["current"].as_str().context("current")?.to_owned();
    wait_file(
        &dir,
        &id,
        "1/3\n$1.495\n12345 * 3\n300000 * 3\n5 + 5\n⟦bbg line 1 dp=2 #72452f0e⟧\n",
    )?;
    let (x, y) = item(&s, "reset")?;
    h.click(&app, x, y, "left", 1)?;
    app.wait_for("reset", T, |s| {
        answer(s, 0) == Some("0.3333333333") && s["lines"][0]["format"] == ""
    })?;
    let (x, y) = item(&s, "dp-3")?;
    h.click(&app, x, y, "left", 1)?;
    app.wait_for("three places again", T, |s| answer(s, 0) == Some("0.333"))?;
    h.press(&[], "Escape")?;
    app.wait_for("the menu closed", T, |s| s["answer_menu"].is_null())?;

    // Money digits, thousands separators and notation, each on its own line.
    for (line, id, then) in [
        (1usize, "money", "$1.495"),
        (2, "group", "37035"),
        (3, "notation", "900,000"),
    ] {
        let s = app.state()?;
        let (x, y) = more(&s, line)?;
        h.click(&app, x, y, "left", 1)?;
        let s = app.wait_for("the menu", T, |s| s["answer_menu"]["line"] == line)?;
        let (x, y) = item(&s, id)?;
        h.click(&app, x, y, "left", 1)?;
        app.wait_for(id, T, |s| answer(s, line) == Some(then))?;
        h.press(&[], "Escape")?;
        app.wait_for("closed", T, |s| s["answer_menu"].is_null())?;
    }
    // The choices are saved with the sheet and come back after a hard kill.
    wait_file(
        &dir,
        &id,
        "1/3\n$1.495\n12345 * 3\n300000 * 3\n5 + 5\n⟦bbg line 1 dp=3 #72452f0e⟧\n⟦bbg line 2 money=off #b93e0d76⟧\n⟦bbg line 3 group=off #d07da113⟧\n⟦bbg line 4 notation=off #bc4b8ba1⟧\n",
    )?;
    app.kill_hard()?;
    let mut again = h.launch("answer-menu-again", Some(&dir), &[])?;
    let s = again.wait_for("the saved sheet", T, |s| answer(s, 4) == Some("10"))?;
    ensure!(
        answer(&s, 0) == Some("0.333")
            && answer(&s, 1) == Some("$1.495")
            && answer(&s, 2) == Some("37035")
            && answer(&s, 3) == Some("900,000"),
        "the formats did not come back: {:?}",
        (0..5).map(|i| answer(&s, i)).collect::<Vec<_>>()
    );
    ensure!(
        s["sheets"][0]["lines"] == 5,
        "the trailer lines are not counted as lines: {}",
        s["sheets"][0]
    );

    // A subtotal line is made under the line, and Delete Line takes a line away.
    let (x, y) = more(&s, 4)?;
    h.click(&again, x, y, "left", 1)?;
    let s = again.wait_for("the menu", T, |s| s["answer_menu"]["line"] == 4)?;
    let (x, y) = item(&s, "subtotal")?;
    h.click(&again, x, y, "left", 1)?;
    let s = again.wait_for("a subtotal line", T, |s| {
        s["lines"][5]["text"] == "subtotal"
    })?;
    ensure!(
        s["answer_menu"].is_null(),
        "the menu closes after Make Subtotal Line"
    );
    ensure!(
        answer(&s, 5).is_some(),
        "the subtotal line has an answer: {:?}",
        answer(&s, 5)
    );
    let (x, y) = more(&s, 5)?;
    h.click(&again, x, y, "left", 1)?;
    let s = again.wait_for("the menu", T, |s| s["answer_menu"]["line"] == 5)?;
    let (x, y) = item(&s, "delete")?;
    h.click(&again, x, y, "left", 1)?;
    let s = again.wait_for("the line deleted", T, |s| {
        s["lines"].as_array().is_some_and(|l| l.len() == 5)
    })?;
    ensure!(
        s["lines"][4]["text"] == "5 + 5" && answer(&s, 2) == Some("37035"),
        "the other lines are untouched"
    );
    again.finish()
}

/// A reference to another line is drawn as a chip holding that line's answer, follows the line when it changes, and
/// is one thing to the caret and to Backspace.
fn line_chips(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("line-chips", None, &[])?;
    h.type_line("12 + 12 + 15")?;
    h.type_line("2 + 2")?;
    h.type_text("line1 / line 2")?;
    let s = app.wait_for("the chips", T, |s| s["lines"][2]["shown"] == "39 / 4")?;
    ensure!(answer(&s, 2) == Some("9.75"), "answer {:?}", answer(&s, 2));
    let chips: Vec<&Value> = s["lines"][2]["spans"]
        .as_array()
        .context("spans")?
        .iter()
        .filter(|x| x["class"] == "lineref")
        .collect();
    ensure!(chips.len() == 2, "two chips, got {chips:?}");
    let (x0, x1) = (f(&chips[0]["x0"])?, f(&chips[0]["x1"])?);
    // The chip is as wide as the answer it holds, which is wider than `line1` is typed... and bright text is on it.
    ensure!(x1 > x0 + 10., "the chip has a width: {x0} to {x1}");
    let shot = h.shot()?;
    let scr = Screen::of(h, &app, &s)?;
    let (y, rh) = (f(&s["lines"][2]["y"])?, f(&s["lines"][2]["row_h"])?);
    let ink = scr
        .brightest(&shot, x0, y + 4., x1, y + rh - 4.)
        .unwrap_or([0, 0, 0]);
    ensure!(
        luminance(ink) > 20.,
        "nothing is painted on the chip: {ink:?}"
    );

    // It follows the line it points at.
    h.press(&["ctrl"], "Home")?;
    h.press(&[], "Down")?;
    h.press(&[], "End")?;
    for _ in 0..5 {
        h.press(&[], "BackSpace")?;
    }
    h.type_text("3 + 4")?;
    app.wait_for("the chip follows line 2", T, |s| {
        s["lines"][2]["shown"] == "39 / 7"
    })?;

    // The caret steps over a whole chip, and Backspace removes the whole reference.
    h.press(&["ctrl"], "End")?;
    app.wait_for("the end of the sheet", T, |s| {
        s["cursor"]["line"] == 2 && s["cursor"]["col"] == 14
    })?;
    h.press(&[], "Left")?;
    app.wait_for("the start of the second reference", T, |s| {
        s["cursor"]["col"] == 8
    })?;
    h.press(&[], "Right")?;
    app.wait_for("past the second reference", T, |s| s["cursor"]["col"] == 14)?;
    h.press(&[], "BackSpace")?;
    let s = app.wait_for("the whole reference gone", T, |s| {
        s["lines"][2]["text"] == "line1 / "
    })?;
    ensure!(
        s["cursor"]["col"] == 8,
        "caret after Backspace: {}",
        s["cursor"]
    );

    // A click on a chip lands at one of its ends, never inside the reference.
    h.type_text("line 2")?;
    let s = app.wait_for("both chips again", T, |s| {
        s["lines"][2]["shown"] == "39 / 7"
    })?;
    let chip = s["lines"][2]["spans"][0].clone();
    ensure!(chip["class"] == "lineref", "first span {chip}");
    let (cx0, cx1) = (f(&chip["x0"])?, f(&chip["x1"])?);
    let (y, rh) = (f(&s["lines"][2]["y"])?, f(&s["lines"][2]["row_h"])?);
    h.click(&app, (cx0 + cx1) / 2. - 3., y + rh / 2., "left", 1)?;
    let s = app.wait_for("the caret next to the chip", T, |s| {
        s["cursor"]["line"] == 2 && s["cursor"]["col"] != 14
    })?;
    ensure!(
        [0u64, 5].contains(&s["cursor"]["col"].as_u64().unwrap_or(99)),
        "a click on the first chip lands at its start or end: {}",
        s["cursor"]
    );
    app.finish()
}

/// A long line wraps onto more rows instead of running under the answers: the rows stack, the answers and line
/// numbers stay on the first row, a click on a lower row puts the caret there, Up and Down move between the rows,
/// and a wider window needs fewer rows.
fn wrapping(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("wrapping", None, &[])?;
    let long = "the quick brown fox jumps over the lazy dog ".repeat(7);
    h.type_line(long.trim_end())?;
    h.type_text("7 + 8")?;
    let s = app.wait_for("a wrapped line and the one after it", T, |s| {
        s["lines"][0]["rows"].as_u64().is_some_and(|r| r >= 2) && answer(s, 1) == Some("15")
    })?;
    let rows = s["lines"][0]["rows"].as_u64().unwrap_or(1);
    let (y0, y1, rh) = (
        f(&s["lines"][0]["y"])?,
        f(&s["lines"][1]["y"])?,
        f(&s["lines"][0]["row_h"])?,
    );
    ensure!(
        (y1 - y0 - rows as f32 * rh).abs() < 0.5,
        "line 2 starts {} below line 1, which has {rows} rows of {rh}",
        y1 - y0
    );
    ensure!(
        s["total_rows"].as_u64() == Some(rows + 1),
        "total rows {} against {rows} and one",
        s["total_rows"]
    );
    let starts: Vec<u64> = s["lines"][0]["row_starts"]
        .as_array()
        .context("row starts")?
        .iter()
        .filter_map(Value::as_u64)
        .collect();
    ensure!(
        starts.len() as u64 == rows && starts[0] == 0,
        "row starts {starts:?}"
    );

    // Nothing is painted between the text column and the answers, and the second row has text on it.
    let shot = h.shot()?;
    let scr = Screen::of(h, &app, &s)?;
    let (text_x, text_w, answer_x) = (f(&s["text_x"])?, f(&s["text_w"])?, f(&s["answer_x"])?);
    for k in 0..rows as usize {
        let row_y = y0 + k as f32 * rh;
        let text = scr
            .brightest(&shot, text_x, row_y + 4., text_x + text_w, row_y + rh - 4.)
            .unwrap_or([0, 0, 0]);
        ensure!(
            luminance(text) > 150.,
            "no text on row {k} of the wrapped line"
        );
        let gap = scr
            .brightest(
                &shot,
                text_x + text_w + 10.,
                row_y + 4.,
                answer_x - 4.,
                row_y + rh - 4.,
            )
            .unwrap_or([0, 0, 0]);
        ensure!(
            luminance(gap) < 120.,
            "text runs past the wrap width on row {k}: {gap:?}"
        );
    }

    // A click on the second row puts the caret in the second row of the text.
    h.click(&app, text_x + 90., y0 + rh * 1.5, "left", 1)?;
    let s = app.wait_for("the caret on the second row", T, |s| {
        s["cursor"]["line"] == 0 && s["cursor"]["col"].as_u64().is_some_and(|c| c >= starts[1])
    })?;
    if starts.len() > 2 {
        ensure!(
            s["cursor"]["col"].as_u64().is_some_and(|c| c < starts[2]),
            "the caret is past the second row: {}",
            s["cursor"]
        );
    }
    // Up goes to the first row of the same line, Down comes back, and Down from the last row leaves the line.
    h.press(&[], "Up")?;
    app.wait_for("the caret on the first row", T, |s| {
        s["cursor"]["line"] == 0 && s["cursor"]["col"].as_u64().is_some_and(|c| c < starts[1])
    })?;
    h.press(&[], "Down")?;
    app.wait_for("the caret back on the second row", T, |s| {
        s["cursor"]["line"] == 0 && s["cursor"]["col"].as_u64().is_some_and(|c| c >= starts[1])
    })?;
    for _ in 1..rows {
        h.press(&[], "Down")?;
    }
    app.wait_for("the caret on the next line", T, |s| {
        s["cursor"]["line"] == 1
    })?;
    // End of the wrapped line, then typing there scrolls and re-wraps without losing text.
    h.press(&["ctrl"], "Home")?;
    h.press(&[], "End")?;
    app.wait_for("the end of the first row's line", T, |s| {
        s["cursor"]["line"] == 0
            && s["cursor"]["col"]
                .as_u64()
                .is_some_and(|c| c as usize >= long.trim_end().len())
    })?;

    // A narrower window takes more rows than a wider one. The app reports the size it last painted.
    let w0 = f(&s["window"]["w"])? + f(&s["window"]["x"])?;
    let mut seen = Vec::new();
    for w in [1000u32, 2400] {
        h.resize(&app, w, 700)?;
        let mut rows_now = 0;
        for _ in 0..40 {
            let s = app.state()?;
            let right =
                s["window"]["x"].as_f64().unwrap_or(0.) + s["window"]["w"].as_f64().unwrap_or(0.);
            rows_now = s["lines"][0]["rows"].as_u64().unwrap_or(0);
            if (right - f64::from(w)).abs() <= 2. {
                break;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        settle();
        rows_now = app.state()?["lines"][0]["rows"]
            .as_u64()
            .unwrap_or(rows_now);
        seen.push((w, rows_now));
    }
    ensure!(
        seen[0].1 > seen[1].1 && seen[1].1 >= 1,
        "rows by width: {seen:?}, {rows} at the start width {w0}"
    );
    app.finish()
}

/// A scripted input method composes text in the real window through the Wayland input-method protocol, the way
/// fcitx or ibus would: the composition shows in the line but is not the sheet's text until it is committed,
/// Return does not end a composition, and a commit puts the text in and clears the mark.
fn ime_composition(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("ime-composition", None, &[])?;
    h.type_text("2 + 2 = ")?;
    app.wait_for("the typed text", T, |s| s["lines"][0]["text"] == "2 + 2 = ")?;
    ensure!(
        app.state()?["composition"].is_null(),
        "a composition at the start"
    );

    h.compose("ni")?;
    let s = app.wait_for("the composition shown", T, |s| s["composition"] == "ni")?;
    ensure!(
        s["lines"][0]["text"] == "2 + 2 = ni",
        "the composition is in the line while it is composed: {}",
        s["lines"][0]
    );
    // A longer composition replaces the first, still one mark.
    h.compose("nihao")?;
    app.wait_for("the composition replaced", T, |s| {
        s["composition"] == "nihao" && s["lines"][0]["text"] == "2 + 2 = nihao"
    })?;
    // Taking it away leaves the text as it was.
    h.clear_composition()?;
    app.wait_for("the composition cleared", T, |s| {
        s["composition"].is_null() && s["lines"][0]["text"] == "2 + 2 = "
    })?;
    // A commit makes it text.
    h.compose("ni")?;
    app.wait_for("a new composition", T, |s| s["composition"] == "ni")?;
    h.commit_composition("你好")?;
    let s = app.wait_for("the committed text", T, |s| {
        s["composition"].is_null() && s["lines"][0]["text"] == "2 + 2 = 你好"
    })?;
    ensure!(
        s["cursor"]["col"].as_u64() == Some("2 + 2 = 你好".len() as u64),
        "the caret is after the committed text: {}",
        s["cursor"]
    );
    // Typing on the keyboard carries on after it, and the line still works out.
    h.type_text("x")?;
    app.wait_for("keyboard text after a commit", T, |s| {
        s["lines"][0]["text"] == "2 + 2 = 你好x"
    })?;
    app.finish()
}

/// Ctrl+Shift+E writes the open sheet with its answers as text, CSV, HTML and PDF; the PDF is checked with the
/// system's own tools, which read it back.
fn export_sheet(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("export", None, &[])?;
    let dir = app.data_dir.clone();
    h.type_line("# Trip")?;
    h.type_line("1 + 2")?;
    h.type_line("hotel 3 nights × $120")?;
    h.type_text("// a note")?;
    app.wait_for("the lines worked out", T, |s| {
        answer(s, 1) == Some("3") && answer(s, 2) == Some("$360.00")
    })?;
    ensure!(
        !app.state()?["notice"].is_string(),
        "a notice is showing before the export"
    );
    h.press(&["ctrl", "shift"], "e")?;
    let s = app.wait_for("the export notice", T, |s| {
        s["notice"]
            .as_str()
            .is_some_and(|n| n.starts_with("Exported"))
    })?;
    let exports = dir.join("exports");
    ensure!(
        s["notice"]
            .as_str()
            .unwrap_or("")
            .contains(&exports.display().to_string()),
        "the notice names the folder: {}",
        s["notice"]
    );
    let name = s["current"].as_str().context("current")?.to_owned();
    // The one file of each kind; its name is the sheet's, made safe.
    let file = |ext: &str| -> anyhow::Result<PathBuf> {
        let found: Vec<PathBuf> = std::fs::read_dir(&exports)
            .with_context(|| format!("{}", exports.display()))?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == ext))
            .collect();
        ensure!(
            found.len() == 1,
            "{} {ext} files in {}: {found:?}",
            found.len(),
            exports.display()
        );
        Ok(found[0].clone())
    };
    let read = |ext: &str| -> anyhow::Result<String> {
        let path = file(ext)?;
        std::fs::read_to_string(&path).with_context(|| format!("{}", path.display()))
    };
    let txt = read("txt")?;
    ensure!(txt.contains("# Trip\n"), "text: {txt:?}");
    ensure!(
        txt.contains("1 + 2") && txt.contains("| 3\n"),
        "text: {txt:?}"
    );
    ensure!(txt.contains("| $360.00"), "text: {txt:?}");
    ensure!(txt.contains("// a note\n"), "text: {txt:?}");
    let csv = read("csv")?;
    ensure!(
        csv.contains("1 + 2,3\r\n") && csv.contains("hotel 3 nights × $120,$360.00\r\n"),
        "csv: {csv:?}"
    );
    let html = read("html")?;
    ensure!(
        html.starts_with("<!doctype html>") && html.contains("<td class=\"answer\">$360.00</td>"),
        "html"
    );
    // The pdf is read back by qpdf (structure) and pdftotext (text), not by this code.
    let pdf = file("pdf")?;
    let check = std::process::Command::new("qpdf")
        .arg("--check")
        .arg(&pdf)
        .output()?;
    ensure!(
        check.status.success(),
        "qpdf --check failed: {}",
        String::from_utf8_lossy(&check.stderr)
    );
    let layout = std::process::Command::new("pdftotext")
        .args(["-layout"])
        .arg(&pdf)
        .arg("-")
        .output()?;
    ensure!(layout.status.success(), "pdftotext failed");
    let words = String::from_utf8_lossy(&layout.stdout).into_owned();
    ensure!(
        words.contains("Trip") && words.contains("1 + 2") && words.contains("$360.00"),
        "pdftotext read: {words:?}"
    );
    // The folder of exports does not turn up as a sheet.
    let s = app.state()?;
    ensure!(
        s["sheets"].as_array().is_some_and(|a| a.len() == 1) && s["current"] == name.as_str(),
        "the exports folder shows as a sheet: {}",
        s["sheets"]
    );
    app.finish()
}

/// Copy, cut and paste of several lines, between sheets: the answers are worked out again where they land.
fn clipboard_lines(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("clipboard-lines", None, &[])?;
    h.type_line("5 + 5")?;
    h.type_line("10 × 2")?;
    h.type_text("$3 + $4")?;
    app.wait_for("three answers", T, |s| answer(s, 2) == Some("$7.00"))?;
    h.press(&["ctrl"], "a")?;
    h.press(&["ctrl"], "c")?;
    h.press(&["ctrl"], "n")?;
    app.wait_for("a new empty sheet", T, |s| {
        s["sheets"].as_array().is_some_and(|a| a.len() == 2) && s["lines"][0]["text"] == ""
    })?;
    h.press(&["ctrl"], "v")?;
    let s = app.wait_for("the pasted lines and their answers", T, |s| {
        answer(s, 0) == Some("10") && answer(s, 1) == Some("20") && answer(s, 2) == Some("$7.00")
    })?;
    ensure!(s["lines"].as_array().is_some_and(|l| l.len() == 3));
    // Cut empties the sheet; paste brings every line back.
    h.press(&["ctrl"], "a")?;
    h.press(&["ctrl"], "x")?;
    app.wait_for("the sheet emptied by the cut", T, |s| {
        s["lines"].as_array().is_some_and(|l| l.len() == 1) && s["lines"][0]["text"] == ""
    })?;
    h.press(&["ctrl"], "v")?;
    app.wait_for("the lines back", T, |s| {
        answer(s, 1) == Some("20") && answer(s, 2) == Some("$7.00")
    })?;
    app.finish()
}

fn quit_saves(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("quit", None, &[])?;
    let dir = app.data_dir.clone();
    h.type_text("3 × 3")?;
    let s = app.wait_for("the answer", T, |s| answer(s, 0) == Some("9"))?;
    let id = s["current"].as_str().context("current")?.to_owned();
    // Ctrl+Q straight after the last key, before the debounce could have written anything.
    h.press(&["ctrl"], "q")?;
    app.wait_exit(Duration::from_secs(8))?;
    let got = read_sheet(&dir, &id);
    if got.as_deref() != Some("3 × 3\n") {
        bail!("the sheet on disk after Ctrl+Q is {got:?}");
    }
    Ok(())
}

const LATTE: &str = "mode = \"light\"\naccent = \"#1e66f5\"\nbackground = \"#eff1f5\"\nforeground = \"#4c4f69\"\nyellow = \"#df8e1d\"\ngreen = \"#40a02b\"\ncyan = \"#179299\"\nblue = \"#1e66f5\"\nmagenta = \"#ea76cb\"\n";
const LIFTOFF: &str = "mode = \"dark\"\naccent = \"#F3B77B\"\nbackground = \"#151E26\"\nforeground = \"#ECE5D8\"\nyellow = \"#EAD093\"\ngreen = \"#A9C59A\"\ncyan = \"#8BC8D0\"\nblue = \"#7BBCE0\"\nmagenta = \"#C3ABCC\"\n";

/// How bright the page is: the brightest pixel of an empty strip of the editor, below the typed lines.
fn page_brightness(h: &Harness, app: &mut App) -> anyhow::Result<f32> {
    let s = app.state()?;
    let scr = Screen::of(h, app, &s)?;
    let (w, hh) = (f(&s["window"]["w"])?, f(&s["window"]["h"])?);
    let shot = h.shot()?;
    let p = scr
        .brightest(&shot, w * 0.4, hh - 90., w * 0.5, hh - 50.)
        .context("the strip is off screen")?;
    Ok(luminance(p))
}

/// A light Omarchy theme paints a light page and a switch to a dark one is followed without a restart. With no theme
/// file the built-in theme follows `BABBAGE_APPEARANCE`.
fn light_theme(h: &mut Harness) -> anyhow::Result<()> {
    let dir = h.logs.join("light-theme");
    std::fs::create_dir_all(&dir)?;
    let theme = dir.join("colors.toml");
    std::fs::write(&theme, LATTE)?;
    let theme = theme.to_string_lossy().into_owned();
    let mut app = h.launch("light-theme", None, &[("BABBAGE_THEME_FILE", &theme)])?;
    h.resize(&app, 900, 520)?;
    for line in ["# Trip", "hotel = $180", "3 * hotel + 25%"] {
        h.type_line(line)?;
    }
    app.wait_for("the answer", T, |s| answer(s, 2) == Some("$675.00"))?;
    settle();
    let light = page_brightness(h, &mut app)?;
    ensure!(
        light > 200.,
        "a light theme drew a page of brightness {light}"
    );
    guide_picture(h, &app, "theme-light", 900, 520)?;

    std::fs::write(&theme, LIFTOFF)?;
    let start = std::time::Instant::now();
    loop {
        let dark = page_brightness(h, &mut app)?;
        if dark < 80. {
            break;
        }
        ensure!(
            start.elapsed() < T,
            "the page stayed bright ({dark}) after the theme file became dark"
        );
        std::thread::sleep(Duration::from_millis(500));
    }
    app.finish()?;

    for (appearance, bright) in [("light", true), ("dark", false)] {
        let mut app = h.launch(
            &format!("appearance-{appearance}"),
            None,
            &[("BABBAGE_APPEARANCE", appearance)],
        )?;
        h.type_line("1 + 1")?;
        app.wait_for("the answer", T, |s| answer(s, 0) == Some("2"))?;
        settle();
        let b = page_brightness(h, &mut app)?;
        ensure!(
            (b > 200.) == bright && (bright || b < 80.),
            "BABBAGE_APPEARANCE={appearance} drew a page of brightness {b}"
        );
        app.finish()?;
    }
    Ok(())
}

fn no_real_desktop(h: &mut Harness) -> anyhow::Result<()> {
    let before = h.host_windows_of("babbage")?;
    let app = h.launch("desktop", None, &[])?;
    let nested = h.nested_windows_of("babbage")?;
    let after = h.host_windows_of("babbage")?;
    ensure!(nested == 1, "{nested} windows in the nested compositor");
    ensure!(
        before == 0 && after == 0,
        "babbage windows on the real desktop: {before} before, {after} after"
    );
    app.finish()
}

/// One scratch home for the command-mode and installer checks: a fake HOME with its own sheet folder and settings file,
/// and the clock and zone pinned. The program is run with nothing else from the environment.
struct Cli {
    home: PathBuf,
    program: PathBuf,
}

/// What one run printed and returned.
struct Ran {
    code: i32,
    out: String,
    err: String,
}

impl Cli {
    fn new(h: &Harness, name: &str) -> anyhow::Result<Cli> {
        let home = h.logs.join("cli").join(name);
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join("sheets"))?;
        Ok(Cli {
            home,
            program: h.repo.join("target/release/babbage"),
        })
    }

    /// Runs `program` with `args`, standard input from `stdin` (empty when None) and `extra` set on top of the scratch
    /// environment.
    fn exec(
        &self,
        program: &Path,
        args: &[&str],
        stdin: Option<&str>,
        extra: &[(&str, &str)],
    ) -> anyhow::Result<Ran> {
        use std::io::Write as _;
        let mut cmd = std::process::Command::new(program);
        cmd.args(args)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", &self.home)
            .env("BABBAGE_DATA_DIR", self.home.join("sheets"))
            .env("BABBAGE_CONFIG_FILE", self.home.join("settings"))
            .env("BABBAGE_NOW", FIXED_NOW)
            .env("BABBAGE_UPDATES", "off")
            .env("TZ", "UTC")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        for (k, v) in extra {
            cmd.env(k, v);
        }
        let mut child = cmd
            .spawn()
            .with_context(|| format!("run {}", program.display()))?;
        child
            .stdin
            .take()
            .context("standard input")?
            .write_all(stdin.unwrap_or("").as_bytes())?;
        let out = child.wait_with_output()?;
        Ok(Ran {
            code: out.status.code().context("the program was killed")?,
            out: String::from_utf8_lossy(&out.stdout).into_owned(),
            err: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }

    /// `babbage calc ARGS`: the command mode under its subcommand name.
    fn calc(&self, args: &[&str], stdin: Option<&str>) -> anyhow::Result<Ran> {
        let mut all = vec!["calc"];
        all.extend_from_slice(args);
        self.exec(&self.program, &all, stdin, &[])
    }
}

/// Expression mode: the words are joined into one line and only the answer is printed; a line with no answer prints
/// nothing and exits 0. The answers are the window's (20% of 85 is 17).
fn cli_expression(h: &mut Harness) -> anyhow::Result<()> {
    let cli = Cli::new(h, "cli-expression")?;
    let r = cli.calc(&["20%", "of", "85"], None)?;
    ensure!(
        r.code == 0 && r.out == "17\n",
        "20% of 85: {} {:?} {:?}",
        r.code,
        r.out,
        r.err
    );
    let r = cli.calc(&["-5", "+", "3"], None)?;
    ensure!(
        r.code == 0 && r.out == "-2\n",
        "-5 + 3: {} {:?}",
        r.code,
        r.out
    );
    let r = cli.calc(&["-p", "4", "1/3"], None)?;
    ensure!(
        r.code == 0 && r.out == "≈ 0.3333\n",
        "-p 4 1/3: {} {:?}",
        r.code,
        r.out
    );
    // A line that sets a variable answers with its value, as in the window; a note answers nothing.
    let r = cli.calc(&["rent", "=", "$900"], None)?;
    ensure!(
        r.code == 0 && r.out == "$900.00\n",
        "a declaration: {} {:?}",
        r.code,
        r.out
    );
    let r = cli.calc(&["// a note"], None)?;
    ensure!(
        r.code == 0 && r.out.is_empty(),
        "a note: {} {:?}",
        r.code,
        r.out
    );
    Ok(())
}

/// Errors: a line with an error prints its message on stderr and exits 1; bad usage exits 2 with the usage on stderr.
fn cli_errors(h: &mut Harness) -> anyhow::Result<()> {
    let cli = Cli::new(h, "cli-errors")?;
    let r = cli.calc(&["1", "+"], None)?;
    ensure!(
        r.code == 1 && r.out.is_empty() && !r.err.is_empty(),
        "1 +: {} {:?} {:?}",
        r.code,
        r.out,
        r.err
    );
    let r = cli.calc(&["--bogus"], None)?;
    ensure!(
        r.code == 2 && r.err.contains("usage: bbg"),
        "--bogus: {} {:?}",
        r.code,
        r.err
    );
    let r = cli.calc(&["-p", "11", "1"], None)?;
    ensure!(
        r.code == 2 && r.err.contains("0 to 10"),
        "-p 11: {} {:?}",
        r.code,
        r.err
    );
    let r = cli.calc(&["-"], Some("1 +\n"))?;
    ensure!(
        r.code == 1 && r.out.contains("error:"),
        "a sheet with an error line: {} {:?}",
        r.code,
        r.out
    );
    Ok(())
}

/// Sheets from standard input and from a file: each line prints its text and its answer in two aligned columns, and a
/// file's trailer line still sets how its answer is written.
fn cli_sheet_input(h: &mut Harness) -> anyhow::Result<()> {
    let cli = Cli::new(h, "cli-sheet-input")?;
    let r = cli.calc(&["-"], Some("Lunch\n12 + 8\n\n"))?;
    ensure!(
        r.code == 0 && r.out == "Lunch\n12 + 8  20\n",
        "stdin sheet: {} {:?}",
        r.code,
        r.out
    );
    let r = cli.calc(&["-"], Some(""))?;
    ensure!(
        r.code == 0 && r.out.is_empty(),
        "an empty stdin sheet: {} {:?}",
        r.code,
        r.out
    );
    let file = cli.home.join("budget.bbg");
    std::fs::write(&file, "1/3\n⟦bbg line 1 dp=2⟧\n")?;
    let path = file.to_string_lossy().into_owned();
    let r = cli.calc(&[path.as_str()], None)?;
    ensure!(
        r.code == 0 && r.out == "1/3  ≈ 0.33\n",
        "file sheet with a trailer: {} {:?}",
        r.code,
        r.out
    );
    Ok(())
}

/// `-a`: only the answers, one per line, blank when a line has none; errors go to stderr and still exit 1.
fn cli_answers(h: &mut Harness) -> anyhow::Result<()> {
    let cli = Cli::new(h, "cli-answers")?;
    let r = cli.calc(&["-a"], Some("Lunch\n12 + 8\n1 +\n"))?;
    ensure!(
        r.code == 1 && r.out == "\n20\n\n" && !r.err.is_empty(),
        "-a: {} {:?} {:?}",
        r.code,
        r.out,
        r.err
    );
    Ok(())
}

/// `-t`: the sheet's total as a last row that lines up with the answers; with `-a` it is one more answer line.
fn cli_total(h: &mut Harness) -> anyhow::Result<()> {
    let cli = Cli::new(h, "cli-total")?;
    let r = cli.calc(&["-t"], Some("10\n20\n"))?;
    ensure!(
        r.code == 0 && r.out == "10     10\n20     20\nTotal  30\n",
        "-t: {} {:?}",
        r.code,
        r.out
    );
    let r = cli.calc(&["-a", "-t"], Some("10\n20\n"))?;
    ensure!(
        r.code == 0 && r.out == "10\n20\n30\n",
        "-a -t: {} {:?}",
        r.code,
        r.out
    );
    Ok(())
}

/// `--json`: one object per line with text, answer, exact and error; `-t` adds a `total` object.
fn cli_json(h: &mut Harness) -> anyhow::Result<()> {
    let cli = Cli::new(h, "cli-json")?;
    let r = cli.calc(&["--json", "-t"], Some("12 + 8\n1 +\n"))?;
    ensure!(r.code == 1, "--json exit code {}", r.code);
    let objects: Vec<Value> = r
        .out
        .lines()
        .map(|l| serde_json::from_str(l).with_context(|| format!("not JSON: {l}")))
        .collect::<anyhow::Result<_>>()?;
    ensure!(objects.len() == 3, "{} objects: {}", objects.len(), r.out);
    ensure!(
        objects[0]["text"] == "12 + 8"
            && objects[0]["answer"] == "20"
            && objects[0]["exact"] == true,
        "first line: {}",
        objects[0]
    );
    ensure!(
        objects[1]["answer"].is_null() && objects[1]["error"].is_string(),
        "error line: {}",
        objects[1]
    );
    ensure!(
        objects[2]["total"]["answer"] == "20",
        "total: {}",
        objects[2]
    );
    Ok(())
}

/// `-s NAME`: a saved sheet is read from the sheet folder by its name; a name with no sheet exits 1.
fn cli_saved(h: &mut Harness) -> anyhow::Result<()> {
    let cli = Cli::new(h, "cli-saved")?;
    std::fs::write(cli.home.join("sheets/Trip.bbg"), "Hotel\n3 * $120\n")?;
    let r = cli.calc(&["-s", "Trip"], None)?;
    ensure!(
        r.code == 0 && r.out == "Hotel\n3 * $120  $360.00\n",
        "-s Trip: {} {:?}",
        r.code,
        r.out
    );
    let r = cli.calc(&["-s", "Nope"], None)?;
    ensure!(
        r.code == 1 && r.err.contains("no sheet named"),
        "-s Nope: {} {:?}",
        r.code,
        r.err
    );
    Ok(())
}

/// `-r`: a repeating decimal is written with its repeating part in brackets; without it the ordinary answer.
fn cli_repeating(h: &mut Harness) -> anyhow::Result<()> {
    let cli = Cli::new(h, "cli-repeating")?;
    let r = cli.calc(&["-r", "1/3"], None)?;
    ensure!(
        r.code == 0 && r.out == "0.(3)\n",
        "-r 1/3: {} {:?}",
        r.code,
        r.out
    );
    let r = cli.calc(&["-r", "1/7"], None)?;
    ensure!(
        r.code == 0 && r.out == "0.(142857)\n",
        "-r 1/7: {} {:?}",
        r.code,
        r.out
    );
    let r = cli.calc(&["1/3"], None)?;
    ensure!(
        r.code == 0 && r.out == "≈ 0.3333333333\n",
        "1/3: {} {:?}",
        r.code,
        r.out
    );
    Ok(())
}

/// The name decides the mode: a symlink named `bbg` runs the command-line calculator, with the words as its
/// expression, or with standard input as its sheet; `BABBAGE_AS=bbg` does the same for any name.
fn cli_bbg_name(h: &mut Harness) -> anyhow::Result<()> {
    let cli = Cli::new(h, "cli-bbg-name")?;
    let bin = cli.home.join("bin");
    std::fs::create_dir_all(&bin)?;
    let link = bin.join("bbg");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(&cli.program, &link)?;
    let r = cli.exec(&link, &["20%", "of", "85"], None, &[])?;
    ensure!(
        r.code == 0 && r.out == "17\n",
        "bbg 20% of 85: {} {:?} {:?}",
        r.code,
        r.out,
        r.err
    );
    let r = cli.exec(&link, &[], Some("2 + 2\n"), &[])?;
    ensure!(
        r.code == 0 && r.out == "2 + 2  4\n",
        "bbg with a sheet on standard input: {} {:?}",
        r.code,
        r.out
    );
    let r = cli.exec(
        &cli.program,
        &["20%", "of", "85"],
        None,
        &[("BABBAGE_AS", "bbg")],
    )?;
    ensure!(
        r.code == 0 && r.out == "17\n",
        "BABBAGE_AS=bbg: {} {:?}",
        r.code,
        r.out
    );
    Ok(())
}

/// `bbg --completions SHELL` prints a script for bash, zsh and fish and refuses any other shell, and `--list-sheets`
/// names the saved sheets while leaving the folder as it found it (tab completion runs while the window may be saving).
fn cli_completions(h: &mut Harness) -> anyhow::Result<()> {
    let cli = Cli::new(h, "cli-completions")?;
    let bbg = [("BABBAGE_AS", "bbg")];
    for (shell, first, needle) in [
        ("bash", "# bash completion for bbg", "complete -F _bbg bbg"),
        ("zsh", "#compdef bbg", "_arguments"),
        ("fish", "# fish completion for bbg", "complete -c bbg"),
    ] {
        let r = cli.exec(&cli.program, &["--completions", shell], None, &bbg)?;
        ensure!(
            r.code == 0
                && r.out.starts_with(first)
                && r.out.contains(needle)
                && r.out.contains("places"),
            "--completions {shell}: {} {:?} {:?}",
            r.code,
            r.out,
            r.err
        );
    }
    let r = cli.exec(&cli.program, &["--completions", "powershell"], None, &bbg)?;
    ensure!(
        r.code == 2 && r.out.is_empty() && r.err.contains("use bash, zsh or fish"),
        "--completions powershell: {} {:?} {:?}",
        r.code,
        r.out,
        r.err
    );
    let r = cli.exec(&cli.program, &["--completions"], None, &bbg)?;
    ensure!(
        r.code == 2,
        "--completions with no shell: {} {:?}",
        r.code,
        r.err
    );

    // The names of the saved sheets, from a folder that has other files in it. A half-written temporary file stays.
    let sheets = cli.home.join("sheets");
    std::fs::write(sheets.join("Budget.bbg"), "1 + 1\n")?;
    std::fs::write(sheets.join("Trip to Rome.bbg"), "2 + 2\n")?;
    std::fs::write(sheets.join("notes.txt"), "x")?;
    let temp = sheets.join("Budget.bbg.tmp");
    std::fs::write(&temp, "half")?;
    let r = cli.exec(&cli.program, &["--list-sheets"], None, &bbg)?;
    ensure!(
        r.code == 0 && r.out == "Budget\nTrip to Rome\n",
        "--list-sheets: {} {:?} {:?}",
        r.code,
        r.out,
        r.err
    );
    ensure!(temp.exists(), "--list-sheets cleared a temporary file");

    // No sheet folder yet: nothing is listed and nothing is created.
    std::fs::remove_dir_all(&sheets)?;
    let r = cli.exec(&cli.program, &["--list-sheets"], None, &bbg)?;
    ensure!(
        r.code == 0 && r.out.is_empty() && r.err.is_empty(),
        "--list-sheets without a folder: {} {:?} {:?}",
        r.code,
        r.out,
        r.err
    );
    ensure!(!sheets.exists(), "--list-sheets created the sheet folder");
    Ok(())
}

/// The sha256 line `sha256sum` writes for `file`, named by its file name, as a .sha256 file holds it.
fn sha256_file(file: &Path) -> anyhow::Result<String> {
    let dir = file.parent().context("a folder for the file")?;
    let name = file.file_name().context("a file name")?;
    let out = std::process::Command::new("sha256sum")
        .current_dir(dir)
        .arg(name)
        .output()?;
    ensure!(out.status.success(), "sha256sum {}", file.display());
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The one-line installer, against a local release server on a scratch home. A good download installs the AppImage
/// and both commands, and `bbg` answers. A wrong checksum stops the install and leaves no files behind. A second run
/// changes nothing. `--uninstall` removes what was installed and nothing else. `--tarball` installs the program
/// from the tarball.
fn install_script(h: &mut Harness) -> anyhow::Result<()> {
    use crate::releases::Releases;
    use std::os::unix::fs::PermissionsExt;
    let cli = Cli::new(h, "install-script")?;
    let stage = cli.home.join("stage");
    std::fs::create_dir_all(&stage)?;
    let version = "99.2.0";
    let arch = std::env::consts::ARCH;
    let appimage = format!("babbage-{version}-{arch}.AppImage");
    let tarball = format!("babbage-{version}-{arch}.tar.gz");
    // The real program stands in for the AppImage, so the installed commands really run.
    let program_bytes = std::fs::read(&cli.program)?;
    std::fs::write(stage.join(&appimage), &program_bytes)?;
    // The tarball holds the same program, a bbg link, LICENSE and README.md, as bin/package makes it.
    let tar_dir = stage.join("tar");
    std::fs::create_dir_all(&tar_dir)?;
    std::fs::write(tar_dir.join("babbage"), &program_bytes)?;
    std::fs::set_permissions(
        tar_dir.join("babbage"),
        std::fs::Permissions::from_mode(0o755),
    )?;
    std::os::unix::fs::symlink("babbage", tar_dir.join("bbg"))?;
    std::fs::copy(h.repo.join("LICENSE"), tar_dir.join("LICENSE"))?;
    std::fs::copy(h.repo.join("README.md"), tar_dir.join("README.md"))?;
    let status = std::process::Command::new("tar")
        .arg("-czf")
        .arg(stage.join(&tarball))
        .arg("-C")
        .arg(&tar_dir)
        .args(["babbage", "bbg", "LICENSE", "README.md"])
        .status()?;
    ensure!(status.success(), "tar failed");
    // A release folder served over HTTP: the version, then each file and its .sha256. `bad` puts a wrong sum on each.
    std::fs::write(stage.join("babbage-version.txt"), format!("{version}\n"))?;
    // `bad` gives the AppImage a wrong .sha256; everything else is served as it is.
    let release = |bad: bool| -> anyhow::Result<Vec<(String, Vec<u8>)>> {
        let mut files = Vec::new();
        for name in ["babbage-version.txt", appimage.as_str(), tarball.as_str()] {
            let body = std::fs::read(stage.join(name))?;
            let sum = if bad && name == appimage {
                format!("{}  {name}\n", "0".repeat(64))
            } else {
                sha256_file(&stage.join(name))?
            };
            files.push((format!("/download/{name}"), body));
            files.push((format!("/download/{name}.sha256"), sum.into_bytes()));
        }
        Ok(files)
    };
    let good = Releases::start_files(release(false)?)?;
    let bad = Releases::start_files(release(true)?)?;
    let installer = h.repo.join("install.sh").to_string_lossy().into_owned();
    let share = cli.home.join(".local/share/babbage");
    let bin = cli.home.join(".local/bin");
    let good_url = format!("{}/download", good.base);
    let bad_url = format!("{}/download", bad.base);
    let tmp = cli.home.join("tmp");
    std::fs::create_dir_all(&tmp)?;
    let tmp_s = tmp.to_string_lossy().into_owned();

    // A wrong checksum: the install stops, and nothing is written under HOME or in the temporary folder.
    let r = cli.exec(
        Path::new("/bin/bash"),
        &[installer.as_str()],
        None,
        &[
            ("BABBAGE_RELEASE_URL", bad_url.as_str()),
            ("TMPDIR", tmp_s.as_str()),
        ],
    )?;
    ensure!(
        r.code != 0 && r.err.contains("checksum mismatch"),
        "a bad checksum: {} {:?}",
        r.code,
        r.err
    );
    ensure!(
        !cli.home.join(".local").exists(),
        "a bad checksum left files under HOME"
    );
    ensure!(
        std::fs::read_dir(&tmp)?.next().is_none(),
        "a bad checksum left files in the temporary folder"
    );

    // A fresh install of the AppImage, and the commands it writes.
    let env = [
        ("BABBAGE_RELEASE_URL", good_url.as_str()),
        ("TMPDIR", tmp_s.as_str()),
    ];
    let r = cli.exec(Path::new("/bin/bash"), &[installer.as_str()], None, &env)?;
    ensure!(r.code == 0, "install: {} {:?}", r.code, r.err);
    ensure!(
        std::fs::read(share.join("babbage.AppImage"))? == program_bytes,
        "the installed AppImage is not the one served"
    );
    ensure!(
        std::fs::read_to_string(share.join("installed-version"))? == format!("{version}\n"),
        "installed-version"
    );
    for name in ["babbage", "bbg"] {
        let mode = std::fs::metadata(bin.join(name))?.permissions().mode();
        ensure!(mode & 0o111 != 0, "{name} is not executable");
    }
    // Tab completion for bash is a file of its own, written by the installed program; fish is not set up here, so
    // there is no fish file.
    let bash_file = cli
        .home
        .join(".local/share/bash-completion/completions/bbg");
    let fish_file = cli.home.join(".config/fish/completions/bbg.fish");
    let text = std::fs::read_to_string(&bash_file).context("the bash completion file")?;
    ensure!(
        text.starts_with("# Written by the Babbage installer")
            && text.contains("complete -F _bbg bbg"),
        "the bash completion file: {text:?}"
    );
    ensure!(
        !cli.home.join(".config").exists(),
        "the installer made a fish folder for someone who does not use fish"
    );
    let r = cli.exec(&bin.join("bbg"), &["20%", "of", "85"], None, &[])?;
    ensure!(
        r.code == 0 && r.out == "17\n",
        "installed bbg: {} {:?} {:?}",
        r.code,
        r.out,
        r.err
    );

    // A second run changes nothing and says so.
    let r = cli.exec(Path::new("/bin/bash"), &[installer.as_str()], None, &env)?;
    ensure!(
        r.code == 0 && r.out.contains("already installed"),
        "second run: {} {:?}",
        r.code,
        r.out
    );
    ensure!(
        std::fs::read(share.join("babbage.AppImage"))? == program_bytes,
        "the second run changed the AppImage"
    );

    // Uninstall removes what it installed; a file of the user's in the same folder stays.
    std::fs::write(bin.join("keep-me"), "mine\n")?;
    let r = cli.exec(
        Path::new("/bin/bash"),
        &[installer.as_str(), "--uninstall"],
        None,
        &env,
    )?;
    ensure!(r.code == 0, "uninstall: {} {:?}", r.code, r.err);
    ensure!(
        !bin.join("babbage").exists() && !bin.join("bbg").exists() && !share.exists(),
        "uninstall left files behind"
    );
    ensure!(
        bin.join("keep-me").is_file(),
        "uninstall removed a file it did not install"
    );
    std::fs::remove_file(bin.join("keep-me"))?;
    ensure!(!bash_file.exists(), "uninstall left the completion file");

    // A fish user's own file of the same name is never overwritten or removed.
    std::fs::create_dir_all(fish_file.parent().context("a folder")?)?;
    std::fs::write(&fish_file, "# mine\n")?;

    // The tarball: the program itself, not the AppImage, and the commands still answer.
    let r = cli.exec(
        Path::new("/bin/bash"),
        &[installer.as_str(), "--tarball"],
        None,
        &env,
    )?;
    ensure!(r.code == 0, "--tarball install: {} {:?}", r.code, r.err);
    ensure!(
        share.join("babbage").is_file() && share.join("LICENSE").is_file(),
        "the tarball's files are not installed"
    );
    let r = cli.exec(&bin.join("bbg"), &["2 + 2"], None, &[])?;
    ensure!(
        r.code == 0 && r.out == "4\n",
        "bbg from the tarball install: {} {:?}",
        r.code,
        r.out
    );
    let r = cli.exec(
        Path::new("/bin/bash"),
        &[installer.as_str(), "--uninstall"],
        None,
        &env,
    )?;
    ensure!(
        r.code == 0 && !share.exists(),
        "uninstall after the tarball: {} {:?}",
        r.code,
        r.err
    );
    ensure!(
        std::fs::read_to_string(&fish_file)? == "# mine\n",
        "the installer changed a fish completion file that was not its own"
    );
    ensure!(!bash_file.exists(), "uninstall left the completion file");

    // With fish set up and no file of the name, the installer adds one, and uninstall takes it away again.
    std::fs::remove_file(&fish_file)?;
    let r = cli.exec(Path::new("/bin/bash"), &[installer.as_str()], None, &env)?;
    ensure!(
        r.code == 0,
        "install for a fish user: {} {:?}",
        r.code,
        r.err
    );
    let text = std::fs::read_to_string(&fish_file).context("the fish completion file")?;
    ensure!(
        text.starts_with("# Written by the Babbage installer") && text.contains("complete -c bbg"),
        "the fish completion file: {text:?}"
    );
    let r = cli.exec(
        Path::new("/bin/bash"),
        &[installer.as_str(), "--uninstall"],
        None,
        &env,
    )?;
    ensure!(
        r.code == 0 && !fish_file.exists() && !bash_file.exists(),
        "uninstall for a fish user: {} {:?}",
        r.code,
        r.err
    );
    drop((good, bad));
    Ok(())
}
