#!/usr/bin/env python3
"""Independent simulator for sheets: variables, line references, subtotals, tags, headings, dividers.

Values are (kind, Fraction, money) with kind plain|percent and money False|True (USD). The rules are
written from the reference's sheet pages; nothing here parses text, so it shares no logic with the engine.

Format: `SHEET <n>` then `<line text>\t<expected>` per line, where expected is the displayed answer or one of
.blank .heading .divider .comment .error .text.
"""
import random
from decimal import Decimal, ROUND_HALF_UP, getcontext
from fractions import Fraction

getcontext().prec = 4000
rng = random.Random(0x5EE7)
STEPS = [("k", 3, 10**5), ("M", 6, 10**6), ("G", 9, 10**9), ("T", 12, 10**12)]
NAMES = ["rent", "tax rate", "bonus", "price", "fee", "salary", "monthly cost"]


def digits(q, places, trim):
    d = Decimal(q.numerator) / Decimal(q.denominator)
    out = format(d.quantize(Decimal(1).scaleb(-places), rounding=ROUND_HALF_UP), "f")
    neg = out.startswith("-")
    out = out.lstrip("-")
    whole, _, frac = out.partition(".")
    if trim:
        frac = frac.rstrip("0")
    text = f"{int(whole):,}" + ("." + frac if frac else "")
    return ("-" if neg and set(text) - set("0.,") else "") + text


def scaled(q):
    a = abs(q)
    idx = None
    for i, (_, _, floor) in enumerate(STEPS):
        if a >= floor:
            idx = i
    if idx is None:
        return None
    while True:
        sym, shift, _ = STEPS[idx]
        text = digits(q / 10**shift, 10, True)
        if len(text.lstrip("-").split(".")[0].replace(",", "")) > 3 and idx + 1 < len(STEPS):
            idx += 1
            continue
        return text + sym


def show(v, exact_money=False):
    kind, q, money = v
    if kind == "percent":
        return digits(q * 100, 10, True) + "%"
    big = scaled(q)
    if money:
        if big is None:
            body = digits(q, 10, True) if exact_money else digits(q, 2, False)
        else:
            body = big
        sign = "-" if body.startswith("-") else ""
        return f"{sign}${body.lstrip('-')}"
    return big if big is not None else digits(q, 10, True)


def add(a, b, sign=1):
    (ka, qa, ma), (kb, qb, mb) = a, b
    if ka == "plain" and kb == "plain":
        return ("plain", qa + sign * qb, ma or mb)
    if ka == "percent" and kb == "percent":
        return ("percent", qa + sign * qb, False)
    if ka == "percent" and kb == "plain" and not mb:
        return ("percent", qa + sign * qb, False)
    if ka == "plain" and kb == "percent":
        return ("plain", qa * (1 + sign * qb), ma)
    raise ValueError("undefined")


def mul(a, b):
    (ka, qa, ma), (kb, qb, mb) = a, b
    if ka == "plain" and kb == "plain":
        if ma and mb:
            raise ValueError("undefined")
        return ("plain", qa * qb, ma or mb)
    if ka == "percent" and kb == "plain":
        return ("plain", qa * qb, mb)
    if ka == "plain" and kb == "percent":
        return ("plain", qa * qb, ma)
    raise ValueError("undefined")


def total(values):
    plain = [v for v in values if v[0] == "plain"]
    if not plain:
        raise ValueError("empty")
    acc = plain[0]
    for v in plain[1:]:
        acc = add(acc, v)
    for p in [v for v in values if v[0] == "percent"]:
        acc = add(acc, p)
    return acc


def number():
    r = rng.random()
    if r < 0.6:
        return str(rng.randint(1, 900)), None
    return f"{rng.randint(0, 99)}.{rng.randint(1, 99):02d}", None


def literal():
    kind = rng.choice(["plain", "money", "percent"])
    t, _ = number()
    q = Fraction(t)
    if kind == "plain":
        return t, ("plain", q, False)
    if kind == "money":
        return f"${t}", ("plain", q, True)
    return f"{t}%", ("percent", q / 100, False)


