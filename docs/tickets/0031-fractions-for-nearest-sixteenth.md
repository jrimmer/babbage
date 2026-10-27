# 0031 Show `0.534 to nearest 16th` as 9/16

Status: closed
Milestone: v1
Area: display

The engine returns 0.5625 (exact). The maintainer's notes shows `9/16`. Needs a fraction display kind (ticket 0014).

Fixed: `to nearest 16th` (and any `Nth`) shows the answer as a fraction, as the real number-rounding page says (`0.534 to nearest 16th` is `9/16`). Pinned in the tests and `tests/functions.rs`.
