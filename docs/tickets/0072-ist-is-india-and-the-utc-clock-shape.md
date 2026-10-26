# 0072 `IST` is India Standard Time, and the shape of a UTC answer

Status: open
Milestone: v1
Area: time zones

The reference app prints `noon IST in UTC` as `06:30`: `IST` is UTC+5:30. The engine refuses `IST` because the name is ambiguous (Irish, Israel). The answer is written `06:30`, with a leading zero and no AM or PM, unlike the `7:29 PM` of other answers; why is unknown (maybe a conversion into UTC prints a 24-hour clock). Needs one more reading: `noon PST in UTC` and `noon IST in PST`.
