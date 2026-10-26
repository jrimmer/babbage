# 0073 `time in Tokyo when it is 9am in London`, airport codes, `Monday 9am Delhi in nyc`

Status: open
Milestone: v1
Area: time zones

The reference app: `time in Tokyo when it is 9am in London` is `6:00 PM` (today London is on winter time) and `7:30am LAX to Japan` is `11:30 PM`, so airport codes and a country name as a zone work. `Monday 9am Delhi in nyc` was reported as `6:00 PM`, the same text as the line above and not a plausible conversion, so that reading is doubtful and needs a recheck. The engine refuses all three. Airport codes are a data list (ticket 0010); the conversion forms are not.
