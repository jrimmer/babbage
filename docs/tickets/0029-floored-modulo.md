# 0029 Floored versus truncated modulo

Status: closed (settled by a maintainer check)
Milestone: v1
Area: operators

`-7 mod 3` is -1 here (sign of the dividend). Many calculators give 2. The reference says only "the remainder after a division". Check a real install.

Sheet G (the expected behaviour, 2026-10-26): `-7 mod 3` is `2` and `7 mod -3` is `-2`: modulo is floored. Built, also for `%` between numbers and `remainder of A divided by B`; the function oracle uses Python's `%`.
