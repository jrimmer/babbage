# 0048 Weeks counted in workdays

Status: settled for weeks and hours; days, months and years assumed
Milestone: v1
Area: units, dates

Checked by the owner on a real reference app: `$500/workday × 4 weeks` is `$10,000.00`, the same as `$500 × 5 × 4`, so a week is five workdays. The reference notes' other example, `55h in work days` = `6.875 workdays`, makes a workday 8 hours.

The engine reads both. A workday is the work-hours setting (8 hours by default), so hours convert at that rate. Whenever a workday is on either side or in the same expression, a calendar day, week, month or year is counted in working time: a week is exactly five workdays, whatever the work-hours setting, and a day is five sevenths of a workday. Without a workday in the expression, time is unchanged (a week is 168 hours and 7 days).

Not yet checked on a real install: `1 day in workdays` (the engine gives 5/7, `0.7142857143`), `1 month in workdays` (`21.740625`) and `1 year in workdays` (`260.8875`). They follow from a week being five workdays, but the reference app may count a day as one workday.
