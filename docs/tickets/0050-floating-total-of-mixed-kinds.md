# 0050 What the floating Total shows for dollars, plain numbers and volumes together

Status: closed (the reference app, Sheets C and F)
Milestone: v1
Area: sheets

The reference sheet is 78 lines long, so `Total 95,3...` includes lines that are not visible, and the 19 visible answers (plain numbers, dollars, volumes) cannot be checked against it. The engine adds the lines it can and refuses a sheet that mixes kinds (`these units measure different things`), and the app shows that message in the chip. The reference app may add the bare numbers of mixed lines, or only some kinds. Check a real install with a small mixed sheet before changing the rule.

Decision (owner, 2026-10-26): keep refusing a Total that mixes kinds; the chip shows "these units measure different things". The owner will confirm in the reference app. To do that, type this sheet and send what the Total shows (and whether it changes when each line is removed):

```
$10
5
3 gallons
```

Expected here: an error chip. If the reference app instead shows a number, say which lines it counted (all three, the bare number only, or the dollars only) and the rule follows from that.

The reference app: for the lines `$10`, `5`, `3 gallons` the Total was `$10.00`; and the owner then reported `5` and `3 gal` for the next two steps, which fits the Total adding only the lines of the first line's kind (the owner probably deleted lines from the top, not the bottom as the instructions said). The engine refuses a mixed sheet. To confirm, one more check is needed.

Settled by the reference app (second pass, Sheet C): `$10`, `5`, `3 gallons` is `3 gal`; `5`, `3 gallons` is `8 gal`; `3 gallons` is `3 gal`; `5`, `$10`, `$20`, `7` is `$42.00`. The rule built: lines that can all be added give the sum (plain numbers take the unit); otherwise the Total is the sum of the lines of the last line with a unit's kind, and plain numbers are left out. Which kind wins when the last line is a plain number is not known.

Caution (a side note, and it is right): this rule rests on ONE run of Sheet C. The owner's first run, taken before the steps were rewritten and which the owner said had not been followed as written, gave `$10.00`, `5` and `3 gal` for `$10, 5, 3 gallons`, then `5, 3 gallons`, then `3 gallons`. Those three also fit a different rule (the Total is the sum of the lines of the FIRST line's kind, deleting from the top), and the first run's `5` for `5, 3 gallons` disagrees with the second run's `8 gal`. I took the second run because it followed the rewritten steps; I have not proved the first was a mix-up. Sheet D now asks the three-line sheet again in both orders: `$10`, `5`, `3 gallons` and `3 gallons`, `5`, `$10`. If the answers are `3 gal` and `$10.00` the rule built is right; if `$10.00` and `3 gal` it is the first-line rule.

Third answer (Sheet D lines 59 and 60): `$10`, `5`, `3 gallons` gave `$10.00` and `3 gallons`, `5`, `$10` gave `$10.00`. So the same three lines have now given `$10.00` (first run), `3 gal` (Sheet C) and `$10.00` (Sheet D), which no rule that depends only on the lines can explain; neither the last-unit rule nor the first-line rule fits both orders. The rule built now is: when the lines cannot all be added, the Total is the sum of the money lines (plain numbers left out), in either order; with no money among them, the lines of the last unit line's kind. It fits every answer but Sheet C's `3 gal`. If the reference app's Total depends on how the sheet was typed (the order lines were added, say), more data would be needed: ticket 0083 lists what to ask.

Sheet E matched all four two-line sheets (`$10`/`3 gallons` and its mirror are `$10.00`; `3 gallons`/`4 km` is `4 km` and its mirror `3 gal`). But the target screenshot's Total on a 78-line mixed sheet reads `95,3…` with no dollar sign, which money-wins would not give (Babbage shows `$13,365.00 · 15 lines left out` for the screenshot's 19 visible lines). A note pointed this out. Not settled: Sheet F asks the reference app for the Total of the 19 lines and of four small mixed sheets. Until then the rule stays and the Total chip says how many lines it left out.

Reverted (same day): the "money wins" rule rested on Sheets D, E and F, whose multi-line entries went through the page's copy buttons, and the answers show the lines were pasted as ONE line (Sheet F3 is `$47.36B`, which is `$740 × 64M mm³`; F1 and F5 are just the last line's answer). So none of those answers is a Total. The rule is back to the one all four clean Sheet C answers fit (the sum of the lines of the last unit line's kind), and the Total chip still says how many lines it left out. The page's copy now copies the way a selection does and the multi-line entries say "its own sheet"; Sheet F, D59, D60 and E32 to E35 need asking again.

Settled (Sheet F redone with the fixed copy button): The reference app adds the lines in order and starts the sum again from any line that cannot be added to it. Its Totals: `12 + 12 + 15`, `85 × 12`, `19/29` is `1,059.6551724138`; `$185 × 4` over the volume line is `64M mm³`; adding a plain `85 × 12` between them still `64M mm³`; plain number plus two volumes `80.778236M mm³`; and the target screenshot's 19 lines `$39,506.77`, which is exactly lines 7 to 19 (the sum restarted at the volume lines and again at `$185 × 4`). This also fits all four Sheet C answers, so the "money wins" and "last unit line" rules are both gone. Built, with the reference sheet's Total in `tests/owner_reference.rs`.
