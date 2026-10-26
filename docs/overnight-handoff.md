# Overnight handoff (started 2026-10-26)

Plan, in order, each step ending in a check:
1. Gate and `bin/accept` on the tree at 7c19ae1 (running, log in the session scratchpad).
2. Mutation run 2 of rows 16 to 20 (running, 1,117 mutants, 4 cores, 6 GiB cap). Write a test for every survivor, rerun only those.
3. Mutation run of the shared engine files, changed lines only (`MUTATE_DIFF`), same loop.
4. More end-to-end checks in the real app for dates, workdays, zones, timecode and fractions.
5. Final gate, acceptance, roadmap and ticket update, commit.

Rules while away: nothing is pushed (there is no remote), nothing outside this repo and its scratchpad is touched, no process I did not start is signalled, every heavy run goes through `bin/lowprio`.

Waiting on the owner (does not block the plan): the reference checks listed in tickets 0018, 0030, 0043, 0045, 0046, 0055.

## Log
- 02:30 The gate at 7c19ae1 failed: a tax-parser test I committed never passed, because a text replace did not apply after `cargo fmt` reflowed the file. Fixed; the lesson is that every scripted edit now asserts that its target text was found.
- 03:10 At the commit "a currency word is a unit": `bin/check` passed (43 suites ok) and `bin/accept` passed 15 of 15, including sales-tax (types `$300 + VAT`) and sheet-functions (`sqrt(16)` is 4 in the real app).
