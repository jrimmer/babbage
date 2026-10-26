#!/usr/bin/env python3
"""Lists zones whose UTC offsets differ between two tzdata zoneinfo folders, from 2020 to 2045.

Usage: tzdiff.py <old zoneinfo dir> <new zoneinfo dir>
"""
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path
from zoneinfo import ZoneInfo, ZoneInfoNotFoundError


def zones(root):
    names = set()
    for p in Path(root).rglob("*"):
        if p.is_file() and p.suffix == "" and p.name[0].isupper():
            names.add(str(p.relative_to(root)))
    return names


def zone(root, name):
    return ZoneInfo.from_file(open(Path(root) / name, "rb"), key=name)


def main(old, new):
    start = datetime(2020, 1, 1, tzinfo=timezone.utc)
    instants = [start + timedelta(days=7 * k) for k in range(0, 52 * 25)]
    a, b = zones(old), zones(new)
    for name in sorted(b - a):
        print(f"added   {name}")
    for name in sorted(a - b):
        print(f"removed {name}")
    for name in sorted(a & b):
        try:
            za, zb = zone(old, name), zone(new, name)
        except Exception:
            continue
        diffs = [t for t in instants if t.astimezone(za).utcoffset() != t.astimezone(zb).utcoffset()]
        if diffs:
            print(f"changed {name}: first {diffs[0].date()} last {diffs[-1].date()} ({len(diffs)} weeks)")


main(sys.argv[1], sys.argv[2])
