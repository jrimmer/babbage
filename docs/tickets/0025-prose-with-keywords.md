# 0025 Prose that contains phrase keywords

Status: open
Milestone: v1
Area: sheets

Stray `is`, `a`, `to`, `in` are dropped when they spell no phrase (`Boing is $5`). A sentence that does form a phrase by accident is evaluated as one. Review with real sheets.

Partly fixed: an operator word with only prose after it is prose, so `I spent $128 + $45 on clothes // on 10-02-2019` is `$173.00`, as the real headings-and-comments page prints it. Other keyword-in-prose cases remain open.
