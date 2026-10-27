# 0024 Typo warnings: tune and surface them

Status: partly built (the tip shows what a near-miss word may mean; tuning which words warn is open)
Milestone: v1
Area: sheets, UI

A skipped word one edit from a unit name comes back in `LineResult::warnings` (`kmm` for `km`). Prose like `tip` (near `tsp`) triggers it too. The UI must show ignored words and warnings plainly. Consider scoring by the word's position next to a number.
