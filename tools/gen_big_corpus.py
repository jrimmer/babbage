#!/usr/bin/env python3
"""Large-result oracle: exact Fraction answers fingerprinted by bit lengths + 10dp text.

Columns: expression <TAB> numer_bits <TAB> denom_bits <TAB> fixed 10dp, or ERR:overflow.
Overflow means numer_bits + denom_bits exceeds 2^20, the engine's size cap.
"""
from decimal import Decimal, ROUND_HALF_UP, getcontext
from fractions import Fraction

getcontext().prec = 400000
CAP = 1 << 20

CASES = [
    ("1000*(1+0.05/365)^10950", lambda: 1000 * (1 + Fraction(5, 100) / 365) ** 10950),
    ("250000*(1+0.065/12)^360", lambda: 250000 * (1 + Fraction(65, 1000) / 12) ** 360),
    ("(1+0.1/52)^2600", lambda: (1 + Fraction(1, 10) / 52) ** 2600),
    ("(1+0.03/365)^36500", lambda: (1 + Fraction(3, 100) / 365) ** 36500),
    ("2^100000", lambda: Fraction(2) ** 100000),
    ("7^-3000", lambda: Fraction(7) ** -3000),
    ("(3/7)^5000", lambda: Fraction(3, 7) ** 5000),
    ("-(2/3)^4097", lambda: -(Fraction(2, 3) ** 4097)),
    ("1/3^20000", lambda: 1 / Fraction(3) ** 20000),
    ("1.0000001^100000", lambda: Fraction(10000001, 10**7) ** 100000),
    ("10^1000000", lambda: Fraction(10) ** 1000000),
]


def fixed10(q):
    d = Decimal(q.numerator) / Decimal(q.denominator)
    out = format(d.quantize(Decimal("1e-10"), rounding=ROUND_HALF_UP), "f")
    return out[1:] if out.startswith("-") and set(out[1:]) <= {"0", "."} else out


for text, fn in CASES:
    q = fn()
    nb, db = q.numerator.bit_length(), q.denominator.bit_length()
    if nb + db > CAP:
        print(f"{text}\tERR:overflow")
    else:
        print(f"{text}\t{nb}\t{db}\t{fixed10(q)}")
