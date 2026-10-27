# 0064 Rates shown and combined as per-period values

Status: closed (settled by a maintainer check and the reference note)
Milestone: v1
Area: units, display

The reference notes show `3 hours / day | 3 hours/day`, `30 bottles / week | 30/week`, `30/week as / month | 130/month`, `500/day to / year | 182,621.25/year`, `$20/day + $300/week | $440.00/week`, `$30/day is what per year | $10,957.28/year`, `$99 per week | $99.00/week`. The engine prints these with the unit symbols and exponents (`30 wk⁻¹`, `182.62125k yr⁻¹`), turns `3 hours / day` into a plain number, and does not add rates of different periods. Related to 0018, 0019 and 0017.

Built: hours, days, weeks, months, years, workdays and frames print as words (plural unless the amount is one, singular after a slash); minutes and seconds stay `min` and `s`; a speed such as `km/h` keeps its symbols. A bare unit after `/` or `per` is a rate (`3 hours / day` is `3 hours/day`, `$99 per week` is `$99.00/week`, `30 bottles / week` is `30/week`); a quantity over a quantity cancels (`3 hours / 3 days` is `0.0416666667`) and `as hour/day` shows it as a rate; two rates add in the longer period (`$20/day + $300/week` is `$440.00/week`); `X is what per year` restates a rate. Pinned by the owner replay, the real-page replay and the regenerated oracles. Still different from the page: `30/week as / month` (the page rounds to 130) and large rates (`500/day to / year` prints `182.62125k/year`, the page `182,621.25/year`).
