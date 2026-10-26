#!/usr/bin/env .oracle-venv/bin/python
"""Exact unit factors from pint, for every unit symbol the engine defines.

Run: .oracle-venv/bin/python tools/gen_unit_corpus.py > crates/babbage-core/tests/corpus/units.tsv
Columns: expression <TAB> exact n/d (value in the base unit) <TAB> 10dp display with the base unit.
The reference app-specific constants the reference documents (month, year, the 1 kilovolt `KV`) are
hand formulas from the reference, not pint, because pint disagrees with them on purpose.
"""
import sys
from decimal import Decimal, ROUND_HALF_UP, getcontext
from fractions import Fraction

import pint
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from unitwords import suffix

getcontext().prec = 4000
ureg = pint.UnitRegistry(non_int_type=Fraction)
Q = ureg.Quantity

PREFIX = {"y": "yocto", "z": "zepto", "a": "atto", "f": "femto", "p": "pico", "n": "nano", "µ": "micro", "m": "milli",
          "c": "centi", "d": "deci", "h": "hecto", "k": "kilo", "M": "mega", "G": "giga", "T": "tera", "P": "peta",
          "E": "exa", "Z": "zetta", "Y": "yotta"}


def pre(prefixes, ours, theirs):
    """(our symbol, pint name) pairs for each prefix on one base unit."""
    return [(p + ours, PREFIX[p] + theirs) for p in prefixes]


# (our symbol, pint unit) grouped by the base unit we convert into (ours, pint)
GROUPS = {
    ("s", "second"): [("s", "second"), ("min", "minute"), ("h", "hour"), ("day", "day"), ("wk", "week")]
    + pre("nµm", "s", "second"),
    ("m", "meter"): pre("nµmcdk", "m", "meter")
    + [("inch", "inch"), ("ft", "foot"), ("yd", "yard"), ("mi", "mile"), ("fur", "furlong"), ("ftm", "fathom"),
       ("NM", "nautical_mile"), ("ly", "light_year"), ("pt", "point")],
    ("m²", "meter**2"): [("ac", "acre"), ("ha", "hectare"), ("are", "are")],
    ("m³", "meter**3"): [("L", "liter"), ("gal", "gallon"), ("qt", "quart"), ("pint", "pint"), ("floz", "fluid_ounce"),
                         ("tbsp", "tablespoon"), ("tsp", "teaspoon"), ("bsh", "bushel"), ("af", "acre_foot")]
    + pre("mcdkM", "L", "liter"),
    ("kg", "kilogram"): pre("nµmk", "g", "gram")
    + [("lb", "pound"), ("oz", "ounce"), ("st", "stone"), ("ton", "ton"), ("t", "metric_ton"), ("ozt", "troy_ounce"),
       ("ct", "carat")],
    ("deg", "degree"): [("arcmin", "arcminute"), ("arcsec", "arcsecond"), ("rev", "turn")],
    ("Hz", "hertz"): pre("nµmkMGT", "Hz", "hertz"),
    ("J", "joule"): pre("kMG", "J", "joule") + [("Wh", "watt_hour"), ("cal", "calorie"), ("kcal", "kilocalorie")]
    + pre("kMGT", "Wh", "watt_hour"),
    ("W", "watt"): pre("mkMGT", "W", "watt") + [("hp", "horsepower")],
    ("N", "newton"): pre("nµmkMG", "N", "newton") + [("lbf", "pound_force"), ("pdl", "poundal")],
    ("Pa", "pascal"): pre("hkMG", "Pa", "pascal")
    + [("bar", "bar"), ("mbar", "millibar"), ("atm", "atmosphere"), ("psi", "psi")],
    ("mol", "mole"): pre("mµn", "mol", "mole"),
    ("V", "volt"): pre("mµkM", "V", "volt"),
    ("A", "ampere"): pre("mµkM", "A", "ampere"),
    ("ohm", "ohm"): pre("mµkM", "ohm", "ohm"),
    ("coulomb", "coulomb"): [("Ah", "ampere_hour")] + pre("mµkM", "Ah", "ampere_hour"),
    ("farad", "farad"): pre("mµkM", "farad", "farad"),
    ("henry", "henry"): pre("mµkM", "henry", "henry"),
    ("siemens", "siemens"): pre("mµkM", "siemens", "siemens"),
    ("Wb", "weber"): pre("mµkM", "Wb", "weber"),
    ("tesla", "tesla"): pre("mµkM", "tesla", "tesla"),
    ("bit", "bit"): [("B", "byte"), ("kb", "kilobit"), ("Mb", "megabit"), ("Gb", "gigabit"), ("Tb", "terabit"),
                     ("Pb", "petabit"), ("Eb", "exabit"), ("Zb", "zettabit"), ("Yb", "yottabit"),
                     ("KiB", "kibibyte"), ("MiB", "mebibyte"), ("GiB", "gibibyte"), ("TiB", "tebibyte"),
                     ("PiB", "pebibyte"), ("EiB", "exbibyte"), ("ZiB", "zebibyte"), ("YiB", "yobibyte"),
                     ("Kib", "kibibit"), ("Mib", "mebibit"), ("Gib", "gibibit"), ("Tib", "tebibit"),
                     ("Pib", "pebibit"), ("Eib", "exbibit"), ("Zib", "zebibit"), ("Yib", "yobibit")]
    + pre("kMGTPEZY", "B", "byte"),
}

