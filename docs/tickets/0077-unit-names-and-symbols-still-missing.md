# 0077 Unit names and symbols the unit reference lists that the engine lacks

Status: open
Milestone: v1
Area: units

From the audit of `unit-reference.md` (claims checked by probing the engine): feet and inches written `'` and `"` (`5'6"`, `3' 4" + 9' 2"`); ares `a` (a bare `a` is also the word, so `5 a in m²` is dropped as prose and gives `5 m²`); `W s` and `W m` (watt seconds and minutes; read as watt-metres); `ppi` and `dpi` as units in their own right (they work only inside `at N ppi`); CSS `em`, `rem`, `Pt`; molar (`ᴍ`, `molar`); `kΩ`, `MΩ`, `mΩ`, `µΩ`; `kilowatt hours`, `milliampere hours` and the other prefixed two-word names; `newton meters`, `pound feet`; `millibars`, `kilopounds per square inch`; `meters per second squared` (also ticket 0019); `khz` in lower case. Display symbols also differ from the reference in places (`°` for degrees, `fl oz`, `mph`, `Ω`); the owner has not been asked. Values below about 1e-10 print `0` (`1 ng in kg`). Done in this pass: `sq ft`, `sq in`, `sq yd`, `sq mi`, `cu ft`, `in²`, `in^2`, `in2`, `in³`, `ounce troy`, `hz`, and every `Bps` and `bps` data rate with SI and binary prefixes.

Built since: imperial gallon, quart, pint and fluid ounce as units of their own (`imperial pint`, `imp pt`, `uk pint`), so the release notes's `1 liter in imperial pints` works without touching the volume setting.

Built since (the reference app, A82 to A88): `kΩ`, `MΩ` and the other ohm prefixes, with `Ω` as the symbol; `3 W s` as a product printed in the order typed; `5' 6"` as `5 feet 6 inches`; amounts too small for ten decimals in scientific form (`1e-12 kg`); degrees printed as `°` with no space; `mph` as the name of miles per hour, and `km/hour` for a speed made by division. Still open: ares, `ppi`/`dpi`/`em`/`rem` as units, molar, millibars, kilopound names, `W m`, `squared`/`cubed`, `khz`.
