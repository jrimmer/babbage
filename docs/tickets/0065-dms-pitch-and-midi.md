# 0065 Degrees-minutes-seconds, musical pitch and MIDI

Status: closed
Milestone: v1
Area: units

The reference notes show `156.742° as DMS | 156° 44′ 31.2″`, `440 hz as pitch | A4` and `C2 to midi | 36`. None is built. DMS is a display of an angle (exact arithmetic is possible); pitch and MIDI need the note naming convention (which octave number `A4` and `C2` use) from a real install before building.

Built: `156.742° as DMS` is `156° 44′ 31.2″` (exact arithmetic, seconds to ten places; checked in Python with fractions). Open: `440 hz as pitch` (A4) and `C2 to midi` (36). The page implies middle C is C4 = MIDI 60 (A4 is 440 Hz, MIDI 69). A note name such as `C2` lexes as a unit today, so it needs a lexer rule; the engine prints `1 °C²` for `C2 to midi`, a quiet wrong answer.

Built: `440 hz as pitch` is `A4`, `C2 to midi` is 36, `A4 to hz` is 440 Hz, with middle C as C4 (MIDI 60), sharps in note names and flats accepted on input (`Bb2`). A note name is a note only before `to`/`as`/`in` and `midi`, `hz`, `hertz` or `frequency`, so `5 C2` is still five degrees Celsius squared. Frequencies checked with mpmath; pinned in the tests. Not known: how the expected behaviour names an in-between frequency (the engine gives the nearest note), and `midi 69 as pitch`.
