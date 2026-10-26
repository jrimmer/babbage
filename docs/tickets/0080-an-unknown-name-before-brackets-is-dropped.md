# 0080 An unknown function name before brackets is dropped and its argument returned

Status: open (design)
Milestone: v1
Area: sheets

In a sheet an unknown word is prose, so `foo(45)` is `45` with `foo` ignored. For a real function that this engine lacks (before the cotangent functions were added, `cot(1)` was `1`) that is a quiet wrong answer. The reference app drops unknown words beside numbers (`3 widgets × 4` is `12`), so refusing every unknown word would be wrong. Open question: for a word directly before an opening bracket (no space), does the reference app treat it as a function call that fails, or as prose? One reading would settle it, for example `foo(5)` and `price (5)`.
