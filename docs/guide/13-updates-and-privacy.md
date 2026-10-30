# Updates and privacy

Babbage is an offline calculator. Your sheets stay on your computer, nothing is uploaded, and there is no account. The one thing Babbage can do online is look for a newer version of itself, and you can turn even that off.

This chapter says exactly what that check is, what it sends, what Babbage keeps on your disk, and how installing an update works, so none of it is a surprise.

## What Babbage does on the network

Once in a while, when it starts, Babbage asks the place it was released from for a list of released versions. That is one request. If a newer version exists, a small button appears; if not, nothing happens. Babbage downloads nothing until you press **Install**.

By default the list comes from GitHub (`github jrimmer/babbage`), so the request goes to `api.github.com`.

### What the request carries

Only what any web request carries: your computer's internet address, and the name of the update library (`praeco`, with a version number) as the program making the request. It does not carry your sheets, your settings, the version of Babbage you run or any identifier for you or your computer.

## When it checks

Babbage checks when it starts, and only if the last check was longer ago than the interval you chose:

| Setting | Checks at start if the last check was more than |
|---|---|
| Off | never |
| Monthly | 30 days ago |
| Weekly (the default) | 7 days ago |
| Daily | 1 day ago |

It also checks at the first start after you change the source or install a new version, since the old answer no longer applies.

A check that fails, because you are offline or there is no release yet, still counts as the check. An unreachable source is not asked again at every start, and nothing is shown to you.

Change the interval in [Settings, Updates](12-settings.html#updates), under **Look for a new version**.

## Turning it off

There are two ways, and either is enough.

- **In Settings.** Open Settings, choose the **Updates** pane, and set **Look for a new version** to **Off**. Babbage then makes no requests on its own.
- **With an environment variable.** Start Babbage with `BABBAGE_UPDATES=off`. This turns the automatic check off whatever the setting says. Use it for a copy of Babbage that must never go online, and put it in your launcher or shell profile to make it permanent.

```text
BABBAGE_UPDATES=off babbage
```

Either way, the **Check now** button on the Updates pane still asks, because you pressed it. If you never press it, Babbage never goes online.

## Checking now

The **Check now** button asks right away, whatever the interval. The notice at the bottom of the sidebar then tells you the result: that a version is available, that Babbage is up to date, or that it could not check, with the reason.

## When there is an update

If a newer version exists, an **Update 1.2.3** button (with the real version number) appears in the toolbar beside the settings button. Click it to open a panel that says which version is available and which one you have, followed by the release notes of every version newer than yours, newest first. It has three buttons:

| Button | What it does |
|---|---|
| **Install** | Downloads the update and installs it (see below) |
| **Later** | Closes the panel and leaves the button where it is |
| **Skip this version** | Hides the button until a version newer than that one comes out |

You can also close the panel with **Escape**.

### Installing

Pressing **Install** downloads the file made for your computer from the release, checks that it is intact, and only then replaces the copy of Babbage you are running. The panel shows what it is doing as it goes. What happens next depends on how you installed Babbage:

| You installed Babbage as | What Install does |
|---|---|
| An AppImage | The file is replaced where it is, with no administrator rights |
| A package (Pacman, deb or rpm) | The package manager installs the checked file after your system's own password prompt |
| A folder in your home directory (a tarball) | The files in that folder are replaced, with no administrator rights |
| Built from source | Babbage says it cannot update this copy, and why |

When it finishes, the panel says the new version is installed and offers **Restart Babbage**. That saves your sheets, starts the new version and quits the old one. Choose **Later** instead to keep working and let the new version start the next time you open Babbage.

If something goes wrong, the panel says what.

### The checksum check

Each release publishes a checksum (a SHA-256 fingerprint) for its files. Before replacing anything, Babbage works out the fingerprint of what it downloaded and compares. If they differ, nothing is installed. This catches a download that was damaged on the way or a file that was swapped on a mirror.

Updates are not signed, so install them only from a source you trust, or leave the check off and update the way you installed.

## Another place to look

**Where to look** on the Updates pane (the `update_source` setting) names the release source. Write it as:

```text
github owner/repo
forgejo https://host owner/repo
```

A bare `owner/repo` means GitHub. Only public sources work; private repositories are not supported. If the text does not fit one of these forms, the check tells you how to write it.

## What Babbage keeps

All of it is on your own computer, in ordinary files:

| What | Where |
|---|---|
| Your sheets, and their `exports` and `.trash` folders | `~/.local/share/babbage/sheets`, or the folder in `BABBAGE_DATA_DIR`; see [Sheets and files](10-sheets-and-files.html#where-sheets-are-stored) |
| Your settings, including the update interval, the source and a skipped version | `~/.config/babbage/settings`; see [Settings](12-settings.html#where-settings-are-kept) |
| The answer to the last update check: when it was asked, which version was running, and the notes of any newer release | `~/.local/state/babbage/updates.json` (under `$XDG_STATE_HOME` if you set it) |
| A small note the update library keeps so repeat requests are cheaper | `~/.cache/praeco` |

The update answer is what keeps the **Update** button showing between checks without asking again. Deleting that file is harmless: Babbage simply checks again at its next start (unless updates are off).

Nothing in any of these files is sent anywhere. To remove everything Babbage has ever stored, delete those folders.
