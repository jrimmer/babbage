# 0083 What Sheet D left open

Status: open, low priority (the owner said compatibility with the reference app is not absolute; these stay as documented guesses unless the owner chooses to answer Sheet E)
Milestone: v1
Area: engine

Sheet D matched all but one line of the engine (59 of 60 single lines and both Totals); what is left, and what each guess rests on:

1. **`hourly interest on $1k for 3 years at 4% compounding quarterly` is `$0.01`** (line 53). The engine refuses `hourly`. The compound interest over the three years is $126.83, so `$0.01` is not the total. It is not the average per hour either (126.83 over about 26,300 hours is $0.0048, which rounds to $0.00). It fits the interest of the last hour on the final balance ($1,126.83 at 4% a year is $0.0051 an hour). One answer cannot say which. Ask for `daily interest on $1k for 3 years at 4% compounding quarterly`, the same with `weekly`, and `hourly interest on $100k for 3 years at 4% compounding quarterly`; three numbers are enough to find the formula. In `tests/owner_reference.rs` this line is on the KNOWN list.
2. **Where plain decimals turn scientific.** `0.000000001` is `1e-9` and `0.00000000004` is `4e-11`. The engine switches below 1e-8; if the reference app switches earlier, small numbers between the two boundaries print wrong. Ask for `0.00001`, `0.000001`, `0.0000001` and `0.00000001`.
3. **Which products of units are units.** `2 N m` is `2 N⋅m`, `3 W s` is `3 W s`, `3 kg m` is `3 kg`. The engine keeps a product only when some unit of its table measures the same thing. Ask for `3 kg m/s`, `2 kg m²`, `4 m s` and `5 km h` to check the guess.
4. **Pounds.** `13.5 lb` is `13 lb 8 oz`. Not known: whether `1 kg in lb` prints `2 lb 3.27 oz` or `2.2046226218 lb`; whether stone and pounds, or yards, feet and inches, behave the same way; whether `5.5 ft` alone prints `5 feet 6 inches`. The engine does it for pounds only, and for feet and inches only when written together.
5. **The Total of clashing kinds** (ticket 0050): the same three lines gave `$10.00`, `3 gal` and `$10.00`. If the Total depends on the order the lines were typed, ask for the same sheet typed top to bottom, then built by typing the middle line last.
6. **Quarts.** The imperial gallon is exact (4.54609 L) and the pint rounded (0.568261 L); the quart is still two pints. Ask for `1 imperial quart in L`.
