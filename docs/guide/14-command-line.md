# The command line

Babbage has a second face. `bbg` is the same program as the window, run as a calculator: give it an expression or a
sheet and it prints the answers, then exits. The answers are the window's, because both use the same settings, the
same global variables and the same formats. Nothing opens on your screen.

The one-line installer puts `bbg` next to `babbage` in `~/.local/bin`; see [Getting started](01-getting-started.html).
`babbage calc` does the same job as `bbg` for anyone who would rather not have the extra command.

## Answer an expression

Put the expression after `bbg`. The words are joined into one line, and only the answer is printed.

```
$ bbg 20% of 85
17

$ bbg 150 lb in kg
68.0388555 kg
```

A line that has no answer, such as a variable being set, prints nothing and exits with status 0.

Start the expression with `-` followed by a number and it is still an expression: `bbg -5 + 3` prints `-2`. If the
expression starts with a dash and a letter, put `--` before it.

## Answer a sheet file

Give `bbg` the path of a sheet, and it answers every line. Each line's text is printed, then its answer, in two
columns that line up. A sheet is the same plain text file the window saves, so a file you wrote in an editor works
too. Here is `groceries.bbg`:

```
Groceries
milk $3.20 × 2
bread $4.50
eggs $5.10
```

```
$ bbg -t groceries.bbg
Groceries
milk $3.20 × 2  $6.40
bread $4.50     $4.50
eggs $5.10      $5.10
Total           $16.00
```

The `-t` option adds the total, which is the same sum the window's Total chip shows. A file's per-line choices
(decimal places and the rest, set from a line's answer menu in the window) are used as they are.

## Answer a saved sheet

`-s` takes the name of a sheet in the sheet folder, the same folder the window uses (see
[Sheets and files](10-sheets-and-files.html#where-sheets-are-stored)). The name is the sheet's file name without
`.bbg`.

```
$ bbg -s Groceries
Groceries
milk $3.20 × 2  $6.40
bread $4.50     $4.50
eggs $5.10      $5.10
```

## Read standard input

With no arguments, `bbg` reads a sheet from standard input when the input is not a terminal. A single `-` does the
same, and so does `-f -`. This is how it works in a pipeline:

```
$ echo "3 * \$4.50" | bbg
3 * $4.50  $13.50
```

## Choose what is printed

| Option | What it does |
|---|---|
| `-a`, `--answers` | Prints only the answers, one per line. A line with no answer prints a blank line, so the answers stay level with the lines. Errors go to standard error. |
| `-t`, `--total` | Adds the sheet's total as a last row. With `-a`, the total is one more answer line. Sheets only. |
| `-p N`, `--places N` | Shows N decimal places, from 0 to 10. It changes the total as well. |
| `-r`, `--repeating` | Shows a repeating decimal with its repeating part in brackets. The total is not affected. |
| `--json` | Prints one JSON object per line, with `text`, `answer`, `exact` and `error`. `-t` adds a last object under `total`. |
| `-h`, `--help` | Shows the options and exits. |
| `-V`, `--version` | Shows the version and exits. |

Decimal places and repeating decimals print the same way the window prints them. When a rounded or approximate
answer is shown, it has the window's `≈` mark:

```
$ bbg -p 2 1/3
≈ 0.33

$ bbg -r 1/3
0.(3)
```

With `--json`, `exact` is `false` for an answer marked `≈`:

```
$ bbg --json "12 + 8"
{"text":"12 + 8","answer":"20","exact":true,"error":null}
```

With `-a`, only the answers are printed, so a script can read them line by line:

```
$ bbg -a groceries.bbg
$6.40
$4.50
$5.10
```

## Errors and exit status

A line that has an error prints its message on standard error when only answers are printed, and in its answer
column in a sheet. Either way the exit status is `1`.

```
$ bbg "1 +"
unexpected end of expression
```

| Status | Meaning |
|---|---|
| `0` | Every line was answered, or had no answer. |
| `1` | At least one line is an error, or a file or saved sheet could not be read. |
| `2` | Bad usage: an unknown option, a decimal place count out of range, or nothing to answer. |

## Tab completion

Press Tab after `bbg -` for the options, after `-p` for the decimal places, and after `-s` for the names of your saved sheets. The one-line installer sets this up for bash, and for fish when you use fish, with one file of its own for each (`~/.local/share/bash-completion/completions/bbg` and `~/.config/fish/completions/bbg.fish`). It never replaces a file of that name that it did not write, and `--uninstall` removes only its own.

To set it up yourself, or for zsh, ask `bbg` for the script:

| Shell | Load it with |
|---|---|
| bash | `source <(bbg --completions bash)` in `~/.bashrc` |
| zsh | `source <(bbg --completions zsh)` in `~/.zshrc`, after `compinit` |
| fish | `bbg --completions fish \| source` in `~/.config/fish/config.fish` |

`bbg --list-sheets` prints the saved sheet names, one per line, for your own scripts. It only reads the sheet folder: it creates nothing and cleans nothing up, so it is safe to run at any time.

## Where it gets its settings

`bbg` reads the same settings file as the window (`~/.config/babbage/settings`, or the file named by
`BABBAGE_CONFIG_FILE`), so your time zone, number format, decimal places and global variables apply. The variables
you set in Settings work in an expression too: `bbg rent * 12` uses the `rent` you defined. See
[Settings](12-settings.html) for each setting.
