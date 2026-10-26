#!.oracle-venv/bin/python
"""Independent oracle for roots, logarithms, trigonometry and constants, using mpmath at 120 digits.

Columns: expression <TAB> E <TAB> n/d        (the answer is exactly rational and the engine must say so)
         expression <TAB> A <TAB> decimal    (irrational: the engine must be within 1e-60 and flag it approximate)
         expression <TAB> D <TAB> n/d        (exact, in degrees)  /  expression <TAB> DA <TAB> decimal  (approximate, in degrees)
Exact cases are decided here, from first principles, not read back from the engine.
"""
import math
import random
from fractions import Fraction

from mpmath import acos, acosh, asin, asinh, atan, atanh, cbrt, cos, cosh, e, exp, ln, log, mp, mpf, nstr, phi, pi, sin, sinh, tan, tanh

mp.dps = 120
rng = random.Random(0x1AA7)


def dec(x):
    return nstr(x, 100, strip_zeros=False, min_fixed=-300, max_fixed=300)


def isqrt_frac(q):
    """sqrt of a non-negative Fraction if it is rational."""
    n, d = q.numerator, q.denominator
    rn, rd = math.isqrt(n), math.isqrt(d)
    return Fraction(rn, rd) if rn * rn == n and rd * rd == d else None


def iroot_frac(q, k):
    """k-th root of a Fraction if rational (odd k allows negatives)."""
    sign = -1 if q < 0 else 1
    if q < 0 and k % 2 == 0:
        return None
    n, d = abs(q.numerator), q.denominator

    def iroot(x):
        lo, hi = 0, 1 << (x.bit_length() // k + 2)
        while lo < hi:
            mid = (lo + hi + 1) // 2
            if mid**k <= x:
                lo = mid
            else:
                hi = mid - 1
        return lo if lo**k == x else None

    rn, rd = iroot(n), iroot(d)
    return None if rn is None or rd is None else sign * Fraction(rn, rd)


def emit_exact(expr, q):
    q = Fraction(q)
    print(f"{expr}\tE\t{q.numerator}/{q.denominator}")


def emit_approx(expr, v):
    print(f"{expr}\tA\t{dec(v)}")


def emit_deg_exact(expr, q):
    q = Fraction(q)
    print(f"{expr}\tD\t{q.numerator}/{q.denominator}")


def emit_deg_approx(expr, v):
    print(f"{expr}\tDA\t{dec(v)}")


def rational(positive=True, decimals=True):
    r = rng.random()
    if r < 0.5:
        t = str(rng.randint(1, 400))
    elif r < 0.8 and decimals:
        t = f"{rng.randint(0, 40)}.{rng.randint(1, 99)}"
    else:
        t = f"{rng.randint(1, 30)}/{rng.randint(1, 30)}"
    return t, (Fraction(t) if "/" not in t else Fraction(t))


SQUARES = [Fraction(k * k, d * d) for k in range(0, 40) for d in (1, 2, 3, 5)]


def sqrt_cases():
    for _ in range(300):
        if rng.random() < 0.4:
            q = rng.choice(SQUARES)
            t = f"{q.numerator}" if q.denominator == 1 else f"({q.numerator}/{q.denominator})"
        else:
            t, q = rational()
            t = f"({t})" if "/" in t else t
        root = isqrt_frac(q)
        for spelling in (f"sqrt({t})", f"sqrt {t}", f"square root of {t}", f"√{t}"):
            if root is not None:
                emit_exact(spelling, root)
            else:
                emit_approx(spelling, mp.sqrt(mpf(q.numerator) / q.denominator))


def root_cases():
    for _ in range(300):
        k = rng.choice([2, 3, 4, 5, 7])
        if rng.random() < 0.4:
            base = Fraction(rng.randint(-6, 12), rng.choice([1, 1, 2, 3]))
            q = base**k
            t = str(q.numerator) if q.denominator == 1 else f"({q.numerator}/{q.denominator})"
        else:
            tq, q = rational()
            t = f"({tq})" if "/" in tq else tq
        r = iroot_frac(q, k)
        spellings = [f"root {k} of {t}"] + ([f"cbrt({t})", f"cube root of {t}"] if k == 3 else [])
        for s in spellings:
            if r is not None:
                emit_exact(s, r)
            elif q > 0:
                emit_approx(s, mp.root(mpf(q.numerator) / q.denominator, k))
            elif k % 2 == 1:
                emit_approx(s, -mp.root(-mpf(q.numerator) / q.denominator, k))


def log_cases():
    for _ in range(300):
        b = rng.choice([2, 3, 5, 10, Fraction(1, 2)])
        k = rng.randint(-6, 12)
        if rng.random() < 0.5:
            q = Fraction(b) ** k
            t = f"{q.numerator}" if q.denominator == 1 else f"({q.numerator}/{q.denominator})"
            if isinstance(b, int):
                emit_exact(f"log {t} base {b}", k)
                emit_exact(f"{t} is {b} to what power", k)
                if b == 10:
                    emit_exact(f"log10({t})", k)
                    emit_exact(f"log({t})", k)
                if b == 2:
                    emit_exact(f"log2({t})", k)
        else:
            tq, q = rational()
            t = f"({tq})" if "/" in tq else tq
            v = mpf(q.numerator) / q.denominator
            if isinstance(b, int):
                is_pow = any(Fraction(b) ** j == q for j in range(-40, 41))
                if not is_pow:
                    emit_approx(f"log {t} base {b}", ln(v) / ln(b))
                    emit_approx(f"{t} is {b} to what power", ln(v) / ln(b))
            if b == 10 and not any(Fraction(10) ** j == q for j in range(-40, 41)):
                emit_approx(f"log10({t})", log(v, 10))
                emit_approx(f"log({t})", log(v, 10))
            if b == 2 and not any(Fraction(2) ** j == q for j in range(-40, 41)):
                emit_approx(f"log2({t})", log(v, 2))
            if q != 1:
                emit_approx(f"ln({t})", ln(v))
    emit_exact("ln(1)", 0)
    emit_exact("exp(0)", 1)
    for _ in range(60):
        tq, q = rational()
        t = f"({tq})" if "/" in tq else tq
        emit_approx(f"exp({t})", exp(mpf(q.numerator) / q.denominator))


# angle in degrees -> exact (sin, cos, tan) or None where irrational/undefined
EXACT_SIN = {0: 0, 30: Fraction(1, 2), 90: 1, 150: Fraction(1, 2), 180: 0, 210: Fraction(-1, 2), 270: -1, 330: Fraction(-1, 2)}
EXACT_COS = {0: 1, 60: Fraction(1, 2), 90: 0, 120: Fraction(-1, 2), 180: -1, 240: Fraction(-1, 2), 270: 0, 300: Fraction(1, 2)}
EXACT_TAN = {0: 0, 45: 1, 135: -1, 180: 0, 225: 1, 315: -1}


def trig_cases():
    for deg in list(range(-360, 721, 15)) + [rng.randint(1, 359) for _ in range(40)]:
        m = deg % 360
        rad = mpf(deg) * pi / 180
        for name, fn, table in (("sin", sin, EXACT_SIN), ("cos", cos, EXACT_COS), ("tan", tan, EXACT_TAN)):
            for spelling in (f"{name}({deg} degrees)", f"{name}d({deg})"):
                if m in table:
                    emit_exact(spelling, table[m])
                elif name == "tan" and m in (90, 270):
                    continue
                else:
                    emit_approx(spelling, fn(rad))
    for _ in range(80):
        tq, q = rational()
        t = f"({tq})" if "/" in tq else tq
        x = mpf(q.numerator) / q.denominator
        if q != 0:
            for name, fn in (("sin", sin), ("cos", cos), ("tan", tan), ("sinh", sinh), ("cosh", cosh), ("tanh", tanh)):
                emit_approx(f"{name}({t})", fn(x))
            emit_approx(f"asinh({t})", asinh(x))
            emit_approx(f"atan({t})", atan(x))
        if q > 1:
            emit_approx(f"acosh({t})", acosh(x))
        if 0 < abs(q) < 1:
            emit_approx(f"asin({t})", asin(x))
            emit_approx(f"atanh({t})", atanh(x))
        if abs(q) < 1:
            emit_approx(f"acos({t})", acos(x))
            if q not in (Fraction(1, 2), Fraction(-1, 2), 0):
                emit_deg_approx(f"asind({t})", asin(x) * 180 / pi)
            if q not in (Fraction(1, 2), Fraction(-1, 2), 0):
                emit_deg_approx(f"acosd({t})", acos(x) * 180 / pi)
        if q not in (0, 1, -1):
            emit_deg_approx(f"atand({t})", atan(x) * 180 / pi)
    for expr, q in [("asind(0.5)", 30), ("asind(0)", 0), ("asind(1)", 90), ("asind(-0.5)", -30), ("acosd(1)", 0), ("acosd(0.5)", 60), ("acosd(0)", 90),
                    ("acosd(-0.5)", 120), ("acosd(-1)", 180), ("atand(0)", 0), ("atand(1)", 45), ("atand(-1)", -45), ("asin(0.5 degrees)", 30)]:
        emit_deg_exact(expr, q)
    for expr, q in [("sin(0)", 0), ("cos(0)", 1), ("tan(0)", 0), ("sinh(0)", 0), ("cosh(0)", 1), ("tanh(0)", 0), ("asin(0)", 0), ("acos(1)", 0), ("atan(0)", 0), ("asinh(0)", 0),
                    ("acosh(1)", 0), ("atanh(0)", 0)]:
        emit_exact(expr, q)


def constants():
    for name, v in (("pi", pi), ("π", pi), ("tau", 2 * pi), ("τ", 2 * pi), ("phi", phi), ("φ", phi)):
        emit_approx(name, v)
        emit_approx(f"2 × {name}", 2 * v)
    emit_approx("exp(1)", e)


sqrt_cases()
root_cases()
log_cases()
trig_cases()
constants()
