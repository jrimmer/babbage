# Mutation testing record

`bin/mutate` runs `cargo-mutants` on the engine: it changes one operator, comparison, constant or branch at a time and requires at least one test to fail. A mutant that no test notices is a **survivor**. The runs are throttled (`bin/lowprio`: lowest priority, four cores, a 6 GiB memory cap) because this is a shared workstation.

## Rows 1 to 15 and 21

Clean runs earlier in the build (the unit, percent, money, finance and function work); each finished with no survivors, apart from loop mutants that make a test hang, which count as caught.

## Rows 16 to 20 (dates, workdays, timestamps, times and timecodes, zones)

| Run | Scope | Mutants | Caught | Missed | Timeouts |
|---|---|---|---|---|---|
| 1 | the five date and zone modules | 1,076 | 844 | 143 | 20 |
| 2 | same, after tests written for run 1 | 1,117 | 973 | 69 | 6 |
| 3 | zones only | 125 | 117 | 2 | |
| 4 | every function with a run-2 survivor | 273 | 264 | 0 | 7 |

Survivors were closed with oracle lines, exact-error pins, direct function tests, or by removing code that no input could reach.

## The shared engine files (lexer, value, units, words, parser, phrase, sheet, settings, eval, number, format, functions)

Changed lines since the last clean engine run.

| Run | Mutants | Caught | Missed | Timeouts |
|---|---|---|---|---|
| A (lexer, value, words, ...) | 819 | 610 | 130 | 1 |
| B (number, format, functions) | 73 | 53 | 13 | 0 |
| confirm, every function with a survivor | 773 | 688 | 72 | 3 |
| confirm again on the final source | 846 | 787 | 42 | 5 |
| narrow | 475 | 452 | 15 | 3 |
| last | 114 | 111 | 2 | 1 |

After the last run, one real gap (a Western-European number with a point and no digit after it) was pinned. Every other survivor in these runs was either closed or is listed below.

## Parenthesised phrase forms (ticket 0013)

`collapse_groups` in `phrase.rs`, run with `MUTATE_RE=collapse_groups` against the parenthesised, percent, near-miss and unit tests: 11 mutants, 9 caught, 2 unviable (they do not compile), 0 missed.

## Real bugs that mutation or the fuzz test found

- `workdays_between` recursed forever on a zero-length span (`workdays in 2024`): a stack overflow that would crash the app.
- Adding workdays walked one day at a time, so `June 5 + 99999999999 workdays` never returned; it now skips whole weeks.
- `New Year`, `Valentine's Day` and `June 1st 2024` never parsed.
- An ISO date was only recognised at the start of a line (`1 + 2019-04-01` was read as a subtraction).
- In a sheet, `sqrt(16)` answered 16 (the function name was dropped as prose), found by replaying the reference examples.

## Survivors accepted as equivalent

Each changes at most which error message appears, or nothing, because a later check refuses the same input. None changes a successful answer.

| Where | Mutant | Why it cannot matter |
|---|---|---|
| `proportion_what` (`words.rs`) | `first + 1` becomes `first * 1` | `what <= first + 1` only guards a slice; the empty part is refused by the next check with the same message |
| lexer, trailing characters after a timecode or clock | `||` becomes `&&` in the "followed by a letter, point or colon" test | input that reaches it is refused later with an error either way; pinned messages cover the observable cases |
| lexer, `T` suffix | `'T' if !currency` guard | a second arm handles `T` for currency and gives the same value; `$5Tn` is pinned |

Loop mutants (`+=` becoming `-=` inside a `while`) hang the test and are counted as caught, as the tool reports them as timeouts.

## What this does not prove

Mutation testing shows that the tests notice changes to the code that exists. It does not show that the answers match a real reference app: that is what the oracle corpora, the replay of the reference examples and the open tickets (0018, 0030, 0043 to 0046, 0055) are for.
