#!/usr/bin/env python3
"""Oracle for money: arithmetic in Fraction, display in Decimal (fixed minor digits, symbol placement, k/M/G/T from 100,000).

Columns: expression <TAB> exact n/d <TAB> display.
"""
import random
from decimal import Decimal, ROUND_HALF_UP, getcontext
from fractions import Fraction

getcontext().prec = 4000
rng = random.Random(0xC0FFEE)
# code, symbol (or None), minor digits, ways to write an amount
CUR = [("USD", "$", 2), ("EUR", "€", 2), ("GBP", "£", 2), ("JPY", "¥", 0), ("CHF", None, 2), ("BHD", None, 3), ("INR", "₹", 2), ("AUD", "A$", 2)]
STEPS = [("k", 3, 10**5), ("M", 6, 10**6), ("G", 9, 10**9), ("T", 12, 10**12)]


def group(whole):
    return f"{int(whole):,}"


def digits(q, places, trim):
    d = Decimal(q.numerator) / Decimal(q.denominator)
    out = format(d.quantize(Decimal(1).scaleb(-places), rounding=ROUND_HALF_UP), "f")
    neg = out.startswith("-")
    out = out.lstrip("-")
    whole, _, frac = out.partition(".")
    if trim:
        frac = frac.rstrip("0")
    text = group(whole) + ("." + frac if frac else "")
    zero = set(text) <= {"0", ".", ","}
    return ("-" if neg and not zero else "") + text


def number_text(q, places):
    a = abs(q)
    idx = None
    for i, (_, _, floor) in enumerate(STEPS):
        if a >= floor:
            idx = i
    if idx is None:
        # An amount that would read as nothing keeps its digits: $0.0046320835, not $0.00.
        if q != 0 and abs(q) < Fraction(1, 2 * 10**places):
            return digits(q, 10, True)
        return digits(q, places, False)
    while True:
        sym, shift, _ = STEPS[idx]
        text = digits(q / 10**shift, 10, True)
        if len(text.lstrip("-").split(".")[0].replace(",", "")) > 3 and idx + 1 < len(STEPS):
            idx += 1
            continue
        return text + sym


def money(q, cur):
    code, symbol, minor = cur
    body = number_text(q, minor)
    sign = "-" if body.startswith("-") else ""
    body = body.lstrip("-")
    return f"{sign}{symbol}{body}" if symbol else f"{sign}{body} {code}"


def amount():
    r = rng.random()
    if r < 0.5:
        t = str(rng.randint(1, 5000))
    elif r < 0.85:
        t = f"{rng.randint(0, 9999)}.{rng.randint(0, 999):03d}".rstrip("0").rstrip(".") or "0"
    else:
        t = f"{rng.randint(1, 99)}"
    return t, Fraction(t)


def lit(cur, t):
    """The ways a user can write this amount of this currency."""
    code, symbol, _ = cur
    forms = [f"{t} {code}"]
    if symbol and symbol != "A$":
        forms.append(f"{symbol}{t}")
    if code == "USD":
        forms.append(f"US${t}")
    if code == "AUD":
        forms.extend([f"A${t}", f"AU${t}"])
    return rng.choice(forms)


def emit(expr, q, cur):
    print(f"{expr}\t{q.numerator}/{q.denominator}\t{money(q, cur)}")


for _ in range(5000):
    cur = rng.choice(CUR)
    (ta, A), (tb, B), (tk, K) = amount(), amount(), amount()
    P = Fraction(rng.randint(0, 200), 100)
    form = rng.randrange(11)
    if form == 0:
        emit(f"{lit(cur, ta)} + {lit(cur, tb)}", A + B, cur)
    elif form == 1:
        emit(f"{lit(cur, ta)} - {lit(cur, tb)}", A - B, cur)
    elif form == 2:
        emit(f"{lit(cur, ta)} × {tk}", A * K, cur)
        emit(f"{tk} × {lit(cur, ta)}", A * K, cur)
    elif form == 3 and K != 0:
        emit(f"{lit(cur, ta)} / {tk}", A / K, cur)
    elif form == 4:
        emit(f"{lit(cur, ta)} + {tk}", A + K, cur)
    elif form == 5:
        emit(f"{lit(cur, ta)} + {int(P * 100)}%", A * (1 + P), cur)
        emit(f"{lit(cur, ta)} - {int(P * 100)}%", A * (1 - P), cur)
    elif form == 6:
        emit(f"{int(P * 100)}% of {lit(cur, ta)}", A * P, cur)
    elif form == 7:
        emit(f"{lit(cur, ta)} × {rng.randint(1, 9)} days", A * 0 + A * Fraction(rng.randint(1, 1)), cur) if False else None
        n = rng.randint(1, 9)
        emit(f"{lit(cur, ta)} × {n} days", A * n, cur)
    elif form == 8:
        for suffix, mult in [("k", 10**3), ("K", 10**3), ("m", 10**6), ("M", 10**6), ("bn", 10**9), ("B", 10**9), ("b", 10**9), ("t", 10**12), ("tn", 10**12)]:
            if cur[1] == "$":
                q = Fraction(ta) * mult
                emit(f"${ta}{suffix}", q, cur)
                break
    elif form == 9 and cur[1] == "$":
        n = rng.randint(2, 9)
        emit(f"${ta}k ÷ {n}", Fraction(ta) * 1000 / n, cur)
    else:
        emit(f"-{lit(cur, ta)}", -A, cur) if lit(cur, ta).startswith(("$", "€", "£", "¥", "₹", "A$", "US")) or True else None