def sheet():
    lines, results = [], []  # results: ("value", v) | (".blank",) ...
    vars_, section, sub_start = {}, 0, 0
    tags = {}
    for _ in range(rng.randint(8, 18)):
        i = len(lines)
        choice = rng.random()
        text, out = None, None
        values_above = [(j, r[1]) for j, r in enumerate(results) if r[0] == "value"]
        if choice < 0.22:
            t, v = literal()
            text, out = t, ("value", v, False)
        elif choice < 0.38:
            name = rng.choice(NAMES)
            t, v = literal()
            text, out = f"{name} = {t}", ("value", v, True)
            vars_[name] = v
        elif choice < 0.52 and vars_:
            name = rng.choice(list(vars_))
            v = vars_[name]
            n, _ = number()
            ops = []
            if v[0] == "plain":
                ops = [(f"{name} + {n}", add(v, ("plain", Fraction(n), False))), (f"{name} - {n}", add(v, ("plain", Fraction(n), False), -1)),
                       (f"{name} × {n}", mul(v, ("plain", Fraction(n), False))), (f"{name} + {n}%", add(v, ("percent", Fraction(n) / 100, False)))]
            elif v[0] == "percent":
                ops = [(f"{name} + {n}%", add(v, ("percent", Fraction(n) / 100, False))), (f"{name} × {n}", mul(v, ("plain", Fraction(n), False)))]
            text, res = rng.choice(ops)
            out = ("value", res, False)
        elif choice < 0.60 and vars_:
            name = rng.choice(list(vars_))
            n, _ = number()
            v = vars_[name]
            sign = rng.choice([1, -1])
            if v[0] == "plain":
                res = add(v, ("plain", Fraction(n), False), sign)
                text = f"{name} {'+' if sign == 1 else '-'}= {n}"
                vars_[name] = res
                out = ("value", res, True)
        elif choice < 0.72 and values_above:
            j, v = rng.choice(values_above)
            n, _ = number()
            if v[0] == "plain":
                ops = [(f"line{j + 1} + {n}", add(v, ("plain", Fraction(n), False))), (f"line{j + 1} × {n}", mul(v, ("plain", Fraction(n), False))),
                       (f"line{j + 1} + {n}%", add(v, ("percent", Fraction(n) / 100, False)))]
                text, res = rng.choice(ops)
                out = ("value", res, False)
            else:
                text, out = f"line{j + 1} × {n}", ("value", mul(v, ("plain", Fraction(n), False)), False)
        elif choice < 0.80:
            vals = [r[1] for r in results[sub_start:] if r[0] == "value" and not r[3]]
            try:
                res = total(vals)
            except ValueError:
                continue
            text, out = "subtotal", ("value", res, False, True)
            sub_start = i + 1
        elif choice < 0.84:
            text, out = "# Heading", (".heading",)
            sub_start = i + 1
        elif choice < 0.87:
            text, out = "---", (".divider",)
            sub_start = section = i + 1
            vars_, tags = {}, {}
        elif choice < 0.90:
            text, out = "// a note 123", (".comment",)
        elif choice < 0.93:
            text, out = "", (".blank",)
        elif choice < 0.97:
            tag = rng.choice(["food", "travel"])
            t, v = literal()
            if v[0] == "percent":
                continue
            text, out = f"{t} #{tag}", ("value", v, False)
            tags.setdefault(tag, []).append(i)
        else:
            tag = rng.choice(["food", "travel"])
            fn = rng.choice(["total", "average"])
            idxs = [j for j in tags.get(tag, []) if results[j][0] == "value"]
            if not idxs:
                continue
            try:
                s = total([results[j][1] for j in idxs])
            except ValueError:
                continue
            res = s if fn == "total" else ("plain", s[1] / len(idxs), s[2])
            text, out = f"{fn} of #{tag}", ("value", res, False)
        if text is None or out is None:
            continue
        lines.append(text)
        if out[0] == "value":
            is_sub = len(out) > 3 and out[3]
            results.append(("value", out[1], out[2], is_sub))
        else:
            results.append((out[0], None, False, False))
    return lines, results


for n in range(600):
    lines, results = sheet()
    print(f"SHEET {n}")
    for text, r in zip(lines, results):
        shown = show(r[1], exact_money=r[2]) if r[0] == "value" else r[0]
        print(f"{text}\t{shown}")
