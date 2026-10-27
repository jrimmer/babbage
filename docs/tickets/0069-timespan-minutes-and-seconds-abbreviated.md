# 0069 Timespans spell hours out but abbreviate minutes and seconds

Status: closed
Milestone: v1
Area: display

The expected answer is `7:45am - 4:20pm` as `8 hours 35 min`, `7:30am to 8:45pm` as `13 hours 15 min`, and `3 hours 5 minutes 10 seconds` as `3 hours 5 min 10 s` (lines A19 to A21). The maintainer's page's other timespans agree: `5 min 30 s`, `235 hours 12 min`, `31 days 12 hours`, `10 weeks 2 days`. So days, weeks, hours (and months, years) print as words and minutes and seconds print as `min` and `s`. A written-out span of several units prints as a timespan, not as a decimal of the largest unit (`3.0861111111 h` today).

Fixed: timespans print weeks, days and hours written out and minutes and seconds as `min` and `s`; several time units written in a row print as a timespan (`3 hours 5 min 10 s`). Pinned in `tests/lexer_edges.rs`, `tests/units_more.rs`, the maintainer replay and the regenerated time oracle.
