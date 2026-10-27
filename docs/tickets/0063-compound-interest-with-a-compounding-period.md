# 0063 Compounding monthly or quarterly in interest phrases

Status: closed
Milestone: v1
Area: finance

The maintainer's notes show `$1,000 for 3 years at 7% compounding monthly | $1,232.93`, `... compounding quarterly | $1,231.44`, and `interest on $1,000 for 3 years at 7% compounding monthly | $232.93`. The engine has `after 3 years at 7%` (`$1,225.04`) but not the `for` form or a compounding period.

Fixed: `for` reads as `after` in the compound, interest-on and present-value forms; the four examples print `$1,232.93`, `$1,231.44`, `$232.93` and `$231.44`, as on the maintainer's notes note. Pinned in `tests/finance.rs`.
