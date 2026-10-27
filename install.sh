#!/usr/bin/env bash
# Babbage one-line installer. Read this before piping it to bash:
#
#   curl -fsSL https://github.com/jrimmer/babbage/releases/latest/download/install.sh | bash
#
# Flags (pass them through bash -s --):
#
#   --tarball     use the .tar.gz instead of the AppImage
#   --uninstall   remove Babbage
#   --help        show this help
#
# Everything goes under ~/.local and no root is ever used: the program is kept in ~/.local/share/babbage, and
# ~/.local/bin gets two commands, `babbage` (the window, or `babbage calc`) and `bbg` (the command-line calculator).
# Files come from $BABBAGE_RELEASE_URL (the GitHub "latest/download" URL by default). Each one is checked against
# its .sha256 before anything is installed, so a bad download leaves nothing behind.

set -euo pipefail

RELEASE_URL="${BABBAGE_RELEASE_URL:-https://github.com/jrimmer/babbage/releases/latest/download}"
share="$HOME/.local/share/babbage"
bindir="$HOME/.local/bin"
# The commands we write carry this line, so uninstall only removes files this script made.
marker="# Written by the Babbage installer (install.sh)."

usage() {
    cat <<'USAGE'
Babbage one-line installer. Read this before piping it to bash:

  curl -fsSL https://github.com/jrimmer/babbage/releases/latest/download/install.sh | bash

Flags (pass them through bash -s --):

  --tarball     use the .tar.gz instead of the AppImage
  --uninstall   remove Babbage
  --help        show this help

Everything goes under ~/.local and no root is ever used: the program is kept in ~/.local/share/babbage, and
~/.local/bin gets two commands, `babbage` (the window, or `babbage calc`) and `bbg` (the command-line calculator).
Files come from $BABBAGE_RELEASE_URL (the GitHub "latest/download" URL by default). Each one is checked against
its .sha256 before anything is installed, so a bad download leaves nothing behind.
USAGE
}

tarball=false
uninstall=false

for arg in "$@"; do
    case "$arg" in
        --tarball) tarball=true ;;
        --uninstall) uninstall=true ;;
        --help|-h) usage; exit 0 ;;
        *) echo "install.sh: unknown option $arg (try --help)" >&2; exit 2 ;;
    esac
done

# Work in a private directory that goes away even if we fail.
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

is_ours() {
    [ -f "$1" ] && grep -qF "$marker" "$1"
}

# --- uninstall ------------------------------------------------------------

if $uninstall; then
    if [ ! -f "$share/installed-version" ] && ! is_ours "$bindir/babbage" && ! is_ours "$bindir/bbg"; then
        echo "Babbage is not installed."
        exit 0
    fi
    for name in babbage bbg; do
        if is_ours "$bindir/$name"; then
            rm -f "$bindir/$name"
        fi
    done
    rm -f "$share/babbage.AppImage" "$share/babbage" "$share/LICENSE" "$share/README.md" "$share/installed-version"
    rmdir "$share" 2>/dev/null || true
    echo "Babbage is uninstalled."
    exit 0
fi

# --- install --------------------------------------------------------------

if command -v curl >/dev/null 2>&1; then
    fetch() { curl -fsSL "$1" -o "$2"; }
elif command -v wget >/dev/null 2>&1; then
    fetch() { wget -q -O "$2" "$1"; }
else
    echo "install.sh: need curl or wget to download Babbage." >&2
    exit 1
fi

# Download one release file and its .sha256 into $tmp, and check it. Sets $got to the file's path. A mismatch
# aborts the whole run before anything under $HOME is touched.
got=""
download() {
    local name="$1"
    if ! fetch "$RELEASE_URL/$name" "$tmp/$name"; then
        echo "install.sh: could not download $name from $RELEASE_URL" >&2
        exit 1
    fi
    if ! fetch "$RELEASE_URL/$name.sha256" "$tmp/$name.sha256"; then
        echo "install.sh: could not download $name.sha256" >&2
        exit 1
    fi
    if ! ( cd "$tmp" && sha256sum -c --quiet "$name.sha256" ); then
        echo "install.sh: checksum mismatch for $name; nothing was installed." >&2
        exit 1
    fi
    got="$tmp/$name"
}

machine=$(uname -m)
case "$machine" in
    x86_64|aarch64) ;;
    *) echo "install.sh: unsupported architecture '$machine' (Babbage ships x86_64 and aarch64)." >&2; exit 1 ;;
esac

download babbage-version.txt
version=$(tr -d '[:space:]' < "$got")
case "$version" in
    ''|*[!0-9A-Za-z._+-]*) echo "install.sh: the release does not name a usable version." >&2; exit 1 ;;
esac

have=""
if [ -f "$share/installed-version" ]; then
    have=$(cat "$share/installed-version")
fi
if [ "$have" = "$version" ]; then
    echo "Babbage $version is already installed."
    exit 0
fi
if [ -z "$have" ]; then
    echo "Installing Babbage $version…"
else
    echo "Updating Babbage $have → $version…"
fi

# Everything is downloaded and checked first; only then does anything change.
if $tarball; then
    download "babbage-$version-$machine.tar.gz"
    mkdir -p "$tmp/unpack"
    tar -xzf "$got" -C "$tmp/unpack"
    for f in babbage LICENSE README.md; do
        if [ ! -f "$tmp/unpack/$f" ]; then
            echo "install.sh: the tarball has no $f; nothing was installed." >&2
            exit 1
        fi
    done
    mkdir -p "$share"
    install -m755 "$tmp/unpack/babbage" "$share/babbage"
    install -m644 "$tmp/unpack/LICENSE" "$share/LICENSE"
    install -m644 "$tmp/unpack/README.md" "$share/README.md"
    program="$share/babbage"
else
    download "babbage-$version-$machine.AppImage"
    mkdir -p "$share"
    install -m755 "$got" "$share/babbage.AppImage"
    program="$share/babbage.AppImage"
fi

# The two commands are small scripts, because an AppImage sees its own path as argv[0]. `bbg` sets BABBAGE_AS, which
# the program reads as "run the command-line mode".
mkdir -p "$bindir"
write_command() {
    local name="$1" as="$2"
    {
        echo '#!/bin/sh'
        echo "$marker"
        if [ -n "$as" ]; then
            echo "export BABBAGE_AS=$as"
        fi
        printf 'exec "%s" "$@"\n' "$program"
    } >"$bindir/$name"
    chmod 755 "$bindir/$name"
}
write_command babbage ""
write_command bbg bbg

printf '%s\n' "$version" >"$share/installed-version"

echo "Babbage $version is installed. Try: bbg 20% of 85"
case ":$PATH:" in
    *":$bindir:"*) ;;
    *) echo "Note: $bindir is not on your PATH; add it to run 'babbage' and 'bbg' directly." >&2 ;;
esac
