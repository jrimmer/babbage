#!/usr/bin/env python3
"""A scripted input method for a Wayland compositor, with no dependencies.

    tools/ime.py <wayland-socket>

Speaks zwp_input_method_v2, the protocol a real input method (fcitx, ibus) uses to put composition text into the focused
application, so the application's own text-input path is what runs. Reads one command per line on stdin and answers
"ok" (or "error: ...") after each; it exits when stdin closes:

    wait            block until a text field is focused and has enabled text input
    preedit TEXT    show TEXT as the composition (underlined, not yet part of the text), the caret at its end
    commit TEXT     end the composition and put TEXT in the text
    clear           take the composition away without committing anything

The process has to stay alive for the whole session: the compositor lets one input method own a seat, and an
application only enables text input once there is one. The socket is the nested compositor's, never the host's.
"""
import os
import select
import socket
import struct
import sys
import time


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

    def pump(self, timeout):
        """Reads what has arrived (waiting up to `timeout`) and returns the complete events."""
        ready, _, _ = select.select([self.sock], [], [], timeout)
        if ready:
            data = self.sock.recv(65536)
            if not data:
                raise SystemExit("compositor closed the connection")
            self.buf += data
        out = []
        while len(self.buf) >= 8:
            obj, word = struct.unpack_from("<II", self.buf)
            size, opcode = word >> 16, word & 0xFFFF
            if len(self.buf) < size:
                break
            body, self.buf = self.buf[8:size], self.buf[size:]
            out.append((obj, opcode, body))
        return out


def main(argv):
    if len(argv) != 1:
        raise SystemExit(__doc__)
    sys.stdin.reconfigure(encoding="utf-8")
    wire = Wire(argv[0])
    registry = wire.new_id()
    wire.send(1, 1, struct.pack("<I", registry))
    sync = wire.new_id()
    wire.send(1, 0, struct.pack("<I", sync))
    globals_ = {}
    done = False
    while not done:
        for obj, opcode, body in wire.pump(5):
            if obj == registry and opcode == 0:
                name, n = struct.unpack_from("<II", body)
                iface = body[8 : 8 + n - 1].decode()
                version = struct.unpack_from("<I", body, 8 + n + (-n % 4))[0]
                globals_[iface] = (name, version)
            elif obj == sync:
                done = True
    for need in ("wl_seat", "zwp_input_method_manager_v2"):
        if need not in globals_:
            raise SystemExit(f"{argv[0]} has no {need}")

    def bind(iface, version):
        name, offered = globals_[iface]
        new = wire.new_id()
        wire.send(registry, 0, struct.pack("<I", name) + string(iface) + struct.pack("<II", min(version, offered), new))
        return new

    seat = bind("wl_seat", 1)
    manager = bind("zwp_input_method_manager_v2", 1)
    method = wire.new_id()
    wire.send(manager, 0, struct.pack("<II", seat, method))

    state = {"serial": 0, "active": False, "pending_active": False, "unavailable": False}

    def handle(events):
        for obj, opcode, _ in events:
            if obj != method:
                continue
            if opcode == 0:      # activate
                state["pending_active"] = True
            elif opcode == 1:    # deactivate
                state["pending_active"] = False
            elif opcode == 5:    # done
                state["serial"] += 1
                state["active"] = state["pending_active"]
            elif opcode == 6:    # unavailable: another input method owns the seat
                state["unavailable"] = True

    def settle(seconds):
        end = time.monotonic() + seconds
        while time.monotonic() < end:
            handle(wire.pump(0.05))

    def commit():
        wire.send(method, 3, struct.pack("<I", state["serial"]))

    settle(0.3)
    if state["unavailable"]:
        raise SystemExit("another input method owns the seat")
    print("ready", flush=True)
    for line in sys.stdin:
        line = line.rstrip("\n")
        verb, _, rest = line.partition(" ")
        if verb == "wait":
            end = time.monotonic() + 10
            while not state["active"] and time.monotonic() < end:
                handle(wire.pump(0.1))
            print("ok" if state["active"] else "error: no text field enabled text input in 10 s", flush=True)
            continue
        handle(wire.pump(0))
        if not state["active"]:
            print("error: no text field has text input enabled", flush=True)
            continue
        if verb == "preedit":
            raw = rest.encode()
            wire.send(method, 1, string(rest) + struct.pack("<ii", len(raw), len(raw)))
            commit()
        elif verb == "commit":
            wire.send(method, 0, string(rest))
            commit()
        elif verb == "clear":
            wire.send(method, 1, string("") + struct.pack("<ii", 0, 0))
            commit()
        else:
            print(f"error: {line}", flush=True)
            continue
        settle(0.3)
        print("ok", flush=True)


if __name__ == "__main__":
    main(sys.argv[1:])
