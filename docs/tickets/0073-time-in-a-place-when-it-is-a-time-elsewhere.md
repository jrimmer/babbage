# 0073 `time in Tokyo when it is 9am in London`, airport codes, `Monday 9am Delhi in nyc`

Status: closed (settled by a maintainer check)
Milestone: v1
Area: time zones

The expected behaviour: `time in Tokyo when it is 9am in London` is `6:00 PM` (today London is on winter time) and `7:30am LAX to Japan` is `11:30 PM`, so airport codes and a country name as a zone work. `Monday 9am Delhi in nyc` was reported as `6:00 PM`, the same text as the line above and not a plausible conversion, so that reading is doubtful and needs a recheck. The engine refuses all three. Airport codes are a data list (ticket 0010); the conversion forms are not.

Built: `time in Tokyo when it is 9am in London` (also `when it's`), airport codes as places (155 major airports mapped to their zones; `IST` stays India), and a bare weekday name as the next such day (on Monday 26 October, `Monday` is 2 November), so `Monday 9am Delhi in nyc` is `November 1, 2026 at 10:30 PM` as expected. A date with a time of day now always carries its year, and reads `Yesterday at` / `Tomorrow at` a day either side of today. Pinned by the owner replay (A33, A34, A35), the regenerated zone and date oracles and the tests.
