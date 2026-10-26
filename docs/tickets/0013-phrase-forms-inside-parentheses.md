# 0013 Phrase forms inside parentheses and chains

Status: closed
Milestone: v1
Area: parser

`(20 as %) x 2` and `1 + (20 is what % of 200)` are errors today because phrase forms match only at the top level of a line. Needs a recursive or staged parse that stays iterative (no stack overflow on deep input).

Fixed: before the top-level parse, each pair of parentheses (innermost first, one pass with an explicit stack) is searched for comma-separated runs that spell a phrase form or conversion; each such run is parsed on its own and stands in as one operand (`Tok::Group`). `(20 as %) x 2`, `1 + (20 is what % of 200)`, `(5 km in m) * 2` and `max(5 km in m, 2 km in m)` work; lines that were errors for other reasons keep their exact error. Tested in `tests/parenthesised_forms.rs`, including 20,000 levels of nesting.
