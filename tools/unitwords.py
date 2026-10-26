"""How the engine writes a unit after an amount, for the oracle generators.

Hours, days, weeks, months, years, workdays and frames are words (plural unless the amount is one); a unit
with an exponent keeps its word without a plural (`day²`); minutes, seconds and every other unit stay symbols.
A unit that is only a denominator reads as `/week`, with no space after the number.
"""
from fractions import Fraction

WORDS = {"h": "hour", "day": "day", "wk": "week", "mo": "month", "yr": "year", "workday": "workday", "frame": "frame"}
SUPERSCRIPTS = "⁰¹²³⁴⁵⁶⁷⁸⁹⁻"


def suffix(unit, amount):
    """The text that follows the number: ` h`-style with its leading space, or `/week`. `amount` is a Fraction."""
    if not unit:
        return ""
    base = unit.rstrip(SUPERSCRIPTS)
    exp = unit[len(base):]
    word = WORDS.get(base)
    if exp == "⁻¹":
        return "/" + (word or base)
    if word is None:
        return " " + unit
    if exp:
        return " " + word + exp
    return " " + (word if amount == 1 or amount == -1 else word + "s")
