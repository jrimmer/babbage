# 0061 Calendar spans and dates moved by several units

Status: mostly closed
Milestone: v1
Area: dates

The reference notes show: `April 1, 2019 − 3 months 5 days | 27 December 2018`, `2 months 3 days after June 5 | 8 August`, `01.05.2005 + 3 years 2 months 3 weeks | 22 July 2008`, `3 March to 30 May | 2 months 3 weeks 6 days`, `April 1 through April 30 in days | 30 days`, `10 March to 17 March in workdays | 5 workdays`, `day of month on March 15, 2024 | 15`. The engine refuses a date moved by months and days together, shows an interval as weeks and days only, and has no `through`, no `A to B in workdays` and no `day of month`.

Built: a date moved by several lengths in turn (`April 1, 2019 − 3 months 5 days` is December 27, 2018; `2 months 3 days after June 5` is August 8; `June 5 + 1 year 2 months 3 weeks`), the calendar span between two dates (`3 March to 30 May` is `2 months 3 weeks 6 days`), `3 March to 30 May in days` and `10 March to 17 March in workdays`, and `April 1 through April 30 in days`. Year-less dates in a range are all this year. Two examples remain listed only for the page's day-first date style. Open: dotted dates (`01.05.2005 + 3 years 2 months 3 weeks`), which depend on the region's day order (Sheet D asks).

Settled by a maintainer check (Sheet D line 47): `01.05.2005 + 1 day` is `1 day`. A dotted run is neither a date nor a number and is skipped, so the region's day order does not matter for it.
