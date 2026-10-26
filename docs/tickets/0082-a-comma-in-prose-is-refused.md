# 0082 A comma after a word makes an ordinary line an error

Status: closed (the reference app, Sheet D)
Milestone: v1
Area: forms

`hotel, 3 nights × $120` is `ERROR unexpected ','` in the sheet, while `hotel 3 nights × $120` is `$360.00`. Prose with a comma is common (`lunch, tip included 18 + 4`). The reference app was not asked; it is on Sheet D. If it ignores the comma the fix is to treat a comma that follows a word, outside any function's brackets, as part of the prose; until then the engine says no, which is loud and not wrong.

Related, found while writing the `resize` check: the word `and` after a number reads as the bitwise operator, so `lunch 3 + 4 and then coffee` is an error while `lunch 3 + 4 then coffee` is 7. Sheet D asks both.

Settled by the reference app (Sheet D lines 58 and 61): `hotel, 3 nights × $120` is `$360.00`, and `lunch 3 + 4 and then coffee` is `7`. A comma straight after a stray word is now part of the remark; the second line already gave 7. Pinned in `tests/sheet_d.rs` and `tests/owner_reference.rs`, with the commas that must stay (`min(3, 4)`, `average of 10, 20, 30`, `April 1, 2019`).
