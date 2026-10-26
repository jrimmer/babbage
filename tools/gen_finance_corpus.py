#!.oracle-venv/bin/python
"""Oracle for compound interest, present value, loans, returns and growth.

Columns: expression <TAB> E <TAB> sha256 of "n/d"  (exact: the engine must say so)  or  A <TAB> decimal (irrational, mpmath).
The amount is in dollars; the exact value is the dollar amount before any rounding for display.
"""
import hashlib
import random
import sys
from fractions import Fraction

sys.set_int_max_str_digits(0)

from mpmath import ln, mp, mpf, nstr

mp.dps = 120
rng = random.Random(0xF1A4CE)
FREQ = [("", 1), (" compounding annually", 1), (" compounding quarterly", 4), (" compounding monthly", 12), (" compounding weekly", 52), (" compounding daily", 365)]


def dec(x):
    return nstr(x, 100, strip_zeros=False, min_fixed=-300, max_fixed=300)


def exact(expr, q):
    q = Fraction(q)
    digest = hashlib.sha256(f"{q.numerator}/{q.denominator}".encode()).hexdigest()
    print(f"{expr}\tE\t{digest}")


def approx(expr, v):
    print(f"{expr}\tA\t{dec(v)}")


def iroot_frac(q, k):
    """The k-th root of a positive Fraction when it is rational."""
    def iroot(x):
        lo, hi = 0, 1 << (x.bit_length() // k + 2)
        while lo < hi:
            mid = (lo + hi + 1) // 2
            if mid**k <= x:
                lo = mid
            else:
                hi = mid - 1
        return lo if lo**k == x else None

    rn, rd = iroot(q.numerator), iroot(q.denominator)
    return None if rn is None or rd is None else Fraction(rn, rd)


def amount():
    return rng.choice([str(rng.randint(100, 90000)), f"{rng.randint(1, 900)}.{rng.randint(1, 99):02d}"])


def rate():
    t = rng.choice([str(rng.randint(1, 15)), f"{rng.randint(0, 14)}.{rng.randint(1, 99)}"])
    return t, Fraction(t) / 100


for _ in range(450):
    pt = amount()
    P = Fraction(pt)
    years = rng.randint(1, 30)
    rt, r = rate()
    suffix, m = rng.choice(FREQ)
    years = max(1, min(years, 1200 // m))
    grown = P * (1 + r / m) ** (m * years)
    exact(f"${pt} after {years} years at {rt}%{suffix}", grown)
    exact(f"interest on ${pt} after {years} years at {rt}%{suffix}", grown - P)
    exact(f"interest on ${pt} after {years} years @ {rt}%{suffix}", grown - P)
    exact(f"present value of ${pt} after {years} years at {rt}%{suffix}", P / (1 + r / m) ** (m * years))
    # fractional years: irrational unless a perfect power
    months = rng.choice([1, 2, 3, 5, 6, 7, 9, 11])
    t = Fraction(months, 12)
    expo = m * t
    base = 1 + r / m
    value = P * mpf(base.numerator) / base.denominator ** 1
    approx_value = P.numerator / mpf(P.denominator) * (mpf(base.numerator) / base.denominator) ** (mpf(expo.numerator) / expo.denominator)
    if expo.denominator == 1:
        exact(f"${pt} after {months} months at {rt}%{suffix}", P * base ** int(expo))
    else:
        approx(f"${pt} after {months} months at {rt}%{suffix}", approx_value)

    # loans
    years_l = rng.randint(1, 30)
    n = 12 * years_l
    rl = r
    if rl == 0:
        pmt = P / n
    else:
        rm = rl / 12
        pmt = P * rm / (1 - (1 + rm) ** (-n))
    total = pmt * n
    for word, val in (("daily", total / (365 * years_l)), ("monthly", pmt), ("annual", total / years_l), ("total", total)):
        exact(f"{word} repayment on ${pt} over {years_l} years at {rt}%", val)
    interest = total - P
    for word, val in (("daily", interest / (365 * years_l)), ("monthly", interest / n), ("annual", interest / years_l), ("total", interest)):
        exact(f"{word} interest on ${pt} over {years_l} years at {rt}%", val)

    # growth per period, integer periods
    k = rng.randint(1, 24)
    exact(f"{pt} after {k} months at {rt}% per month", P * (1 + r) ** k)
    exact(f"${pt} after {k} weeks at {rt}% per week", P * (1 + r) ** k)

    # multiples and returns
    a, b = Fraction(rng.randint(100, 5000)), Fraction(rng.randint(100, 20000))
    exact(f"${a} invested ${b} returned", b / a)
    y = rng.randint(1, 12)
    root = iroot_frac(b / a, y)
    if root is not None:
        exact(f"annual return on ${a} invested ${b} returned after {y} years", root - 1)
    else:
        approx(f"annual return on ${a} invested ${b} returned after {y} years", (mpf(b.numerator) / a.numerator) ** (mpf(1) / y) - 1)

    # time to reach, steps (exact) and growth (irrational)
    start, step = rng.randint(100, 5000), rng.randint(10, 500)
    target = start + step * rng.randint(1, 60)
    exact(f"time from ${start} to ${target} at ${step} every month in months", Fraction(target - start, step))
    hi = start * rng.randint(2, 30)
    gt, g = rate()
    if g > 0:
        approx(f"time from {start} to {hi} at {gt}% growth per month in months", ln(mpf(hi) / start) / ln(1 + mpf(g.numerator) / g.denominator))
    approx(f"growth per day from {start} to {hi} over {rng.randint(2, 20)} days", None) if False else None
