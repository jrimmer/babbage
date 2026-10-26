# 0052 Gaps between the app and the target screenshot

Status: open
Milestone: v1
Area: app

Reported by the build and found by comparing against the reference by eye, not by pixels: values referenced by `lineN` are highlighted as text and not shown as the value in a chip (the engine would need to return each reference's byte range and value); there is no folder pill or toolbar; sidebar cards are tighter than the reference; undo history is dropped when another sheet opens; long lines are clipped, not wrapped; IME composition is implemented but not exercised by any check; fonts and spacing are not tuned. Judging looks is the owner's call.

Done: rename and delete by mouse. A right click on a sidebar card opens the sheet and shows Rename and Move to trash under it; a double click renames; Escape, any key or a click in the editor closes the menu. The `sidebar-mouse` acceptance check drives it with a real virtual mouse (`tools/pointer.py`).
