#!/usr/bin/env python3
"""Independent oracle for percentage forms.

Each form is a direct formula from the documented percentage rules, computed
with Fraction. Nothing here parses text, so it shares no logic with the engine.

Columns: expression <TAB> kind (plain|percent|ERR:div0|ERR:unsupported) <TAB> exact n/d <TAB> 10dp display
A percent's display is the figure shown (fraction x 100) followed by %.
"""
import random
from decimal import Decimal, ROUND_HALF_UP, getcontext
from fractions import Fraction

getcontext().prec = 4000
rng = random.Random(0x9E2CE47)


def num():
    r = rng.random()
    if r < 0.5:
        t = str(rng.choice([rng.randint(0, 20), rng.randint(0, 5000)]))
    elif r < 0.9:
        t = f"{rng.randint(0, 999)}.{''.join(rng.choice('0123456789') for _ in range(rng.randint(1, 4)))}"
        if t.startswith("0.") and rng.random() < 0.3:
            t = t[1:]
    else:
        t = f"{rng.randint(1, 99)}e{rng.randint(-3, 3)}"
    return t, Fraction(t)


def pct():
    t, v = num()
    return t, v / 100


def frac():
    n, d = rng.randint(1, 9), rng.randint(1, 12)
    return f"{n}/{d}", Fraction(n, d)


def scientific10(d):
    """A value below a millionth the way the engine writes it: a mantissa of up to ten decimals, then `e` and the
    exponent (`8.1e-9`)."""
    e = d.adjusted()
    mantissa = (d / Decimal(10) ** e).quantize(Decimal("1e-10"), rounding=ROUND_HALF_UP)
    text = format(mantissa, "f").rstrip("0").rstrip(".")
    return f"{text}e{e}"


def fixed10(q):
    d = Decimal(q.numerator) / Decimal(q.denominator)
    if d != 0 and abs(d) < Decimal("1e-6"):
        return scientific10(d)
    out = format(d.quantize(Decimal("1e-10"), rounding=ROUND_HALF_UP), "f")
    return out[1:] if out.startswith("-") and set(out[1:]) <= {"0", "."} else out


def emit(text, kind, q):
    if kind.startswith("ERR"):
        print(f"{text}\t{kind}\t-\t-")
    elif kind == "percent":
        print(f"{text}\tpercent\t{q.numerator}/{q.denominator}\t{fixed10(q * 100)}%")
    else:
        print(f"{text}\tplain\t{q.numerator}/{q.denominator}\t{fixed10(q)}")


def div(a, b):
    return ("ERR:div0", None) if b == 0 else ("ok", a / b)


def safe(text, kind, thunk):
    try:
        emit(text, kind, thunk())
    except ZeroDivisionError:
        emit(text, "ERR:div0", None)


