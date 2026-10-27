# Babbage

A precision-first notepad calculator for Linux, in the style of natural-language notepad calculators. You type lines of plain language and
numbers in a sheet and the answers appear beside them. Written in Rust: an exact-arithmetic engine
(`babbage-core`), sheet storage in our own `.bbg` text files (`babbage-store`), and a gpui front end
(`babbage-app`).

## Network use

Babbage works offline. The only thing it can do online is look for a newer version of itself: when it starts, at
most once a week by default, it asks the release page (GitHub unless you change it) for its list of releases. The
request carries your IP address and nothing from your sheets or settings. You can turn it off in Settings,
Updates, or with `BABBAGE_UPDATES=off`, and then Babbage makes no requests at all. Everything it does and keeps is
written down in [docs/updates.md](docs/updates.md).

## Command line

`bbg` is the same program as the window, run as a calculator, so its answers are the window's. `bbg 20% of 85`
prints `17`, `bbg budget.bbg` answers a sheet file, and `babbage calc` does the same as `bbg`. `bbg --help` lists
the options. The one-line installer and the guide's Getting started chapter are the way to get it.

## Building and testing

```sh
bin/build      # release build
bin/check      # format, lint, licences and every unit test
bin/accept     # end-to-end checks against the real window, in a nested headless Hyprland
bin/package    # tarball, AppImage (when appimagetool is installed) and checksums, in dist/
```

`bin/accept` never opens a window on your desktop. The roadmap is in [docs/roadmap.md](docs/roadmap.md) and the
decisions behind the behavior in [docs/decisions.md](docs/decisions.md).
