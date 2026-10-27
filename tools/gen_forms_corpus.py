#!/usr/bin/env python3
"""Oracle for the word forms: `as fraction`, `N% is B in what`, `a is b, what is c`, `unit in quantity` and
`X in px at N ppi`. Fractions and Decimals only; no floats.

Columns: expression <TAB> expected text (10 places, trailing zeros trimmed, grouped, nothing over 99,999).
"""
import random
from decimal import Decimal, ROUND_HALF_UP, getcontext
from fractions import Fraction
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from unitwords import suffix, tiny

getcontext().prec = 200
rng = random.Random(0xF04A)
rows = []


def dec(q, places=10):
    t = tiny(q) if places == 10 else None
    if t:
        return t
    d = Decimal(q.numerator) / Decimal(q.denominator)
    text = format(d.quantize(Decimal(1).scaleb(-places), rounding=ROUND_HALF_UP), "f")
    neg = text.startswith("-")
    whole, _, frac = text.lstrip("-").partition(".")
    frac = frac.rstrip("0")
    out = f"{int(whole):,}" + ("." + frac if frac else "")
    return ("-" if neg and set(out) - set("0.,") else "") + out


def terminates(q):
    d = q.denominator
    for p in (2, 5):
        while d % p == 0:
            d //= p
    return d == 1


def num(q):
    """A terminating Fraction as plain typed text."""
    return format(Decimal(q.numerator) / Decimal(q.denominator), "f") if q.denominator != 1 else str(q.numerator)


def frac_text(q):
    return f"{q.numerator:,}" if q.denominator == 1 else f"{q.numerator:,}/{q.denominator:,}"


for _ in range(1500):
    a, b = rng.randint(-300, 300), rng.randint(1, 300)
    q = Fraction(a, b)
    rows.append((f"{a}/{b} as fraction" if a >= 0 else f"({a})/{b} as fraction", frac_text(q)))
    pct = Fraction(rng.randint(1, 1000), rng.choice([1, 2, 4, 10]))
    rows.append((f"{num(pct)}% as fraction", frac_text(pct / 100)))
    dd = Fraction(rng.randint(0, 99999), 10 ** rng.randint(0, 5))
    rows.append((f"{num(dd)} as fraction", frac_text(dd)))

for _ in range(1500):
    pct = Fraction(rng.randint(1, 10000), rng.choice([1, 10, 100]))
    top = Fraction(rng.randint(1, 50), rng.choice([1, 2, 5]))
    below = top / (pct / 100)
    if not terminates(below) or below > 99999:
        continue
    rows.append((f"{num(pct)}% is {num(top)} in what", f"{dec(top)}/{dec(below)}"))

for _ in range(3000):
    a = Fraction(rng.randint(1, 300), rng.choice([1, 2, 4, 10]))
    b = Fraction(rng.randint(1, 300), rng.choice([1, 2, 4, 10]))
    c = Fraction(rng.randint(1, 300), rng.choice([1, 2, 4, 10]))
    x = a * c / b
    if x > 99999:
        continue
    lead = rng.choice(["", "if "])
    comma = rng.choice([",", ""])
    sep = f"{comma} " if comma else " "
    kind = rng.choice(["plain", "percent-first", "percent-all"])
    if kind == "plain":
        rows.append((f"{lead}{num(a)} is {num(b)}{sep}what is {num(c)}", dec(x)))
    elif kind == "percent-first":
        rows.append((f"{lead}{num(a)}% is {num(b)}{sep}what is {num(c)}", dec(x) + "%"))
    else:
        rows.append((f"{lead}{num(a)} is {num(b)}%{sep}what is {num(c)}%", dec(x * Fraction(1) )))

UNITS = {  # symbol, plural name, seconds or meters per unit
    "time": [("s", "seconds", Fraction(1)), ("min", "minutes", Fraction(60)), ("h", "hours", Fraction(3600)),
             ("day", "days", Fraction(86400)), ("wk", "weeks", Fraction(604800))],
    "length": [("m", "meters", Fraction(1)), ("km", "kilometers", Fraction(1000)), ("cm", "centimeters", Fraction(1, 100)),
               ("mm", "millimeters", Fraction(1, 1000)), ("ft", "feet", Fraction(3048, 10000)),
               ("yd", "yards", Fraction(9144, 10000)), ("mi", "miles", Fraction(1609344, 1000))],
}
for _ in range(4000):
    kind = rng.choice(list(UNITS))
    (sym_a, name_a, f_a), (sym_b, name_b, f_b) = rng.sample(UNITS[kind], 2)
    n = Fraction(rng.randint(1, 5000), rng.choice([1, 2, 10]))
    result = n * f_b / f_a
    if result >= 100000:
        continue
    rows.append((f"{name_a} in {num(n)} {name_b}", f"{dec(result)}{suffix(sym_a, result)}"))
    rows.append((f"{sym_a} in {num(n)} {sym_b}", f"{dec(result)}{suffix(sym_a, result)}"))
    rows.append((f"{name_a} in a {name_b[:-1]}" if name_b.endswith("s") and name_b not in ("feet",) else f"{name_a} in a {name_b}",
                 None))

rows = [r for r in rows if r[1] is not None]
SINGULAR = {"meters": "meter", "kilometers": "kilometer", "centimeters": "centimeter", "millimeters": "millimeter",
            "yards": "yard", "miles": "mile", "seconds": "second", "minutes": "minute", "hours": "hour",
            "days": "day", "weeks": "week"}
for kind, table in UNITS.items():
    for (sym_a, name_a, f_a) in table:
        for (sym_b, name_b, f_b) in table:
            if sym_a == sym_b or name_b not in SINGULAR:
                continue
            result = f_b / f_a
            if result < 100000:
                rows.append((f"{name_a} in a {SINGULAR[name_b]}", f"{dec(result)}{suffix(sym_a, result)}"))

INCH = Fraction(254, 10000)
for _ in range(1500):
    cm = Fraction(rng.randint(1, 5000), rng.choice([1, 10, 100]))
    ppi = rng.randint(50, 600)
    px = cm / 100 / INCH * ppi
    if px < 100000:
        rows.append((f"{num(cm)} cm in px at {ppi} ppi", f"{dec(px)}px"))  # The expected answer is pixels with no space
    count = rng.randint(1, 99999)
    cm_out = Fraction(count, ppi) * INCH * 100
    rows.append((f"{count} px in cm at {ppi} dpi", f"{dec(cm_out)} cm"))

for r in rows:
    print("\t".join(r))
