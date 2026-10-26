# 0014 Fraction display, multipliers and proportion forms

Status: built
Milestone: v1
Area: engine, percentages

From the percentage reference: `2/10 as fraction` (1/5), `5% is 1 in what` (1/20), multipliers (`20/5 as multiplier` 4x, `50 as x of 5`, `1 as x off 2`), proportions (`20% is 500, what is 750` 30%, `if 20 is 30%, what is 60%` 40). Needs a Fraction and a Multiplier kind. Oracle: Python Fraction.

Built: `X as fraction` (exact, reduced, grouped), `N% is B in what` (`1/20`, `1/2.5`), and the comma proportion `20% is 500, what is 750` / `if 20 is 30%, what is 60%`, each checked against 15k Python Fraction lines in `tests/forms.rs`. Multipliers (`4x`, `as multiplier`) were built earlier.
