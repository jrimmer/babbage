# Decisions (settled; reopen only on purpose)

| Date | Decision | Why |
|---|---|---|
| 2026-10-25 | All arithmetic is exact rationals on bigints. No f64 in the engine. | Precision is the top requirement. |
| 2026-10-25 | Irrationals (sqrt, trig, ln, pi) use 50 working digits, correctly rounded, always flagged approximate unless the result is exactly rational. | Owner choice. Symbolic is ticket 0001. |
| 2026-10-25 | UI is gpui. Engine is a separate UI-free crate. | Owner choice, matches casabon2. |
| 2026-10-25 | Anything needing an outside data source is post-v1 (tickets 0003 to 0010). | Owner rule. |
| 2026-10-25 | Result size cap is 2^20 bits. Over the cap is an error, never a silent approximation. Fallback is ticket 0002. | An unflagged approximation would break the precision promise. The 10,950-period daily compounding case fits and is exact. |
| 2026-10-25 | The exponent has no fixed cap. Only the predicted result size limits `^`. `0^n`, `1^n` and `(-1)^n` work for any integer n. | A fixed 4096 cap wrongly refused real finance cases. |
| 2026-10-25 | Default display rounding is half away from zero at 10 places. Half-even, toward zero and away from zero are available. | Matches the target screenshot. |
| 2026-10-25 | `0^0` is an error. | No single convention is safe. Revisit if it bites. |
| 2026-10-25 | A value that rounds to zero prints `0`, never `-0`, and is flagged inexact. | Avoids a misleading sign. |
| 2026-10-25 | Thousands separators are strict: `1,024` is a number, `1,2` and `1234,567` are errors. | A comma that is not a valid group is never silently merged into a number. |
| 2026-10-25 | No trailing-dot numbers: `5.` is an error. | Strictness over guessing. |
| 2026-10-26 | A percentage is stored as its fraction (10% is 1/10) and carries a `Kind`. Display multiplies by 100. | Keeps every percent operation exact rational arithmetic. |
| 2026-10-26 | Attached `%` (`10%`, `(1/2)%`, `10%%`) is a postfix on the value before it. A detached trailing `%` (`20/200 %`) means "as percent". | Both forms appear in the reference docs and mean different things. |
| 2026-10-26 | Where the percentage docs are silent, the engine returns `UnsupportedOperation` instead of guessing: `10% x 10%`, any division involving a percentage, `10%^2`, `2^10%`. | A guess would be an unverifiable precision claim. Verify against the real app, then relax (ticket 0012). |
| 2026-10-26 | Fractions and decimals work wherever a percentage rate is expected in `of`, `on`, `off` and the `is ... of what` forms. | The reference notes say a decimal or fraction can replace a percentage. |
| 2026-10-26 | Multi-word phrase forms apply to a whole line only, matched from a table (`phrase.rs`). `of`, `on`, `off` alone are infix operators; `of` equals `x`. | Keeps parsing iterative and the phrase list data-driven. Nested phrases are ticket 0013. |
| 2026-10-26 | Unknown words are an error (`UnexpectedWord`), not ignored. | The reference app ignores free text as labels; that is row 9 and must be deliberate, not an accident of the parser. |
| 2026-10-26 | Money: each currency is its own dimension, so `$ + EUR` is `CurrencyRateNeeded`, never a number. Live rates are post-v1. A rate the user writes (`50 EUR in USD at 1.05 USD/EUR`, or `@`) works now. | Precision: a wrong rate is worse than no answer. |
| 2026-10-26 | `$` means USD. Other dollars: `A$`, `C$`, `NZ$`, `S$`, `HK$`, `NT$`, `R$`, `US$`. Region settings are ticket 0027. | The reference ties `$` to the Mac region; this engine has none yet. |
| 2026-10-26 | Money shows its ISO 4217 digits (2, 0 for JPY, 3 for BHD); crypto and commodities show every digit. Declaration lines show money unrounded. | Matches the reference's `$999.00` and the variables notes. |
| 2026-10-26 | Default display abbreviates from 100,000 (`300k`, `3.3M`, `16.777216M`), moving the decimal point only. The threshold is inferred from the reference's `300k` and `16,384` examples. | Unverified against the app: ticket 0028. |
| 2026-10-26 | `$30 x 4 days` is `$120.00`: a price times a count of anything keeps the price's unit. | The reference states it. |
| 2026-10-26 | Sheets: top to bottom. A line sees only lines above it. Subtotals stop at the previous subtotal, a heading or a divider; percentage lines apply to the subtotal at the end. Dividers reset variables and tags. Declaration lines count in totals. | The reference says so, except declarations in totals (it names a setting, no default). |
| 2026-10-26 | Natural mode ignores unknown words and lists every one. It refuses a dropped stretch that stands where a number should be (`y + 1`, `(y) + 1`, `5 x oops`). It only warns about a word one edit from a unit (`5 kmm`). | `25% tip` must work; a mistyped variable must never become a quiet wrong answer. |
| 2026-10-26 | A lone `x` is the times sign between two operands and a variable name anywhere else. | Variables named `x` are common. |
| 2026-10-26 | Variable names match case-exactly, else case-insensitively when only one variable fits. Declare before use. | The reference gives neither rule beyond declare-before-use. |
