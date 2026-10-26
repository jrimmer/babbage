# 0069 Timespans spell hours out but abbreviate minutes and seconds

Status: open
Milestone: v1
Area: display

The reference app prints `7:45am - 4:20pm` as `8 hours 35 min`, `7:30am to 8:45pm` as `13 hours 15 min`, and `3 hours 5 minutes 10 seconds` as `3 hours 5 min 10 s` (lines A19 to A21). The reference page's other timespans agree: `5 min 30 s`, `235 hours 12 min`, `31 days 12 hours`, `10 weeks 2 days`. So days, weeks, hours (and months, years) print as words and minutes and seconds print as `min` and `s`. A written-out span of several units prints as a timespan, not as a decimal of the largest unit (`3.0861111111 h` today).