# Hand formulas from the reference, in the group's base unit.
HAND = {
    ("s", "s"): [("mo", Fraction("30.436875") * 86400), ("yr", Fraction("365.2425") * 86400)],
    ("m", "m"): [("px", Fraction("0.0254") / 96)],
    ("m³", "m^3"): [("mcup", Fraction(1, 4000))],
    ("kg", "kg"): [("mcg", Fraction(1, 10**9))],
    ("Pa", "Pa"): [("ksi", Fraction(1000) * Q(1, "psi").to("pascal").magnitude),
                   ("Mpsi", Fraction(10**6) * Q(1, "psi").to("pascal").magnitude)],
    ("N", "N"): [("ozf", Q(1, "pound_force").to("newton").magnitude / 16)],
    ("V", "V"): [("KV", Fraction(1000))],
}


def fixed10(q):
    d = Decimal(q.numerator) / Decimal(q.denominator)
    out = format(d.quantize(Decimal("1e-10"), rounding=ROUND_HALF_UP), "f")
    return out[1:] if out.startswith("-") and set(out[1:]) <= {"0", "."} else out


DISPLAY = {"deg": "°", "deg/s": "°/s", "ohm": "Ω"}


def scientific(q):
    """An amount too small for ten decimal places is written 1e-12, ten decimals in the mantissa at most."""
    d = Decimal(q.numerator) / Decimal(q.denominator)
    exponent = d.adjusted()
    mantissa = (d / (Decimal(10) ** exponent)).quantize(Decimal("1e-10"), rounding=ROUND_HALF_UP)
    if abs(mantissa) >= 10:
        mantissa /= 10
        exponent += 1
    text = format(mantissa.normalize(), "f")
    return f"{text}e{exponent}"


def shown_amount(q, unit):
    """The amount and unit as the engine writes them: degrees as `°` with no space, ohms as `Ω`."""
    q = Fraction(q)
    text = scientific(q) if q != 0 and abs(q) * 10**10 < Fraction(1, 2) else fixed10(q)
    shown = DISPLAY.get(unit, unit)
    return f"{text}{'' if shown == '°' else ' '}{shown}"


def row(sym, base_sym, q):
    q = Fraction(q)
    print(f"1 {sym} in {base_sym}\t{q.numerator}/{q.denominator}\t{shown_amount(q, base_sym)}")


COMPOUND = [
    ("mph", "m/s", "mile/hour"), ("kph", "m/s", "kilometer/hour"), ("kmh", "m/s", "kilometer/hour"),
    ("kn", "m/s", "knot"), ("knots", "m/s", "knot"), ("Bps", "bit/s", "byte/second"), ("bps", "bit/s", "bit/second"),
    ("rpm", "deg/s", "turn/minute"), ("rps", "deg/s", "turn/second"), ("mpg", "km/L", "mile/gallon"),
    ("MB/s", "bit/s", "megabyte/second"), ("GiB/s", "bit/s", "gibibyte/second"),
]


TEMPS = [("°C", "degC"), ("°F", "degF"), ("K", "kelvin")]
TEMP_VALUES = ["0", "100", "-40", "36.6", "98.6", "-273.15", "0.1", "212", "451", "-459.67", "1e3", "37.5", "300", "20"]


def temperatures():
    for ours_a, pint_a in TEMPS:
        for ours_b, pint_b in TEMPS:
            if ours_a == ours_b:
                continue
            for v in TEMP_VALUES:
                q = Q(Fraction(v), pint_a).to(pint_b).magnitude
                q = Fraction(q)
                print(f"{v} {ours_a} in {ours_b}\t{q.numerator}/{q.denominator}\t{fixed10(q)} {ours_b}")


def main():
    temperatures()
    for (base_sym, base_pint), units in GROUPS.items():
        for sym, name in units:
            qty = Q(Fraction(1), name).to(base_pint).magnitude
            row(sym, base_sym.replace("²", "^2").replace("³", "^3") if False else base_sym, qty)
    for sym, base, theirs in COMPOUND:
        target = base.replace("km/L", "kilometer/liter").replace("m/s", "meter/second").replace("deg/s", "degree/second").replace("bit/s", "bit/second")
        q = Fraction(Q(Fraction(1), theirs).to(target).magnitude)
        print(f"1 {sym} in {base}\t{q.numerator}/{q.denominator}\t{shown_amount(q, base)}")
    for (base_sym, base_ascii), items in HAND.items():
        for sym, value in items:
            print(f"1 {sym} in {base_ascii}\t{Fraction(value).numerator}/{Fraction(value).denominator}\t{shown_amount(Fraction(value), base_sym)}")


main()
