#!/usr/bin/env python3
"""Oracle for dates: the standard library and dateutil, with the clock fixed at Saturday 2024-06-15 12:00 UTC.

Columns: expression <TAB> mode <TAB> expected. Modes: text (the whole display), days, workdays, hours
(the number of that unit, which does not depend on how the unit is spelled).
"""
import calendar
import random
from datetime import date, datetime, timedelta

from dateutil.easter import easter
from dateutil.relativedelta import relativedelta

rng = random.Random(0xDA7E5)
TODAY = date(2024, 6, 15)
MONTHS = list(calendar.month_name)[1:]
SHORT = {m: m[:3] for m in MONTHS}
SHORT["September"] = rng.choice(["Sep", "Sept"])
DAYS = list(calendar.day_name)


def shown(d, time=None):
    text = f"{MONTHS[d.month - 1]} {d.day}"
    if d.year != TODAY.year:
        text += f", {d.year}"
    return text + (f" at {time}" if time else "")


def literal(d):
    month = MONTHS[d.month - 1]
    style = rng.choice(["long", "short", "day-first", "dayfirst-short"])
    if style == "long":
        return f"{month} {d.day}, {d.year}"
    if style == "short":
        return f"{month[:3]} {d.day}, {d.year}"
    if style == "day-first":
        return f"{d.day} {month} {d.year}"
    return f"{d.day} {month[:3]} {d.year}"


def rand_date(lo=1000, hi=3000):
    while True:
        try:
            return date(rng.randint(lo, hi), rng.randint(1, 12), rng.randint(1, 31))
        except ValueError:
            continue


