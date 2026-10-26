# 0075 `in` followed by an unknown word is ignored

Status: open
Milestone: v1
Area: sheets

The reference app prints `40 lb in watermelons` as `40 lb`: the unknown word and the `in` before it are dropped. The sheet path of the engine reports `unexpected word "in"`. There are no fun units (`watermelons`), so the line is just `40 lb`.
