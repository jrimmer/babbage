# Goal (set by the owner, 2026-10-26)

Implement all 24 rows of `docs/roadmap.md`, with comprehensive passing unit and end-to-end tests,
including UI/UX tests: start the app on buildhost and automate its testing. Work unattended.

## Gate for a row to count as done

1. Built from the reference app docs page for that area (cite the page in the commit body).
2. Independent oracle: Python Fraction, `pint` (exact mode, `tools/setup-oracle.sh`) or a hand formula taken from the reference notes. Never the Rust table checking itself.
3. Unit tests, property tests and an oracle corpus all pass: `bin/check`.
4. Mutation testing has no survivors in the touched modules: `bin/mutate '**/file.rs'`. Loop-counter timeouts count as caught.
5. Committed locally with LR trailers. No push anywhere. No attribution of any kind in commits, docs or code.
6. Counts in reports are measured, never recalled.

## Rules carried over from the owner's other Rust projects

- Never show a window on the owner's desktop. GUI checks run in the nested headless Hyprland (`tools/nested-hyprland.sh`), on a hidden workspace. Close everything started.
- Do not change Omarchy, Hyprland or system configuration. No sudo. Do not kill processes not started here.
- Never weaken a check to make it pass. If a check is wrong, say so and why.
- Where the reference notes are silent on semantics, refuse (`UnsupportedOperation`) and file a ticket. Never guess a value.
- Where the reference notes give a rounded constant and a standard exact definition exists, use the exact one and log it in `docs/units.md`.
- Heavy runs (mutation testing, big test sweeps) are throttled: `nice`, `-j 2`. This is the owner's workstation and a past review swamped it.
- `/tmp` is a shared 16 GB tmpfs. Mutation runs use `TMPDIR` on /home (`bin/mutate` does this).

## Stop-and-ask list

Pushing to any remote, spending money, anything needing a real reference app install to decide, and judging GUI looks.
Everything else: decide, record in `docs/decisions.md`, keep going.
- Kill my own processes by PID or an anchored pattern, never an unanchored pkill -f: the pattern matches the shell running it.
