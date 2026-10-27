# 0021 Temperature differences and arithmetic

Status: closed (settled by a maintainer check)
Milestone: v1
Area: units

Today a temperature can be converted and shifted by a plain number. `20 C + 5 C`, scaling and percentages are refused because the reference does not define them. Decide with a maintainer check (ticket 0012 style).

Sheet G (the expected behaviour, 2026-10-26): `20 C + 5 C` is `25 °C`, `20 C × 2` is `40 °C` and `20 C + 10%` is `22 °C`. Built: a temperature adds to and subtracts from a plain number, a percentage or a temperature of the same scale, and is scaled by a plain number. Two scales (`20 C + 5 F`) and a product of two temperatures stay refused: no sheet asked.
