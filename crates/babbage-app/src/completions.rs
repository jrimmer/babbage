//! Tab completion for `bbg` in bash, zsh and fish. The scripts are written from `cli::OPTIONS`, so they can name
//! only options the parser knows, and they ask `bbg --list-sheets` for the saved sheets, which reads the sheet folder
//! and changes nothing.

use std::fmt::Write;

use crate::cli::{OPTIONS, Takes};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    pub fn parse(name: &str) -> Option<Shell> {
        match name {
            "bash" => Some(Shell::Bash),
            "zsh" => Some(Shell::Zsh),
            "fish" => Some(Shell::Fish),
            _ => None,
        }
    }
}

const PLACES: &str = "0 1 2 3 4 5 6 7 8 9 10";
const SHELLS: &str = "bash zsh fish";

pub fn script(shell: Shell) -> String {
    match shell {
        Shell::Bash => bash(),
        Shell::Zsh => zsh(),
        Shell::Fish => fish(),
    }
}

/// Every spelling of every option, short then long.
fn spellings() -> Vec<String> {
    OPTIONS
        .iter()
        .flat_map(|(short, long, _, _)| {
            short
                .map(|c| format!("-{c}"))
                .into_iter()
                .chain([format!("--{long}")])
        })
        .collect()
}

fn bash() -> String {
    let mut cases = String::new();
    for (short, long, takes, _) in OPTIONS {
        let names = match short {
            Some(c) => format!("-{c}|--{long}"),
            None => format!("--{long}"),
        };
        let body = match takes {
            Takes::Nothing => continue,
            Takes::Places => format!("COMPREPLY=($(compgen -W \"{PLACES}\" -- \"$cur\"))"),
            Takes::Shell => format!("COMPREPLY=($(compgen -W \"{SHELLS}\" -- \"$cur\"))"),
            Takes::File => "compopt -o filenames 2>/dev/null; COMPREPLY=($(compgen -f -- \"$cur\"))"
                .to_string(),
            Takes::Sheet => "local IFS=$'\\n'; COMPREPLY=($(compgen -W \"$(\"${COMP_WORDS[0]}\" --list-sheets 2>/dev/null)\" -- \"$cur\")); COMPREPLY=(\"${COMPREPLY[@]// /\\\\ }\")".to_string(),
        };
        writeln!(cases, "        {names}) {body}; return ;;").unwrap();
    }
    format!(
        r#"# bash completion for bbg. Load it with:  source <(bbg --completions bash)
_bbg() {{
    local cur prev
    cur=${{COMP_WORDS[COMP_CWORD]}}
    prev=${{COMP_WORDS[COMP_CWORD-1]}}
    case "$prev" in
{cases}    esac
    if [[ $cur == -* ]]; then
        COMPREPLY=($(compgen -W "{options}" -- "$cur"))
        return
    fi
    compopt -o filenames 2>/dev/null
    COMPREPLY=($(compgen -f -- "$cur"))
}}
# nosort (bash 5.1 and later) keeps 0 to 10 in number order; older bash sorts them as text.
complete -o nosort -F _bbg bbg 2>/dev/null || complete -F _bbg bbg
"#,
        options = spellings().join(" ")
    )
}

fn zsh() -> String {
    let mut specs = String::new();
    for (short, long, takes, about) in OPTIONS {
        let (flag_short, flag_long, action) = match takes {
            Takes::Nothing => (
                format!("-{}", short.unwrap_or(' ')),
                format!("--{long}"),
                String::new(),
            ),
            Takes::Places => (
                format!("-{}+", short.unwrap_or(' ')),
                format!("--{long}="),
                format!(":places:({PLACES})"),
            ),
            Takes::File => (
                format!("-{}+", short.unwrap_or(' ')),
                format!("--{long}="),
                ":sheet file:_files".to_string(),
            ),
            Takes::Sheet => (
                format!("-{}+", short.unwrap_or(' ')),
                format!("--{long}="),
                ":saved sheet:_bbg_sheets".to_string(),
            ),
            Takes::Shell => (
                String::new(),
                format!("--{long}="),
                format!(":shell:({SHELLS})"),
            ),
        };
        let line = match short {
            Some(c) => {
                format!("    '(-{c} --{long})'{{{flag_short},{flag_long}}}'[{about}]{action}' \\\n")
            }
            None => format!("    '{flag_long}[{about}]{action}' \\\n"),
        };
        specs.push_str(&line);
    }
    format!(
        r#"#compdef bbg
# zsh completion for bbg. Save it as _bbg in a folder on your $fpath, or load it with:  source <(bbg --completions zsh)
_bbg_sheets() {{
  local -a names
  names=("${{(@f)$(bbg --list-sheets 2>/dev/null)}}")
  compadd -a names
}}
_bbg() {{
  _arguments -s \
{specs}    '*:expression or file:_files'
}}
if [ "$funcstack[1]" = "_bbg" ]; then
  _bbg "$@"
else
  compdef _bbg bbg
fi
"#
    )
}

