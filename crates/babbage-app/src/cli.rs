//! The command-line mode: `bbg` (or `babbage calc`) answers an expression or a sheet without a window.
//!
//! It is the same program as the window. It evaluates and formats through `computed::compute_in`, with the
//! settings and global variables the window uses, so the two give the same answers. Nothing here touches the
//! GUI library.
//!
//! ```text
//! bbg [options] EXPRESSION...      bbg [options] FILE      bbg [options] -f FILE | -s NAME | -
//! ```

use std::ffi::OsString;
use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};

use babbage_core::settings::Settings;
use babbage_store::{SheetBook, SheetId, decode};
use serde_json::json;

use crate::completions::Shell;
use crate::computed::{Answer, Computed, compute_in};
use crate::linefmt::{self, LineFormat};

/// What an option takes after it, for the shell completions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Takes {
    Nothing,
    Places,
    File,
    Sheet,
    Shell,
}

/// Every option of command mode: its short and long names, what it takes and what it does. The shell completions are
/// written from this table, so they cannot name an option the parser does not know.
pub const OPTIONS: &[(Option<char>, &str, Takes, &str)] = &[
    (
        Some('a'),
        "answers",
        Takes::Nothing,
        "print only the answers",
    ),
    (
        Some('t'),
        "total",
        Takes::Nothing,
        "also print the total of a sheet",
    ),
    (
        Some('p'),
        "places",
        Takes::Places,
        "decimal places, 0 to 10",
    ),
    (
        Some('r'),
        "repeating",
        Takes::Nothing,
        "show repeating decimals as 0.(3)",
    ),
    (None, "json", Takes::Nothing, "one JSON object per line"),
    (Some('f'), "file", Takes::File, "a sheet file"),
    (Some('s'), "sheet", Takes::Sheet, "a saved sheet"),
    (
        None,
        "completions",
        Takes::Shell,
        "print the completion script for a shell",
    ),
    (None, "list-sheets", Takes::Nothing, "list the saved sheets"),
    (Some('h'), "help", Takes::Nothing, "show help"),
    (Some('V'), "version", Takes::Nothing, "show the version"),
];

pub const USAGE: &str = "usage: bbg [options] EXPRESSION...\n       bbg [options] FILE\n       bbg [options] -f FILE | -s NAME | -";

/// The text of `bbg --help`.
#[cfg(test)]
pub fn help_text() -> &'static str {
    HELP
}

const HELP: &str = "\
usage: bbg [options] EXPRESSION...
       bbg [options] FILE
       bbg [options] -f FILE | -s NAME | -

Answers an expression, or a sheet, the way the Babbage window does. The program is also
run as `babbage calc`.

  EXPRESSION...     the words are joined into one line
  FILE              a sheet file, when the only argument is an existing file
  -f FILE, --file FILE, -
                    a sheet file, or standard input
  -s NAME, --sheet NAME
                    a saved sheet from the sheet folder
  (no arguments)    reads a sheet from standard input when it is not a terminal
  -a, --answers     print only the answers, one per line
  -t, --total       also print the sheet's total (sheets only)
  -p N, --places N  decimal places, 0 to 10
  -r, --repeating   show repeating decimals as 0.(3)
      --json        one JSON object per line: text, answer, exact, error
  -h, --help        show this help
  -V, --version     show the version
      --completions SHELL  print the tab-completion script for bash, zsh or fish
      --list-sheets        list the saved sheets, one per line

Exit status: 0 when no line is an error, 1 when one is, 2 for bad usage.
";

/// Where the lines to answer come from.
#[derive(Debug, PartialEq, Eq)]
pub enum Source {
    /// The arguments joined into one line.
    Words(String),
    /// Standard input, as a sheet.
    Stdin,
    /// A sheet file (`-` is standard input).
    File(PathBuf),
    /// A saved sheet, by name.
    Saved(String),
}

