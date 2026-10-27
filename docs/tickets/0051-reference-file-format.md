# 0051 Importing the reference app files and exporting sheets

Status: closed (owner decision: our own .bbg file; no reference app import)
Milestone: v1
Area: files

Sheets are stored as plain text, one `.bbg` file per sheet (first named `.babbage`; the book renames those when it opens), one line of the sheet per line of the file. The reference describes the reference app's own reference-format files, sheetbooks and syncing, and PDF and HTML export. The reference-format layout was not available to read, so importing one is not built. Exporting to plain text is just the file. PDF and HTML export need no outside data and are open scope (see the roadmap note). Syncing between machines is out of scope for an offline v1.

Built (the reference notes list these export formats): `Ctrl+Shift+E` writes the open sheet with its answers as plain text (`line | answer`, aligned), CSV (`line,answer` rows and a `Total` row), a self-contained HTML page and a PDF (A4, monospaced, as many pages as it takes, long lines wrapped, answers in a right-hand column) into the sheetbook's `exports` folder and names the folder in a notice. The code is `babbage-store/src/export.rs` with unit tests, and the `export` acceptance check reads the PDF back with `qpdf --check` and `pdftotext`. The layouts of the text and CSV forms are my reading of the reference notes (they show `line | answer` in their examples and say only `csv` and `txt`), not checked against the reference app.

Still open: importing and exporting the reference app's own reference-format files. The reference notes say only that it is JSON; Sheet D line 59 asks the owner to export a one-line sheet and paste the file, which is all an importer needs.

Sheet D line 62 asked for the exported text of a one-line sheet; the answer pasted was `3` (the line's own answer), not a file. The importer stays blocked on a real reference-format sample. To get one: File > Export on a sheet holding only `1 + 2`, open the saved file in a text editor and paste its whole text.

Closed by the owner (2026-10-26): Compatibility with the reference app is not absolute, the target is its breadth of features, and any file the tooling produces should be our own `.bbg` file. So there is no reference-format importer or exporter and none is waiting on a sample. The sheet file is now `<name>.bbg` (plain text, one sheet line per file line); `SheetBook::open` renames `<name>.babbage` files from the first version to `.bbg` where no `.bbg` of that name exists and leaves a pair alone (`a_sheet_saved_under_the_first_extension_is_renamed_when_the_book_opens`).