fn fish() -> String {
    let mut out =
        String::from("# fish completion for bbg. Load it with:  bbg --completions fish | source\n");
    for (short, long, takes, about) in OPTIONS {
        let mut line = String::from("complete -c bbg");
        if let Some(c) = short {
            write!(line, " -s {c}").unwrap();
        }
        write!(line, " -l {long}").unwrap();
        match takes {
            Takes::Nothing => {}
            Takes::Places => write!(line, " -x -a '{PLACES}'").unwrap(),
            Takes::Shell => write!(line, " -x -a '{SHELLS}'").unwrap(),
            Takes::File => line.push_str(" -r"),
            Takes::Sheet => line.push_str(" -x -a '(bbg --list-sheets 2>/dev/null)'"),
        }
        writeln!(out, "{line} -d '{about}'").unwrap();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use std::process::{Command, Stdio};

    #[test]
    fn every_option_is_named_in_every_script() {
        for shell in [Shell::Bash, Shell::Zsh, Shell::Fish] {
            let s = script(shell);
            for (short, long, _, _) in OPTIONS {
                assert!(
                    s.contains(&format!("--{long}")) || s.contains(&format!("-l {long}")),
                    "{shell:?} lacks --{long}"
                );
                if let Some(c) = short {
                    assert!(
                        s.contains(&format!("-{c}")) || s.contains(&format!("-s {c}")),
                        "{shell:?} lacks -{c}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_table_matches_the_parser_and_the_help() {
        for (short, long, takes, about) in OPTIONS {
            assert!(
                !about.contains('\''),
                "{long}: an apostrophe would break the quoting"
            );
            let names = short
                .map(|c| format!("-{c}"))
                .into_iter()
                .chain([format!("--{long}")]);
            for name in names {
                let value = match takes {
                    Takes::Nothing => None,
                    Takes::Places => Some("2"),
                    Takes::File => Some("budget.bbg"),
                    Takes::Sheet => Some("Budget"),
                    Takes::Shell => Some("bash"),
                };
                let mut args = vec![name.clone()];
                args.extend(value.map(str::to_owned));
                args.push("1 + 1".to_owned());
                let parsed = crate::cli::parse(&args, true, &|_| false);
                assert!(
                    !matches!(&parsed, Err(e) if e.contains("unknown option")),
                    "{name} is not an option of the parser: {parsed:?}"
                );
            }
            assert!(
                crate::cli::help_text().contains(&format!("--{long}")),
                "the help lacks --{long}"
            );
        }
    }

    #[test]
    fn only_bash_zsh_and_fish_are_known() {
        assert_eq!(Shell::parse("bash"), Some(Shell::Bash));
        assert_eq!(Shell::parse("zsh"), Some(Shell::Zsh));
        assert_eq!(Shell::parse("fish"), Some(Shell::Fish));
        assert_eq!(Shell::parse("powershell"), None);
        assert_eq!(Shell::parse(""), None);
    }

    /// What `$COMPREPLY` holds after Tab at the end of `words`, in a real bash.
    fn bash_replies(words: &[&str], sheets: &str) -> Vec<String> {
        let dir = tempfile::tempdir().unwrap();
        let fake = dir.path().join("bbg");
        std::fs::write(
            &fake,
            format!("#!/bin/sh\n[ \"$1\" = --list-sheets ] && printf '%s' '{sheets}'\n"),
        )
        .unwrap();
        std::fs::set_permissions(&fake, std::os::unix::fs::PermissionsExt::from_mode(0o755))
            .unwrap();
        let mut quoted = String::new();
        for w in words {
            write!(quoted, "'{w}' ").unwrap();
        }
        let program = format!(
            "{script}\nCOMP_WORDS=({fake} {quoted})\nCOMP_CWORD=${{#COMP_WORDS[@]}}\nCOMP_CWORD=$((COMP_CWORD-1))\ncd '{cwd}'\n_bbg\nprintf '%s\\n' \"${{COMPREPLY[@]}}\"\n",
            script = bash(),
            fake = fake.display(),
            cwd = dir.path().display(),
        );
        let mut child = Command::new("bash")
            .args(["--norc", "--noprofile"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("bash");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(program.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout)
            .unwrap()
            .lines()
            .filter(|l| !l.is_empty())
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn bash_completes_options_places_shells_and_saved_sheets() {
        let options = bash_replies(&["--"], "");
        for want in [
            "--answers",
            "--places",
            "--json",
            "--completions",
            "--list-sheets",
        ] {
            assert!(options.contains(&want.to_owned()), "{want} in {options:?}");
        }
        assert_eq!(bash_replies(&["--pl"], ""), ["--places"]);
        assert_eq!(bash_replies(&["-p", "1"], ""), ["1", "10"]);
        assert_eq!(
            bash_replies(&["-p", ""], ""),
            ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10"]
        );
        // Bash sorts what Tab shows as text (0 1 10 2) unless told not to; the places are numbers. `compopt` inside
        // the function does not stop that, `complete -o nosort` does.
        assert!(script(Shell::Bash).contains("complete -o nosort -F _bbg bbg"));
        assert_eq!(bash_replies(&["--completions", "z"], ""), ["zsh"]);
        assert_eq!(
            bash_replies(&["-s", "T"], "Budget\nTrip\nTax return"),
            ["Trip", "Tax\\ return"]
        );
        assert_eq!(bash_replies(&["--sheet", ""], "Budget"), ["Budget"]);
        assert!(bash_replies(&["-s", "x"], "Budget").is_empty());
    }

    #[test]
    fn bash_completes_files_for_a_file_argument() {
        // The scratch folder holds only the fake bbg, so a file argument offers it.
        assert_eq!(bash_replies(&["-f", "bb"], ""), ["bbg"]);
        assert_eq!(bash_replies(&["bb"], ""), ["bbg"]);
    }

    #[test]
    fn each_script_is_valid_for_its_shell_when_that_shell_is_here() {
        for (shell, program, args) in [
            (Shell::Bash, "bash", ["-n"]),
            (Shell::Zsh, "zsh", ["-n"]),
            (Shell::Fish, "fish", ["--no-execute"]),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("script");
            std::fs::write(&file, script(shell)).unwrap();
            match Command::new(program).args(args).arg(&file).output() {
                Ok(out) => assert!(
                    out.status.success(),
                    "{program}: {}",
                    String::from_utf8_lossy(&out.stderr)
                ),
                Err(_) => assert_ne!(program, "bash", "bash is needed to check the bash script"),
            }
        }
    }
}
