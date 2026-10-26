# v1 roadmap (offline features only)

Status as of 2026-10-25. Every row ships only after a Python-oracle differential test and a clean cargo-mutants run.

| # | Area | Status |
|---|---|---|
| 1 | Arithmetic core: + - × ÷ ^, parentheses, literals, aliases | done |
| 2 | Display: rounding modes, grouping, exact/rounded flag | done |
| 3 | Percentages: of, +/-, on/off, reverse forms, ratios, as % | done (sub-forms below are todo) |
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
| 15 | Rates, growth and change, cooking volumes | todo |
| 16 | Dates and calendar math | todo |
| 17 | Workdays and weekdays | todo |
| 18 | Timestamps and ISO 8601 | todo |
| 19 | Clock time, timespans, lap times, video timecode | todo |
| 20 | Time zones and cities (bundled tzdb) | todo |
| 21 | Finance formulas: compound interest, loans, sales tax | todo |
| 22 | Number locale settings | todo |
| 23 | Sheets, sheetbooks, reference-format-like file format | todo |
| 24 | gpui front end | todo |

14 of 24 done. The hardest remaining precision areas are 16 and 21 (dates, finance).

Row 3 leftovers, each filed as a ticket: fraction display (`as fraction`), multipliers (`4x`), proportion forms (`20% is 500, what is 750`), and phrase forms inside parentheses.

Open scope question: PDF and HTML export, Alfred/Raycast, URL schemes and its Studio edition are not in the list. The first two need no outside data, so by your rule they would be v1. I assumed they are not wanted on Linux.

Acceptance targets from the target screenshot live in `crates/babbage-core/tests/screenshot_acceptance.rs` as ignored tests. `cargo test -- --ignored` lists what is still missing.
