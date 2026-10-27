#!/usr/bin/env .oracle-venv/bin/python
"""Semantic oracle for quantity arithmetic, from the documented unit rules, factors from pint.

Run: .oracle-venv/bin/python tools/gen_quantity_corpus.py > crates/babbage-core/tests/corpus/quantity.tsv
Columns: expression <TAB> kind (plain|percent|ERR:incompatible) <TAB> exact n/d <TAB> 10dp display with unit.
Rules: a plain number takes the other side's unit; in a sum of two units of one kind the larger unit wins;
a quotient of one kind is a plain number; multiplying two units of one kind keeps the left unit (squared).
"""
import random
from decimal import Decimal, ROUND_HALF_UP, getcontext
from fractions import Fraction

import pint
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from unitwords import suffix, tiny

getcontext().prec = 4000
rng = random.Random(0x51A7E)
ureg = pint.UnitRegistry(non_int_type=Fraction)
Q = ureg.Quantity

# our spelling -> (display symbol, pint name, family)
UNITS = {
    "mm": ("mm", "millimeter", "len"), "cm": ("cm", "centimeter", "len"), "m": ("m", "meter", "len"),
    "km": ("km", "kilometer", "len"), "inch": ("in", "inch", "len"), "ft": ("ft", "foot", "len"),
    "yd": ("yd", "yard", "len"), "mi": ("mi", "mile", "len"),
    "mg": ("mg", "milligram", "mass"), "g": ("g", "gram", "mass"), "kg": ("kg", "kilogram", "mass"),
    "lb": ("lb", "pound", "mass"), "oz": ("oz", "ounce", "mass"), "st": ("st", "stone", "mass"),
    "ms": ("ms", "millisecond", "time"), "s": ("s", "second", "time"), "min": ("min", "minute", "time"),
    "h": ("h", "hour", "time"), "day": ("day", "day", "time"), "wk": ("wk", "week", "time"),
    "mL": ("mL", "milliliter", "vol"), "L": ("L", "liter", "vol"), "gal": ("gal", "gallon", "vol"),
    "qt": ("qt", "quart", "vol"), "cup": ("cup", "cup", "vol"), "tsp": ("tsp", "teaspoon", "vol"),
    "B": ("B", "byte", "data"), "kB": ("kB", "kilobyte", "data"), "MB": ("MB", "megabyte", "data"),
    "GB": ("GB", "gigabyte", "data"), "MiB": ("MiB", "mebibyte", "data"), "GiB": ("GiB", "gibibyte", "data"),
    "bit": ("bit", "bit", "data"), "J": ("J", "joule", "energy"), "kJ": ("kJ", "kilojoule", "energy"),
    "kWh": ("kWh", "kilowatt_hour", "energy"), "W": ("W", "watt", "power"), "kW": ("kW", "kilowatt", "power"),
    "Pa": ("Pa", "pascal", "press"), "kPa": ("kPa", "kilopascal", "press"), "bar": ("bar", "bar", "press"),
    "psi": ("psi", "psi", "press"), "N": ("N", "newton", "force"), "kN": ("kN", "kilonewton", "force"),
}
FAMILIES = {}
for k, (sym, p, fam) in UNITS.items():
    FAMILIES.setdefault(fam, []).append(k)


def factor(spelling):
    return Fraction(Q(Fraction(1), UNITS[spelling][1]).to_base_units().magnitude)


def num():
    r = rng.random()
    if r < 0.5:
        t = str(rng.randint(1, 5000))
    elif r < 0.9:
        t = f"{rng.randint(0, 999)}.{rng.randint(0, 9999):04d}".rstrip("0").rstrip(".") or "0"
    else:
        t = f"{rng.randint(1, 99)}e{rng.randint(0, 3)}"
    return t, Fraction(t)


def fixed10(q, sci=True):
    t = tiny(q) if sci else None
    if t:
        return t
    d = Decimal(q.numerator) / Decimal(q.denominator)
    out = format(d.quantize(Decimal("1e-10"), rounding=ROUND_HALF_UP), "f")
    return out[1:] if out.startswith("-") and set(out[1:]) <= {"0", "."} else out


def emit(text, kind, q=None, unit=""):
    if kind.startswith("ERR"):
        print(f"{text}\t{kind}\t-\t-")
        return
    shown = fixed10(q * (100 if kind == "percent" else 1), sci=kind != "percent")
    shown += "%" if kind == "percent" else suffix(unit, q)
    print(f"{text}\t{kind}\t{q.numerator}/{q.denominator}\t{shown}")


def pair():
    fam = rng.choice(list(FAMILIES))
    a, b = rng.choice(FAMILIES[fam]), rng.choice(FAMILIES[fam])
    return fam, a, b


def one():
    fam, a, b = pair()
    (ta, A), (tb, B), (tp, P) = num(), num(), num()
    P = P / 100
    fa, fb = factor(a), factor(b)
    sa, sb = UNITS[a][0], UNITS[b][0]
    kind = rng.randrange(14)
    if kind in (0, 1):
        op = "+" if kind == 0 else "-"
        bigger = b if fb > fa else a
        fbig = factor(bigger)
        x = (A * fa / fbig) + (B * fb / fbig) * (1 if op == "+" else -1)
        emit(f"{ta} {a} {op} {tb} {b}", "plain", x, UNITS[bigger][0])
    elif kind == 2:
        emit(f"{ta} {a} + {tb}", "plain", A + B, sa)
        emit(f"{ta} + {tb} {a}", "plain", A + B, sa)
    elif kind == 3:
        emit(f"{ta} {a} × {tb}", "plain", A * B, sa)
        emit(f"{tb} × {ta} {a}", "plain", A * B, sa)
    elif kind == 4:
        emit(f"{ta} {a} / {tb}", "plain", A / B, sa)
    elif kind == 5:
        emit(f"{ta} {a} / {tb} {b}", "plain", (A * fa) / (B * fb))
    elif kind == 6:
        emit(f"{ta} {a} in {b}", "plain", A * fa / fb, sb)
        emit(f"{ta} {a} to {b}", "plain", A * fa / fb, sb)
    elif kind == 7:
        x, y = rng.choice(list(UNITS)), rng.choice(list(UNITS))
        if UNITS[x][2] != UNITS[y][2]:
            emit(f"{ta} {x} + {tb} {y}", "ERR:incompatible")
            emit(f"{ta} {x} in {y}", "ERR:incompatible")
    elif kind == 8:
        emit(f"{ta} {a} + {tp}%", "plain", A * (1 + P), sa)
        emit(f"{ta} {a} - {tp}%", "plain", A * (1 - P), sa)
        emit(f"{tp}% of {ta} {a}", "plain", A * P, sa)
    elif kind == 9:
        emit(f"{ta} {a} is what % of {tb} {b}", "percent", (A * fa) / (B * fb))
        emit(f"{ta} {a} to {tb} {b} is what %", "percent", (B * fb - A * fa) / (A * fa))
    elif kind == 10:
        # same unit: the product keeps the unit, squared
        emit(f"{ta} {a} × {tb} {a}", "plain", A * B, sa + "²")
    elif kind == 11:
        # different units of one kind: left unit, squared
        emit(f"{ta} {a} × {tb} {b}", "plain", A * B * fb / fa, sa + "²")
    elif kind == 12:
        emit(f"{ta} {a} × {tb} {a} × {tb} {a}", "plain", A * B * B, sa + "³")
    else:
        emit(f"{ta} {a} / ({tb} {a} × {tb} {a})", "plain", A / (B * B), f"{sa}⁻¹")


for _ in range(9000):
    try:
        one()
    except ZeroDivisionError:
        pass