def span(days):
    sign = "-" if days < 0 else ""
    days = abs(days)
    parts = []
    for n, name in ((days // 7, "week"), (days % 7, "day")):
        if n:
            parts.append(f"{n} {name}{'' if n == 1 else 's'}")
    return sign + " ".join(parts)


def workdays(a, b):
    if b < a:
        return -workdays(b, a)
    return sum(1 for i in range((b - a).days) if (a + timedelta(i)).weekday() < 5)


def add_workdays(a, n):
    step = 1 if n >= 0 else -1
    left = abs(n)
    while left:
        a += timedelta(step)
        if a.weekday() < 5:
            left -= 1
    return a


def thanksgiving(y):
    thursdays = [w[calendar.THURSDAY] for w in calendar.monthcalendar(y, 11) if w[calendar.THURSDAY]]
    return date(y, 11, thursdays[3])


def fmt(d, pattern):
    out, i = [], 0
    while i < len(pattern):
        c = pattern[i]
        run = next(k for k in range(1, len(pattern) - i + 1) if i + k == len(pattern) or pattern[i + k] != c)
        if not c.isalpha():
            out.append(c)
            i += 1
            continue
        piece = {
            ("y", 4): str(d.year).zfill(4), ("y", 2): str(d.year % 100).zfill(2),
            ("M", 1): str(d.month), ("M", 2): f"{d.month:02}", ("M", 3): MONTHS[d.month - 1][:3], ("M", 4): MONTHS[d.month - 1],
            ("d", 1): str(d.day), ("d", 2): f"{d.day:02}",
            ("E", 3): DAYS[d.weekday()][:3], ("E", 4): DAYS[d.weekday()],
            ("H", 2): "00", ("h", 2): "12", ("m", 2): "00", ("a", 1): "AM",
        }.get((c, run))
        if piece is None:
            raise KeyError((c, run))
        out.append(piece)
        i += run
    return "".join(out)


rows = []


def add(expr, mode, expected):
    rows.append((expr, mode, str(expected)))


for _ in range(450):
    d = rand_date()
    lit = literal(d)
    add(f"day of the week on {lit}", "text", DAYS[d.weekday()])
    add(f"day number on {lit}", "text", d.timetuple().tm_yday)
    add(f"week number on {lit}", "text", d.isocalendar().week)
    add(f"day of the month on {lit}", "text", d.day)
    add(f"days in {MONTHS[d.month - 1]} {d.year}", "days", calendar.monthrange(d.year, d.month)[1])
    add(lit, "text", shown(d))

for y in range(1000, 3001, 7):
    add(f"days in {y}", "days", 366 if calendar.isleap(y) else 365)
    for q in (1, 2, 3, 4):
        months = range(3 * q - 2, 3 * q + 1)
        add(f"days in Q{q} {y}", "days", sum(calendar.monthrange(y, m)[1] for m in months))

for _ in range(1500):
    d = rand_date(1100, 2900)
    n = rng.randint(0, 60)
    unit = rng.choice(["day", "week", "month", "year"])
    if unit == "year":
        n = rng.randint(0, 40)
    delta = {"day": relativedelta(days=n), "week": relativedelta(weeks=n), "month": relativedelta(months=n), "year": relativedelta(years=n)}[unit]
    word = unit + ("" if n == 1 else "s")
    lit = literal(d)
    add(f"{lit} + {n} {word}", "text", shown(d + delta))
    add(f"{lit} - {n} {word}", "text", shown(d - delta))
    add(f"{n} {word} after {lit}", "text", shown(d + delta))
    add(f"{n} {word} before {lit}", "text", shown(d - delta))

for _ in range(800):
    a, b = rand_date(1500, 2500), rand_date(1500, 2500)
    if a == b:
        continue
    la, lb = literal(a), literal(b)
    add(f"{la} - {lb}", "text", span((a - b).days))
    add(f"days between {la} and {lb}", "days", abs((a - b).days))
    add(f"workdays between {la} and {lb}", "workdays", workdays(a, b))
    add(f"work hours between {la} and {lb}", "hours", workdays(a, b) * 8)
    add(f"midpoint between {la} and {lb}", "text", shown(a + (b - a) / 2) if (b - a).days % 2 == 0 else shown(min(a, b) + timedelta(days=abs((b - a).days) // 2)) + " at 12:00 pm")
    lo, hi = sorted((a, b))
    add(f"{literal(lo)} through {literal(hi)}", "days", (hi - lo).days + 1)

for _ in range(400):
    y, m = rng.randint(1500, 2500), rng.randint(1, 12)
    first = date(y, m, 1)
    nxt = date(y + (m == 12), m % 12 + 1, 1)
    add(f"workdays in {MONTHS[m - 1]} {y}", "workdays", workdays(first, nxt))
    add(f"work hours in {MONTHS[m - 1]} {y}", "hours", workdays(first, nxt) * 8)
    a, b = rand_date(1500, 2500), rand_date(1500, 2500)
    add(f"workdays from {literal(a)} to {literal(b)}", "workdays", workdays(a, b))

for _ in range(500):
    d = rand_date(1500, 2500)
    add(f"days until {literal(d)}", "days", (d - TODAY).days)
    add(f"days since {literal(d)}", "days", (TODAY - d).days)

for _ in range(500):
    d = rand_date(1500, 2500)
    n = rng.randint(0, 400)
    add(f"{literal(d)} + {n} workdays", "text", shown(add_workdays(d, n)))
    add(f"{literal(d)} - {n} workdays", "text", shown(add_workdays(d, -n)))

for y in range(1583, 2600):
    add(f"Easter {y}", "text", shown(easter(y)))
    add(f"Thanksgiving {y}", "text", shown(thanksgiving(y)))
    add(f"Black Friday {y}", "text", shown(thanksgiving(y) + timedelta(1)))
    add(f"Christmas {y}", "text", shown(date(y, 12, 25)))
    add(f"Halloween {y}", "text", shown(date(y, 10, 31)))

PIECES = ["yyyy", "yy", "MMMM", "MMM", "MM", "M", "dd", "d", "EEEE", "EEE"]
SEPS = [", ", " ", "/", "-", ". "]
for _ in range(700):
    d = rand_date(1100, 2900)
    chosen = rng.sample(PIECES, rng.randint(1, 4))
    pattern = ""
    for k, piece in enumerate(chosen):
        pattern += (rng.choice(SEPS) if k else "") + piece
    if not any(p in pattern for p in ("yy", "MM", "dd", "EEE")):
        continue
    add(f"{literal(d)} as {pattern}", "text", fmt(d, pattern))

def fmt_time(d, secs, pattern):
    """Like fmt, with the time letters and 'quoted text'."""
    h, m, sec = secs // 3600, secs % 3600 // 60, secs % 60
    out, i = [], 0
    while i < len(pattern):
        c = pattern[i]
        if c == "'":
            j = pattern.index("'", i + 1)
            out.append(pattern[i + 1:j])
            i = j + 1
            continue
        if not c.isalpha():
            out.append(c)
            i += 1
            continue
        run = next(k for k in range(1, len(pattern) - i + 1) if i + k == len(pattern) or pattern[i + k] != c)
        h12 = h % 12 or 12
        piece = {
            ("y", 4): str(d.year).zfill(4), ("y", 2): str(d.year % 100).zfill(2),
            ("M", 1): str(d.month), ("M", 2): f"{d.month:02}", ("M", 3): MONTHS[d.month - 1][:3], ("M", 4): MONTHS[d.month - 1],
            ("d", 1): str(d.day), ("d", 2): f"{d.day:02}",
            ("E", 3): DAYS[d.weekday()][:3], ("E", 4): DAYS[d.weekday()],
            ("H", 1): str(h), ("H", 2): f"{h:02}", ("h", 1): str(h12), ("h", 2): f"{h12:02}",
            ("m", 1): str(m), ("m", 2): f"{m:02}", ("s", 1): str(sec), ("s", 2): f"{sec:02}",
            ("a", 1): "AM" if h < 12 else "PM",
        }.get((c, run))
        if piece is None:
            raise KeyError((c, run))
        out.append(piece)
        i += run
    return "".join(out)


TIME_PIECES = ["HH", "H", "hh", "h", "mm", "m", "ss", "s", "a", "yyyy", "yy", "MMMM", "MMM", "MM", "M", "dd", "d", "EEEE", "EEE"]
WORDS = ["at", "on", "of", "the", "day", "time", "hour"]
for _ in range(2500):
    d = rand_date(1100, 2900)
    secs = rng.randrange(0, 86400)
    parts = []
    for k in range(rng.randint(2, 6)):
        piece = rng.choice(TIME_PIECES)
        if k:
            parts.append(rng.choice([":", " ", ", ", "-", "/"]))
        if rng.random() < 0.2:
            parts.append("'" + rng.choice(WORDS) + "'")
            parts.append(" ")
        parts.append(piece)
    pattern = "".join(parts)
    if not any(p in pattern for p in ("yy", "MM", "dd", "EEE", "HH", "hh")) and not (":" in pattern and any(c in pattern for c in "hHms")):
        continue
    h, m, sec = secs // 3600, secs % 3600 // 60, secs % 60
    clock_text = f"{h % 12 or 12}:{m:02}:{sec:02} {'am' if h < 12 else 'pm'}"
    add(f"{MONTHS[d.month - 1]} {d.day}, {d.year} {clock_text} as {pattern}", "text", fmt_time(d, secs, pattern))

weekday_names = {w: i for i, w in enumerate(DAYS)}
for name, idx in weekday_names.items():
    ahead = lambda n: (idx - TODAY.weekday()) % 7
    add(f"next {name}", "text", shown(TODAY + timedelta(ahead(0) or 7)))
    add(f"this {name}", "text", shown(TODAY + timedelta(ahead(0))))
    add(f"last {name}", "text", shown(TODAY - timedelta((TODAY.weekday() - idx) % 7 or 7)))

for n in range(0, 300):
    now = datetime(2024, 6, 15, 12, 0) + timedelta(days=n)
    add(f"{n} days from now", "text", shown(now.date(), "12:00 pm"))
    add(f"{n} days ago", "text", shown((datetime(2024, 6, 15, 12) - timedelta(days=n)).date(), "12:00 pm"))

for row in rows:
    print("\t".join(row))
