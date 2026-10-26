#!/bin/bash
# A throwaway nested Hyprland with a 60 Hz headless output, so GUI checks get
# real compositor frame callbacks without showing a window on the desktop.
#
#   tools/nested-hyprland.sh start <dir>          start; prints sig=... wayland=...
#   tools/nested-hyprland.sh exec <dir> <script>  run <script> on HEADLESS-2
#   tools/nested-hyprland.sh stop <dir>           exit the nested instance
#   tools/nested-hyprland.sh run <cmd> [args...]  start, run cmd there, wait, stop
#   tools/nested-hyprland.sh card                 the vkms card standalone mode uses
#
# The nested compositor is started on the host's hidden special workspace
# (special:babbage silent). Its own window is never shown and never gets
# frame callbacks; only HEADLESS-2 renders. <dir> holds the state (nested.env,
# hyprland.log). Needs Hyprland 0.55+ with a Lua config on the host.
# Inside, Wayland clients reach the nested compositor through the socket
# printed as wayland=..., e.g. `WAYLAND_DISPLAY=$wayland grim -o HEADLESS-2`.
#
# Standalone mode, for a CI machine with no desktop: when `hyprctl instances`
# lists no host Hyprland (or BABBAGE_NESTED=standalone), start runs Hyprland
# on its own, as the user that runs this (never root, which Hyprland
# refuses), with its own XDG_RUNTIME_DIR under <dir>, printed as runtime=...;
# every client of it needs that XDG_RUNTIME_DIR. Hyprland cannot run on its
# headless backend alone, which has no buffer allocator, so standalone mode
# uses its DRM backend, and only on a vkms card (see vkms_card), the kernel's
# virtual KMS device, which drives no screen: AQ_DRM_DEVICES names that card
# alone, and with no vkms card start refuses. The seat comes from a seatd the
# caller started (LIBSEAT_BACKEND=seatd, never logind or libseat's builtin
# backend), and WAYLAND_DISPLAY is unset in a runtime dir no other compositor
# uses, so it never nests in a Wayland session either. It cannot take a real
# screen or GPU on any machine.
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)

instances() {
  hyprctl instances -j | python3 -c 'import json,sys; [print(i["instance"], i["wl_socket"], i["pid"]) for i in json.load(sys.stdin)]'
}

# Whether there is no host Hyprland to nest in. A hyprctl that cannot list
# instances (no XDG_RUNTIME_DIR, say) counts as none.
standalone() {
  [ "${BABBAGE_NESTED:-}" = standalone ] && return 0
  [ -z "${BABBAGE_HOST_SIG:-}" ] && [ -z "$(instances 2>/dev/null || true)" ]
}

