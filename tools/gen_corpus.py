#!/usr/bin/env python3
"""Independent oracle for babbage-core.

Generates random expressions, evaluates them with Python's Fraction (exact),
and renders a 10-place half-away-from-zero string via decimal. Rust never
sees this code, so agreement is evidence the engine is right, not that it
matches itself.

Usage: tools/gen_corpus.py > crates/babbage-core/tests/corpus/arith.tsv
Columns: expression <TAB> exact n/d (or ERR:div0 / ERR:zero0) <TAB> fixed 10dp
"""
import random
import sys
from decimal import Decimal, ROUND_HALF_UP, getcontext
from fractions import Fraction

getcontext().prec = 4000
rng = random.Random(0xBABBA6E)

MUL = ["*", "×", "x", "·", "⋅"]
DIV = ["/", "÷", "∕"]
SUB = ["-", "–", "−"]
POW = ["^", "**"]
PREC = {"+": 1, "-": 1, "*": 2, "/": 2, "^": 4}


class Err(Exception):
    pass


def literal():
    kind = rng.random()
    if kind < 0.45:
        v = rng.choice([rng.randint(0, 12), rng.randint(0, 1000), rng.randint(0, 10**7)])
        text = str(v)
        if v >= 1000 and rng.random() < 0.6:
            text = f"{v:,}"
    elif kind < 0.85:
        places = rng.randint(1, 8)
        whole = rng.choice([0, rng.randint(0, 100), rng.randint(0, 99999)])
        frac = "".join(rng.choice("0123456789") for _ in range(places))
        if whole == 0 and rng.random() < 0.4:
            text = "." + frac
        else:
            whole_text = f"{whole:,}" if whole >= 1000 and rng.random() < 0.5 else str(whole)
            text = f"{whole_text}.{frac}"
    else:
        m = rng.randint(1, 999)
        e = rng.randint(-6, 6)
        text = f"{m}e{e}" if rng.random() < 0.5 else f"{m / 10:.1f}e{e}"
    return ("lit", text, Fraction(text.replace(",", "")))


def small_int_expr():
    r = rng.random()
    n = rng.randint(0, 6)
    base = ("lit", str(n), Fraction(n))
    if r < 0.6:
        return base
    if r < 0.8:
        return ("neg", base)
    a, b = rng.randint(0, 3), rng.randint(0, 3)
    return ("bin", "+", ("lit", str(a), Fraction(a)), ("lit", str(b), Fraction(b)))


def gen(depth):
    if depth == 0 or rng.random() < 0.25:
        return literal()
    r = rng.random()
    if r < 0.12:
        return ("neg", gen(depth - 1))
    if r < 0.2:
        return ("bin", "^", gen(depth - 1), small_int_expr())
    op = rng.choice(["+", "-", "*", "/"])
    return ("bin", op, gen(depth - 1), gen(depth - 1))


def evaluate(t):
    k = t[0]
    if k == "lit":
        return t[2]
    if k == "neg":
        return -evaluate(t[1])
    _, op, l, r = t
    a, b = evaluate(l), evaluate(r)
    if op == "+":
        return a + b
    if op == "-":
        return a - b
    if op == "*":
        return a * b
    if op == "/":
        if b == 0:
            raise Err("ERR:div0")
        return a / b
    assert b.denominator == 1
    if a == 0:
        if b == 0:
            raise Err("ERR:zero0")
        if b < 0:
            raise Err("ERR:div0")
        return Fraction(0)
    return a ** int(b)


def sp():
    return rng.choice(["", " ", "  "])


def render(t, parent=0, side=None):
    k = t[0]
    if k == "lit":
        return t[1]
    if k == "neg":
        inner = render(t[1], 3)
        s = ("-" if rng.random() < 0.5 else rng.choice(SUB)) + inner
        return f"({s})" if parent >= 4 or (parent == 3 and False) else s if parent < 3 else f"({s})"
    _, op, l, r = t
    if op == "^":
        ls = render(l, 5)
        rs = render(r, 3 if r[0] != "bin" else 5)
        s = f"{ls}{sp()}{rng.choice(POW)}{sp()}{rs}"
        return s if parent <= 4 else f"({s})"
    sym = {"+": "+", "-": rng.choice(SUB), "*": rng.choice(MUL), "/": rng.choice(DIV)}[op]
    own = PREC[op]
    ls = render(l, own)
    rs = render(r, own + (0 if op in "+*" and rng.random() < 0.5 else 1))
    if sym in ("x",):
        sym = " x "
    s = f"{ls}{sp()}{sym}{sp()}{rs}"
    return f"({s})" if parent > own else s


def fixed10(q):
    d = Decimal(q.numerator) / Decimal(q.denominator)
    out = format(d.quantize(Decimal("1e-10"), rounding=ROUND_HALF_UP), "f")
    if out.startswith("-") and set(out[1:]) <= {"0", "."}:
        out = out[1:]
    return out


def emit(expr, q):
    if isinstance(q, str):
        print(f"{expr}\t{q}\t-")
    else:
        print(f"{expr}\t{q.numerator}/{q.denominator}\t{fixed10(q)}")


def main():
    target = int(sys.argv[1]) if len(sys.argv) > 1 else 12000
    n = 0
    while n < target:
        tree = gen(rng.randint(1, 5))
        text = render(tree)
        if len(text) > 400:
            continue
        try:
            q = evaluate(tree)
        except Err as e:
            q = str(e)
        if not isinstance(q, str) and (abs(q.numerator) > 10**300 or q.denominator > 10**300):
            continue
        emit(text, q)
        n += 1
    for _ in range(1500):
        m = rng.randint(0, 10**12)
        sign = rng.choice(["", "-"])
        text = f"{sign}{2 * m + 1}/(2*10^10)"
        emit(text, Fraction((-1 if sign else 1) * (2 * m + 1), 2 * 10**10))
    for k in range(1, 400, 2):
        emit(f"{k}/2^11", Fraction(k, 2**11))
        emit(f"-{k}/2^11", Fraction(-k, 2**11))


main()
