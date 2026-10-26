# 0070 A date with no time is noon in the local zone for timestamps and ISO

Status: open
Milestone: v1
Area: dates

The reference app prints `April 1, 2019 to timestamp` as `1554145200`, which is 12:00 in Los Angeles (19:00 UTC), where the engine takes midnight local (`1554102000`). It matches the reference page's remark that intervals run from midday on the first date. ISO output carries the local offset (`2019-04-01T15:30:00-07:00`), which the engine already does. Applies to `to timestamp`, `as iso8601` and `to date` of a bare date.
