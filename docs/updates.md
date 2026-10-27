# Updates and the network

Babbage is an offline calculator. The one thing it can do online is look for a newer version of itself. This page
says exactly what that is, so nothing about it is a surprise. The same text belongs in every release note and
announcement (a short form is at the end).

## What happens

- **When.** When Babbage starts, and only if the last check was more than a week ago. The choices in Settings,
  Updates, are Off, Monthly, Weekly (the default) and Daily. A check that fails (no network, no release yet) counts
  as the check, so an unreachable source is not asked again at every start.
- **What is asked.** One request to the release source for its list of releases. By default the source is
  `github jrimmer/babbage`, so the request goes to `api.github.com`. Nothing else is requested unless you press
  Install.
- **What is sent.** What any web request carries: your IP address, and the name of the update library
  (`praeco/0.1.1`) as the user agent. No sheet, no setting, no version number of Babbage, no identifier. The
  library keeps a small cache of the answer's ETag under `~/.cache/praeco`.
- **What is kept.** The answer, in `~/.local/state/babbage/updates.json` (or under `$XDG_STATE_HOME`): when it was
  asked, which version was running, and the notes of any newer release. That is what keeps the Update pill up
  between checks without asking again.
- **What you see.** If a newer release exists, an "Update 1.2.3" pill appears beside the toolbar buttons. It opens a
  sheet with the notes of every release newer than yours, newest first, and three buttons: Install, Later and
  Skip this version. Later closes the sheet and leaves the pill. Skip hides that version until a newer one
  appears. Nothing is downloaded until you press Install.

## Turning it off

- **Settings, Updates, Off.** Babbage then makes no requests at all. Check now (the button on the same pane) still
  asks, because you pressed it.
- **`BABBAGE_UPDATES=off` in the environment** turns the check off whatever the settings say. Use it for an
  install that must never go online.
- The settings file (`update_check = off`) holds the same choice, along with `update_source` and `update_skip`.

## Another source

`update_source` is `github owner/repo` or `forgejo https://host owner/repo`. A bare `owner/repo` means GitHub.
Private repositories are not supported.

## Installing

Install downloads the file for your machine from the release, checks its SHA-256 against the checksum published
with the release, and only then replaces the installed copy:

| Installed as | What happens |
|---|---|
| AppImage | the file is replaced in place, no administrator rights |
| Pacman, deb or rpm package | the package manager installs the verified file after the system's own password prompt (`pkexec`) |
| A folder under your home (tarball) | the files are replaced below that folder, no administrator rights |
| Built from source | Install says it cannot update this copy and why |

Then "Restart Babbage" saves your sheets, starts the new version and quits this one.

**Limit, stated plainly:** the checksum comes from the same release as the file, so it catches a damaged or
tampered download or mirror, but not a compromised release account. There are no signatures yet. Install only
from a source you trust, or leave the check off and update the way you installed.

## Short form for release notes and announcements

> Babbage works offline. It can check for a new version: when it starts, at most once a week, it asks the release
> page (GitHub by default) for its list of releases. That request carries your IP address and nothing from your
> sheets or settings. Turn it off in Settings, Updates (or set `BABBAGE_UPDATES=off`) and Babbage never goes
> online. Details: `docs/updates.md`.
