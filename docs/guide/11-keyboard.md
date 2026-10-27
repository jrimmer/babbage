# The keyboard

Everything you do all day in Babbage has a key. This page lists every shortcut and mouse gesture in the sheet, the
sheet list and the answers column. The **Help** button in the answers column shows a short version of the list at
any time.

Babbage uses **Ctrl**. A key it does not know, held with Ctrl or Alt, does nothing at all, so a slip never types a
stray character into your sheet.

## Sheets

| Keys | What they do |
|---|---|
| **Ctrl+N** | New sheet. It goes first in the sidebar and opens ready to type. |
| **Ctrl+Tab** | Next sheet, in the order the sidebar shows. |
| **Ctrl+Shift+Tab** | Previous sheet. |
| **Ctrl+B** | Show or hide the sidebar. |
| **F2** | Rename the open sheet. |
| **Ctrl+Shift+Delete** | Move the open sheet to the trash. |
| **Ctrl+Shift+T** | Bring back the sheet moved to the trash last, and open it. |
| **Ctrl+Shift+E** | Export the open sheet as text, CSV, HTML and PDF. |
| **Ctrl+Q** | Save and quit. |

Ctrl+Tab wraps round: from the last sheet it goes to the first. With one sheet, it does nothing.

### While you rename

**F2** turns the sheet's name in the sidebar into a field. Type the new name, then press **Enter** to accept it,
or **Esc** to leave the name as it was. **Backspace** deletes a character.

## Moving around

| Keys | What they do |
|---|---|
| **Left**, **Right** | One character. |
| **Up**, **Down** | One line. In a line that wraps onto several rows, they move between its rows first. |
| **Ctrl+Left**, **Ctrl+Right** | One word. |
| **Home**, **End** | Start or end of the line. |
| **Ctrl+Home**, **Ctrl+End** | Start or end of the sheet. |
| **Page Up**, **Page Down** | About one screen up or down. |
| **Mouse wheel** | Scrolls the sheet. |

A reference to an earlier line, such as `line3`, is drawn as a small chip showing that line's answer. The caret
steps over a chip in one move, rather than into it.

## Selecting

Hold **Shift** with any of the movement keys above to extend the selection.

| Gesture | What it does |
|---|---|
| **Ctrl+A** | Select the whole sheet. |
| **Click**, then **drag** | Select by dragging. |
| **Shift+click** | Extend the selection to where you click. |
| **Double-click** | Select a word. |
| **Triple-click** | Select the whole line. |

## Editing

