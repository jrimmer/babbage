#!/usr/bin/env python3
"""Oracle for functions and word forms: statistics, rounding, gcd/lcm, permutations, operator words.

Columns: expression <TAB> exact n/d. Every form is written in its function spelling and its word spelling.
Rounding is half away from zero; the remainder keeps the sign of the dividend.
"""
import math
import random
import statistics
from fractions import Fraction

rng = random.Random(0xF00D)


def num(allow_neg=False, decimals=True):
    r = rng.random()
    if r < 0.6 or not decimals:
        t = str(rng.randint(1, 500))
    else:
        t = f"{rng.randint(0, 99)}.{rng.randint(1, 999)}"
    if allow_neg and rng.random() < 0.2:
        t = "-" + t
    return t, Fraction(t)


def trunc(q):
    return math.floor(q) if q >= 0 else math.ceil(q)


def round_half_away(q):
    f = math.floor(q)
    frac = q - f
    if frac > Fraction(1, 2) or (frac == Fraction(1, 2) and q >= 0):
        return Fraction(f + 1)
    return Fraction(f)


def to_multiple(q, step, mode):
    r = q / step
    if mode == "nearest":
        return round_half_away(r) * step
    if mode == "up":
        return Fraction(math.ceil(r)) * step
    return Fraction(math.floor(r)) * step


def emit(text, q):
    q = Fraction(q)
    print(f"{text}\t{q.numerator}/{q.denominator}")


def listing(items):
    return ", ".join(items[:-1]) + " and " + items[-1] if len(items) > 1 else items[0]


for _ in range(2500):
    k = rng.randint(2, 6)
    items = [num(True) for _ in range(k)]
    texts, vals = [t for t, _ in items], [v for _, v in items]
    csv = ", ".join(texts)
    emit(f"sum({csv})", sum(vals))
    emit(f"total of {listing(texts)}", sum(vals))
    emit(f"total of {csv}", sum(vals))
    emit(f"average({csv})", sum(vals) / len(vals))
    emit(f"average of {listing(texts)}", sum(vals) / len(vals))
    emit(f"avg({csv})", sum(vals) / len(vals))
    emit(f"median({csv})", statistics.median(vals))
    emit(f"median of {listing(texts)}", statistics.median(vals))
    emit(f"count of {csv}", len(vals))
    emit(f"max({csv})", max(vals))
    emit(f"min({csv})", min(vals))
    a, b = items[0], items[1]
    emit(f"larger of {a[0]} and {b[0]}", max(a[1], b[1]))
    emit(f"greater of {a[0]} and {b[0]}", max(a[1], b[1]))
    emit(f"smaller of {a[0]} and {b[0]}", min(a[1], b[1]))
    emit(f"lesser of {a[0]} and {b[0]}", min(a[1], b[1]))
    emit(f"midpoint between {a[0]} and {b[0]}", (a[1] + b[1]) / 2)
    emit(f"half of {a[0]}", a[1] / 2)
    emit(f"abs({a[0]})", abs(a[1]))
    # integers
    x, y = rng.randint(1, 5000), rng.randint(1, 5000)
    emit(f"gcd({x}, {y})", math.gcd(x, y))
    emit(f"gcd of {x} and {y}", math.gcd(x, y))
    emit(f"lcm({x}, {y})", math.lcm(x, y))
    emit(f"lcm of {x} and {y}", math.lcm(x, y))
    n = rng.randint(0, 25)
    emit(f"fact({n})", math.factorial(n))
    n2, k2 = rng.randint(1, 30), 0
    k2 = rng.randint(0, n2)
    emit(f"{n2} permutation {k2}", math.perm(n2, k2))
    emit(f"{k2} permutations of {n2}", math.perm(n2, k2))
    emit(f"{n2} combination {k2}", math.comb(n2, k2))
    emit(f"{k2} combinations of {n2}", math.comb(n2, k2))
    # clamp
    lo, hi = sorted([num(True)[1] for _ in range(2)])
    xt, xv = num(True)
    lo_t, hi_t = str(lo.numerator) if lo.denominator == 1 else None, str(hi.numerator) if hi.denominator == 1 else None
    if lo_t and hi_t and xv.denominator != 0:
        res = min(max(xv, lo), hi)
        emit(f"clamp {xt} between {lo_t} and {hi_t}", res)
        emit(f"clamp {xt} from {lo_t} to {hi_t}", res)
        emit(f"clamp({xt}, {lo_t}, {hi_t})", res)
    # rounding
    vt, v = num(True)
    emit(f"round({vt})", round_half_away(v))
    emit(f"{vt} rounded", round_half_away(v))
    emit(f"floor({vt})", math.floor(v))
    emit(f"{vt} rounded down", math.floor(v))
    emit(f"ceil({vt})", math.ceil(v))
    emit(f"ceiling({vt})", math.ceil(v))
    emit(f"{vt} rounded up", math.ceil(v))
    places = rng.randint(0, 4)
    step = Fraction(1, 10**places)
    emit(f"{vt} to {places} dp", to_multiple(v, step, "nearest"))
    emit(f"{vt} to {places} decimal places", to_multiple(v, step, "nearest"))
    emit(f"{vt} to {places} digits", to_multiple(v, step, "nearest"))
    s = rng.choice([5, 10, 25, 100, 3, 7])
    emit(f"{vt} to nearest {s}", to_multiple(v, Fraction(s), "nearest"))
    emit(f"{vt} rounded to nearest {s}", to_multiple(v, Fraction(s), "nearest"))
    emit(f"{vt} rounded up to nearest {s}", to_multiple(v, Fraction(s), "up"))
    emit(f"{vt} rounded down to nearest {s}", to_multiple(v, Fraction(s), "down"))
    word, wv = rng.choice([("ten", 10), ("hundred", 100), ("thousand", 1000)])
    emit(f"{vt} to nearest {word}", to_multiple(v, Fraction(wv), "nearest"))
    d = rng.choice([2, 4, 8, 16, 32])
    emit(f"{vt} to nearest {d}th", to_multiple(v, Fraction(1, d), "nearest"))
    # operators
    pt, p = num(True)
    qt, q = num(True)
    emit(f"{pt} plus {qt}", p + q)
    emit(f"{pt} minus {qt}", p - q)
    emit(f"{pt} times {qt}", p * q)
    emit(f"{pt} multiplied by {qt}", p * q)
    if q != 0:
        emit(f"{pt} divided by {qt}", p / q)
        emit(f"{pt} mod {qt}", p - q * trunc(p / q))
        emit(f"{pt} % {qt}", p - q * trunc(p / q))
        emit(f"remainder of {pt} divided by {qt}", p - q * trunc(p / q))
    e = rng.randint(0, 5)
    emit(f"({pt}) to the power of {e}", p**e)
    # proportions
    a_t, A = num(False, False)
    b_t, B = num(False, False)
    c_t, C = num(False, False)
    emit(f"{a_t} is to {b_t} as {c_t} is to what", B * C / A)
    emit(f"{a_t} is to {b_t} as what is to {c_t}", A * C / B)
