#!/usr/bin/env python3
"""Oracle for time zones: Python's zoneinfo reading the pip tzdata pinned to the IANA version the engine bundles.

The clock is fixed at Saturday 2024-06-15 12:00 UTC. Columns: expression <TAB> expected text <TAB> local UTC
offset in seconds.
"""
import calendar
import random
import zoneinfo
from datetime import date, datetime, timedelta, timezone

zoneinfo.reset_tzpath(to=[])
from zoneinfo import ZoneInfo

rng = random.Random(0x20E5)
NOW = datetime(2024, 6, 15, 12, 0, tzinfo=timezone.utc)
TODAY_YEAR = 2024
MONTHS = list(calendar.month_name)[1:]
OFFSETS = [0, 39600, -18000]

CITIES = {
    "Sydney": "Australia/Sydney", "New York": "America/New_York", "Chicago": "America/Chicago",
    "Los Angeles": "America/Los_Angeles", "Denver": "America/Denver", "Phoenix": "America/Phoenix",
    "Anchorage": "America/Anchorage", "Honolulu": "Pacific/Honolulu", "Toronto": "America/Toronto",
    "Vancouver": "America/Vancouver", "Mexico City": "America/Mexico_City", "Sao Paulo": "America/Sao_Paulo",
    "Buenos Aires": "America/Argentina/Buenos_Aires", "Lima": "America/Lima", "Bogota": "America/Bogota",
    "Santiago": "America/Santiago", "London": "Europe/London", "Dublin": "Europe/Dublin",
    "Lisbon": "Europe/Lisbon", "Paris": "Europe/Paris", "Berlin": "Europe/Berlin", "Madrid": "Europe/Madrid",
    "Rome": "Europe/Rome", "Amsterdam": "Europe/Amsterdam", "Athens": "Europe/Athens",
    "Istanbul": "Europe/Istanbul", "Moscow": "Europe/Moscow", "Cairo": "Africa/Cairo",
    "Johannesburg": "Africa/Johannesburg", "Lagos": "Africa/Lagos", "Nairobi": "Africa/Nairobi",
    "Dubai": "Asia/Dubai", "Tehran": "Asia/Tehran", "Karachi": "Asia/Karachi", "Kolkata": "Asia/Kolkata",
    "Dhaka": "Asia/Dhaka", "Bangkok": "Asia/Bangkok", "Jakarta": "Asia/Jakarta", "Singapore": "Asia/Singapore",
    "Hong Kong": "Asia/Hong_Kong", "Shanghai": "Asia/Shanghai", "Tokyo": "Asia/Tokyo", "Seoul": "Asia/Seoul",
    "Manila": "Asia/Manila", "Perth": "Australia/Perth", "Adelaide": "Australia/Adelaide",
    "Brisbane": "Australia/Brisbane", "Melbourne": "Australia/Melbourne", "Auckland": "Pacific/Auckland",
    "Kathmandu": "Asia/Kathmandu", "Kabul": "Asia/Kabul", "Colombo": "Asia/Colombo",
    "Chatham": "Pacific/Chatham", "Kiritimati": "Pacific/Kiritimati", "Pago Pago": "Pacific/Pago_Pago",
    "St Johns": "America/St_Johns", "Reykjavik": "Atlantic/Reykjavik", "Casablanca": "Africa/Casablanca",
    "Adak": "America/Adak", "Lord Howe": "Australia/Lord_Howe",
}
ALIASES = {
    "San Francisco": "America/Los_Angeles", "Seattle": "America/Los_Angeles", "Boston": "America/New_York",
    "Beijing": "Asia/Shanghai", "Mumbai": "Asia/Kolkata", "Munich": "Europe/Berlin",
    "Manchester": "Europe/London", "Cape Town": "Africa/Johannesburg", "Montreal": "America/Toronto",
    "Houston": "America/Chicago", "Osaka": "Asia/Tokyo", "Canberra": "Australia/Sydney",
}
COUNTRIES = {
    "Germany": "Europe/Berlin", "France": "Europe/Paris", "Japan": "Asia/Tokyo", "India": "Asia/Kolkata",
    "Brazil": "America/Sao_Paulo", "Australia": "Australia/Sydney", "Canada": "America/Toronto",
    "USA": "America/New_York", "United Kingdom": "Europe/London", "China": "Asia/Shanghai",
    "Mexico": "America/Mexico_City", "Egypt": "Africa/Cairo", "New Zealand": "Pacific/Auckland",
    "South Africa": "Africa/Johannesburg", "Russia": "Europe/Moscow", "Argentina": "America/Argentina/Buenos_Aires",
}
ABBREVIATIONS = {
    "EST": -5, "EDT": -4, "CST": -6, "CDT": -5, "MST": -7, "MDT": -6, "PST": -8, "PDT": -7,
    "AKST": -9, "AKDT": -8, "HST": -10, "GMT": 0, "UTC": 0, "UT": 0, "Z": 0, "WET": 0, "WEST": 1,
    "CET": 1, "CEST": 2, "EET": 2, "EEST": 3, "MSK": 3,
    "JST": 9, "KST": 9, "AWST": 8, "AEST": 10, "AEDT": 11, "NZST": 12, "NZDT": 13,
}
REGIONS = {"eastern time": "America/New_York", "central time": "America/Chicago", "mountain time": "America/Denver",
           "pacific time": "America/Los_Angeles", "alaska time": "America/Anchorage", "hawaii time": "Pacific/Honolulu"}

