# 0034 Bitwise operators on negative numbers

Status: closed (the reference app, one deliberate difference)
Milestone: v1
Area: operators

`-1 & 1` is refused. Two's complement of unbounded width has no single obvious display. Check what the reference does, and whether NOT exists (the page lists AND, OR, XOR and the shifts only).

Sheet G (the reference app, 2026-10-26): `-1 & 1` and `-8 >> 1` print `0` in the reference app; Babbage refuses a negative number in a bitwise operation rather than print a silent 0 (deliberate, G22 and G24). `NOT 5` prints `5` (the word is skipped), as Babbage does.
