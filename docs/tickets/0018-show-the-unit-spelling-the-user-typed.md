# 0018 Show the unit spelling the user typed

Status: closed (the reference app)
Milestone: v1
Area: display

The reference shows `19,800 seconds` after `5 hours 30 minutes to seconds` but `10,000 m` after `10 km in m`. The engine always prints the symbol. Needs a per-token spelling carried on the unit.

Settled by the reference app (docs/reference-answers.md): a conversion prints the symbol (`19,800 s`, `10,000 m`, `120 min`), so the engine already matches. The page's `seconds` is older or another region. Timespans and rates have their own display rules: tickets 0069 and 0064.

Unit names: conversions print the symbol (`19,800 s`, `120 min`); hours, days, weeks, months, years, workdays and frames print as words wherever they appear (ticket 0064); minutes and seconds are `min` and `s`.
