#!/usr/bin/env python3
"""An idle keyboard and mouse for the standalone compositor's seat, until killed.

On a desktop the nested Hyprland's seat has the keyboard and pointer the
host lends it. A standalone Hyprland (tools/nested-hyprland.sh) on a CI
machine has no input device at all, so its seat has neither:

- With no keyboard no window gets the keyboard focus, and
  `hl.dsp.send_shortcut` refuses ("no keyboard").
- With no pointer the seat gains the pointer capability only when a check
  makes its virtual pointer, so the app binds its wl_pointer only then,
  misses the first motion, and gets an enter at (-1, -1): a hover, scroll
  or drag that follows a single move lands there.

This makes one of each through uinput, which the compositor reads through
libinput (the keyboard on its own keymap), as it would devices plugged in.
They never send an event; the checks type and point through the
compositor, wtype and their virtual pointers.

It needs write access to /dev/uinput (root), and it refuses to run where
there is no vkms card, so it only ever adds devices to a CI machine whose
compositor is the standalone one, never to a desktop.

Usage: tools/ci-input.py (as root; stop it with SIGTERM)
"""

import fcntl
import os
import signal
import struct
import subprocess
import sys

UI_SET_EVBIT = 0x40045564
UI_SET_KEYBIT = 0x40045565
UI_SET_RELBIT = 0x40045566
UI_DEV_CREATE = 0x5501
UI_DEV_DESTROY = 0x5502
EV_KEY = 0x01
EV_REL = 0x02
REL_X = 0x00
REL_Y = 0x01
REL_WHEEL = 0x08
BTN_LEFT = 0x110
BTN_RIGHT = 0x111
BTN_MIDDLE = 0x112
BUS_VIRTUAL = 0x06
# KEY_ESC (1) to KEY_MICMUTE (248): every key of a full keyboard.
KEYS = range(1, 249)


def device(name, events, keys, rels):
    fd = os.open("/dev/uinput", os.O_WRONLY | os.O_NONBLOCK)
    for event in events:
        fcntl.ioctl(fd, UI_SET_EVBIT, event)
    for key in keys:
        fcntl.ioctl(fd, UI_SET_KEYBIT, key)
    for rel in rels:
        fcntl.ioctl(fd, UI_SET_RELBIT, rel)
    # struct uinput_user_dev: name[80], struct input_id (bustype, vendor,
    # product, version), ff_effects_max, then absmax, absmin, absfuzz and
    # absflat, 64 each.
    dev = struct.pack("=80s4HI256i", name, BUS_VIRTUAL, 1, 1, 1, 0, *([0] * 256))
    os.write(fd, dev)
    fcntl.ioctl(fd, UI_DEV_CREATE)
    return fd


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    card = subprocess.run(
        [os.path.join(here, "nested-hyprland.sh"), "card"],
        capture_output=True,
        text=True,
    )
    if card.returncode != 0:
        sys.exit("ci-input.py: no vkms card; these devices are for a CI machine's standalone compositor only")
    fds = [
        device(b"babbage ci keyboard", [EV_KEY], KEYS, []),
        device(
            b"babbage ci mouse",
            [EV_KEY, EV_REL],
            [BTN_LEFT, BTN_RIGHT, BTN_MIDDLE],
            [REL_X, REL_Y, REL_WHEEL],
        ),
    ]
    print("ci-input.py: babbage ci keyboard and mouse are up", flush=True)

    def stop(*_):
        for fd in fds:
            fcntl.ioctl(fd, UI_DEV_DESTROY)
            os.close(fd)
        sys.exit(0)

    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    while True:
        signal.pause()


if __name__ == "__main__":
    main()
