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
