# 0023 Confirm how subtotals, totals and tags are typed

Status: open (asked in Sheet G)
Milestone: v1
Area: sheets

The reference describes subtotals as a UI action on a blank line and never gives typed text. The engine reads a line `subtotal`, `name =` with nothing after it, and `total of #tag`, `sum of`, `average of`, `avg of`, `count of`. The home page also shows `#jude total`. Check each against a real install and align.
