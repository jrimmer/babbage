# Running the whole gate on a Spoond MicroVM

`bin/check` and `bin/accept` also run on a machine with no desktop: a Spoond lease of the `rust-gpui-accept` image. This page is what that took, for the next app that wants it.

## Run it

```
POOL_PROJECT=<your project> sb-run --root --image rust-gpui-accept \
  --allow index.crates.io,static.crates.io,crates.io \
  REPO_DIR COMMIT -- sh tools/ci-gate
```

`tools/ci-gate` builds, runs `bin/check`, then `bin/accept`, timing each, and ends with `gate: pass` or `gate: FAIL`. It needs a token for your project in `~/.config/pool/<project>.token` (the owner asks the Spoond operator for it). One lease holds 16 GiB, about a project's whole fair share, so run one at a time and let `sb-run` delete it.

## What the image needs, and what bit us

| Problem | Cause | What the script does |
|---|---|---|
| Hyprland will not start | it refuses root | everything runs as a throwaway user on a copy of the checkout |
| Build fails with `/root/.cache/sccache` permission errors | the image sets `RUSTC_WRAPPER=sccache` with a cache under `/root` | unsets `RUSTC_WRAPPER` for that user |
| Out of space | the lease's `/tmp` is a small tmpfs | the copy and `TMPDIR` are on the root disk, under the user's home |
| No keyboard focus, hover and scroll land wrong | a standalone compositor has no input devices | `tools/ci-input.py` adds an idle keyboard and mouse through uinput |
| Hyprland has no seat | no logind | `seatd -u <user>`, with `LIBSEAT_BACKEND=seatd` (set by `tools/nested-hyprland.sh`) |
| Every click and hover is off | the VM's own 1024x768 output lands at x=5048, so no output is at 0,0 and the layout box starts at x=3000; Hyprland maps an absolute pointer move onto that box | `tools/pointer.py` takes the box's origin, and the harness measures it (`pointer_layout`) |
| `export` fails | the image has no `qpdf` or `pdftotext` | requested for the image (the recipe is casabon2's); the check now says which tool is missing |

`/dev/dri/card0` (a virtual KMS display) is present on the image; without it `tools/nested-hyprland.sh` refuses to start a standalone compositor.

## Measured on 2026-10-30 (8 vCPU, no GPU, everything drawn in software)

| Phase | Time |
|---|---|
| Cold release build | 500 s |
| `bin/check`, cold (compiles the test and clippy profiles) | 703 s |
| `bin/check`, warm | 23 s for 725 tests |
| `bin/accept`, 61 of 62 checks passing (only `export` fails, for the missing tools) | 241 s |

The three phases are one run of `tools/ci-gate` through `sb-run` (the warm `bin/check` time is from an earlier run of the same tree). A cold run is about 24 minutes end to end.

For comparison, on the maintainer's workstation `bin/check` takes about 28 s warm and `bin/accept` about 167 s of check time.

## For another app

Copy `tools/ci-gate`, `tools/ci-input.py`, `tools/nested-hyprland.sh` and `tools/pointer.py`, and keep three rules in the harness: start the compositor through `tools/nested-hyprland.sh` and read its `runtime=` line, point `XDG_RUNTIME_DIR` at it, and give the pointer helper the layout's origin as well as its size.
