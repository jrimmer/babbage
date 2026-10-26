# 0050 What the floating Total shows for dollars, plain numbers and volumes together

Status: decided: Total adds the first line's kind
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