| Keys | What they do |
|---|---|
| **Enter** | New line. |
| **Backspace**, **Delete** | Remove a character before or after the caret, or the selection. |
| **Ctrl+Backspace**, **Ctrl+Delete** | Remove a word before or after. |
| **Ctrl+Z** | Undo. |
| **Ctrl+Shift+Z** or **Ctrl+Y** | Redo. |
| **Ctrl+C**, **Ctrl+X**, **Ctrl+V** | Copy, cut, paste. |
| **Ctrl+/** | Comment the selected lines out with `// `, or take the comments off again if every line with text is already one. |

Backspace right after a chip removes the whole reference, and Delete just before one does the same.

A run of typing without a space in it is one undo step, so Ctrl+Z takes back a word at a time rather than a letter
at a time. Pasting, cutting, and each format change are steps of their own. Each sheet keeps its own undo history while you
work in another one, and it comes back when you return to a sheet that has not changed.

If you have turned on **Convert foreign numbers when pasting** in [Settings](12-settings.html), a number pasted in
another region's format, such as `1.234,5`, is turned into the format Babbage is using.

**Tab** does nothing in a sheet, unless you have chosen it to finish variable names (below).

## Finishing a variable name

Start typing the name of a variable defined above the line, and press **Esc**. Babbage finishes it. If you type
`mon` and you have a variable called `monthly rent`, Esc makes it `monthly rent`.

- If only one name fits, it is put in at once.
- If several fit, a small list appears under the caret. **Up** and **Down** move through it, and **Enter**,
  **Tab** or **Esc** takes the highlighted name. Any other key closes the list and carries on typing.

The names are matched without regard to case and listed in alphabetical order. They are the names defined on
earlier lines, never the line you are on. In [Settings](12-settings.html) under Editing, **Key that finishes a
variable name** can be changed from Esc to **Tab**. Then Tab starts the completion and Esc does nothing.

## Counting a number up and down

Hold **Alt**, press on a number in the sheet, and drag left or right. The number counts by its last digit, a step
for about every 6 points you move. Drag right to count up and left to count down.

| The number | One step up |
|---|---|
| `12` | `13` |
| `12.5` | `12.6` |
| `9` | `10` |
| `1,234` | `1,235` |

The number keeps its decimal places and its thousands commas, and it never goes below zero. Every answer on the sheet
updates as you drag. Releasing the button ends the drag, and the whole drag is one undo step, so **Ctrl+Z** puts
the number back as it was.

Only a plain number is counted: digits with an optional decimal part. A number with a sign, an exponent, such as
`1e3`, or a suffix, such as `5k`, is not.

## The answers column

| Gesture | What it does |
|---|---|
| **Hover** over an answer | A round **⋯** button appears at its right. |
| Click the **⋯** | Open that answer's menu. See [Answers and formats](09-answers-and-formats.html). |
| **Click** an answer | Highlight it. The caret moves to the start of that line. |
| **Ctrl+C** on a highlighted answer | Copy the answer's text. |
| **Up**, **Down** on a highlighted answer | Highlight the answer above or below, moving the caret with it. |
| **Left**, **Esc** on a highlighted answer | Go back to the sheet. |
| **Alt+Right** in the sheet | Highlight the answer of the line the caret is on. |
| **Esc** with a menu open | Close it. |

While an answer is highlighted, every other key does what it normally does. Typing or editing in the sheet clears the
highlight.

In the sheet itself, **hover** over the name of a variable to see its value in a small tip beside it.

## The sheet list

| Gesture | What it does |
|---|---|
| **Click** a card | Open that sheet. |
| **Double-click** a card | Open it and start renaming it. |
| **Right-click** a card | Open it and show its menu: **Rename** and **Move to trash**. |
| Click **+ Sheet** | Make a new sheet. |

Any key closes the card's menu. A sheet moved to the trash is not erased: see
[Sheets and files](10-sheets-and-files.html).

## Toolbar and panels

| Gesture | What it does |
|---|---|
| Click **Settings** or **Help** | Open that panel; click again to close it. |
| Click the **sidebar** button | Show or hide the sidebar, like **Ctrl+B**. |
| **Esc** | Close the open panel or menu. |

In a text field in the Settings panel, type and press **Enter** to save the value, or **Esc** to throw away what you
typed. **Esc** there cancels that edit only; the panel stays open.

When you rename a variable that later lines use, Babbage may offer to rename those uses too. Click **Update** to
change them or **Keep** to leave them. What it does on its own is a setting under Editing: **When a variable is
renamed**.

## Quick reference

| Keys | |
|---|---|
| Ctrl+N / Ctrl+Tab / Ctrl+Shift+Tab | New, next, previous sheet |
| Ctrl+B | Sidebar |
| F2 | Rename sheet |
| Ctrl+Shift+Delete | Trash |
| Ctrl+Shift+T | Bring back the last trashed sheet |
| Ctrl+Shift+E | Export |
| Ctrl+Q | Quit |
| Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y | Undo / redo |
| Ctrl+A / C / X / V | Select all, copy, cut, paste |
| Ctrl+/ | Comment in or out |
| Esc | Finish a variable name; close a menu or panel |
| Alt+Right | Highlight the line's answer |
| Alt + drag a number | Count it up or down |
