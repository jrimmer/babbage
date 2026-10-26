# 0051 Importing the reference app files and exporting sheets

Status: half closed
Milestone: v1
Area: files

Sheets are stored as plain text, one `.babbage` file per sheet, one line of the sheet per line of the file. The reference describes the reference app's own reference-format files, sheetbooks and syncing, and PDF and HTML export. The reference-format layout was not available to read, so importing one is not built. Exporting to plain text is just the file. PDF and HTML export need no outside data and are open scope (see the roadmap note). Syncing between machines is out of scope for an offline v1.

Built (the reference notes list these export formats): `Ctrl+Shift+E` writes the open sheet with its answers as plain text (`line | answer`, aligned), CSV (`line,answer` rows and a `Total` row), a self-contained HTML page and a PDF (A4, monospaced, as many pages as it takes, long lines wrapped, answers in a right-hand column) into the sheetbook's `exports` folder and names the folder in a notice. The code is `babbage-store/src/export.rs` with unit tests, and the `export` acceptance check reads the PDF back with `qpdf --check` and `pdftotext`. The layouts of the text and CSV forms are my reading of the reference notes (they show `line | answer` in their examples and say only `csv` and `txt`), not checked against the reference app.

Still open: importing and exporting the reference app's own reference-format files. The reference notes say only that it is JSON; Sheet D line 59 asks the owner to export a one-line sheet and paste the file, which is all an importer needs.