#[derive(Debug, PartialEq, Eq, Default)]
pub struct Options {
    pub answers: bool,
    pub total: bool,
    pub json: bool,
    pub repeating: bool,
    pub places: Option<u32>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Help,
    Version,
    /// The tab-completion script for a shell.
    Completions(Shell),
    /// The names of the saved sheets, for the completion scripts.
    ListSheets,
    Run(Source, Options),
}

/// The arguments of command mode, or None when the program is the window. `babbage calc ...` is command mode, and so is
/// any run under the name `bbg` or with `BABBAGE_AS=bbg` (the AppImage's `bbg` launcher sets it).
pub fn command_args(argv: &[OsString]) -> Option<&[OsString]> {
    if argv.get(1).and_then(|a| a.to_str()) == Some("calc") {
        return Some(&argv[2..]);
    }
    let named_bbg = argv
        .first()
        .and_then(|a| Path::new(a).file_name())
        .and_then(|n| n.to_str())
        == Some("bbg");
    let env_bbg =
        std::env::var_os("BABBAGE_AS").and_then(|v| v.into_string().ok()) == Some("bbg".into());
    (named_bbg || env_bbg).then(|| &argv[1..])
}

/// An argument that is an option: `--name`, or a dash and a letter. A dash and a digit is a negative number.
fn is_option(arg: &str) -> bool {
    arg.starts_with("--")
        || arg
            .strip_prefix('-')
            .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_alphabetic()))
}

fn parse_places(text: &str) -> Result<u32, String> {
    text.parse::<u32>()
        .ok()
        .filter(|p| *p <= 10)
        .ok_or_else(|| format!("decimal places must be 0 to 10, not {text:?}"))
}

fn set_source(slot: &mut Option<Source>, source: Source) -> Result<(), String> {
    if slot.is_some() {
        return Err("give one sheet: a file, -f, -s or -".into());
    }
    *slot = Some(source);
    Ok(())
}

/// Reads the arguments. `is_file` says whether a lone argument names an existing file, and `stdin_is_tty` whether
/// standard input is a terminal (then no arguments is a usage error, not a read).
pub fn parse(
    args: &[String],
    stdin_is_tty: bool,
    is_file: &dyn Fn(&str) -> bool,
) -> Result<Command, String> {
    let mut opts = Options::default();
    let mut source: Option<Source> = None;
    let mut words: Vec<String> = Vec::new();
    let mut words_only = false;
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        if words_only || !is_option(arg) {
            words.push(arg.clone());
            continue;
        }
        if arg == "--" {
            words_only = true;
            continue;
        }
        // `--places=4` carries its value after the equals sign.
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) if name.starts_with("--") => (name, Some(value.to_owned())),
            _ => (arg.as_str(), None),
        };
        let mut value = || {
            inline
                .clone()
                .or_else(|| rest.next().cloned())
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match name {
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "--list-sheets" => return Ok(Command::ListSheets),
            "--completions" => {
                let shell = value()?;
                return Shell::parse(&shell)
                    .map(Command::Completions)
                    .ok_or_else(|| format!("no completions for {shell:?}: use bash, zsh or fish"));
            }
            "-a" | "--answers" => opts.answers = true,
            "-t" | "--total" => opts.total = true,
            "-r" | "--repeating" => opts.repeating = true,
            "--json" => opts.json = true,
            "-p" | "--places" => opts.places = Some(parse_places(&value()?)?),
            "-f" | "--file" => set_source(&mut source, Source::File(value()?.into()))?,
            "-s" | "--sheet" => set_source(&mut source, Source::Saved(value()?))?,
            _ => return Err(format!("unknown option {arg}")),
        }
    }
    let source = match source {
        Some(s) if words.is_empty() => s,
        Some(_) => return Err("give a sheet or an expression, not both".into()),
        None => match words.as_slice() {
            [] if stdin_is_tty => return Err("nothing to answer".into()),
            [] => Source::Stdin,
            [w] if w == "-" => Source::Stdin,
            [w] if is_file(w.as_str()) => Source::File(w.into()),
            _ => Source::Words(words.join(" ")),
        },
    };
    Ok(Command::Run(source, opts))
}

