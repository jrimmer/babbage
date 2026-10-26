# 0071 An ISO-shaped line that is not a date is a subtraction

Status: closed
Milestone: v1
Area: dates

The reference app prints `2019-02-30` as `1,987`, which is `2019 - 02 - 30`. The engine reports an invalid number. A date that is not a date (month 13, day 30 in February) should fall back to ordinary arithmetic, as `1 + 2019-04-01` does for a valid date written after other tokens.

Fixed: text shaped like an ISO date that is not one (`2019-02-30`, `2019-13-01`) is plain arithmetic (`1,987`), as in the reference app. Pinned in `tests/lexer_edges.rs`, `tests/time_edges.rs` and the regenerated ISO oracle.
