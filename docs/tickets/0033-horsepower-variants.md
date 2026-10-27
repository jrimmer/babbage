# 0033 Which horsepower

Status: mostly built (which `hp` is the default is a question)
Milestone: v1
Area: units

`hp` is mechanical horsepower, 550 ft·lbf/s (745.69987158227022 W), exactly. The reference lists 746 W, a rounding. Metric horsepower (735.49875 W) and electrical horsepower (746 W exactly) are separate units not defined here. Confirm the intended one against a real install.

Audit 2026-10-26: metric horsepower (`metric hp`, `cv`, 735.49875 W exactly) and electrical horsepower (`ehp`, 746 W) are units of their own (`tests/horsepower.rs`); before, `metric` was a skipped word and the line answered with the mechanical value. Sheet G asks what the reference app shows for `1 hp in W`.
