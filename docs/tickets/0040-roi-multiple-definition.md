# 0040 What `X invested Y returned` shows

Status: closed (settled by a maintainer check)
Milestone: v1
Area: finance

The maintainer's notes lists `$500 invested $1,500 returned` as `2x` and `annual return on $1,000 invested $2,500 returned after 7 years` as `13.99%`. The second uses returned / invested (2.5). The first would be 3x by the same rule, or 2x if the multiple means the gain. The engine uses returned / invested for both. Check a real install.

A maintainer check was not asked this; the page prints `2x` and `13.99%`, which a maintainer check might also differ on. Add `$500 invested $1,500 returned` to the next check.

Settled by a maintainer check (second pass, A39): `$500 invested $1,500 returned` is `2x`, the gain over the stake. The annual-return forms still use returned over invested (the factor an annual rate is the root of), which the page's `13.99%` agrees with.
