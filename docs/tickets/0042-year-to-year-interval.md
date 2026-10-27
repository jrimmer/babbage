# 0042 `1978 to 2021` as an interval of years

Status: half closed (settled by a maintainer check)
Milestone: v1
Area: dates

The maintainer's notes lists `1978 to 2021` as `42 years`. The difference of the two years is 43, so a maintainer check is measuring something else (possibly midday to midday, or the two years taken as dates with today's month and day). The engine reads `to` as a unit conversion and refuses the line. Check a real install before choosing a rule.

The expected answer is `1978 to 2021` as `43` (ticket 0074).
