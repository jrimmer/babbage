# 0061 Calendar spans and dates moved by several units

Status: open
Milestone: v1
Area: dates

The reference notes show: `April 1, 2019 − 3 months 5 days | 27 December 2018`, `2 months 3 days after June 5 | 8 August`, `01.05.2005 + 3 years 2 months 3 weeks | 22 July 2008`, `3 March to 30 May | 2 months 3 weeks 6 days`, `April 1 through April 30 in days | 30 days`, `10 March to 17 March in workdays | 5 workdays`, `day of month on March 15, 2024 | 15`. The engine refuses a date moved by months and days together, shows an interval as weeks and days only, and has no `through`, no `A to B in workdays` and no `day of month`.
