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
- An ISO date was only recognized at the start of a line (`1 + 2019-04-01` was read as a subtraction).
- In a sheet, `sqrt(16)` answered 16 (the function name was dropped as prose), found by replaying the maintainer's examples.

## Survivors accepted as equivalent

Each changes at most which error message appears, or nothing, because a later check refuses the same input. None changes a successful answer.

| Where | Mutant | Why it cannot matter |
|---|---|---|
| `proportion_what` (`words.rs`) | `first + 1` becomes `first * 1` | `what <= first + 1` only guards a slice; the empty part is refused by the next check with the same message |
| lexer, trailing characters after a timecode or clock | `||` becomes `&&` in the "followed by a letter, point or colon" test | input that reaches it is refused later with an error either way; pinned messages cover the observable cases |
| lexer, `T` suffix | `'T' if !currency` guard | a second arm handles `T` for currency and gives the same value; `$5Tn` is pinned |

Loop mutants (`+=` becoming `-=` inside a `while`) hang the test and are counted as caught, as the tool reports them as timeouts.

## What this does not prove

Mutation testing shows that the tests notice changes to the code that exists. It does not show that the answers match a maintainer check: that is what the oracle corpora, the replay of the maintainer's examples and the open tickets (0018, 0030, 0043 to 0046, 0055) are for.

## Large powers (2026-10-26)

`approx::pow_large` and the fallback in `value::power`: 7 mutants, 6 caught, 1 unviable, 0 missed (after a boundary test for the 400,000 exponent limit was added). Run with `MUTATE_RE=pow_large MUTATE_TESTS="--test large_powers --test limits" bin/mutate crates/babbage-core/src/approx.rs`.

## Sheet G changes (2026-10-26)

`bin/mutate` limited to the changed engine lines (`MUTATE_DIFF`, the diff written with `git -c diff.mnemonicPrefix=false diff HEAD`). First run: 125 mutants, 94 caught, 15 missed, 15 unviable, 1 timeout. The misses were gaps in the tests (the tag forms `#tag average` and `#tag count`, the characters a tag name may hold, the temperature refusals, a percentage divided by a quantity, the exactness mark on `6h 0m 0s`), two redundant lines (a date-kind guard and an arithmetic bound, both removed), and the near-zero trig boundary (now a helper with its own test). Second run: 123 mutants, 106 caught, 1 missed, 15 unviable, 1 timeout. The one miss was an equivalent mutant (a percentage never carries a unit, so the check on it was redundant); the check is removed. The timeout is `per_hundred_km` with `+=` turned into `*=`, a loop that hangs, which counts as caught.

## Engine pass over `units`, `parser`, `phrase` and `functions` (2026-10-27)

Full tests, the whole of the four files: 726 mutants, 492 caught, 74 unviable, 61 timeouts (loop mutants that hang), 99 missed. The misses sat in code whose coverage had gone with the removed example corpus: primes, pitch, degrees-minutes-seconds, screen density, the seeded random numbers, combinations, standard deviation, the `in lb oz` split, conversions at a rate and the rules that cancel or merge units. Each now has a test (`function_near_misses.rs`, `phrase_unit_near_misses.rs`, `unit_targets.rs`, `spoken_forms.rs`). Reruns of the touched functions: 335 mutants (92 missed), then 209 (23 missed). The 23 that remain are accepted below.

| Where | Mutant | Why it cannot matter |
|---|---|---|
| `call`, `factorial` | the size test `bits > MAX_BITS` becomes `>=` or `==` | only a result of exactly the limit differs, and no factorial or combination has exactly that many bits |
| `standard_deviation` | the kind test `||` becomes `&&` | a quantity of another kind fails the conversion on the next line with the same error |
| `keyword_signature`, `parse_line` | the bracket-depth guard on the loose `%` becomes `true` | a lone `%` is never dropped as prose, so counting one inside brackets changes nothing |
| `parse_line` | `at + 1 < len` becomes `<=` or `-` | a conversion word at the very end falls through to the same "unexpected word" error |
| `parse_line` | the bracket test `==` becomes `!=` | collapsing groups of a line with no brackets is a no-op |
| `keywords_form_a_phrase` | the `[conv, "at"]` guard becomes `true` | no other two-keyword pair that reaches it spells a phrase either |
| `is_conversion_word` | becomes `true` | it only matters for a bare word kept at the end of a line, which the parser refuses anyway |
| `UnitExpr::has_division`, `is_time_over_time` | each comparison | the callers also test that the target has no dimension, which every target these would wrongly accept already fails; the targets are pinned in `a_plain_number_shown_in_a_unit_of_another_kind_is_refused` |
| `Spelling::partial_cmp` | becomes `None` | spellings never take part in an ordering; equality ignores them |
| `UnitExpr::reduce` | `e > 0` becomes `>= 0` | an exponent is never zero in a unit expression |

Noticed while probing, left as a ticket: `5 m to cm at 2` answers `1,000 cm`. A conversion at a rate on units that are not money is accepted and the rate multiplies the result.
