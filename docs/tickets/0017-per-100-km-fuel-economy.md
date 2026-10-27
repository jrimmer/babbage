# 0017 L/100km and other units with a number in the denominator

Status: closed (settled by a maintainer check)
Milestone: v1
Area: units, parser

`L/100km` has a plain number inside the unit. Unit runs accept only unit tokens today. MPG and km/L are supported.

A maintainer check (2026-10-26): `8 L/100km` prints `8 L/100km`; `8 L/100km in mpg` is `29.4018229464 MPG` and `30 mpg in L/100km` is `7.840486119 L/100km`. Built: `L/100km` is a unit that keeps its name and converts to and from distance per volume by the reciprocal (`MPG` is now a unit of its own and prints `MPG`). Babbage prints `29.4018229167` and `7.8404861111`, the exact US gallon and mile; a maintainer check is about one part in a billion off (deliberate, G30 and G31).
