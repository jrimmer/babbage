# 0081 `now as HH:mm` in a sheet is `1 mm`

Status: closed
Milestone: v1
Area: sheets, dates

In a one-line evaluation `now as HH:mm` is `12:00` and `now as HH:mm:ss` is `12:00:00`, but in a sheet the colon is read as the end of a label (`now as HH: mm`), so the line prints `1 mm` and `now as HH:mm:ss` is an error. `today as EEEE` and the other patterns without a colon are fine. This is a quiet wrong answer in the app for a form the dates notes documents (`as` with a format). Found while writing Sheet D of the owner checks; not yet checked against the expected behaviour (the sheet asks `now as HH:mm:ss`).

Built: a colon after `as` belongs to the date pattern, so `now as HH:mm` is `13:11` in a sheet and `now as HH:mm:ss` shows seconds. Pinned in the tests. Compared with the expected behaviour (Sheet D lines 7 and 8): `now as HH:mm` is `14:53` and `now as HH:mm:ss` is `14:53:00`, so `now` is read to the minute. Matched.
