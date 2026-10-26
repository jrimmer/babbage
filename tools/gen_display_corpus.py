#!/usr/bin/env python3
"""Oracle for the app's default number display: grouping, 10 places, trimmed, k/M/G/T abbreviation from 100,000.

Columns: expression <TAB> exact n/d <TAB> display. Display is computed from Decimal, never from the engine.
"""
import random
from decimal import Decimal, ROUND_HALF_UP, getcontext
from fractions import Fraction

import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from unitwords import tiny

getcontext().prec = 4000
rng = random.Random(0xD15F1A)
STEPS = [("k", 3, Fraction(10**5)), ("M", 6, Fraction(10**6)), ("G", 9, Fraction(10**9)), ("T", 12, Fraction(10**12))]


def plain(q):
    t = tiny(q)
    if t:
        return t
    d = Decimal(q.numerator) / Decimal(q.denominator)
    out = format(d.quantize(Decimal("1e-10"), rounding=ROUND_HALF_UP), "f")
    neg = out.startswith("-")
    out = out.lstrip("-")
    whole, _, frac = out.partition(".")
    frac = frac.rstrip("0")
    whole = f"{int(whole):,}"
    text = whole + ("." + frac if frac else "")
    return ("-" if neg and text != "0" else "") + text


def shown(q):
    a = abs(q)
    idx = None
    for i, (_, _, floor) in enumerate(STEPS):
        if a >= floor:
            idx = i
    if idx is None:
        return plain(q)
    while True:
        sym, shift, _ = STEPS[idx]
        text = plain(q / 10**shift)
        digits = len(text.lstrip("-").split(".")[0].replace(",", ""))
        if digits > 3 and idx + 1 < len(STEPS):
            idx += 1
            continue
        return text + sym


def lit():
    kind = rng.randrange(6)
    if kind == 0:
        return str(rng.randint(0, 99999))
    if kind == 1:
        return str(rng.randint(100000, 999999))
    if kind == 2:
        return str(rng.randint(10**6, 10**9))
    if kind == 3:
        return str(rng.randint(10**9, 10**13))
    if kind == 4:
        return f"{rng.randint(0, 10**7)}.{rng.randint(0, 99999):05d}"
    return rng.choice(["99999.99999999999", "999999.99999999999", "999999999.9999999999", "999999999999.99999999999", "99999.99999", "100000", "1000000", "999999", "1000", "0.00000000004", "0.00000000005"])


for _ in range(4000):
    t = lit()
    q = Fraction(t)
    if rng.random() < 0.3:
        t, q = "-" + t, -q
    print(f"{t}\t{q.numerator}/{q.denominator}\t{shown(q)}")
# input spellings of large numbers
for t, q in [("2.5k", 2500), ("5M", 5 * 10**6), ("10G", 10**10), ("20T", 2 * 10**13), ("3 million", 3 * 10**6), ("3 million + 10%", Fraction(33, 10) * 10**6),
             ("1.5 billion", 15 * 10**8), ("2 thousand", 2000), ("0.5 trillion", 5 * 10**11), ("1e3k", 10**6), ("7 Million", 7 * 10**6)]:
    q = Fraction(q)
    print(f"{t}\t{q.numerator}/{q.denominator}\t{shown(q)}")
