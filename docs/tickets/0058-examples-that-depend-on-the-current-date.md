# 0058 Reference examples that depend on today's date or time

Status: open (cannot be compared)
Milestone: v1
Area: tests

The reference notes print answers computed on the author's day: `now + 3 hours 15 minutes | 6:26 pm`, `3 days ago | 7 September`, `days since July 15 | 57 days`, `time in Paris | 6:30 pm`, `week of year | 36`, `current timestamp | 1559740303.48`, `6pm Sydney in Chicago | 1:00 am` (written with Sydney on summer time and Chicago on winter time). The engine answers them with the clock it is given, so the replay lists them as known differences. Each form has its own tests with a fixed clock in `tests/dates.rs`, `tests/time.rs` and `tests/zones.rs`; what cannot be checked here is only the exact printed answer. A real reference app run on a known day would let them be pinned.
