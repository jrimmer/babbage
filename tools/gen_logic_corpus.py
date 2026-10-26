#!.oracle-venv/bin/python
"""Oracle for comparisons, boolean logic, number bases and bitwise operators.

Columns: expression <TAB> expected display. Comparisons across units use pint's exact factors.
"""
import random
from fractions import Fraction

import pint

rng = random.Random(0xB001)
ureg = pint.UnitRegistry(non_int_type=Fraction)
Q = ureg.Quantity
OPS = {"==": lambda a, b: a == b, "!=": lambda a, b: a != b, "<": lambda a, b: a < b, ">": lambda a, b: a > b, "<=": lambda a, b: a <= b, ">=": lambda a, b: a >= b}
UNITS = [("m", "meter"), ("km", "kilometer"), ("cm", "centimeter"), ("mi", "mile"), ("ft", "foot"), ("kg", "kilogram"), ("g", "gram"), ("lb", "pound"), ("s", "second"), ("h", "hour"), ("min", "minute"), ("L", "liter"), ("gal", "gallon"), ("GB", "gigabyte"), ("MiB", "mebibyte")]
FAMILY = {"m": "len", "km": "len", "cm": "len", "mi": "len", "ft": "len", "kg": "mass", "g": "mass", "lb": "mass", "s": "time", "h": "time", "min": "time", "L": "vol", "gal": "vol", "GB": "data", "MiB": "data"}


def num():
    r = rng.random()
    if r < 0.5:
        t = str(rng.randint(0, 50))
    else:
        t = f"{rng.randint(0, 99)}.{rng.randint(0, 99)}"
    return t, Fraction(t)


def show(b):
    return "true" if b else "false"


def emit(text, out):
    print(f"{text}\t{out}")


for _ in range(3000):
    (ta, a), (tb, b) = num(), num()
    for sym, fn in OPS.items():
        emit(f"{ta} {sym} {tb}", show(fn(a, b)))
    # equal values written differently
    emit(f"{ta} == {ta}.0", "true") if "." not in ta else None
    # units
    u1, u2 = rng.choice(UNITS), rng.choice(UNITS)
    if FAMILY[u1[0]] == FAMILY[u2[0]]:
        qa = Fraction(Q(a, u1[1]).to_base_units().magnitude)
        qb = Fraction(Q(b, u2[1]).to_base_units().magnitude)
        for sym, fn in OPS.items():
            emit(f"{ta} {u1[0]} {sym} {tb} {u2[0]}", show(fn(qa, qb)))
    # money, same currency
    for sym, fn in OPS.items():
        emit(f"${ta} {sym} ${tb}", show(fn(a, b)))
    # percent
    for sym, fn in OPS.items():
        emit(f"{ta}% {sym} {tb}%", show(fn(a, b)))
    # booleans
    (tc, c), (td, d) = num(), num()
    p, q = a < b, c < d
    emit(f"({ta} < {tb}) && ({tc} < {td})", show(p and q))
    emit(f"({ta} < {tb}) || ({tc} < {td})", show(p or q))
    emit(f"{ta} < {tb} and {tc} < {td}", show(p and q))
    emit(f"{ta} < {tb} or {tc} < {td}", show(p or q))
    emit(f"({ta} < {tb}) xor ({tc} < {td})", show(p != q))
    emit(f"({ta} < {tb}) == ({tc} < {td})", show(p == q))
    emit(f"({ta} < {tb}) != ({tc} < {td})", show(p != q))
    emit(f"{ta} < {tb} or {tc} < {td} and {ta} == {tb}", show(p or (q and a == b)))

# number bases and bitwise
for _ in range(3000):
    n = rng.randint(0, 10**rng.randint(1, 15))
    emit(f"{n} as hex", f"0x{n:X}")
    emit(f"{n} in hex", f"0x{n:X}")
    emit(f"{n} to hexadecimal", f"0x{n:X}")
    emit(f"{n} in binary", f"0b{n:b}")
    emit(f"{n} as octal", f"0o{n:o}")
    emit(f"{n} as base 8", f"0o{n:o}")
    emit(f"{n} as base 2", f"0b{n:b}")
    emit(f"{n} as base 16", f"0x{n:X}")
    emit(f"hex({n})", f"0x{n:X}")
    emit(f"bin({n})", f"0b{n:b}")
    emit(f"oct({n})", f"0o{n:o}")
    emit(f"0x{n:X} to decimal", f"{n:,}" if n < 10**5 else None) if n < 10**5 else None
    emit(f"int(0x{n:X})", f"{n:,}") if n < 10**5 else None
    emit(f"0x{n:x} as base 2", f"0b{n:b}")
    emit(f"0b{n:b} as hex", f"0x{n:X}")
    emit(f"0o{n:o} as binary", f"0b{n:b}")
    m = rng.randint(0, 10**rng.randint(1, 12))
    for sym, op in (("&", lambda x, y: x & y), ("|", lambda x, y: x | y), ("AND", lambda x, y: x & y), ("OR", lambda x, y: x | y), ("xor", lambda x, y: x ^ y), ("XOR", lambda x, y: x ^ y)):
        r = op(n, m)
        emit(f"{n} {sym} {m}", f"{r:,}" if r < 10**5 else None) if r < 10**5 else None
        emit(f"0x{n:X} {sym} 0x{m:X}", f"0x{r:X}")
        emit(f"0b{n:b} {sym} 0b{m:b}", f"0b{r:b}")
    k = rng.randint(0, 20)
    emit(f"0x{n:X} << {k}", f"{n << k:,}" if (n << k) < 10**5 else None) if (n << k) < 10**5 else None
    emit(f"0x{n:X} >> {k}", f"{n >> k:,}" if (n >> k) < 10**5 else None) if (n >> k) < 10**5 else None
    emit(f"0b{n:b} << {k}", f"0b{n << k:b}") if False else None
