# 0062 `Yesterday at` and `Tomorrow at` in time answers

Status: closed
Milestone: v1
Area: time

The maintainer's notes show `9:45 am - 15 hours 10 minutes | Yesterday at 6:35 pm` and `time in Thailand | Tomorrow at 12:30 am`. When a time of day lands on the day before or after today the answer names that day. The engine prints a bare `6:35 pm` for the first and `June 14 at 6:35 pm` style dated times elsewhere.

Built together with 0073: a date with a time reads `Yesterday at 6:35 PM` or `Tomorrow at 12:30 AM` a day either side of today (page: `time in Thailand | Tomorrow at 12:30 am`; but see the second-pass note below: `time in X` itself stays a bare time), and any other date with a time carries its year. Shifting a clock time across midnight (`9:45 am - 15 hours 10 minutes`) still wraps to a bare time of day; showing it as `Yesterday at` needs the clock to carry a day, which is not built.

Second pass (a maintainer check, A69 and A70): a time of day moved across midnight names its day (`9:45 am - 15 hours 10 minutes` is `Yesterday at 6:35 PM`), built; but `time in Thailand` is a plain `3:12 AM` even though it is already tomorrow there, so `time in X` stays a bare time of day and the page's `Tomorrow at 12:30 am` is not what a maintainer check prints. A time on today's date prints as just the time (`now` is `12:55 PM`).