/// The lines a source holds, and each line's choices. A sheet's trailing blank lines are left out.
fn load(source: &Source) -> Result<(Vec<String>, Vec<LineFormat>), String> {
    let sheet = |lines: Vec<String>| {
        let (mut lines, mut formats) = linefmt::split(&lines);
        let keep = lines
            .iter()
            .rposition(|l| !l.trim().is_empty())
            .map_or(0, |i| i + 1);
        lines.truncate(keep);
        formats.truncate(keep);
        Ok((lines, formats))
    };
    match source {
        Source::Words(text) => Ok((vec![text.clone()], vec![LineFormat::default()])),
        Source::Stdin => sheet(decode(&read_stdin()?)),
        Source::File(path) if path.as_os_str() == "-" => sheet(decode(&read_stdin()?)),
        Source::File(path) => {
            let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
            let text = String::from_utf8(bytes)
                .map_err(|_| format!("{} is not UTF-8 text", path.display()))?;
            sheet(decode(&text))
        }
        Source::Saved(name) => {
            let book = SheetBook::open_default().map_err(|e| e.to_string())?;
            let id = SheetId::new(name).map_err(|e| e.to_string())?;
            sheet(book.read(&id).map_err(|e| e.to_string())?)
        }
    }
}

fn read_stdin() -> Result<String, String> {
    let mut text = String::new();
    std::io::stdin()
        .read_to_string(&mut text)
        .map_err(|e| format!("standard input: {e}"))?;
    Ok(text)
}

/// One line as the output shows it.
#[derive(Debug, PartialEq, Eq)]
struct Row {
    line: String,
    answer: Option<String>,
    exact: bool,
    error: Option<String>,
}

impl Row {
    fn of(line: &str, a: &Answer) -> Row {
        Row {
            line: line.to_owned(),
            answer: a.text.clone(),
            // A line with no answer is not approximate; the window's default says otherwise.
            exact: a.exact || a.text.is_none(),
            error: a.error.clone(),
        }
    }

    /// The answer as the window shows it (`≈` when approximate), or nothing.
    fn shown(&self) -> String {
        match &self.answer {
            Some(text) if self.exact => text.clone(),
            Some(text) => format!("≈ {text}"),
            None => String::new(),
        }
    }

    /// The answer column of a sheet: the answer, else the error, else nothing.
    fn cell(&self) -> String {
        match (&self.answer, &self.error) {
            (None, Some(e)) => format!("error: {e}"),
            _ => self.shown(),
        }
    }

    fn json(&self) -> String {
        json!({
            "text": self.line,
            "answer": self.answer,
            "exact": self.exact,
            "error": self.error,
        })
        .to_string()
    }
}

/// What a run prints: stdout, stderr, and whether a line was an error.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Printed {
    pub out: String,
    pub err: String,
    pub failed: bool,
}

