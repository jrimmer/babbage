#!/usr/bin/env python3
"""Oracle for working time: a workday is the work-hours setting; against a workday a calendar day is 5/7 of one
workday of a week, so a week is exactly five workdays and an hour is 1/hours of a workday. Exact Fractions.

Columns: work hours per day <TAB> expression <TAB> expected text (10 places, trailing zeros trimmed).
"""
import random
from decimal import Decimal, ROUND_HALF_UP, getcontext
from fractions import Fraction
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from unitwords import suffix

getcontext().prec = 100
rng = random.Random(0x5EEC)
SECONDS = {"second": 1, "minute": 60, "hour": 3600, "day": 86400, "week": 604800,
           "month": Fraction(30436875, 1000000) * 86400, "year": Fraction(3652425, 10000) * 86400}
CALENDAR = {"day", "week", "month", "year"}
rows = []


def digits(q):
    d = Decimal(q.numerator) / Decimal(q.denominator)
    text = format(d.quantize(Decimal(1).scaleb(-10), rounding=ROUND_HALF_UP), "f")
    neg = text.startswith("-")
    whole, _, frac = text.lstrip("-").partition(".")
    frac = frac.rstrip("0")
    out = f"{int(whole):,}" + ("." + frac if frac else "")
    return ("-" if neg and out != "0" else "") + out


def money(q):
    d = Decimal(q.numerator) / Decimal(q.denominator)
    text = format(d.quantize(Decimal("0.01"), rounding=ROUND_HALF_UP), "f")
    neg = text.startswith("-")
    whole, _, frac = text.lstrip("-").partition(".")
    return ("-" if neg else "") + f"${int(whole):,}.{frac}"


def seconds_in(unit, hours, against_workday):
    if unit == "workday":
        return Fraction(hours) * 3600
    base = Fraction(SECONDS[unit])
    if against_workday and unit in CALENDAR:
        return base * Fraction(5 * hours) / (7 * 24)
    return base


def amount():
    whole = rng.randint(0, 400)
    places = rng.choice([0, 1, 2])
    return Fraction(whole * 10**places + rng.randint(0, 10**places - 1), 10**places) if places else Fraction(whole)


def text_of(q):
    return digits(q) if q.denominator == 1 else format(Decimal(q.numerator) / Decimal(q.denominator), "f")


def plural(unit, q):
    return unit if q == 1 else unit + "s"


for hours in (8, 8, 6, 7, 10, 12):
    for _ in range(500):
        unit = rng.choice(list(SECONDS))
        q = amount()
        word_in = f"{text_of(q)} {plural(unit, q)}"
        work = seconds_in("workday", hours, True)
        ans = q * seconds_in(unit, hours, True) / work
        rows.append((hours, f"{word_in} in workdays", digits(ans) + suffix("workday", ans)))
        w = amount()
        sym = {"second": "s", "minute": "min", "hour": "h", "day": "day", "week": "wk", "month": "mo", "year": "yr"}[unit]
        ans2 = w * work / seconds_in(unit, hours, True)
        rows.append((hours, f"{text_of(w)} {plural('workday', w)} in {unit}s", digits(ans2) + suffix(sym, ans2)))
        rate = rng.randint(1, 900)
        rows.append((hours, f"${rate}/workday * {word_in}", money(rate * q * seconds_in(unit, hours, True) / work)))

for r in rows:
    print("\t".join(map(str, r)))
