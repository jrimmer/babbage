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
