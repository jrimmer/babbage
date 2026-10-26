#!/usr/bin/env python3
"""Oracle for number formats: Decimal does the arithmetic and the grouping, written once per region.

Columns: region (na, we, ee) <TAB> expression <TAB> expected (unabbreviated, 10 places, trailing zeros trimmed).
"""
import random
from decimal import Decimal, ROUND_HALF_UP, getcontext

getcontext().prec = 200
rng = random.Random(0x4E6F)
MARKS = {"na": (",", "."), "we": (".", ","), "ee": (" ", ",")}
rows = []


def show(d, region):
    group, point = MARKS[region]
    q = d.quantize(Decimal(1).scaleb(-10), rounding=ROUND_HALF_UP)
    text = format(q, "f")
    neg = text.startswith("-")
    text = text.lstrip("-")
    whole, _, frac = text.partition(".")
    frac = frac.rstrip("0")
    grouped = f"{int(whole):,}".replace(",", group)
    out = grouped + (point + frac if frac else "")
    zero = set(out) <= set("0" + group + point)
    return ("-" if neg and not zero else "") + out


def write(whole, frac, region, style):
    group, point = MARKS[region]
    if style == "plain":
        g = whole
    elif style == "underscore":
        g = "_".join(whole[max(0, i - 3):i] for i in range(len(whole) % 3 or 3, len(whole) + 3, 3)) if len(whole) > 3 else whole
        g = "_".join(f"{int(whole):,}".split(","))
    else:
        g = f"{int(whole):,}"
        if style == "group-ascii":
            g = g.replace(",", " " if region == "ee" else group)
        else:
            g = g.replace(",", group)
    sep = point
    if region == "ee" and style == "dot-decimal":
        sep = "."
    return g + (sep + frac if frac else "")


def pieces():
    whole = str(rng.randint(0, 10 ** rng.randint(1, 12)))
    frac = "".join(rng.choice("0123456789") for _ in range(rng.choice([0, 0, 1, 2, 3, 5, 10, 12]))).rstrip("0") if rng.random() < 0.7 else ""
    return whole, frac


for region in MARKS:
    styles = ["plain", "group", "underscore"] + (["group-ascii", "dot-decimal"] if region == "ee" else [])
    for _ in range(3000):
        whole, frac = pieces()
        style = rng.choice(styles)
        neg = rng.random() < 0.2
        d = Decimal(whole + ("." + frac if frac else ""))
        typed = write(whole, frac, region, style)
        if style == "plain" and len(whole) > 3 and False:
            continue
        expr = f"{'-' if neg else ''}{typed} + 0"
        add = rng.choice([0, 0, 1])
        value = (-d if neg else d)
        rows.append((region, expr, show(value, region)))
        whole2, frac2 = pieces()
        d2 = Decimal(whole2 + ("." + frac2 if frac2 else ""))
        typed2 = write(whole2, frac2, region, rng.choice(styles))
        rows.append((region, f"{typed} + {typed2}", show(d + d2, region)))
        rows.append((region, f"{typed} * 2 - {typed2}", show(d * 2 - d2, region)))

for row in rows:
    print("\t".join(row))
