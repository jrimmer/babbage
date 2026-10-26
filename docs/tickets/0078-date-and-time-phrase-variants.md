# 0078 Date and time phrase variants the audits found

Status: open
Milestone: v1
Area: dates, time

Forms the pages or the release notes show that the engine refuses or reads wrongly (audit claims; each needs a line of its own and, where the page prints no answer, a reading from the owner): `weeks`, `months`, `hours` in place of `days` after `between`, `since`, `until`; `A to B in days`, `A to B in weeks`, `7:30am to 8:45pm in minutes` (with parentheses they work); `days left in 2024`, `days left in March`, `days until April`; `time since January 1`, `time to December 25`; `March 12-15`; `1953 - today`; `13 March 23`; `04-05-2020 + 1 day` (day-first slash and dash dates, ticket 0059); `Tuesday next week`, `Friday last week 3pm`; `3 hours ago in London`, `30 min from now in Sydney`, `time in Copenhagen 3 hours from now`; `9.30am to 5.45pm`; `09:05:03 P.M.`; `8:00 pm EST` style zone words `3pm et to pt`, `3pm California to New York`, `3pm Sydney time to Lisbon time`; `new timestamp`; `today as iso`; quarters (`next quarter`, `days until next quarter`); `moon phase`, `sunrise in Tokyo` (astronomy, on-device).
