# 0075 `in` followed by an unknown word is ignored

Status: closed
Milestone: v1
Area: sheets

The reference app prints `40 lb in watermelons` as `40 lb`: the unknown word and the `in` before it are dropped. The sheet path of the engine reports `unexpected word "in"`. There are no fun units (`watermelons`), so the line is just `40 lb`.

Fixed: a conversion word (`in`, `to`, `as`, `into`) with only unrecognised words after it is prose in a sheet, so `40 lb in watermelons` is `40 lb`.
