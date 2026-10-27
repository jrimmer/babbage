# 0041 Sign of `earlier - later` for dates

Status: closed (settled by a maintainer check)
Milestone: v1
Area: dates

The maintainer's notes lists `January 10 - February 5` as `3 weeks 5 days`. The exact result of an earlier date minus a later one is negative. The engine keeps the sign (`-3 weeks 5 days`), and `February 5 - January 10` gives `3 weeks 5 days`. Check a real install to see whether the expected answer is the magnitude, and which year it picks for year-less dates when the two dates fall on either side of today.

A maintainer check (2026-10-26): `January 10 - February 5` is `3 weeks 5 days` and `February 5 - January 10` is `11 months 5 days`. Built: two dates without a year on a line of their own subtract to the span forward from the first to the next occurrence of the second. With a year written the difference keeps its sign.
