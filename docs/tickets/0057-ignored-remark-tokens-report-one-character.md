# 0057 A dropped number in a remark is reported as its first character

Status: closed
Milestone: v1
Area: sheets, app

In natural mode `$999 (for iPhone 16)` drops the whole bracket, and `ignored` lists `(`, `for`, `iPhone`, `1`, `)`. The `16` is reported as `1`: tokens that are not words are reported as the first character at their position. The app dims ignored text by this span, so only part of a dropped number is dimmed. The fix needs each token's end offset (the lexer returns it, but the sheet layer substitutes variables and line references first, which changes the token list), or the replacement tokens carrying their own span.

Fixed: a dropped token that is not a word is reported over its whole width (up to the next token or the end of the line, less spaces), so `16` is reported as `16`. Pinned in `tests/near_misses.rs`.
