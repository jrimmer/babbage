# 0013 Phrase forms inside parentheses and chains

Status: open
Milestone: v1
Area: parser

`(20 as %) x 2` and `1 + (20 is what % of 200)` are errors today because phrase forms match only at the top level of a line. Needs a recursive or staged parse that stays iterative (no stack overflow on deep input).
