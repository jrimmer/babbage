# 0055 How many places a converted quantity shows

Status: closed (the reference app)
Milestone: v1
Area: display

The reference prints `100 pounds in kg` as `45.36 kg` and `1 cm in px at 326 ppi` as `128.35 px`, two places, where the engine prints every digit up to ten (`45.359237 kg`, `128.3464566929 px`). The home screenshot shows plain numbers at ten places (`3.8146972656`) and `16.777216M mm³` with six, so two places is not a general rule. It may apply to converted quantities, or the page text may only be abbreviating. Check a real install with `100 pounds in kg` and `1 cm in px at 326 ppi` before changing the display.

Settled by the reference app: `100 pounds in kg` prints `45.359237 kg` and `1 cm in px at 326 ppi` prints `128.3464566929px`: every digit up to ten places, no two-place rounding. Only the missing space before `px` differs (ticket 0067).
