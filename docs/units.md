# Units: where the engine follows the reference and where it does not

Every conversion factor is an exact rational. The unit table is `crates/babbage-core/src/units.rs`.
It is checked against `pint` in exact-fraction mode (`tools/gen_unit_corpus.py`), except the constants below that
the reference-app reference defines itself.

## Constants taken from the reference even though a standard differs

| Unit | Engine | Standard (pint) | Why |
|---|---|---|---|
| month | 30.436875 days | 30.4375 | the reference states 30.436875 days |
| year | 365.2425 days | 365.25 (Julian) | the reference states 365.2425 days (Gregorian) |
| `KV` | 1000 V | not defined | the reference spells kilovolt with a capital K |

## Where the reference gives a rounded value and the engine uses the exact definition

| Unit | Reference text | Engine |
|---|---|---|
| troy ounce | "31.1035 g" | 31.1034768 g |
| horsepower | "746 W" | 550 ft·lbf/s = 745.69987158227022 W exactly (mechanical) | the reference's figure is a rounding of a unit with an exact definition; ticket 0033 covers metric horsepower |
| calorie | "about 4.184 J" | 4.184 J (thermochemical, exact by definition) |
| psi | "about 6,894.76 Pa" | lbf per square inch, exactly (about 6894.757293168 Pa) |
| pound-force | "about 4.44822 N" | 0.45359237 x 9.80665 N exactly |
| light year | "about 9.46 trillion km" | 9,460,730,472,580,800 m exactly |
| gallon, fluid ounce, tablespoon, teaspoon, bushel | US customary | NIST exact values |

## The reference contradicts itself

- `rps` is listed as "60 rpm". One revolution per second is 60 rpm, and the engine says so.

## Spellings the engine refuses on purpose

- `in`, `a`, `as`, `to`, `is`, `of`, `on`, `off`, `what`, `percent`, `into` belong to the phrase layer. Inches are `inch` and `inches`. Ares are `are` and `ares`.
- `"` and `'` are not feet or inches: they are reserved for quoted comments.
- The small-capital molar symbol is not supported.

## Rules the reference states

- A plain number takes the unit of the quantity it meets: `300 + 20 km` is `320 km`.
- Quantities of one kind in a sum meet in the larger unit: `1km + 1,000m` is `2 km`.
- Quantities of one kind in a quotient give a plain number.
- Plain numbers added to a temperature shift it. Everything else with a temperature is refused (see ticket 0021).

## Rules the reference does not state, chosen by the engine

- Two quantities of one kind in different units multiply in the left unit, squared: `10 m x 50 cm` is `5 m²`. The value is exact either way, only the displayed unit is a choice.
- `1 h 30 min` (adjacent quantities) is one quantity: it binds tighter than `x`.
- Results are shown with the unit symbol, not the spelling the user typed (ticket 0018).
