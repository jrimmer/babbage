# 0011 Show repeating decimals exactly

Status: built (the answer menu's Show as Repeating, kept per line as repeat=on, period capped at 50 digits)
Milestone: v1.1
Area: formatting, precision

Render `1/3` as `0.(3)` or with a vinculum on request, using the exact period instead of rounding. Pure exact arithmetic on the denominator, no data needed. Test with Python long division as the oracle.
