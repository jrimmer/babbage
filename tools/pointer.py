#!/usr/bin/env python3
"""A virtual mouse for a Wayland compositor, with no dependencies.

    tools/pointer.py <wayland-socket> <extent-w> <extent-h>

Reads one command per line on stdin and answers "ok" after each; it exits when stdin closes:

    move X Y
    click X Y [left|right] [count]

X and Y are layout points. The extents are the size of the compositor's whole layout (every output
together), which the protocol needs to turn a point into an absolute position. It speaks the
zwlr_virtual_pointer_v1 protocol, which the nested Hyprland offers, so the real pointer path (focus,
hit testing, click counts) is what the app sees. The process has to stay alive for the whole session:
a seat with no pointer device tells its clients it has no pointer, and an app that started without one
never listens for the mouse. The socket is the nested compositor's, never the host's.
"""
import os
import socket
import struct
import sys
import time

BUTTONS = {"left": 0x110, "right": 0x111}


def string(s):
    raw = s.encode() + b"\0"
    return struct.pack("<I", len(raw)) + raw + b"\0" * (-len(raw) % 4)


class Wire:
    def __init__(self, name):
        path = name if name.startswith("/") else os.path.join(os.environ["XDG_RUNTIME_DIR"], name)
        self.sock = socket.socket(socket.AF_UNIX)
        self.sock.connect(path)
        self.buf = b""
        self.next_id = 2

    def new_id(self):
        self.next_id += 1
        return self.next_id - 1

    def send(self, obj, opcode, args=b""):
        self.sock.sendall(struct.pack("<II", obj, ((8 + len(args)) << 16) | opcode) + args)

    def events(self):
        while True:
            while len(self.buf) >= 8:
                obj, word = struct.unpack_from("<II", self.buf)
                size, opcode = word >> 16, word & 0xFFFF
                if len(self.buf) < size:
                    break
                body, self.buf = self.buf[8:size], self.buf[size:]
                yield obj, opcode, body
            data = self.sock.recv(65536)
            if not data:
                raise SystemExit("compositor closed the connection")
            self.buf += data


def main(argv):
    if len(argv) != 3:
        raise SystemExit(__doc__)
    wayland, ext_w, ext_h = argv[0], int(argv[1]), int(argv[2])
    wire = Wire(wayland)
    registry = wire.new_id()
    wire.send(1, 1, struct.pack("<I", registry))
    sync = wire.new_id()
    wire.send(1, 0, struct.pack("<I", sync))
    globals_ = {}
    for obj, opcode, body in wire.events():
        if obj == registry and opcode == 0:
            name, n = struct.unpack_from("<II", body)
            globals_[body[8 : 8 + n - 1].decode()] = (name, struct.unpack_from("<I", body, 8 + n + (-n % 4))[0])
        elif obj == sync:
            break
    iface = "zwlr_virtual_pointer_manager_v1"
    if iface not in globals_:
        raise SystemExit(f"{wayland} has no {iface}")
    name, version = globals_[iface]
    manager = wire.new_id()
    wire.send(registry, 0, struct.pack("<I", name) + string(iface) + struct.pack("<II", min(version, 2), manager))
    pointer = wire.new_id()
    wire.send(manager, 0, struct.pack("<II", 0, pointer))

    def now():
        return int(time.monotonic() * 1000) & 0xFFFFFFFF

    def roundtrip():
        done = wire.new_id()
        wire.send(1, 0, struct.pack("<I", done))
        for obj, _, _ in wire.events():
            if obj == done:
                return

    def motion(x, y):
        wire.send(pointer, 1, struct.pack("<IIIII", now(), x, y, ext_w, ext_h))
        wire.send(pointer, 4)

    def button(which, state):
        wire.send(pointer, 2, struct.pack("<III", now(), BUTTONS[which], state))
        wire.send(pointer, 4)

    roundtrip()
    print("ready", flush=True)
    for line in sys.stdin:
        words = line.split()
        if not words:
            continue
        verb = words[0]
        if verb not in ("move", "click") or len(words) < 3:
            print(f"error: {line.strip()}", flush=True)
            continue
        x, y = int(float(words[1])), int(float(words[2]))
        which = words[3] if len(words) > 3 else "left"
        count = int(words[4]) if len(words) > 4 else 1
        if which not in BUTTONS:
            print(f"error: no button {which}", flush=True)
            continue
        motion(x, y)
        time.sleep(0.15)
        if verb == "click":
            for _ in range(count):
                button(which, 1)
                time.sleep(0.04)
                button(which, 0)
                time.sleep(0.06)
        roundtrip()
        time.sleep(0.1)
        print("ok", flush=True)


if __name__ == "__main__":
    main(sys.argv[1:])
