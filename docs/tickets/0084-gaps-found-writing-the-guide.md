# 0084 Gaps found while writing the user guide

Status: open
Milestone: v1
Area: engine, app

Found by running every example of the guide through the engine and reading the app's source for every claim.

1. **A fractional exponent is refused.** `2 ^ 0.5`, `4 ^ 0.5` and `8 ^ (1/3)` give "exponent must be a whole number". `sqrt` and `N root of X` work, but a reader who types a power with a decimal exponent expects an answer (exact when the root is rational, marked approximate otherwise).
2. **The near-miss hint has no text.** A skipped word one letter from a unit (`5 kmm`) gets a wavy underline, and the engine knows what it looks like (`Warning::looks_like`), but the app never shows it. The guide describes the underline only.
3. **Escape in a settings text field** closes the whole settings window instead of canceling the edit, because the panel-close key runs first.
4. **No way back from the trash in the app.** A sheet moved to the trash is a file under `.trash`; restoring it is a manual move. There is also no "open the sheets folder" button.
5. **The answer menu's switches start lit.** The `,`, `M` and `$` buttons look on when nothing is set; the first click stores "off", which changes nothing visible if the setting is already off. The slider reads "10 dp" when unset whatever the Decimal places setting says.
6. **`total` alone on a line is an error** ("unexpected end of expression"); `subtotal` alone works. Decide whether `total` should add the sheet so far.
7. **Number format "System"** maps to nothing, so it behaves as `1,234.56`.
8. **Currency choice.** The settings window offers USD, AUD, CAD and NZD for "what $ means"; the engine also reads SGD and HKD.
9. **Words skipped that change the meaning.** `not true` is `true` (the `not` is skipped), `5 factorial` is `5` (`5!` is an error, `fact(5)` works), `round 2.567 to 1 dp` is `3`, `int(3.9)` is `3.9`, `half(50)` is an error while `half of 50` works, `12pm HQ in New York` with an unknown place answers `12:00 PM`, and `day of the year on <date>` fails because `the` is skipped. The skipped-word mark (an underline) now shows some of these; each should either work or be refused.
10. **Date and time forms that are refused:** a date-to-date span `in weeks`, `9am to 5pm in minutes`, `from 9am to 5pm`, `8am Europe/Berlin in Asia/Tokyo`, `90 minutes as timecode at 25 fps`, and `what time is it in Tokyo` (no answer). `9am - 17:00` is `8 hours`, which looks wrong.
11. **No shipped holiday calendar.** Workday math skips only the holidays listed in Settings, Dates & Times; the "No holiday calendar" row in the decisions log reads as if that were all.
12. **More quiet wrong answers from a skipped word** (the same family as 9): `£2 + 50p` is `£52.00` (the `p` is skipped, so 50 is read as £50), `30% discount on $200` is `$260.00` (it adds), `15 percent of 80` and `15 % of 200` are errors, and `1 MB in bytes` is `1M B`.
13. **Unit spellings.** Lower-case `ml`, `cl` and `dl` are not units (only `mL`, `cL`, `dL`, `kL`), nor is `KB` (only `kB`); the farad, henry, siemens and tesla take no prefixes (`mH`, `µF`); `1 rpm in Hz` is an error because a revolution is an angle.
14. **Rates and labels.** `3 hours x $25/hour` and `40 hours * $25/hour` are right but labeled `/hour`; `$25/hour * 40 hours` is correct. `25 mpg * 10 gallons` is `250 MPG gal`, `$15/hour * 1 year` is `$131.4873k`, `100 EUR in USD at 1/0.92 USD/EUR` is garbage, a currency code before an amount works only for currencies that have a symbol (`EUR 5` works, `CHF 5` is an error), and `100 + 5% * 2` is `100.1`.
15. **A long Total chip.** When the lines of a sheet cannot be added (a length and a temperature), the Total chip shows the whole error sentence and it runs off the left edge of the window. It should be cut short, with the full text on hover.
16. **A hint that overlaps its neighbor.** In Settings, Calculator, the hint under "Your own names for places" wraps onto the "Sales tax" row.
