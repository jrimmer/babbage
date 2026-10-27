# 0054 Custom unit words and fun units

Status: closed (settled by a maintainer check)
Milestone: v1
Area: units

The units notes shows `3 operations/second × 5 minutes` giving `900 operations`, so a word after a number that is not a known unit can act as a unit of its own, and `40 lb in watermelons` giving `2 watermelons` shows built-in fun units. The engine treats an unknown word as prose and drops it (`900`, with the word reported as ignored), and has no fun units. A real install is needed for how a custom unit is recognised (any word after a number? only after a rate?) and the list of fun units, before building either.

Settled by a maintainer check: a made-up word is dropped (`3 operations/second × 5 minutes` is `900`, `3 widgets × 4` is `12`) and there are no fun units (`40 lb in watermelons` is `40 lb`, ticket 0075). The engine already matches the first two.
