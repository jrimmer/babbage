# v1 roadmap (offline features only)

Status as of 2026-10-26. Every row ships only after a Python-oracle differential test and a clean cargo-mutants run.

| # | Area | Status |
|---|---|---|
| 1 | Arithmetic core: + - × ÷ ^, parentheses, literals, aliases | done |
| 2 | Display: rounding modes, grouping, exact/rounded flag | done |
| 3 | Percentages: of, +/-, on/off, reverse forms, ratios, as % | done, including phrase forms inside parentheses |
| 4 | Units and exact conversion factors (200+) | done (temperatures convert only; radians wait for row 12) |
| 5 | Currency symbols and money display (no rates) | done (live rates are post-v1; custom rates work) |
| 6 | Variables | done (global definition sheet is ticket 0022) |
| 7 | Line references | done (`line3`, `line 3`) |
| 8 | Totals, subtotals, tags | done (typed `subtotal`, `total of #tag`; ticket 0023) |
| 9 | Comments, labels, headings | done |
| 10 | Large number symbols (k, M, B) and SI display | done (display threshold unverified, ticket 0028) |
| 11 | Rounding, averages, median, stats, gcd/lcm, permutations, clamp, operator words, mod | done |
| 12 | Roots, logs, trig, constants (115-digit, flagged approximate) | done |
| 13 | Conditionals and comparisons | done |
| 14 | Bases and bitwise | done (non-negative whole numbers) |
| 15 | Rates, growth and change, multipliers (cooking is post-v1, ticket 0035) | done |
| 16 | Dates and calendar math | done: 24k oracle lines against Python datetime and dateutil; mutation clean (see `docs/mutation.md`) |
| 17 | Workdays and weekdays | done: workdays follow the work-hours setting and a week is five workdays (checked on a real reference app); holidays are ticket 0047 |
| 18 | Timestamps and ISO 8601 | done: 11k ISO 8601 lines against Python datetime, timestamps in seconds and milliseconds; reference zone question is ticket 0043 |
| 19 | Clock time, timespans, lap times, video timecode | done: 82k oracle lines for clock times, timespans, lap times and timecode; timecode syntax beyond the basics is ticket 0044, display choices are 0045 |
| 20 | Time zones and cities (bundled tzdb) | done: 8k lines against Python zoneinfo plus 8k against the machine's own rules; the bundled database is 2025b and the app reads the machine's (ticket 0053) |
| 21 | Finance formulas: compound interest, present value, returns, loans, sales tax (CPI, income tax tables: post-v1) | done |
| 22 | Number locale settings | done: three number formats, underscore separators, the user's own zone with daylight saving, the region dollar, tax setting; the volume setting has no effect in the reference app, so there is none (ticket 0049) |
| 23 | Sheets, sheetbooks, own file format | done as `babbage-store`: one `.bbg` text file per sheet (our own format, by the owner's choice, not the reference app's own file format), atomic writes, a debounced autosaver, a sheetbook with create, rename and move-to-trash, and export of a sheet as text, CSV, HTML and PDF (`Ctrl+Shift+E`, read back by `qpdf` and `pdftotext` in `bin/accept`); importing reference-format files is dropped (owner decision, ticket 0051) |
| 24 | gpui front end | done: sidebar, editor, answer column, total chip, Omarchy theme; 29 end-to-end checks in a nested Hyprland, keyboard and mouse included (`bin/accept`); gaps against the reference are ticket 0052 and judging looks is yours |

All 24 rows are built. Every row has oracle or reference tests, and the engine rows have a mutation record (`docs/mutation.md`). What is left is filed as tickets: things that need a real reference app to settle (0018, 0030, 0043 to 0046, 0055), shipped data (0035 to 0038, 0047), and app gaps (0052). `bin/check` runs 519 tests across 49 suites over about 663,000 oracle and corpus lines, plus a replay of the 99 lines (and four Totals) the owner ran in the reference app (`docs/reference-answers.md`, which beats the reference notes) and 350 examples read from the raw reference notes (104 known differences are listed with their tickets), and `bin/accept` passes 29 of 29 in the nested Hyprland.

Row 3 leftovers: fraction display, multipliers and the proportion forms are built (ticket 0014). Phrase forms inside parentheses work too (ticket 0013).

Open scope question: PDF and HTML export, Alfred/Raycast, URL schemes and its Studio edition are not in the list. The first two need no outside data, so by your rule they would be v1. I assumed they are not wanted on Linux.

Acceptance targets from the target screenshot live in `crates/babbage-core/tests/screenshot_acceptance.rs` as ignored tests. `cargo test -- --ignored` lists what is still missing.
