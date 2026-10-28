# Quick calculation and the Omarchy door (2026-10-28, later)

**Done.** `babbage --quick` (a 760 by 480 window titled "Babbage Quick", or the open window brought forward through `hyprctl`: Hyprland 0.55 and later take Lua in a dispatch, so that form is tried first and the classic one second). `babbage setup omarchy [--remove] [--key K]` (default key `SUPER + ALT + C`): its own file `~/.config/hypr/babbage.lua`, one marked line in `hyprland.lua`, a marked block in the menu extension; each file backed up; a bound key refused (read from `omarchy menu keybindings --print` and `hyprctl binds -j`); `hyprctl configerrors` compared before and after, and every file put back when the change adds an error. The installer adds a launcher entry and an icon (`babbage --print-icon`). Tests: 20 unit tests (`omarchy_setup.rs`, `quick.rs`) and the acceptance checks `omarchy-setup` (fake home, stubbed `omarchy` and `hyprctl`), `quick-window` (nested Hyprland: a window titled "Babbage Quick" appears; with two windows open `--quick` brings the first forward and opens none) and the extended `install-script`.

**Found on the way.** gpui does not send the titlebar option to a Wayland compositor, so every Babbage window had an empty title; a window rule that matches the title could never have fired. The title is now set with `set_window_title`. Fairfax's `fairfax-omarchy-setup` checks a key with `grep "^SUPER + SHIFT + T"` but `omarchy menu keybindings --print` writes `SUPER SHIFT + T`, so its conflict check never matches a key with two modifiers (checked on this machine with `SUPER SHIFT + RETURN`).

**Verified.** `bin/check` passed (723 tests) and `bin/accept` passed 62 of 62. Never run against the real Omarchy config (the rules forbid it): the setup is tested on a fake home with stubbed commands, and `hyprctl reload`, `configerrors` and `binds -j` were checked for their output format in the nested compositor.

# Light theme, weekly updates, completions (2026-10-28)

**Done.** (1) Light themes: `theme.rs` reads `mode = "light"` from Omarchy's `colors.toml` (a light background decides when there is no mode line) and moves every text color until it reads on the page and on the answers column (contrast 4.5, or 3 for dim text and 2.5 for line numbers); the selection is thinned the same way. Off Omarchy the built-in theme follows the system's light or dark setting, and `BABBAGE_APPEARANCE=light|dark` overrides it. The dark colors are unchanged and a test pins them. Tests: six in `theme.rs` (three real light themes, a dark one and both built-ins keep every text pair readable) and the acceptance check `light-theme` (a light theme paints a light page, a switch to a dark file is followed within 15 s, the two `BABBAGE_APPEARANCE` values). (2) The weekly update check was already the default and a test pins it; nothing changed. (3) `bbg --completions bash|zsh|fish` and `bbg --list-sheets`, written from one option table in `cli.rs`; the installer adds the bash file, and the fish file when `~/.config/fish` exists, under its own marker and never over a file it did not write; `--uninstall` removes only its own. Tests: eight in `completions.rs` (a real bash completes options, places, shells, saved sheets with spaces and files), one in the store (`names_in` changes nothing), and the acceptance checks `cli-completions` and the extended `install-script`.

**Found on the way.** `bbg --help` never listed the long forms `--file` and `--sheet`; now it does, and a test keeps the table, the parser and the help in step.

**Verified.** `bin/check` passed (702 tests) and `bin/accept` passed 60 of 60 (accept ran before two more unit tests were added; `bin/check` ran again after). Mutation testing of `theme.rs` and `completions.rs`: 74 mutants, 4 real survivors fixed and rerun (27 of 27 caught), 3 equivalent in `docs/mutation.md`. Not verified in a real shell: the zsh and fish scripts (neither is installed on buildhost); they are checked for their options and quoting, and `zsh -n` and `fish --no-execute` run when those shells are present.

**Changed behavior to know about.** A machine that states no light or dark preference now gets the light built-in theme off Omarchy (gpui reads no preference as light). The acceptance harness pins `dark`, so pictures do not depend on the session. `docs/shots/guide/theme-light.png` shows a clock time in its sidebar and will change on every run like the other nine.

**Still open.** The public repository. Pictures that show a clock time. A plugin or tray icon (asked, not built).

# Open items worked through (2026-10-27, later)

**Done.** The merged helper worktree and its branch are removed. A conversion at a rate on units that convert without one is refused (`5 m to cm at 2`, `3 hours to minutes at 60`, `1 cm in px at 326`); the two older pins for the px forms now pin the refusal. Fuel economy cancels against what it measures (`10 mpg * 3 gallons` is `30 mi`, `300 mi / 30 mpg` is `10 gal`, `8 L/100km * 250 km` is `20 L`). The AppImage builds and runs (`packaging/babbage.svg`, `appimagetool` from the AppImage releases, checksum verified, kept outside the repo). A retaken screenshot replaces the kept one only when it differs visibly (`keep_unless_changed`, two tests): 34 of the 42 noisy pictures stay put.

**Not done, by decision.** `5% * 2` stays `0.1`: showing `10%` would turn the pinned `25% × 200` (50) into `5000%`.

**Still open.** Pictures that show a clock time (the sheet list in `sidebar`, `sheet-menu`, `sheet-rename`, `window-parts`, `export-notice`, `sidebar-menu`, `settings-updates`) change every run and are restored by hand. The public repository. (Ticket 0025 is closed: prose with keywords is specified in `prose_with_keywords.rs`, and `20% of what is $30` now answers `$150.00`.)

**Verified.** `bin/check` passed (687 tests, 60 suites) and `bin/accept` passed 58 of 58 on the tree before the commit that closed ticket 0025.