/// Prints the answers for `lines` the way `opts` asks. `expression` is set for a line given on the command line,
/// which prints just its answer (and nothing when it has none).
pub fn render(lines: &[String], c: &Computed, opts: &Options, expression: bool) -> Printed {
    let rows: Vec<Row> = lines
        .iter()
        .zip(&c.answers)
        .map(|(line, a)| Row::of(line, a))
        .collect();
    let total = c.total.as_ref().filter(|_| opts.total && !expression);
    let mut p = Printed {
        failed: rows.iter().any(|r| r.error.is_some()),
        ..Printed::default()
    };
    if opts.json {
        for r in &rows {
            p.out.push_str(&r.json());
            p.out.push('\n');
        }
        if let Some(t) = total {
            let obj = json!({"total": {"answer": t.text, "exact": t.exact, "error": t.error}});
            p.out.push_str(&obj.to_string());
            p.out.push('\n');
        }
    } else if expression || opts.answers {
        // Only answers reach standard output, one line each; an error goes to standard error and leaves a blank line
        // in a sheet, so the answers still line up with the lines.
        for r in &rows {
            if let Some(e) = &r.error {
                p.err.push_str(e);
                p.err.push('\n');
            }
            if expression && r.answer.is_none() {
                continue;
            }
            p.out.push_str(&r.shown());
            p.out.push('\n');
        }
        if let Some(t) = total {
            p.out.push_str(&Row::of("", t).shown());
            p.out.push('\n');
        }
    } else {
        p.out = sheet_text(&rows, total);
    }
    p
}

/// The sheet as two aligned columns: each line's text, then its answer. A total is a last row labelled `Total`.
fn sheet_text(rows: &[Row], total: Option<&Answer>) -> String {
    let width = rows
        .iter()
        .map(|r| r.line.trim_end().chars().count())
        .chain(total.map(|_| "Total".len()))
        .max()
        .unwrap_or(0);
    let mut out = String::new();
    for r in rows {
        let text = r.line.trim_end();
        let cell = r.cell();
        if cell.is_empty() {
            out.push_str(text);
        } else {
            out.push_str(&format!("{text:<width$}  {cell}"));
        }
        out.push('\n');
    }
    if let Some(t) = total {
        out.push_str(&format!("{:<width$}  {}\n", "Total", Row::of("", t).cell()));
    }
    out
}

/// Answers the source and prints the result. Standard input and the sheet folder are read here.
pub fn run(args: &[String]) -> i32 {
    let is_tty = std::io::stdin().is_terminal();
    let command = match parse(args, is_tty, &|w| Path::new(w).is_file()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("bbg: {e}\n{USAGE}");
            return 2;
        }
    };
    let (source, opts) = match command {
        Command::Help => {
            print!("{HELP}");
            return 0;
        }
        Command::Version => {
            println!("babbage {}", env!("CARGO_PKG_VERSION"));
            return 0;
        }
        Command::Completions(shell) => {
            print!("{}", crate::completions::script(shell));
            return 0;
        }
        Command::ListSheets => {
            // Read only: tab completion must not create the folder or clear files the window is saving.
            for name in babbage_store::data_dir()
                .map(|dir| SheetBook::names_in(&dir))
                .unwrap_or_default()
            {
                println!("{name}");
            }
            return 0;
        }
        Command::Run(source, opts) => (source, opts),
    };
    let (lines, formats) = match load(&source) {
        Ok(loaded) => loaded,
        Err(e) => {
            eprintln!("bbg: {e}");
            return 1;
        }
    };
    let expression = matches!(source, Source::Words(_));
    let c = compute_in(
        &lines,
        &formats_for(&opts, &formats),
        &settings_for(crate::prefs::current().clone(), &opts),
    );
    let printed = render(&lines, &c, &opts, expression);
    // Output may end early when the reader goes away (a pipe to head); that is not an error here.
    let _ = std::io::stdout().write_all(printed.out.as_bytes());
    let _ = std::io::stderr().write_all(printed.err.as_bytes());
    i32::from(printed.failed)
}

/// The settings with `-p` laid over them: the places apply to the total too, as the window's setting does.
fn settings_for(mut settings: Settings, opts: &Options) -> Settings {
    if let Some(p) = opts.places {
        settings.decimal_places = Some(p);
    }
    settings
}

