#!/usr/bin/env python3
"""Oracle for zone rules read from zoneinfo files: Python reads the same vendored TZif files (tzdata 2026e).

Columns: local zone (IANA name or -) <TAB> clock (UTC seconds) <TAB> expression <TAB> expected text.
Focus is on dates around the rule changes since 2025b: Morocco from 2026-09-23, western Canada from 2026-11-04.
"""
import calendar
import random
from datetime import date, datetime, timedelta, timezone
from pathlib import Path
from zoneinfo import ZoneInfo

ROOT = Path(__file__).resolve().parent.parent / "crates/babbage-core/tests/fixtures/zoneinfo"
PLACES = {
    "Casablanca": "Africa/Casablanca", "Vancouver": "America/Vancouver", "Edmonton": "America/Edmonton",
    "New York": "America/New_York", "Los Angeles": "America/Los_Angeles", "London": "Europe/London",
    "Berlin": "Europe/Berlin", "Sydney": "Australia/Sydney", "Tokyo": "Asia/Tokyo", "Kolkata": "Asia/Kolkata",
    "Auckland": "Pacific/Auckland", "Sao Paulo": "America/Sao_Paulo", "Lord Howe": "Australia/Lord_Howe",
}
ZONES = {name: ZoneInfo.from_file(open(ROOT / iana, "rb"), key=iana) for name, iana in PLACES.items()}
MONTHS = list(calendar.month_name)[1:]
NOW = datetime(2026, 12, 1, 12, 0, tzinfo=timezone.utc)
rng = random.Random(0x2026E)
rows = []


def clock_text(sec):
    h, m = sec // 3600, sec % 3600 // 60
    return f"{h % 12 or 12}:{m:02} {'AM' if h < 12 else 'PM'}"


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
        return dated(d, clock_text(secs), today or NOW.date())
    return f"{MONTHS[d.month - 1]} {d.day}" + ("" if d.year == NOW.year else f", {d.year}")


def literal(d, secs):
    h, m = secs // 3600, secs % 3600 // 60
    return f"{MONTHS[d.month - 1]} {d.day}, {d.year} {h % 12 or 12}:{m:02}{'am' if h < 12 else 'pm'}"


def once(naive, zone):
    a, b = naive.replace(tzinfo=zone, fold=0), naive.replace(tzinfo=zone, fold=1)
    return a.utcoffset() == b.utcoffset() and a.astimezone(timezone.utc).astimezone(zone).replace(tzinfo=None) == naive


def add(zone, expr, expected):
    rows.append((zone, int(NOW.timestamp()), expr, expected))


names = list(PLACES)
for _ in range(4000):
    a, b = rng.sample(names, 2)
    if rng.random() < 0.6:
        d = date(2026, 1, 1) + timedelta(days=rng.randint(200, 420))
    else:
        d = date(rng.randint(2020, 2040), rng.randint(1, 12), rng.randint(1, 28))
    secs = rng.randrange(60, 86400, 60)
    naive = datetime(d.year, d.month, d.day) + timedelta(seconds=secs)
    if not once(naive, ZONES[a]):
        continue
    there = naive.replace(tzinfo=ZONES[a]).astimezone(ZONES[b])
    add("-", f"{literal(d, secs)} {a} in {b}", shown(there.date(), there.hour * 3600 + there.minute * 60))

for name in names:
    z = ZONES[name]
    here = NOW.astimezone(z)
    tod = here.hour * 3600 + here.minute * 60
    add(PLACES[name], "now", shown(here.date(), tod, here.date()))
    add("-", f"time in {name}", clock_text(tod))
    for _ in range(150):
        d = date(2026, 1, 1) + timedelta(days=rng.randint(200, 420))
        secs = rng.randrange(60, 86400, 60)
        naive = datetime(d.year, d.month, d.day) + timedelta(seconds=secs)
        if not once(naive, z):
            continue
        stamp = int((naive.replace(tzinfo=z) - datetime(1970, 1, 1, tzinfo=timezone.utc)).total_seconds())
        add(PLACES[name], f"{literal(d, secs)} to timestamp", stamp)
        add(PLACES[name], f"{stamp} to date", shown(d, secs, here.date()))

for row in rows:
    print("\t".join(map(str, row)))
