# 0053 The bundled time zone database is 2025b and newer rules exist

Status: built, one part open
Milestone: v1 (before release)
Area: time zones

The engine reads zone rules from `chrono-tz` 0.10.4, which bundles IANA release 2025b (March 2025). It is the newest `chrono-tz` published. The oracle is pinned to the same release on purpose, so the two agree with each other and would both be wrong about a change made after 2025b.

Measured with `tools/tzdiff.py` against the newest tzdata on this machine (2026.5), comparing every zone's UTC offset weekly from 2020 to 2045:
- Africa/Casablanca and Africa/El_Aaiun differ from 2026-09-23 onwards.
- America/Vancouver, Edmonton, Winnipeg, Inuvik, Rainy_River and Yellowknife (and the `Canada/` aliases) differ from 2026-11-04 onwards.
- Nothing differs before those dates, and no zone was added or removed.

So answers about Morocco from late September 2026 and western Canada from November 2026 can be an hour off.

Options, with a recommendation:
1. Keep the bundled database and update the dependency whenever a newer `chrono-tz` ships, re-pinning the oracle in the same change. Deterministic, but it lags.
2. Switch to a crate that bundles a newer release (`jiff-tzdb`, `tzdb`); the data release inside each is not stated and has to be checked first.
3. Generate the tables from the IANA source in a build step. Fresh, but adds a network or vendoring step to every build.
4. Read the machine's own zoneinfo in the app and keep the bundled copy as a fallback. Fresh in the app, but results then vary by machine and the engine tests need the bundled path.

Recommendation: 2 or 4 for the app, with the engine tests staying on a pinned release. Choosing between them is the maintainer's call because it trades repeatability against freshness.

## Decision and what was built (2026-10-26)

The maintainer chose option 4. The app reads the machine's zoneinfo (`/usr/share/zoneinfo`, or `BABBAGE_TZDIR`) and falls back to the bundled 2025b database for any zone the folder lacks. The engine's default is still the bundled database, so its tests give the same answers on every machine. `tz_source = bundled` in the app's settings file switches the app back. The release in use is reported by the app's state hook (`tzdb`). Tests read real TZif files from tzdata 2026e vendored in `crates/babbage-core/tests/fixtures/zoneinfo`, with 7,926 oracle lines from Python reading the same files, and `bin/accept` has a `tz-database` check that launches the real app under both sources.

Still open: a machine without tzdata silently uses 2025b for every zone (the release shows in the state hook, but nothing warns the user), and the bundled copy still needs a dependency bump when a newer `chrono-tz` ships.