/// Each line's choices with `-p` and `-r` laid over them, as the window's answer menu would set them.
fn formats_for(opts: &Options, formats: &[LineFormat]) -> Vec<LineFormat> {
    formats
        .iter()
        .map(|f| LineFormat {
            places: opts.places.or(f.places),
            repeating: if opts.repeating {
                Some(true)
            } else {
                f.repeating
            },
            ..*f
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(ws: &[&str]) -> Vec<String> {
        ws.iter().map(|w| (*w).to_owned()).collect()
    }

    /// Parses with `budget.bbg` as the only existing file and a terminal on standard input.
    fn parsed(ws: &[&str]) -> Result<Command, String> {
        parse(&words(ws), true, &|w| w == "budget.bbg")
    }

    fn run_of(ws: &[&str]) -> Command {
        parsed(ws).expect("a command")
    }

    #[test]
    fn words_are_joined_into_one_expression() {
        assert_eq!(
            run_of(&["20%", "of", "85"]),
            Command::Run(Source::Words("20% of 85".into()), Options::default())
        );
        // A negative number is an expression, not an option.
        assert_eq!(
            run_of(&["-5", "+", "3"]),
            Command::Run(Source::Words("-5 + 3".into()), Options::default())
        );
        assert_eq!(
            run_of(&["--", "-p", "3"]),
            Command::Run(Source::Words("-p 3".into()), Options::default())
        );
    }

    #[test]
    fn a_lone_existing_file_is_a_sheet_and_anything_else_is_words() {
        assert_eq!(
            run_of(&["budget.bbg"]),
            Command::Run(Source::File("budget.bbg".into()), Options::default())
        );
        assert_eq!(
            run_of(&["budget.bbg", "+", "1"]),
            Command::Run(Source::Words("budget.bbg + 1".into()), Options::default())
        );
    }

    #[test]
    fn sources_and_flags_parse() {
        let all = Options {
            answers: true,
            total: true,
            json: true,
            repeating: true,
            places: Some(4),
        };
        assert_eq!(
            run_of(&["-a", "-t", "-r", "--json", "--places=4", "-s", "Trip"]),
            Command::Run(Source::Saved("Trip".into()), all)
        );
        assert_eq!(
            run_of(&["-p", "4", "-a", "-t", "-r", "--json", "-s", "Trip"]),
            Command::Run(
                Source::Saved("Trip".into()),
                Options {
                    answers: true,
                    total: true,
                    json: true,
                    repeating: true,
                    places: Some(4),
                }
            )
        );
        assert_eq!(
            run_of(&["-f", "-"]),
            Command::Run(Source::File("-".into()), Options::default())
        );
        assert_eq!(
            run_of(&["-"]),
            Command::Run(Source::Stdin, Options::default())
        );
        assert_eq!(run_of(&["-h", "nonsense"]), Command::Help);
        assert_eq!(run_of(&["5", "-V"]), Command::Version);
    }

    #[test]
    fn bad_usage_is_refused_with_a_reason() {
        assert!(
            parsed(&["-x", "1"])
                .unwrap_err()
                .contains("unknown option -x")
        );
        assert!(parsed(&["-p", "11", "1"]).unwrap_err().contains("0 to 10"));
        assert!(parsed(&["-p"]).unwrap_err().contains("needs a value"));
        assert!(parsed(&["-s"]).unwrap_err().contains("needs a value"));
        assert!(
            parsed(&["-s", "A", "-f", "B"])
                .unwrap_err()
                .contains("one sheet")
        );
        assert!(
            parsed(&["-s", "A", "1 + 1"])
                .unwrap_err()
                .contains("not both")
        );
        // With a terminal on standard input and nothing to answer, there is nothing to read.
        assert!(
            parse(&[], true, &|_| false)
                .unwrap_err()
                .contains("nothing")
        );
        assert_eq!(
            parse(&[], false, &|_| false),
            Ok(Command::Run(Source::Stdin, Options::default()))
        );
    }

    #[test]
    fn the_command_name_picks_the_mode() {
        let osv = |v: &[&str]| -> Vec<OsString> { v.iter().map(OsString::from).collect() };
        let bbg = osv(&["/home/me/.local/bin/bbg", "1", "+", "1"]);
        assert_eq!(command_args(&bbg), Some(&bbg[1..]));
        let calc = osv(&["babbage", "calc", "2"]);
        assert_eq!(command_args(&calc), Some(&calc[2..]));
        let window = osv(&["babbage", "--control", "s"]);
        assert_eq!(command_args(&window), None);
    }

    fn sheet(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|s| (*s).to_owned()).collect()
    }

    /// Runs `lines` through the same steps as `run`, with the engine's default settings so the tests do not read the
    /// machine's own settings file.
    fn show(lines: &[&str], opts: &Options, expression: bool) -> Printed {
        let lines = sheet(lines);
        let formats = vec![LineFormat::default(); lines.len()];
        let c = compute_in(
            &lines,
            &formats_for(opts, &formats),
            &settings_for(Settings::default(), opts),
        );
        render(&lines, &c, opts, expression)
    }

    #[test]
    fn an_expression_prints_only_its_answer() {
        let p = show(&["20% of 85"], &Options::default(), true);
        assert_eq!(
            p,
            Printed {
                out: "17\n".into(),
                ..Printed::default()
            }
        );
    }

    #[test]
    fn an_error_goes_to_standard_error_and_fails() {
        let p = show(&["1 +"], &Options::default(), true);
        assert!(p.out.is_empty());
        assert!(p.failed && !p.err.is_empty());
    }

    #[test]
    fn a_line_with_no_answer_prints_nothing_and_does_not_fail() {
        let p = show(&["// a note"], &Options::default(), true);
        assert_eq!(p, Printed::default());
    }

    #[test]
    fn a_sheet_prints_its_lines_and_answers_in_two_columns() {
        let p = show(
            &["Lunch", "12 + 8", "tip = 15%", "tip of 40"],
            &Options::default(),
            false,
        );
        assert_eq!(
            p.out,
            "Lunch\n12 + 8     20\ntip = 15%  15%\ntip of 40  6\n"
        );
        assert!(!p.failed);
    }

    #[test]
    fn the_total_is_the_last_row_of_a_sheet_and_lines_up_with_it() {
        let opts = Options {
            total: true,
            ..Options::default()
        };
        let p = show(&["10", "20"], &opts, false);
        assert_eq!(p.out, "10     10\n20     20\nTotal  30\n");
    }

    #[test]
    fn answers_only_print_one_per_line_blank_when_none_and_errors_go_to_stderr() {
        let opts = Options {
            answers: true,
            ..Options::default()
        };
        let p = show(&["Lunch", "12 + 8", "1 +"], &opts, false);
        assert_eq!(p.out, "\n20\n\n");
        assert!(p.err.ends_with('\n') && p.err.len() > 1);
        assert!(p.failed);
    }

    #[test]
    fn json_has_text_answer_exact_and_error_per_line() {
        let opts = Options {
            json: true,
            ..Options::default()
        };
        let p = show(&["12 + 8", "1 +"], &opts, false);
        let rows: Vec<serde_json::Value> = p
            .out
            .lines()
            .map(|l| serde_json::from_str(l).expect("json"))
            .collect();
        assert_eq!(rows[0]["text"], "12 + 8");
        assert_eq!(rows[0]["answer"], "20");
        assert_eq!(rows[0]["exact"], true);
        assert!(rows[0]["error"].is_null());
        assert!(rows[1]["answer"].is_null());
        assert!(rows[1]["error"].is_string());
        assert!(p.failed);
    }

    #[test]
    fn places_and_repeating_change_the_answer() {
        let places = Options {
            places: Some(4),
            ..Options::default()
        };
        assert_eq!(show(&["1/3"], &places, true).out, "≈ 0.3333\n");
        let repeating = Options {
            repeating: true,
            ..Options::default()
        };
        assert_eq!(show(&["1/3"], &repeating, true).out, "0.(3)\n");
    }
}
