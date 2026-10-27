# 0047 Public holidays in workday calculations

Status: built for holidays the user lists; a shipped regional list is open
Milestone: post-v1 (region data)
Area: dates

The workdays notes excludes public holidays for the user's region: `December 24 + 2 workdays` gives `December 30`, because December 25 and 26 are holidays there. The engine counts Monday to Friday only. Holiday lists differ by country and state and change every year, so they are a shipped dataset (the same kind of data as tickets 0035 to 0038) and are not built yet. `December 24 + 2 workdays` gives `December 26` until then.

Audit 2026-10-26: Settings, Dates & Times takes holidays (`12-25`, `December 25`, a full date, a list) and workday arithmetic skips them (`tests/holidays.rs`, `settings-window` check). Which holidays a region has is shipped data and is not built, so `December 24 + 2 workdays` skips December 25 only when the user lists it.
