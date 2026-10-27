# 0051 Importing another app's files and exporting sheets

Status: closed (maintainer decision: our own .bbg file; no import of another app's files)
Milestone: v1
Area: files

Sheets are stored as plain text, one `.bbg` file per sheet (first named `.babbage`; the book renames those when it opens), one line of the sheet per line of the file. Other apps keep their own file formats, sheetbooks and syncing, with PDF and HTML export. Their file layout was not available to read, so importing one is not built. Exporting to plain text is just the file. PDF and HTML export need no outside data and are open scope (see the roadmap note). Syncing between machines is out of scope for an offline v1.

Built (the maintainer's notes list these export formats): `Ctrl+Shift+E` writes the open sheet with its answers as plain text (`line | answer`, aligned), CSV (`line,answer` rows and a `Total` row), a self-contained HTML page and a PDF (A4, monospaced, as many pages as it takes, long lines wrapped, answers in a right-hand column) into the sheetbook's `exports` folder and names the folder in a notice. The code is `babbage-store/src/export.rs` with unit tests, and the `export` acceptance check reads the PDF back with `qpdf --check` and `pdftotext`. The layouts of the text and CSV forms are my reading of the maintainer's notes (they show `line | answer` in their examples and say only `csv` and `txt`), not checked against a maintainer check.

Still open: importing and exporting another app's files. All that is known is that the format is JSON; Sheet D line 59 asks the maintainer to export a one-line sheet and paste the file, which is all an importer needs.

Sheet D line 62 asked for the exported text of a one-line sheet; the answer pasted was `3` (the line's own answer), not a file. The importer stays blocked on a real sample file. To get one: File > Export on a sheet holding only `1 + 2`, open the saved file in a text editor and paste its whole text.

Closed by the maintainer (2026-10-26): Compatibility with other apps is not absolute, the target is breadth of features, and any file the tooling produces should be our own `.bbg` file. So there is no importer or exporter and none is waiting on a sample. The sheet file is now `<name>.bbg` (plain text, one sheet line per file line); `SheetBook::open` renames `<name>.babbage` files from the first version to `.bbg` where no `.bbg` of that name exists and leaves a pair alone (`a_sheet_saved_under_the_first_extension_is_renamed_when_the_book_opens`).