# The device of the first DRM card that is vkms's: its device's driver is
# vkms, or, as Linux 6.1 lays it out, its device is the platform device
# vkms makes for itself, named vkms, which no driver is bound to.
# BABBAGE_DRM_SYSFS stands in for /sys/class/drm in the script's tests.
vkms_card() {
  local sys=${BABBAGE_DRM_SYSFS:-/sys/class/drm} c device
  for c in "$sys"/card*; do
    # card0-Virtual-1 and the like are connectors, not cards.
    case ${c##*/} in *-*) continue ;; esac
    [ -e "$c/device" ] || continue
    device=$(readlink -f "$c/device")
    if [ -e "$c/device/driver" ]; then
      [ "$(basename "$(readlink -f "$c/device/driver")")" = vkms ] || continue
    else
      [ "${device##*/}" = vkms ] || continue
      [ "$(basename "$(readlink -f "$device/subsystem")")" = platform ] || continue
    fi
    echo "/dev/dri/${c##*/}"
    return 0
  done
  return 1
}

host_sig() {
  if [ -n "${BABBAGE_HOST_SIG:-}" ]; then echo "$BABBAGE_HOST_SIG"; return; fi
  # The oldest instance is the user's session; nested ones start later.
  hyprctl instances -j | python3 -c 'import json,sys; l=json.load(sys.stdin); l.sort(key=lambda i:i["time"]); print(l[0]["instance"])'
}

start_standalone() {
  local dir=$1 card
  card=$(vkms_card) || {
    echo "nested-hyprland.sh: no host Hyprland, and no vkms card for a standalone one (standalone mode runs Hyprland's DRM backend on a vkms card only)" >&2
    exit 1
  }
  if [ "$(id -u)" -eq 0 ]; then
    echo "nested-hyprland.sh: Hyprland refuses to run as root; run standalone mode as an unprivileged user" >&2
    exit 1
  fi
  # The seat hands Hyprland the card for its outputs, but its renderer opens
  # the card again by path, as vkms has no render node, so the user needs
  # the card's group (video on Arch) as well.
  if [ ! -r "$card" ] || [ ! -w "$card" ]; then
    echo "nested-hyprland.sh: $(id -un) cannot open $card; Hyprland's renderer opens it by path (vkms has no render node), so the user needs its group ($(stat -c %G "$card" 2>/dev/null || echo unknown))" >&2
    exit 1
  fi
  mkdir -p "$dir/runtime"
  chmod 700 "$dir/runtime"
  local rt
  rt=$(cd "$dir/runtime" && pwd)
  env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET -u DISPLAY -u HYPRLAND_INSTANCE_SIGNATURE \
    XDG_RUNTIME_DIR="$rt" AQ_DRM_DEVICES="$card" LIBSEAT_BACKEND=seatd \
    Hyprland -c "$here/nested.lua" < /dev/null > "$dir/hyprland.log" 2>&1 &
  local pid=$! sig="" wl="" s w p
  for _ in $(seq 1 50); do
    while read -r s w p; do
      if [ "$p" = "$pid" ]; then sig=$s; wl=$w; fi
    done < <(XDG_RUNTIME_DIR=$rt instances 2>/dev/null || true)
    [ -n "$sig" ] && break
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.2
  done
  if [ -z "$sig" ]; then
    echo "standalone Hyprland did not start; the end of $dir/hyprland.log:" >&2
    tail -n 30 "$dir/hyprland.log" >&2
    kill "$pid" 2>/dev/null || true
    exit 1
  fi
  sleep 0.5
  XDG_RUNTIME_DIR=$rt HYPRLAND_INSTANCE_SIGNATURE=$sig hyprctl output create headless HEADLESS-2 >/dev/null
  sleep 0.5
  printf 'sig=%s\nwayland=%s\npid=%s\nruntime=%s\n' "$sig" "$wl" "$pid" "$rt" | tee "$dir/nested.env"
}

start() {
  local dir=$1
  if standalone; then
    start_standalone "$dir"
    return
  fi
  mkdir -p "$dir"
  local before
  before=$(instances | cut -d' ' -f1)
  cat > "$dir/nested.sh" <<EOF
#!/bin/bash
unset HYPRLAND_INSTANCE_SIGNATURE
exec Hyprland -c "$here/nested.lua" > "$dir/hyprland.log" 2>&1
EOF
  chmod +x "$dir/nested.sh"
  HYPRLAND_INSTANCE_SIGNATURE=$(host_sig) hyprctl dispatch \
    "hl.dsp.exec_cmd(\"$dir/nested.sh\", {workspace = \"special:babbage silent\"})" >/dev/null
  local sig="" wl="" pid=""
  for _ in $(seq 1 50); do
    while read -r s w p; do
      grep -qx "$s" <<<"$before" || { sig=$s; wl=$w; pid=$p; }
    done < <(instances)
    [ -n "$sig" ] && break
    sleep 0.2
  done
  [ -n "$sig" ] || { echo "nested Hyprland did not start" >&2; exit 1; }
  sleep 0.5
  HYPRLAND_INSTANCE_SIGNATURE=$sig hyprctl output create headless HEADLESS-2 >/dev/null
  sleep 0.5
  printf 'sig=%s\nwayland=%s\npid=%s\n' "$sig" "$wl" "$pid" | tee "$dir/nested.env"
}

load() {
  runtime=""
  # shellcheck disable=SC1091
  source "$1/nested.env"
  # A standalone instance is reached only through its own runtime dir.
  if [ -n "$runtime" ]; then export XDG_RUNTIME_DIR=$runtime; fi
}

exec_in() {
  load "$1"
  HYPRLAND_INSTANCE_SIGNATURE=$sig hyprctl dispatch \
    "hl.dsp.exec_cmd(\"$2\", {workspace = \"2 silent\"})" >/dev/null
}

stop() {
  [ -f "$1/nested.env" ] || return 0
  load "$1"
  HYPRLAND_INSTANCE_SIGNATURE=$sig hyprctl dispatch 'hl.dsp.exit()' >/dev/null 2>&1 || true
  for _ in $(seq 1 25); do
    kill -0 "$pid" 2>/dev/null || return 0
    sleep 0.2
  done
  # Only the instance this script started, by its own pid.
  kill "$pid" 2>/dev/null || true
}

case "${1:-}" in
  start) start "$2" ;;
  exec) exec_in "$2" "$3" ;;
  stop) stop "$2" ;;
  card) vkms_card ;;
  run)
    shift
    dir=$(mktemp -d)
    trap 'stop "$dir"; rm -rf "$dir"' EXIT
    start "$dir" >/dev/null
    {
      echo '#!/bin/bash'
      printf 'echo $$ > %q\n' "$dir/app.pid"
      printf 'exec'; printf ' %q' "$@"; printf ' > %q 2> %q\n' "$dir/app.out" "$dir/app.err"
    } > "$dir/app.sh"
    chmod +x "$dir/app.sh"
    exec_in "$dir" "$dir/app.sh"
    for _ in $(seq 1 50); do [ -s "$dir/app.pid" ] && break; sleep 0.1; done
    while kill -0 "$(cat "$dir/app.pid")" 2>/dev/null; do sleep 0.5; done
    cat "$dir/app.out"
    cat "$dir/app.err" >&2
    ;;
  *)
    sed -n '2,9p' "$0" >&2
    exit 2
    ;;
esac
