#!/usr/bin/env python3
"""Oracle for ISO 8601 literals: Python datetime does the calendar and zone arithmetic.

Columns: expression <TAB> expected. Valid literals are written back in UTC with `as iso8601`; a date that is not one is the subtraction of its three numbers; other malformed tails expect ERR.
"""
import random
from datetime import datetime, timedelta, timezone

rng = random.Random(0x150)
rows = []


def utc_text(dt):
    dt = dt.astimezone(timezone.utc)
    frac = f"{dt.microsecond:06d}".rstrip("0")
    return f"{dt.year:04}-{dt.month:02}-{dt.day:02}T{dt.hour:02}:{dt.minute:02}:{dt.second:02}" + (f".{frac}" if frac else "") + "Z"


def zone_text(offset_minutes, style):
    if style == "none":
        return "", 0
    if style == "Z":
        return rng.choice(["Z", "z"]), 0
    sign = "+" if offset_minutes >= 0 else "-"
    h, m = divmod(abs(offset_minutes), 60)
    return (f"{sign}{h:02}:{m:02}" if style == "colon" else f"{sign}{h:02}{m:02}"), offset_minutes


for _ in range(6000):
    y, mo, d = rng.randint(1000, 2999), rng.randint(1, 12), rng.randint(1, 28)
    form = rng.choice(["date", "minute", "second", "fraction"])
    h, mi, s = rng.randint(0, 23), rng.randint(0, 59), rng.randint(0, 59)
    if form in ("date", "minute"):
        s = 0
    micro = rng.randint(1, 999999) if form == "fraction" else 0
    text = f"{y:04}-{mo:02}-{d:02}"
    tz = timezone.utc
    if form != "date":
        text += rng.choice(["T", "T", "t"]) + f"{h:02}:{mi:02}"
        if form in ("second", "fraction"):
            text += f":{s:02}"
        if form == "fraction":
            digits = f"{micro:06d}".rstrip("0")
            text += "." + digits
        style = rng.choice(["none", "Z", "colon", "compact"])
        offset = rng.choice([0, 60, 330, 345, 570, 720, 840, 1380, 1439, 59, -59, -1380, -1439, -60, -300, -570, -720])
        ztext, off = zone_text(offset, style)
        text += ztext
        tz = timezone(timedelta(minutes=off))
    else:
        h = mi = s = 0
    dt = datetime(y, mo, d, h, mi, s, micro, tzinfo=tz)
    rows.append((f"{text} as iso8601", utc_text(dt)))
    if rng.random() < 0.3:
        # The same literal away from the start of the line.
        rows.append((f"({text}) as iso8601", utc_text(dt)))
        rows.append((f"  {text} as iso8601", utc_text(dt)))
        rows.append((f"0 days + {text} as iso8601", utc_text(dt)))

BAD_DATES = ["2019-00-10", "2019-13-01", "2019-02-30", "2019-04-31", "2019-06-00", "2019-06-32", "2021-02-29", "1900-02-29"]
for date in BAD_DATES:
    # Text shaped like a date that is not one is arithmetic in the expected behaviour: `2019-02-30` is 2019 - 02 - 30.
    y, mo, d = (int(p) for p in date.split("-"))
    rows.append((date, f"{y - mo - d:,}"))
    rows.append((f"{date}T10:00", "ERR"))
for tail in ["T24:00", "T12:60", "T12:00:60", "T25:00", "T99:99", "T12:00+24:00", "T12:00+12:60", "T12:00+1260", "T12:00:00Zx", "T12:00:00.5Zx", "T12:00x", "T12:0", "T1200", "T12:00:", "x"]:
    rows.append((f"2019-04-01{tail}", "ERR"))
rows.append(("2020-02-29 as iso8601", "2020-02-29T00:00:00Z"))
rows.append(("2000-02-29T23:59:59 as iso8601", "2000-02-29T23:59:59Z"))
rows.append(("2019-04-01T24:00 as iso8601", "ERR"))

for r in rows:
    print("\t".join(r))
