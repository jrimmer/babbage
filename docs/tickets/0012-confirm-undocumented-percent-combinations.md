# 0012 Confirm undocumented percentage combinations against the real app

Status: closed (settled by a maintainer check)
Milestone: v1
Area: engine, percentages

The maintainer's notes do not define `10% x 10%`, `20 / 50%`, `50% / 2`, `10%^2`. The engine refuses them. Check each against a maintainer check, then relax the rule and add the answers to `tests/corpus/percent.tsv` via `tools/gen_percent_corpus.py`.

A maintainer check (2026-10-26): `10% x 10%` is `1%`, `20 / 50%` is `40`, `50% / 2` is `25%` and `10%^2` is `0.01`. Built: a percentage times a percentage is a percentage, a number divided by a percentage is a plain number, a percentage divided by a number is a percentage and a percentage to a power is a plain number. Still refused, because no sheet asked: a percentage divided by a percentage, a power of a percentage, and `on`/`off` with a percentage on both sides. The percent oracle (`tools/gen_percent_corpus.py`) now computes these forms.
