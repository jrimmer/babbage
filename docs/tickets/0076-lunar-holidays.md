# 0076 Chinese New Year, Ramadan and Hanukkah

Status: mostly built
Milestone: v1
Area: dates

The real dates notes lists `Chinese New Year`, `Chinese New Year's Eve`, `Ramadan`, `End of Ramadan`, `Hanukkah` and `End of Hanukkah`; the release notes gives `Hanukkah 2026` as `4 December` and says Chinese New Year 2025 fell on 29 January. The engine drops the words and shows the year (`Hanukkah 2026` is `2,026`; `Chinese New Year 2025` is `January 1, 2025`), a quiet wrong answer. Hanukkah follows the Hebrew calendar (a published algorithm); Chinese New Year needs a table or lunar computation; Ramadan depends on moon sighting or a tabular calendar. Until built, these should be an error rather than a quiet wrong number.

Built, from the maintainer's answers (A46 to A48): `Hanukkah`, `End of Hanukkah`, `Chinese New Year` (also `Lunar New Year`), `Chinese New Year's Eve`, `Ramadan`, `End of Ramadan`. Festivals are given as the evening they begin and the day they end (`Hanukkah 2026` is December 4, `Ramadan 2026` is February 17, `End of Ramadan 2026` is March 19). Hanukkah follows the Hebrew calendar rules (checked against convertdate for 1583 to 2599), Ramadan the arithmetic Islamic calendar (checked against convertdate; it agrees with the Umm al-Qura calendar for 2025 to 2027 but can differ by a day in other years), Chinese New Year a table of 1900 to 2099 (checked against lunardate). Open: `Chinese New Year 2100` is a quiet wrong answer (the word is dropped); a maintainer check may use observed Ramadan dates.
