# 0025 Prose that contains phrase keywords

Status: closed (specified and tested; further cases wait for use after 1.0)
Milestone: v1
Area: sheets

Stray `is`, `a`, `to`, `in` are dropped when they spell no phrase (`Boing is $5`). A sentence that does form a phrase by accident is evaluated as one. Review with real sheets.

Partly fixed: an operator word with only prose after it is prose, so `I spent $128 + $45 on clothes // on 10-02-2019` is `$173.00`, as the real headings-and-comments page prints it. Other keyword-in-prose cases remain open.

Specified 2026-10-27 in `crates/babbage-core/tests/prose_with_keywords.rs`. A skipped word never changes the maths (`Boing is $5`); prose that spells a complete form is read as the form (`Book 3 of 5 volumes` is 15, `Room 5 to 7` is 2); a keyword that opens a form it cannot complete is an error, not a guess (`Meeting in 5 minutes`, `Flight to Denver $320`, `Dinner for 4 is $86`); a comment hides its keywords. The one wrong answer the probe found, `20% of what is $30` giving `$6.00`, is now `$150.00` (the `of`, `on` and `off` forms in both orders). Real sheets are not needed to close the ticket: sentences beyond these shapes are left to be found in use.