PLACES = {}
for name, iana in {**CITIES, **ALIASES, **COUNTRIES, **{k.title(): v for k, v in REGIONS.items()}}.items():
    PLACES[name] = ZoneInfo(iana)
for abbr, hours in ABBREVIATIONS.items():
    PLACES[abbr] = timezone(timedelta(hours=hours))

rows = []


def add(expr, expected, offset=0):
    rows.append((expr, str(expected), offset))


def spell(name):
    style = rng.choice(["asis", "lower", "upper"])
    return {"asis": name, "lower": name.lower(), "upper": name.upper()}[style]


def clock_text(sec):
    h, m, s = sec // 3600, sec % 3600 // 60, sec % 60
    body = f"{h % 12 or 12}:{m:02}" + (f":{s:02}" if s else "")
    return f"{body} {'AM' if h < 12 else 'PM'}"


def dated(d, clock, today):
    """A date with a time: Yesterday or Tomorrow a day either side of today, else the full date with its year."""
    if d == today - timedelta(1):
        return f"Yesterday at {clock}"
    if d == today + timedelta(1):
        return f"Tomorrow at {clock}"
    return f"{MONTHS[d.month - 1]} {d.day}, {d.year} at {clock}"


def shown_date(d, secs=0, today=None, timed=False):
    """`timed` marks an answer that came from a time of day, so midnight still prints as a time."""
    if secs or timed:
        return dated(d, clock_text(secs), today or NOW.date())
    text = f"{MONTHS[d.month - 1]} {d.day}"
    if d.year != TODAY_YEAR:
        text += f", {d.year}"
    return text


def clock_literal(sec):
    h, m = sec // 3600, sec % 3600 // 60
    h12, suffix = h % 12 or 12, "am" if h < 12 else "pm"
    return f"{h12}:{m:02}{suffix}" if m else f"{h12}{suffix}"


def exists_once(naive, zone):
    """True when this wall-clock reading happens exactly once in the zone."""
    if isinstance(zone, timezone):
        return True
    a = naive.replace(tzinfo=zone, fold=0)
    b = naive.replace(tzinfo=zone, fold=1)
    if a.utcoffset() != b.utcoffset():
        return False
    return a.astimezone(timezone.utc).astimezone(zone).replace(tzinfo=None) == naive


names = list(PLACES)

# a time of day, today
for _ in range(2500):
    a, b = rng.sample(names, 2)
    secs = rng.randrange(0, 86400, 60)
    za, zb = PLACES[a], PLACES[b]
    source_day = NOW.astimezone(za).date()
    naive = datetime(source_day.year, source_day.month, source_day.day) + timedelta(seconds=secs)
    if not exists_once(naive, za):
        continue
    there = naive.replace(tzinfo=za).astimezone(zb)
    if there.date() == source_day:
        want = clock_text(there.hour * 3600 + there.minute * 60 + there.second)
    else:
        want = shown_date(there.date(), there.hour * 3600 + there.minute * 60 + there.second, timed=True)
    connector = rng.choice(["in", "to"])
    add(f"{clock_literal(secs)} {spell(a)} {connector} {spell(b)}", want)

