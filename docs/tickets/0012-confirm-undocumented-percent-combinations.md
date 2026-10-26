# 0012 Confirm undocumented percentage combinations against the real app

Status: open
Milestone: v1
Area: engine, percentages

The reference notes do not define `10% x 10%`, `20 / 50%`, `50% / 2`, `10%^2`. The engine refuses them. Check each against a real reference app install, then relax the rule and add the answers to `tests/corpus/percent.tsv` via `tools/gen_percent_corpus.py`.
