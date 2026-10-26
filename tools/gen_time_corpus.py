#!/usr/bin/env python3
"""Oracle for times of day, timespans, lap times, timestamps and ISO 8601, in exact Fractions.

The clock is fixed at Saturday 2024-06-15 12:00 UTC. Columns: expression <TAB> expected text <TAB> local UTC
offset in seconds.
"""
import calendar
import random
from datetime import date, datetime, timedelta, timezone
from decimal import Decimal
from fractions import Fraction

rng = random.Random(0x71E5)
TODAY = date(2024, 6, 15)
MONTHS = list(calendar.month_name)[1:]
OFFSETS = [0, 39600, -18000, 19800, -12600, 3600]
rows = []


def add(expr, expected, offset=0):
    rows.append((expr, str(expected), offset))


def terminates(q):
    d = q.denominator
    for p in (2, 5):
        while d % p == 0:
            d //= p
    return d == 1


def dec(q):
    """A terminating Fraction as plain decimal text."""
    if q.denominator == 1:
        return str(q.numerator)
    d = Decimal(q.numerator) / Decimal(q.denominator)
    return format(d.normalize(), "f")


def grouped(q):
    text = dec(q)
    whole, dot, frac = text.partition(".")
    sign = "-" if whole.startswith("-") else ""
    return sign + f"{int(whole.lstrip('-')):,}" + dot + frac


def seconds_text(whole, frac):
    if frac == 0:
        return str(whole)
    return str(whole) + dec(frac)[1:]