def forms():
    (px, X), (pa, A), (pb, B) = num(), num(), num()
    (pp, P), (pq, Q), (pr, R) = pct(), pct(), pct()
    Pt, Qt, Rt = (pp, pq, pr)
    (pp_, _), (pq_, _), (pr_, _) = (num(), num(), num())
    # percent literals use the figure text: "12.5%" means fraction 0.125
    pf = lambda t: f"{t}%"
    out = []
    # word forms
    out += [(f"{Pt}% of {px}", "plain", lambda: P * X)]
    out += [(f"{Pt}% × {px}", "plain", lambda: P * X), (f"{px} × {Pt}%", "plain", lambda: X * P)]
    out += [(f"{px} + {Pt}%", "plain", lambda: X * (1 + P)), (f"{px} - {Pt}%", "plain", lambda: X * (1 - P))]
    out += [(f"{Pt}% on {px}", "plain", lambda: X * (1 + P)), (f"{Pt}% off {px}", "plain", lambda: X * (1 - P))]
    # reverse forms
    out += [(f"{pa} is {Pt}% of what", "plain", lambda: A / P)]
    out += [(f"{pa} is {Pt}% on what", "plain", lambda: A / (1 + P))]
    out += [(f"{pa} is {Pt}% off what", "plain", lambda: A / (1 - P))]
    # ratios
    out += [(f"{pa} is what % of {pb}", "percent", lambda: A / B)]
    out += [(f"{pa} as a % of {pb}", "percent", lambda: A / B)]
    out += [(f"{pa} as % of {pb}", "percent", lambda: A / B)]
    out += [(f"{pa} is what % on {pb}", "percent", lambda: (A - B) / B)]
    out += [(f"{pa} is what % off {pb}", "percent", lambda: (B - A) / B)]
    out += [(f"{pa} to {pb} is what %", "percent", lambda: (B - A) / A)]
    out += [(f"{pa} to {pb} as %", "percent", lambda: (B - A) / A)]
    out += [(f"{pa} in {pb} is what %", "percent", lambda: A / B)]
    out += [(f"{pa} as %", "percent", lambda: A), (f"{pa} as percent", "percent", lambda: A), (f"{pa} is what %", "percent", lambda: A)]
    out += [(f"{pa}/{pb} as %", "percent", lambda: A / B), (f"{pa}/{pb} %", "percent", lambda: A / B)]
    # percent arithmetic
    out += [(f"{Pt}% + {Qt}%", "percent", lambda: P + Q), (f"{Pt}% - {Qt}%", "percent", lambda: P - Q)]
    out += [(f"{Pt}% + {px}", "percent", lambda: P + X), (f"{Pt}% - {px}", "percent", lambda: P - X)]
    out += [(f"{Pt}% + {px} + {Qt}%", "percent", lambda: P + X + Q)]
    out += [(f"{px} + {Pt}% + {Qt}%", "plain", lambda: X * (1 + P) * (1 + Q))]
    out += [(f"{px} - {Pt}% - {Qt}%", "plain", lambda: X * (1 - P) * (1 - Q))]
    out += [(f"{px} + {Pt}% - {Qt}%", "plain", lambda: X * (1 + P) * (1 - Q))]
    out += [(f"{px} + {Pt}% + {Qt}% - {Rt}%", "plain", lambda: X * (1 + P) * (1 + Q) * (1 - R))]
    out += [(f"-{Pt}% + {Qt}%", "percent", lambda: -P + Q)]
    out += [(f"({px} + {Pt}%) × 2", "plain", lambda: X * (1 + P) * 2)]
    out += [(f"2 × ({px} - {Pt}%)", "plain", lambda: 2 * X * (1 - P))]
    # fractions where a percentage can go
    fa, FA = frac()
    out += [(f"{fa} of {px}", "plain", lambda: FA * X)]
    out += [(f"{pa} is {fa} of what", "plain", lambda: A / FA)]
    out += [(f"{fa} on {px}", "plain", lambda: X * (1 + FA)), (f"{fa} off {px}", "plain", lambda: X * (1 - FA))]
    # Ticket 0012: 10% x 10% is 1%, 20 / 50% is 40, 50% / 2 is 25%, 10%^2 is 0.01
    out += [(f"{Pt}% × {Qt}%", "percent", lambda: P * Q)]
    out += [(f"{px} / {Pt}%", "plain", lambda: X / P)]
    out += [(f"{Pt}% / {px}", "percent", lambda: P / X)]
    out += [(f"{Pt}%^2", "plain", lambda: P**2)]
    # still undefined: strictly an error
    for text in [f"{Pt}% / {Qt}%", f"{px}^{Pt}%", f"{Pt}% on {Qt}%", f"{Pt}% off {Qt}%", f"{Pt}%%"]:
        out.append((text, "ERR:unsupported", None))
    return out


def main():
    for _ in range(900):
        for text, kind, thunk in forms():
            if kind.startswith("ERR"):
                emit(text, kind, None)
            else:
                safe(text, kind, thunk)
    # forced zero divisors
    for text in ["5 is 0% of what", "5 is 100% off what", "5 is -100% on what", "5 is what % of 0", "5 is what % on 0", "5 is what % off 0", "0 to 5 is what %", "0 to 5 as %", "5 in 0 is what %", "5/0 as %", "5/0 %", "5 is 0/3 of what", "0/1 of 5 + 1/0"]:
        print(text + "\tERR:div0\t-\t-")
main()
