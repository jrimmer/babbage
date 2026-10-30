# 0092 A selection highlight reaches into the answers column

Status: open
Milestone: 1.0.1
Area: app

Found by the maintainer on the real desktop (retest R4, 2026-10-30): after pasting a long block of lines, holding Shift and pressing Up to select them showed the selection highlight running over the answers column. It repainted correctly as the sheet scrolled up. The highlight of a selection should stop at the text column's right edge.

Not reproduced yet. Start from the selection drawing in `element.rs`: the selection rectangles for lines that wrap or end in a newline are probably extended to the pane's right edge instead of `answer_x`.
