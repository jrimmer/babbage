# 0072 `IST` is India Standard Time, and the shape of a UTC answer

Status: closed (the reference app)
Milestone: v1
Area: time zones

The reference app prints `noon IST in UTC` as `06:30`: `IST` is UTC+5:30. The engine refuses `IST` because the name is ambiguous (Irish, Israel). The answer is written `06:30`, with a leading zero and no AM or PM, unlike the `7:29 PM` of other answers; why is unknown (maybe a conversion into UTC prints a 24-hour clock). Needs one more reading: `noon PST in UTC` and `noon IST in PST`.

`IST` is now India Standard Time (UTC+5:30), so `noon IST in UTC` is `6:30 AM`. The reference app wrote `06:30`; the shape is still unexplained and needs more readings (`noon PST in UTC`, `noon IST in GMT`, `noon IST in Tokyo`).

Settled by the reference app: `noon IST in UTC` is `06:30` and `noon PST in UTC` is `19:00`. A time of day converted into UTC reads on the 24-hour clock, and `PST` (like the other US abbreviations) is the Pacific region with its daylight saving, so noon PST on 26 October is 19:00 UTC. Other abbreviations stay fixed offsets; whether `CET` or `BST` follow their regions is not known.
