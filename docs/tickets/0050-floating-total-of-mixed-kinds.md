# 0050 What the floating Total shows for dollars, plain numbers and volumes together

Status: closed (the reference app)
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
