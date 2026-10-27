#!/usr/bin/env python3
"""Oracle for the user's own time zone with daylight saving: zoneinfo on the pinned tzdata.

The clock is fixed at Saturday 2024-06-15 12:00 UTC. Columns: IANA zone <TAB> expression <TAB> expected.
"""
import calendar
import random
import zoneinfo
from datetime import date, datetime, timedelta, timezone

zoneinfo.reset_tzpath(to=[])
from zoneinfo import ZoneInfo

import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from unitwords import zone_label, offset_label

rng = random.Random(0x10CA1)
MONTHS = list(calendar.month_name)[1:]
ZONES = ["America/New_York", "America/Los_Angeles", "Europe/London", "Australia/Sydney", "Asia/Kolkata",
         "Asia/Tokyo", "Pacific/Auckland", "America/Sao_Paulo", "Europe/Berlin", "Australia/Lord_Howe"]
EPOCH = datetime(1970, 1, 1, tzinfo=timezone.utc)
rows = []


def clock_text(sec):
    h, m, s = sec // 3600, sec % 3600 // 60, sec % 60
    body = f"{h % 12 or 12}:{m:02}" + (f":{s:02}" if s else "")
    return f"{body} {'AM' if h < 12 else 'PM'}"


def dated(d, clock, today):
    """A date with a time: just the time today, Yesterday or Tomorrow a day either side, else the full date."""
    if d == today:
        return clock
    if d == today - timedelta(1):
        return f"Yesterday at {clock}"
    if d == today + timedelta(1):
        return f"Tomorrow at {clock}"
    return f"{MONTHS[d.month - 1]} {d.day}, {d.year} at {clock}"


def shown(d, secs=0, today=None):
    if secs:
        return dated(d, clock_text(secs), today or date(2024, 6, 15))
    return f"{MONTHS[d.month - 1]} {d.day}" + ("" if d.year == 2024 else f", {d.year}")


def clock_literal(sec):
    h, m = sec // 3600, sec % 3600 // 60
    return f"{h % 12 or 12}:{m:02}{'am' if h < 12 else 'pm'}"


def once(naive, zone):
    a, b = naive.replace(tzinfo=zone, fold=0), naive.replace(tzinfo=zone, fold=1)
    return a.utcoffset() == b.utcoffset() and a.astimezone(timezone.utc).astimezone(zone).replace(tzinfo=None) == naive


for name in ZONES:
    zone = ZoneInfo(name)
    today_here = datetime(2024, 6, 15, 12, tzinfo=timezone.utc).astimezone(zone).date()
    for _ in range(900):
        d = date(rng.randint(1995, 2035), rng.randint(1, 12), rng.randint(1, 28))
        secs = rng.randrange(60, 86400, 60)
        naive = datetime(d.year, d.month, d.day) + timedelta(seconds=secs)
        if not once(naive, zone):
            continue
        aware = naive.replace(tzinfo=zone)
        stamp = int((aware - EPOCH).total_seconds())
        lit = f"{MONTHS[d.month - 1]} {d.day}, {d.year} {clock_literal(secs)}"
        rows.append((name, f"{lit} to timestamp", stamp))
        off = int(aware.utcoffset().total_seconds())
        z = "Z" if off == 0 else f"{'+' if off > 0 else '-'}{abs(off) // 3600:02}:{abs(off) % 3600 // 60:02}"
        rows.append((name, f"{lit} as iso8601", f"{d.year:04}-{d.month:02}-{d.day:02}T{secs // 3600:02}:{secs % 3600 // 60:02}:00{z}"))
        rows.append((name, f"{stamp} to date", shown(d, secs, today_here) + " " + zone_label(aware)))
        utc = aware.astimezone(timezone.utc)
        rows.append((name, f"{utc.year:04}-{utc.month:02}-{utc.day:02}T{utc.hour:02}:{utc.minute:02}:00Z to date", shown(d, secs, today_here)))
    here = datetime(2024, 6, 15, 12, tzinfo=timezone.utc).astimezone(zone)
    tod = here.hour * 3600 + here.minute * 60
    rows.append((name, "now", shown(here.date(), tod, here.date())))
    rows.append((name, "today", shown(here.date())))
    rows.append((name, "current timestamp", 1718452800))

for row in rows:
    print("\t".join(map(str, row)))
