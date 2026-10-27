# 0070 A date with no time is noon in the local zone for timestamps and ISO

Status: closed
Milestone: v1
Area: dates

The expected answer is `April 1, 2019 to timestamp` as `1554145200`, which is 12:00 in Los Angeles (19:00 UTC), where the engine takes midnight local (`1554102000`). It matches the maintainer's page's remark that intervals run from midday on the first date. ISO output carries the local offset (`2019-04-01T15:30:00-07:00`), which the engine already does. Applies to `to timestamp`, `as iso8601` and `to date` of a bare date.

Fixed: a date with no time converts to a timestamp at midday local (`April 1, 2019 to timestamp` is `1554145200` in Los Angeles); a time that is explicitly midnight stays midnight (`12:00am`, marked on the date). ISO output of a bare date is unchanged (not shown by a maintainer check).