# Overnight handoff (2026-10-27)

**Done.** Mutation testing of the lines changed today (117 mutants, then 68: two survivors, one equivalent, one fixed by `unit_targets.rs`). Then a full pass over `units`, `parser`, `phrase` and `functions` (726 mutants, 99 missed) with two reruns of the touched functions (335, then 209 mutants): 23 remain, all accepted as equivalent with reasons in `docs/mutation.md`. New tests: `function_near_misses.rs` (primes, pitch, degrees-minutes-seconds, screen density, combinations, roots, standard deviation, the seeded random sequence, refusal messages) and `phrase_unit_near_misses.rs` (`in lb oz`, conversions in brackets, rate merging, unit cancelling, plain numbers shown as rates, conversions at a rate, the written form of a parsed line). One redundant arm in `Func::arity` was removed. `CHANGELOG.md` (1.0.0 draft) and `docs/release-plan.md` are written.

**Verified.** `bin/check` passed (678 tests, 56 suites) and `bin/accept` passed 58 of 58 on the final tree; the evidence file and the roadmap counts are refreshed.

**Stuck, or noticed.** `5 m to cm at 2` answers `1,000 cm`: a conversion at a rate on units that are not money is accepted and the rate multiplies the result. Not changed; it needs a decision (refuse it, or keep it). Bash calls failed for a while because the auto-mode classifier gave no verdict, which was not caused by the commands.

**Next.** Waiting on you, in one batch: the pushes (the private repo has about 10 commits not pushed, the site repo 3), `appimagetool` and an icon for the AppImage, the choice on `5% * 2` and on `mpg * gallons` as miles, the public repository (a fresh one-commit history), real sheets for the skipped-words report (ticket 0025), and whether to delete the merged helper worktree.

Rules held: nothing was pushed, published or downloaded; no process I did not start was signalled.

# Earlier handoff (2026-10-26)

Plan, in order, each step ending in a check:
1. Gate and `bin/accept` on the tree at 7c19ae1 (running, log in the session scratchpad).
2. Mutation run 2 of rows 16 to 20 (running, 1,117 mutants, 4 cores, 6 GiB cap). Write a test for every survivor, rerun only those.
3. Mutation run of the shared engine files, changed lines only (`MUTATE_DIFF`), same loop.
4. More end-to-end checks in the real app for dates, workdays, zones, timecode and fractions.
5. Final gate, acceptance, roadmap and ticket update, commit.

Rules while away: nothing is pushed (there is no remote), nothing outside this repo and its scratchpad is touched, no process I did not start is signalled, every heavy run goes through `bin/lowprio`.

Waiting on the maintainer (does not block the plan): the maintainer's checks listed in tickets 0018, 0030, 0043, 0045, 0046, 0055.

## Log
- 02:30 The gate at 7c19ae1 failed: a tax-parser test I committed never passed, because a text replace did not apply after `cargo fmt` reflowed the file. Fixed; the lesson is that every scripted edit now asserts that its target text was found.
- 03:10 At the commit "a currency word is a unit": `bin/check` passed (43 suites ok) and `bin/accept` passed 15 of 15, including sales-tax (types `$300 + VAT`) and sheet-functions (`sqrt(16)` is 4 in the real app).
- 03:40 New: the real-app check `everyday-lines` (15 lines typed into the nested Hyprland); both pass. `bin/accept` now has 16 checks.
- 05:10 Mutation run 2 (rows 16 to 20, 1,117 mutants) finished: 973 caught, 69 missed, 6 timeouts. The survivors were word-form near-misses, date-format time letters, zone abbreviations and a few equivalent comparisons; all have tests or simplifications now (commits 750be6f, 7dbfd49). Rerunning only the functions that had survivors (run 3, log `mutate-dates3.log`).
- 06:30 Targeted rerun of every function that had survivors: 264 caught, 0 missed, 7 timeouts (loop mutants that hang). Started the shared-engine pass (lexer, value, units, words, parser, phrase, sheet, settings, eval, number, format, functions: lines changed since b9dc549, 892 mutants), after a mini-run on three functions edited late. Log: scratchpad `chain.log`.
- 07:20 Real bug found by the shared-engine mutation pass: an ISO date (`2019-04-01`) was only recognized at the very start of a line, because the shape check looked at the first four characters of the line, not of the date; `1 + 2019-04-01` and `(2019-04-01)` read it as a subtraction. Fixed; an 11,000-line ISO oracle (offsets, zones, fractions, invalid dates) and lexer edge tests now pin it.
- 09:10 Shared-engine passes finished: A (lexer, value, units, words, parser, phrase, sheet, settings, eval) 819 mutants, 610 caught, 130 missed; B (number, format, functions) 73 mutants, 53 caught, 13 missed. Every survivor now has a test or was dead code and was removed (commits 617fd45, d895b77, 50ecf93). Confirming rerun of all 27 affected functions plus the four `lex_raw` lines is running (773 mutants, log `chain2.log`).
- 13:50 Mutation runs finished. Rows 16 to 20 and the shared engine files have no unexplained survivors; the three accepted equivalents are in `docs/mutation.md` with reasons. Next: final gate and acceptance on the final tree, then the roadmap.
- Final: `bin/check` passed (49 suites, 468 tests, about 634,000 oracle and corpus lines, all guards) and `bin/accept` passed 16 of 16 on the final tree. Roadmap and tickets updated. Nothing was pushed (there is no remote). Open items need you: the maintainer's checks in tickets 0018, 0030, 0043 to 0046 and 0055, the choice about shipped data (0035 to 0038, 0047), judging the app's looks (0052), and the unverified-equivalent mutants listed in `docs/mutation.md`.
