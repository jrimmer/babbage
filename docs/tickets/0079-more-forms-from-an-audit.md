# 0079 Calculation forms the release notes lists that the engine lacks

Status: open
Milestone: v1
Area: forms

From the audit of the release notes (claims; the engine's answers were probed): `5 root of 2`; `2 exponent 4`, `4 raised to 2`; `min 5, 3, 7`, `max 50, 20, 10`, `gcd 10, 20, 5`, `max (100 * 2) 50 100`, `total of 3, 4, 5 7 and 9`, `min of 3 and 4` and `5 feet and 3 inches in cm` (`and` is read as a bitwise AND and refuses); `is 59 prime`; `1 in 20 as %`; `100 is what multiple of 50` and `multiple on` / `off` (the engine drops the words and prints 5,000); `half of half of 30% of 90`; `round 21 up to nearest 5`, `round down 20 to nearest 8`; `500 to 3000` as a change; `10:15 to decimal` (the engine prints 36,900); `1 liter in imperial pints` (the engine drops `imperial`); `1 1/2 pounds`; `200 oz in pounds & ounces`; `$3mn` (dropped); `USD1K`; `（2+3）×4` (full-width brackets); `100²`; `cos(60º)`; `5 + 5 # comment`; `hourly interest on $1k for 3 years at 4%`; `investment required for $10k/month at 5%`; `daily payment on $1M over 30 years at 6.5%`; `ppi of 15" screen at 1024x768`; `3 mph to minutes`; `8g/2 lb as grams/lb`; `3 hours / 4 days to hours / day`; `1 L / 1 m^2`. Done in this pass: the cotangent and cosecant functions and their variants, `multiple` as a word for `x`, `halfway between`. Several are quiet wrong answers and should be fixed before the rest.
