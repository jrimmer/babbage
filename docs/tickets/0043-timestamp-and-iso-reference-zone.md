# 0043 Zone behind the reference timestamp examples

Status: half closed (the reference app)
Milestone: v1
Area: dates

The reference lists `April 1, 2019` to timestamp as `1554109200` and `April 1, 2019 3:30pm` as ISO `2019-04-01T15:30:00+11:00`. 1554109200 is 09:00 UTC, not midnight in any zone, so the example was probably typed with a time of day and in the author's zone (UTC+11). The engine takes midnight local for a date with no time and reads the offset from the local UTC offset setting (UTC by default); it writes `Z` for a zero offset. Check a real install for how a bare date maps to a timestamp, and whether a zero offset prints `Z` or `+00:00`.

The reference app: a bare date to timestamp is noon local (ticket 0070) and ISO output carries the local offset (`-07:00`, which the engine already writes).
