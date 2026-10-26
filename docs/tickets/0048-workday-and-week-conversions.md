# 0048 Weeks counted in workdays

Status: open
Milestone: v1
Area: units, dates

The workdays notes lists `$500/workday × 4 weeks` as `$10,000.00`, which counts 4 weeks as 20 workdays, and `55h in work days` as `6.875 workdays`, which counts a workday as 8 hours. Those cannot both come from one linear unit: 4 weeks is 672 hours, which is 84 workdays of 8 hours. The engine takes a workday as the work-hours setting (8 hours by default), so `55h in work days` is `6.875` and `$500/workday × 4 weeks` is `$42,000.00`. Check a real install for how a week or a day converts to workdays.
