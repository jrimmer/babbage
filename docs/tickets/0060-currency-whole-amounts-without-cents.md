# 0060 Whole currency amounts printed without cents

Status: closed (the reference app)
Milestone: v1
Area: display

The reference notes print `cost = $550 | $550`, `monthly rent = $1,900 // 2018 | $1,900` and `$490 rounded to nearest hundred | $500`, but `$20 + 30 | $50.00`, `$30 × 4 days | $120.00` and `$99 per week | $99.00/week`. The engine prints cents everywhere (`$550.00`). The rule may be that a literal on an assignment line and a rounded result print as typed, or that the pages were written with different settings. Ask for `cost = $550` and `$490 rounded to nearest hundred` on a real reference app.

Audit 2026-10-26: the owner's Sheet D answers D44 and D45 print `$550.00` and `$500.00`: cents everywhere, as Babbage does.
