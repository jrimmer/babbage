# 0022 Global variables and a definition sheet

Status: mostly built
Milestone: v1
Area: sheets

The variables notes describes variables shared by every sheet, ordered local, then global, then definition sheet, and definition-sheet variables may use ones declared above them. The engine has local variables only. Needs a settings file and a precedence test.

Audit 2026-10-26: Settings, Variables holds global names (`globals`), ordered local over global, and dividers can end the variables above them (`tests/sheet.rs`). A definition sheet shared by every sheet is not built.
