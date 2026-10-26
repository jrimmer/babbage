# 0071 An ISO-shaped line that is not a date is a subtraction

Status: open
Milestone: v1
Area: dates

The reference app prints `2019-02-30` as `1,987`, which is `2019 - 02 - 30`. The engine reports an invalid number. A date that is not a date (month 13, day 30 in February) should fall back to ordinary arithmetic, as `1 + 2019-04-01` does for a valid date written after other tokens.
