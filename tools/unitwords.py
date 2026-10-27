"""How the engine writes a unit after an amount, for the oracle generators.

Hours, days, weeks, months, years, workdays and frames are words (plural unless the amount is one); a unit
with an exponent keeps its word without a plural (`day²`); minutes, seconds and every other unit stay symbols.
A unit that is only a denominator reads as `/week`, with no space after the number.
"""
from decimal import Decimal, ROUND_HALF_UP, getcontext
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
        # An hour typed `h` stays `h` after a slash (`100 km/h`); only one typed `hours` reads `/hour`, and the
        # corpora never type it.
        return "/" + ("h" if base == "h" else word or base)
    if word is None:
        return " " + unit
    if exp:
        return " " + word + exp
    return " " + (word if amount == 1 or amount == -1 else word + "s")


def tiny(q):
    """An amount below a millionth is written in scientific form, `1e-9` or `-4e-11`; else None."""
    if q == 0 or abs(q) >= Fraction(1, 10**6):
        return None
    getcontext().prec = 4000
    d = Decimal(abs(q).numerator) / Decimal(abs(q).denominator)
    exp = d.adjusted()
    mantissa = (d / Decimal(10) ** exp).quantize(Decimal("1e-10"), rounding=ROUND_HALF_UP)
    if mantissa >= 10:
        mantissa /= 10
        exp += 1
    text = format(mantissa, "f").rstrip("0").rstrip(".")
    return ("-" if q < 0 else "") + f"{text}e{exp}"


def offset_label(seconds):
    """A zone with no name of its own: `UTC`, `UTC+8`, `UTC-5:30`."""
    if seconds == 0:
        return "UTC"
    sign, abs_s = ("-", -seconds) if seconds < 0 else ("+", seconds)
    h, m = abs_s // 3600, abs_s % 3600 // 60
    return f"UTC{sign}{h}" if m == 0 else f"UTC{sign}{h}:{m:02}"


def zone_label(aware):
    """The zone's short name at a moment, as a timestamp's `to date` ends with it (`PST`); an offset-only name
    such as `+03` becomes `UTC+3`."""
    name = aware.tzname() or ""
    if name and name[0] not in "+-":
        return name
    return offset_label(int(aware.utcoffset().total_seconds()))
