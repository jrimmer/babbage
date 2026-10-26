# 0064 Rates shown and combined as per-period values

Status: open
Milestone: v1
Area: units, display

The reference notes show `3 hours / day | 3 hours/day`, `30 bottles / week | 30/week`, `30/week as / month | 130/month`, `500/day to / year | 182,621.25/year`, `$20/day + $300/week | $440.00/week`, `$30/day is what per year | $10,957.28/year`, `$99 per week | $99.00/week`. The engine prints these with the unit symbols and exponents (`30 wk⁻¹`, `182.62125k yr⁻¹`), turns `3 hours / day` into a plain number, and does not add rates of different periods. Related to 0018, 0019 and 0017.
