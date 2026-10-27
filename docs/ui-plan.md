# Desktop app plan

Three crates sit beside the engine. Only `babbage-app` may depend on gpui.

| crate | job |
|---|---|
| `babbage-store` | Sheet files and the sheetbook. No UI, no engine. |
| `babbage-app` | The `babbage` binary: a gpui window. |
| `babbage-accept` | Starts the real binary in the nested headless Hyprland, types real keys, reads the screen back. |

## Layout

Left to right, as in the target screenshot:

1. Sidebar (toggle with Ctrl+B): a "New Sheet" button, then one card per sheet with its title,
   modified time and line count. The current sheet is highlighted.
2. Editor: line numbers in a gutter, then the text, one sheet line per row, no wrapping.
3. Answer column, aligned to the editor rows, on its own darker panel.
4. A floating "Total" chip at the bottom right of the answer column.

One dark theme ships. The app also reads Omarchy's `colors.toml` (the file named by `BABBAGE_THEME_FILE`,
else `~/.local/state/omarchy/current/theme/colors.toml`) at start and when the file changes.

## State model

`AppState` (one gpui entity) holds the sheetbook, the list of `SheetInfo`, the open sheet's lines, the
cursor and selection, an undo stack, the `Vec<LineResult>` of the last evaluation, the sidebar state and
the focused answer line. The editor text is `Vec<String>`, one entry per sheet line, with a cursor of
(line, byte column). Undo and redo keep snapshots of (lines, cursor) and merge runs of typing.

## Input and answers

- Keys: `on_key_down` on the root handles shortcuts (Ctrl+N, Ctrl+Tab, Ctrl+B, Ctrl+Q, Ctrl+Z,
  Ctrl+Shift+Z, Ctrl+C/X/V/A, arrows, Home/End, Backspace/Delete, Enter, Alt+Up/Down for the answer focus).
  Printable text arrives through gpui's `EntityInputHandler`, which also carries IME composition.
- Every committed change calls `evaluate_sheet` over all lines, then `floating_total`. The results feed
  both the paint of the answer column and the syntax coloring. A change also tells the autosaver, which
  writes the file 400 ms after the last keystroke, atomically.
- Answers use `format_value` with `line.display_options(...)`. `Formatted.exact == false` shows a small
  "≈" first. Errors show `Error`'s message in a muted warning color. Ignored words are dimmed in the
  editor, `warnings` get a dotted marker under the word, `lineN` references are chips.

## Test hook and harness

`babbage --control <socket>` serves newline-delimited JSON on a unix socket (mode 0600). Requests:
`state` (sheets, current sheet, per-line text and answer text, ignored spans, total, sidebar rows, focus),
`shot_info` (window geometry). It never changes state: all input in the checks is real keystrokes sent
with `wtype` into the focused window and real mouse clicks from `tools/pointer.py`, a virtual pointer that stays alive for the whole run (a seat with no pointer tells the app it has none). The harness (`babbage-accept`):

1. starts the nested Hyprland (`tools/nested-hyprland.sh`), which is hidden on a special workspace;
2. starts `babbage` there with a temporary `BABBAGE_DATA_DIR`, a fixed theme file and the control socket;
3. types, polls `state` until it settles, asserts, and takes a `grim` screenshot of the headless output;
4. stops everything it started, by PID.

`bin/accept` builds release binaries and runs every check; `bin/check` is the unit gate and guards.

## Keys

| key | does |
|---|---|
| Ctrl+N | new sheet |
| Ctrl+Tab, Ctrl+Shift+Tab | next, previous sheet (sidebar order, which does not reshuffle while saving) |
| Ctrl+B | show or hide the sidebar |
| Ctrl+Q | save and quit |
| F2 | rename the open sheet (type, Enter to accept, Esc to cancel) |
| Ctrl+Shift+Delete | move the open sheet to `.trash` |
| Ctrl+Z, Ctrl+Shift+Z or Ctrl+Y | undo, redo (a run of typing without spaces is one step) |
| Ctrl+A, Ctrl+C, Ctrl+X, Ctrl+V | select all, copy, cut, paste |
| Alt+Right | focus the answer of the caret line; Ctrl+C then copies the answer text; Esc or Left returns |
| Shift with arrows, Home, End | extend the selection; Ctrl adds word steps |

## Decisions made while building

- The key handler is one `on_key_down` on the root instead of gpui actions and a keymap file: the shortcuts
  are few, and it avoids the layout-dependent re-binding the maintainer's other app needs. Printable text still
  arrives through the input handler, so IME composition works the same way.
- The answer column is painted by the same element as the text so rows cannot drift apart.
- A sheet's name in the sidebar is its title (first non-empty line) until the user renames it; then the
  name is the file name. Rename is F2, a double click on the card, or Rename in the card's right-click menu; Move to trash is Ctrl+Shift+Delete or the menu.
- The theme file is polled once a second; no file-watching crate is needed.
- Undo history is kept per sheet while another sheet is open. It comes back only if the sheet still reads the way it did when it was left, so an undo never lands on text that changed meanwhile.
- Not built: the "General" folder pill and the toolbar icons of the maintainer's notes, chips that show the
  referenced value instead of `lineN`.
