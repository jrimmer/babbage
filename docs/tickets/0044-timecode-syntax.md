# 0044 Timecode syntax beyond the basics

Status: mostly closed
Milestone: v1
Area: time

Built from a summary of the timecode notes: `01:02:03:04` literals (24 fps unless the line says `at 30 fps`), sums and differences, a length of time added to a timecode, `as frames`, `as timecode`, and frame arithmetic through the `frame` and `fps` units. Not covered by what was available, and not built: how a frame rate is given in a maintainer check's own syntax, a global `fps` variable, non-integer rates (23.976, 29.97), drop-frame timecode, and 25 fps PAL. Check a real install and the full page before adding any of them.

Built from the maintainer's notes note: `@` and `at N fps`, `N frames @ 24 fps` as a timecode, frames added to a timecode, a count of frames written with time units (`3h 2m 25s 10 frames at 24 fps`), three-digit frames (`00:00:00:010`), and the 12k-row timecode oracle. Not built: non-integer rates (23.976, 29.97), drop-frame, 25 fps as a named choice, a global `fps` variable (the page shows it only as a picture).
