//! The acceptance checks. Each one starts the real binary on a temporary data folder, types real
//! keystrokes into its window, and reads the result back from the screen and from the app's state hook.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, bail, ensure};
use serde_json::Value;

use crate::harness::{App, Harness, Shot, luminance};

pub type Check = fn(&mut Harness) -> anyhow::Result<()>;

pub const CHECKS: &[(&str, Check)] = &[
    ("sum", sum_shows_39),
    ("reference-sheet", reference_sheet),
    ("error-line", error_line),
    ("ignored-dim", ignored_dim),
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
    ("scrolling", scrolling),
    ("mouse-editing", mouse_editing),
    ("drag-select", drag_select),
    ("wheel-scroll", wheel_scroll),
    ("comment-toggle", comment_toggle),
    ("export", export_sheet),
    ("clipboard-lines", clipboard_lines),
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
    std::fs::read_to_string(dir.join(format!("{name}.babbage"))).ok()
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
            "{name}.babbage never became {want:?}; it holds {got:?}"
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
        ("10 June + 3 weeks", "July 1"),
        ("days until Christmas", "193 days"),
        ("Easter 2027", "March 28, 2027"),
        ("workdays in 3 weeks", "15 workdays"),
        ("January 10 - February 5", "-3 weeks 5 days"),
        ("7:30am to 8:45pm", "13 hours 15 min"),
        ("01:02:03:04 + 00:00:01:00", "01:02:04:04"),
        ("20% is 500, what is 750", "30%"),
        ("2/10 as fraction", "1/5"),
        ("meters in 10 km", "10,000 m"),
        ("6pm Sydney in Chicago", "3:00 AM"),
        ("$500/workday * 4 weeks", "$10,000.00"),
        ("day of the week on January 24, 1984", "Tuesday"),
        ("New Year 2025", "January 1, 2025"),
        ("June 1st 2024", "June 1"),
    ];
    let s = answers_for(h, &mut app, &lines);
    app.finish()?;
    s.map(|_| ())
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


fn reference_sheet(h: &mut Harness) -> anyhow::Result<()> {
    let mut app = h.launch("reference", None, &[])?;
    for (i, (line, _)) in REFERENCE.iter().enumerate() {
        if i + 1 < REFERENCE.len() {
            h.type_line(line)?;
        } else {
            h.type_text(line)?;
        }
    }
    let s = app.wait_for("all 19 answers", Duration::from_secs(30), |s| {
        REFERENCE
            .iter()
            .enumerate()
            .all(|(i, (_, want))| answer(s, i) == Some(*want))
    })?;
    for (i, (line, want)) in REFERENCE.iter().enumerate() {
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
    h.save_shot(&h.doc_shot("reference-sheet.png"))?;
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
    // The message is in the warning colour: more red than blue, unlike the white answers.
    let p = scr.answer_cell(&shot, &s, 1)?;
    ensure!(
        p[0] as i32 > p[2] as i32 + 40,
        "the error text is not in the warning colour: {p:?}"
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
    ensure!(
        dir.join("Budget.babbage").is_file() && !dir.join(format!("{second}.babbage")).exists()
    );

    // Delete moves the file to the trash.
    h.press(&["ctrl", "shift"], "Delete")?;
    let s = app.wait_for("the sheet gone", T, |s| s["current"] == first.as_str())?;
    ensure!(sheet_names(&s) == ["1 + 1"], "{:?}", sheet_names(&s));
    ensure!(!dir.join("Budget.babbage").exists());
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
    ensure!(dir.join("Mouse.babbage").is_file());

    // A double click renames the sheet under it, and Escape leaves the name alone.
    let (x, y) = bar.card(sheet_index(&s, &first)?);
    h.click(&app, x, y, "left", 2)?;
    let s = app.wait_for("rename by double click", T, |s| {
        s["current"] == first.as_str() && s["rename"].is_string()
    })?;
    ensure!(s["lines"][0]["text"] == "1 + 1");
    h.press(&[], "Escape")?;
    let s = app.wait_for("the rename cancelled", T, |s| s["rename"].is_null())?;
    ensure!(s["current"] == first.as_str() && sheet_names(&s).len() == 2);
    ensure!(dir.join(format!("{first}.babbage")).is_file());

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
    ensure!(!dir.join("Mouse.babbage").exists());
    let trashed: Vec<_> = std::fs::read_dir(dir.join(".trash"))?.flatten().collect();
    ensure!(trashed.len() == 1, "{} files in the trash", trashed.len());
    let name = trashed[0].file_name().to_string_lossy().into_owned();
    ensure!(name.starts_with("Mouse."), "{name}");
    ensure!(std::fs::read_to_string(trashed[0].path())? == "7 + 7\n");
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
    ensure!(dir.join(format!("{id}.babbage")).is_file());

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
    let window_h = f(&s["window"]["h"])?;
    ensure!(
        y >= -1. && y + h <= window_h + 1.,
        "line {} is off the window: y {y}, row {h}, window {window_h}",
        line + 1
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
