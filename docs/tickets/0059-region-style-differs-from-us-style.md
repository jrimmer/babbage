# 0059 The reference notes are written in an Australian region

Status: closed for the US region (the reference app)
Milestone: v1
Area: region

The reference notes print dates day first (`10 June + 3 weeks | 1 July`, `3 weeks after March 14, 2019 | 4 April 2019`, `12/02/1988 + 32 years | 12 February 2020`, `01.05.2005 + 3 years 2 months 3 weeks | 22 July 2008`), a year only when it is not the current one, and a mix of spaced and comma styles (`February 29 2020` against `April 1, 2019`). The engine prints `July 1` and `February 29, 2020`. The date style is probably a region setting, so the engine may need a day-first option plus day-first reading of `12/02/1988` and `01.05.2005`. Compare lines 17 and 18 of Sheet A in `docs/reference-checks.md` with the reference app before building.

The reference app (region United States) prints `July 1` and `February 29, 2020`: month first with a comma, as the engine does. Day-first output and day-first slash dates for other regions are still unbuilt, but no owner reading needs them.
