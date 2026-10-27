# 0023 Confirm how subtotals, totals and tags are typed

Status: closed (settled by a maintainer check, one deliberate difference)
Milestone: v1
Area: sheets

The maintainer's notes describes subtotals as a UI action on a blank line and never gives typed text. The engine reads a line `subtotal`, `name =` with nothing after it, and `total of #tag`, `sum of`, `average of`, `avg of`, `count of`. The home page also shows `#jude total`. Check each against a real install and align.

A maintainer check (2026-10-26): `total of #jude` and `#jude total` both give `$12.00`; built. A typed `subtotal` line shows nothing in a maintainer check, but Babbage keeps it: the app's Make Subtotal Line writes it, so it stays (deliberate).