# a date and time, any year
for _ in range(2500):
    a, b = rng.sample(names, 2)
    za, zb = PLACES[a], PLACES[b]
    d = date(rng.randint(1995, 2035), rng.randint(1, 12), rng.randint(1, 28))
    secs = rng.randrange(60, 86400, 60)
    naive = datetime(d.year, d.month, d.day) + timedelta(seconds=secs)
    if not exists_once(naive, za):
        continue
    there = naive.replace(tzinfo=za).astimezone(zb)
    tod = there.hour * 3600 + there.minute * 60 + there.second
    add(f"{MONTHS[d.month - 1]} {d.day}, {d.year} {clock_literal(secs)} {spell(a)} in {spell(b)}", shown_date(there.date(), tod, timed=True))

# the current time and date in a place
for name in names:
    z = PLACES[name]
    here = NOW.astimezone(z)
    tod = here.hour * 3600 + here.minute * 60
    here_text = clock_text(tod) if here.date() == NOW.date() else dated(here.date(), clock_text(tod), NOW.date())
    add(f"time in {spell(name)}", here_text)
    add(f"{spell(name)} time", here_text) if " " not in name and name.upper() not in ABBREVIATIONS else None
    add(f"date in {spell(name)}", shown_date(here.date()))
    add(f"now in {spell(name)}", shown_date(here.date(), tod, timed=True))

# differences
def span_hours(delta):
    secs = int(delta.total_seconds())
    sign = "-" if secs < 0 else ""
    secs = abs(secs)
    parts = []
    for n, name in ((secs // 604800, "week"), (secs % 604800 // 86400, "day"), (secs % 86400 // 3600, "hour"), (secs % 3600 // 60, "min")):
        if n:
            parts.append(f"{n} {name}{'' if n == 1 or name == 'min' else 's'}")
    if secs % 60 or not parts:
        parts.append(f"{secs % 60} s")
    return sign + " ".join(parts)


for _ in range(800):
    a, b = rng.sample(names, 2)
    off = lambda z: NOW.astimezone(z).utcoffset()
    want = span_hours(off(PLACES[b]) - off(PLACES[a]))  # the second place minus the first, as the reference app prints it
    connector = rng.choice(["and", "&"])
    add(f"time difference between {spell(a)} {connector} {spell(b)}", want)

# offsets written GMT+N and UTC-N:MM
for _ in range(600):
    sign = rng.choice([1, -1])
    h, m = rng.randint(0, 14), rng.choice([0, 0, 30, 45])
    b = rng.choice(names)
    secs = rng.randrange(0, 86400, 60)
    zone = timezone(sign * timedelta(hours=h, minutes=m))
    source_day = NOW.astimezone(zone).date()
    naive = datetime(source_day.year, source_day.month, source_day.day) + timedelta(seconds=secs)
    there = naive.replace(tzinfo=zone).astimezone(PLACES[b])
    tod = there.hour * 3600 + there.minute * 60 + there.second
    want = clock_text(tod) if there.date() == source_day else shown_date(there.date(), tod, timed=True)
    text = f"{h}" if m == 0 else f"{h}:{m:02}"
    add(f"{clock_literal(secs)} {rng.choice(['GMT', 'UTC'])}{'+' if sign > 0 else '-'}{text} in {spell(b)}", want)

# a non-zero local zone changes only a time written with no zone
for offset in OFFSETS[1:]:
    local = timezone(timedelta(seconds=offset))
    for _ in range(600):
        b = rng.choice(names)
        d = date(rng.randint(1995, 2035), rng.randint(1, 12), rng.randint(1, 28))
        secs = rng.randrange(60, 86400, 60)
        naive = datetime(d.year, d.month, d.day) + timedelta(seconds=secs)
        there = naive.replace(tzinfo=local).astimezone(PLACES[b])
        tod = there.hour * 3600 + there.minute * 60 + there.second
        add(f"{MONTHS[d.month - 1]} {d.day}, {d.year} {clock_literal(secs)} in {spell(b)}", shown_date(there.date(), tod, (NOW + timedelta(seconds=offset)).date(), timed=True), offset)

for row in rows:
    print("\t".join(map(str, row)))
