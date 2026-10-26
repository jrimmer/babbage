# 0056 Two readings that are looser than they should be

Status: closed
Milestone: v1
Area: dates, zones

Found while pinning near-miss shapes for mutation testing, and left out of the pinned table on purpose:
- `weeks after March 14` is `March 21`: a bare plural unit with no number counts as one. `3 after` and `after March 14` are errors, so this one is inconsistent. A bare unit after `after` or `before` should probably be refused.
- `noon GMT+5pm in UTC` is accepted and read as GMT+17, because `5pm` is a clock value like `5:30` and the offset reader accepts any clock value. Only `GMT+5:30` style should be accepted; the lexer does not say whether the clock had `am` or `pm`.

Fixed: `weeks after March 14` and `weeks before March 14` (a unit with no count) are refused; `1 week after March 14` still works. Also fixed: `GMT+5pm` (and `UTC-5:30pm`) is refused; the lexer no longer reads a 12-hour clock right after `GMT+` or `UTC-`, so only `GMT+5` and `GMT+5:30` style offsets work.
