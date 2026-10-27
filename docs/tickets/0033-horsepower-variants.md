# 0033 Which horsepower

Status: closed (settled by a maintainer check, one deliberate difference)
Milestone: v1
Area: units

`hp` is mechanical horsepower, 550 ft·lbf/s (745.69987158227022 W), exactly. The reference lists 746 W, a rounding. Metric horsepower (735.49875 W) and electrical horsepower (746 W exactly) are separate units not defined here. Confirm the intended one against a real install.

Audit 2026-10-26: metric horsepower (`metric hp`, `cv`, 735.49875 W exactly) and electrical horsepower (`ehp`, 746 W) are units of their own (`tests/horsepower.rs`); before, `metric` was a skipped word and the line answered with the mechanical value. Sheet G asks what the expected answer is for `1 hp in W`.

Sheet G (the expected behaviour, 2026-10-26): The expected answer is `1 hp in W` as `745.7 W` and `1 hp in kW` as `0.7457 kW` (a rounded constant), and does not know `metric hp`. Babbage keeps the exact mechanical horsepower (745.69987158227022 W) and has metric (735.49875 W) and electrical (746 W) horsepower as units (deliberate, G19 to G21).
