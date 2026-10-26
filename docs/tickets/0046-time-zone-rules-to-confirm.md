# 0046 Time zone choices to confirm, and data to extend

Status: open
Milestone: v1
Area: time zones

Choices made without a real install to check:
- A time of day written in a zone means that time today in that zone. If it lands on a different date in the target zone, the answer is a dated time (`June 14 at 6:00 pm`) and not a bare time of day.
- `time difference between A and B` is how far ahead A is of B (`-8 hours` for PST and GMT).
- A skipped local time (clocks jump forward) uses the offset from before the change, and a repeated one takes its first reading.
- Abbreviations are fixed offsets for the whole year (`PST` is UTC-8 even in summer). `IST`, `BST` and a China `CST` are left out because each names more than one zone.
- A country means its capital's zone, for about 110 common countries; the rest are refused.

Data to extend after v1: the full country list, the large city dataset and airport codes (ticket 0010). The IANA database is bundled with the `chrono-tz` crate (2025b at the time of writing), so rule changes after that release need a dependency update.

The bundled database is out of date for two places; see ticket 0053.
