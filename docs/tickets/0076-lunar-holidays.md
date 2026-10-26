# 0076 Chinese New Year, Ramadan and Hanukkah

Status: open (data or an astronomical rule)
Milestone: v1
Area: dates

The real dates notes lists `Chinese New Year`, `Chinese New Year's Eve`, `Ramadan`, `End of Ramadan`, `Hanukkah` and `End of Hanukkah`; the release notes gives `Hanukkah 2026` as `4 December` and says Chinese New Year 2025 fell on 29 January. The engine drops the words and shows the year (`Hanukkah 2026` is `2,026`; `Chinese New Year 2025` is `January 1, 2025`), a quiet wrong answer. Hanukkah follows the Hebrew calendar (a published algorithm); Chinese New Year needs a table or lunar computation; Ramadan depends on moon sighting or a tabular calendar. Until built, these should be an error rather than a quiet wrong number.
