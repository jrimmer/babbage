# 0041 Sign of `earlier - later` for dates

Status: open
Milestone: v1
Area: dates

The reference lists `January 10 - February 5` as `3 weeks 5 days`. The exact result of an earlier date minus a later one is negative. The engine keeps the sign (`-3 weeks 5 days`), and `February 5 - January 10` gives `3 weeks 5 days`. Check a real install to see whether the reference app prints the magnitude, and which year it picks for year-less dates when the two dates fall on either side of today.