def timespan(total):
    neg = total < 0
    total = abs(total)
    whole = total.numerator // total.denominator
    frac = total - whole
    parts = []
    for n, name in ((whole // 604800, "week"), (whole % 604800 // 86400, "day"), (whole % 86400 // 3600, "hour"), (whole % 3600 // 60, "min")):
        if n:
            # weeks, days and hours are written out; minutes and seconds are `min` and `s`
            parts.append(f"{n} {name}{'' if n == 1 or name == 'min' else 's'}")
    secs = seconds_text(whole % 60, frac)
    if whole % 60 or frac or not parts:
        parts.append(f"{secs} s")
    return ("-" if neg else "") + " ".join(parts)


def laptime(total):
    neg = total < 0
    total = abs(total)
    whole = total.numerator // total.denominator
    frac = total - whole
    secs = seconds_text(whole % 60, frac)
    if len(secs.split(".")[0]) < 2:
        secs = "0" + secs
    return f"{'-' if neg else ''}{whole // 3600:02}:{whole % 3600 // 60:02}:{secs}"


def clock(sec):
    sec %= 86400
    h, m, s = sec // 3600, sec % 3600 // 60, sec % 60
    h12 = h % 12 or 12
    body = f"{h12}:{m:02}" + (f":{s:02}" if s else "")
    return f"{body} {'AM' if h < 12 else 'PM'}"


def parts(total, units):
    neg = total < 0
    rest = abs(total)
    out = []
    for i, (name, factor) in enumerate(units):
        last = i == len(units) - 1
        q = rest / factor
        n = q if last else Fraction(q.numerator // q.denominator)
        rest -= n * factor
        if n or (last and not out):
            out.append(f"{grouped(n)} {name}")
    return ("-" if neg else "") + " ".join(out)


def shown_date(d, secs=0):
    text = f"{MONTHS[d.month - 1]} {d.day}"
    if d.year != TODAY.year:
        text += f", {d.year}"
    if secs:
        h, m, s = secs // 3600, secs % 3600 // 60, secs % 60
        body = f"{h % 12 or 12}:{m:02}" + (f":{s:02}" if s else "")
        text += f" at {body} {'AM' if h < 12 else 'PM'}"
    return text


def clock_literal(sec):
    h, m = sec // 3600, sec % 3600 // 60
    style = rng.choice(["12", "12", "24", "bare"])
    if style == "24":
        return f"{h:02}:{m:02}"
    h12 = h % 12 or 12
    suffix = "am" if h < 12 else "pm"
    if style == "bare" and m == 0:
        return f"{h12}{suffix}"
    return f"{h12}:{m:02}{suffix}" if rng.random() < 0.5 else f"{h12}:{m:02} {suffix.upper()}"


UNITS = {"second": 1, "minute": 60, "hour": 3600, "day": 86400, "week": 604800}
SYMBOL = {"second": "s", "minute": "min", "hour": "h", "day": "day", "week": "wk"}


def amount():
    whole = rng.randint(0, 200)
    places = rng.choice([0, 1, 2, 3])
    return Fraction(whole * 10**places + rng.randint(0, 10**places - 1), 10**places) if places else Fraction(whole)


def amount_text(q):
    return dec(q)


for _ in range(1500):
    a, b = rng.randrange(0, 86400, 60), rng.randrange(0, 86400, 60)
    la, lb = clock_literal(a), clock_literal(b)
    add(f"{la} to {lb}", timespan(Fraction((b - a) % 86400 or 0)) if a != b else "0 s")
    add(f"{la} - {lb}", timespan(Fraction(abs(a - b))) if a != b else "0 s")
    add(f"{la}", clock(a))
    minutes = rng.randint(0, 3000)
    add(f"{la} + {minutes} minutes", clock(a + minutes * 60))
    add(f"{la} - {minutes} minutes", clock(a - minutes * 60))
    hours = rng.randint(0, 40)
    add(f"{la} + {hours} hours {minutes % 60} minutes", clock(a + hours * 3600 + (minutes % 60) * 60))

for _ in range(2500):
    def lap():
        whole = rng.randint(0, 99 * 3600)
        frac = Fraction(rng.randint(0, 999), 1000) if rng.random() < 0.4 else Fraction(0)
        return Fraction(whole) + frac

    def lap_text(q):
        return laptime(q) if q >= 0 else None
    x, y = lap(), lap()
    add(f"{laptime(x)} + {laptime(y)}", laptime(x + y))
    add(f"{laptime(x)} - {laptime(y)}", laptime(x - y))
    add(f"{laptime(x)} as timespan", timespan(x))

for _ in range(3000):
    unit = rng.choice(list(UNITS))
    q = amount()
    seconds = q * UNITS[unit]
    word = unit + ("" if q == 1 else "s")
    add(f"{amount_text(q)} {word} as timespan", timespan(seconds))
    add(f"{amount_text(q)} {word} as laptime", laptime(seconds))

for _ in range(3000):
    chosen = sorted(rng.sample(list(UNITS), rng.choice([2, 3])), key=lambda u: -UNITS[u])
    src = rng.choice(list(UNITS))
    q = amount()
    seconds = q * UNITS[src]
    last = UNITS[chosen[-1]]
    if not terminates(seconds / last):
        continue
    word = src + ("" if q == 1 else "s")
    names = " and ".join(u + "s" for u in chosen)
    add(f"{amount_text(q)} {word} in {names}", parts(seconds, [(SYMBOL[u], Fraction(UNITS[u])) for u in chosen]))

for _ in range(1500):
    h, m = rng.randint(0, 3), rng.randint(0, 59)
    secs = h * 3600 + m * 60
    speed = rng.choice([Fraction(3, 2), Fraction(5, 4), Fraction(2), Fraction(1, 2), Fraction(5, 2), Fraction(3, 4), Fraction(4)])
    if secs == 0:
        continue
    played = Fraction(secs) / speed
    if not terminates(played):
        continue
    text = f"{h} hours {m} minutes" if h else f"{m} minutes"
    sp = dec(speed)
    add(f"{text} at {sp}x", timespan(played))
    saved = Fraction(secs) - played
    if saved != 0:
        add(f"time saved {text} at {sp}x", timespan(saved))

def timecode(frames, rate):
    neg = frames < 0
    frames = abs(frames)
    secs, frame = divmod(frames, rate)
    return f"{'-' if neg else ''}{secs // 3600:02}:{secs % 3600 // 60:02}:{secs % 60:02}:{frame:02}"


def tc_literal(frames, rate):
    return timecode(frames, rate)


RATES = [24, 25, 30, 48, 50, 60]
for _ in range(3000):
    rate = rng.choice(RATES)
    suffix = "" if rate == 24 and rng.random() < 0.5 else f" at {rate} fps"
    a, b = rng.randint(0, 40 * 3600 * rate), rng.randint(0, 40 * 3600 * rate)
    ta, tb = tc_literal(a, rate), tc_literal(b, rate)
    add(f"{ta}{suffix}", timecode(a, rate))
    add(f"{ta} + {tb}{suffix}", timecode(a + b, rate))
    add(f"{ta} - {tb}{suffix}", timecode(a - b, rate))
    secs = rng.randint(0, 3600)
    add(f"{ta} + {secs} seconds{suffix}", timecode(a + secs * rate, rate))
    small = rng.randint(0, 30 * 60 * rate)
    if small < 100_000:
        add(f"{tc_literal(small, rate)}{suffix} as frames" if suffix else f"{tc_literal(small, 24)} as frames", f"{small:,} frame" if suffix else f"{small:,} frame")
        add(f"{small} frames at {rate} fps as timecode", timecode(small, rate))
    d = rng.randint(1, 600)
    if rate * d < 100_000:
        add(f"{rate} fps * {d} seconds", f"{rate * d:,} frame")

EPOCH = datetime(1970, 1, 1, tzinfo=timezone.utc)
for offset in OFFSETS:
    tz = timezone(timedelta(seconds=offset))
    for _ in range(800):
        d = date(rng.randint(1971, 2400), rng.randint(1, 12), rng.randint(1, 28))
        secs = rng.randrange(0, 86400)
        local = datetime(d.year, d.month, d.day, tzinfo=tz) + timedelta(seconds=secs)
        stamp = int((local - EPOCH).total_seconds())
        add(f"{MONTHS[d.month - 1]} {d.day}, {d.year} to timestamp", int((datetime(d.year, d.month, d.day, tzinfo=tz) - EPOCH).total_seconds()), offset)
        local_view = local
        add(f"{stamp} to date", shown_date(local_view.date(), local_view.hour * 3600 + local_view.minute * 60 + local_view.second), offset)
        if stamp * 1000 >= 10**11:
            add(f"{stamp * 1000} to date", shown_date(local_view.date(), local_view.hour * 3600 + local_view.minute * 60 + local_view.second), offset)
        zone = "Z" if offset == 0 else f"{'+' if offset > 0 else '-'}{abs(offset) // 3600:02}:{abs(offset) % 3600 // 60:02}"
        iso = f"{local.year:04}-{local.month:02}-{local.day:02}T{local.hour:02}:{local.minute:02}:{local.second:02}{zone}"
        add(f"{MONTHS[d.month - 1]} {d.day}, {d.year} {clock_literal(0)} to timestamp", int((datetime(d.year, d.month, d.day, tzinfo=tz) - EPOCH).total_seconds()), offset)
        add(f"{iso} as iso8601", iso, offset)
        add(f"{iso} to date", shown_date(local.date(), local.hour * 3600 + local.minute * 60 + local.second), offset)
        elsewhere = local.astimezone(timezone(timedelta(seconds=rng.choice(OFFSETS))))
        z2 = elsewhere.utcoffset().total_seconds()
        zone2 = "Z" if z2 == 0 else f"{'+' if z2 > 0 else '-'}{abs(int(z2)) // 3600:02}:{abs(int(z2)) % 3600 // 60:02}"
        iso2 = f"{elsewhere.year:04}-{elsewhere.month:02}-{elsewhere.day:02}T{elsewhere.hour:02}:{elsewhere.minute:02}:{elsewhere.second:02}{zone2}"
        add(f"{iso2} as iso8601", iso, offset)

for row in rows:
    print("\t".join(map(str, row)))
