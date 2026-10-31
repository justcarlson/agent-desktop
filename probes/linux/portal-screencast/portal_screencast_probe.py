#!/usr/bin/env python3
"""Portal ScreenCast feasibility probe for R7 on Hyprland. Runs only inside a throwaway VM.

One run opens a ScreenCast session for a single window, pulls frames through PipeWire, and writes one JSON
record: picker invocations (each one is a dialog under stock xdph), portal response codes, the restore token,
the stream description, and per-frame arrival time, header pts, sequence, corrupted flag and pixel hash.

The `revoke` command deletes a restore token from the permission store, which is how a user withdraws it.
"""

import argparse
import hashlib
import json
import os
import subprocess
import sys
import time

import gi

gi.require_version("Gst", "1.0")
from gi.repository import Gio, GLib, Gst  # noqa: E402

PORTAL_BUS = "org.freedesktop.portal.Desktop"
PORTAL_PATH = "/org/freedesktop/portal/desktop"
SCREENCAST = "org.freedesktop.portal.ScreenCast"
PERMISSION_STORE_BUS = "org.freedesktop.impl.portal.PermissionStore"
PERMISSION_STORE_PATH = "/org/freedesktop/impl/portal/PermissionStore"
SOURCE_WINDOW = 2
PICKER_LOG = "/tmp/xdph-portal-probe/picker.log"


def refuse_outside_vm():
    if os.environ.get("PORTAL_PROBE_VM") != "1":
        sys.exit("refused: set PORTAL_PROBE_VM=1, and only inside a throwaway VM")
    virt = subprocess.run(["systemd-detect-virt", "--vm"], capture_output=True, text=True, check=False)
    if virt.returncode != 0:
        sys.exit("refused: systemd-detect-virt reports no VM; never run this on a host session")


def mono_ns():
    return time.clock_gettime_ns(time.CLOCK_MONOTONIC)


def picker_invocations():
    try:
        with open(PICKER_LOG, encoding="utf-8") as log:
            return sum(1 for line in log if line.startswith("invocation "))
    except FileNotFoundError:
        return 0


class Portal:
    def __init__(self, deadline_s):
        self.bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
        self.sender = self.bus.get_unique_name()[1:].replace(".", "_")
        self.deadline_s = deadline_s
        self.counter = 0

    def token(self):
        self.counter += 1
        return f"adprobe{os.getpid()}_{self.counter}"

    def request(self, method, leading, options):
        handle_token = self.token()
        options = dict(options, handle_token=GLib.Variant("s", handle_token))
        path = f"/org/freedesktop/portal/desktop/request/{self.sender}/{handle_token}"
        reply = {}

        def on_response(_bus, _sender, _path, _iface, _signal, params):
            reply["code"], reply["results"] = params.unpack()

        sub = self.bus.signal_subscribe(
            PORTAL_BUS, "org.freedesktop.portal.Request", "Response", path, None, Gio.DBusSignalFlags.NONE, on_response
        )
        params = GLib.Variant(f"({''.join(v.get_type_string() for v in leading)}a{{sv}})", (*[v.unpack() for v in leading], options))
        self.bus.call_sync(PORTAL_BUS, PORTAL_PATH, SCREENCAST, method, params, None, Gio.DBusCallFlags.NONE, -1, None)
        context = GLib.MainContext.default()
        end = time.monotonic() + self.deadline_s
        while "code" not in reply and time.monotonic() < end:
            context.iteration(False)
            time.sleep(0.005)
        self.bus.signal_unsubscribe(sub)
        return reply.get("code", "timeout"), reply.get("results", {})

    def open_pipewire_remote(self, session):
        params = GLib.Variant("(oa{sv})", (session, {}))
        result, fds = self.bus.call_with_unix_fd_list_sync(
            PORTAL_BUS, PORTAL_PATH, SCREENCAST, "OpenPipeWireRemote", params, GLib.VariantType("(h)"),
            Gio.DBusCallFlags.NONE, -1, None, None,
        )
        return fds.get(result.unpack()[0])

    def close(self, session):
        self.bus.call_sync(PORTAL_BUS, session, "org.freedesktop.portal.Session", "Close", None, None,
                           Gio.DBusCallFlags.NONE, -1, None)


def pull_frames(fd, node, count, deadline_s):
    Gst.init(None)
    pipeline = Gst.parse_launch(
        f"pipewiresrc fd={fd} path={node} keepalive-time=0 always-copy=true do-timestamp=false "
        "! video/x-raw ! appsink name=sink sync=false max-buffers=16 drop=false"
    )
    sink = pipeline.get_by_name("sink")
    pipeline.set_state(Gst.State.PLAYING)
    frames = []
    end = time.monotonic() + deadline_s
    while len(frames) < count and time.monotonic() < end:
        sample = sink.try_pull_sample(int(0.05 * Gst.SECOND))
        if sample is None:
            continue
        arrived = mono_ns()
        buf = sample.get_buffer()
        caps = sample.get_caps().get_structure(0)
        ok, info = buf.map(Gst.MapFlags.READ)
        digest = hashlib.sha256(bytes(info.data)).hexdigest()[:16] if ok else None
        if ok:
            buf.unmap(info)
        frames.append({
            "arrived_mono_ns": arrived,
            "pts_ns": buf.pts if buf.pts != Gst.CLOCK_TIME_NONE else None,
            "seq": buf.offset,
            "corrupted": bool(buf.get_flags() & Gst.BufferFlags.CORRUPTED),
            "size": [caps.get_value("width"), caps.get_value("height")],
            "sha": digest,
        })
    pipeline.set_state(Gst.State.NULL)
    return frames


def run(args):
    portal = Portal(args.deadline_s)
    picker_before = picker_invocations()
    record = {"label": args.label, "persist_mode": args.persist_mode, "restore_token_in": args.restore_token}

    code, results = portal.request("CreateSession", [], {"session_handle_token": GLib.Variant("s", portal.token())})
    record["create_code"] = code
    session = results.get("session_handle")
    if code != 0 or not session:
        return finish(record, picker_before)

    options = {
        "types": GLib.Variant("u", SOURCE_WINDOW),
        "multiple": GLib.Variant("b", False),
        "persist_mode": GLib.Variant("u", args.persist_mode),
    }
    if args.restore_token:
        options["restore_token"] = GLib.Variant("s", args.restore_token)
    record["select_code"], _ = portal.request("SelectSources", [GLib.Variant("o", session)], options)

    record["request_mono_ns"] = mono_ns()
    code, results = portal.request("Start", [GLib.Variant("o", session), GLib.Variant("s", "")], {})
    record["start_code"] = code
    record["restore_token_out"] = results.get("restore_token")
    streams = results.get("streams", [])
    record["streams"] = [{"node": node, **{k: v for k, v in props.items()}} for node, props in streams]
    if code == 0 and streams:
        fd = portal.open_pipewire_remote(session)
        frames = pull_frames(fd, streams[0][0], args.frames, args.deadline_s)
        for frame in frames:
            frame["pts_after_request"] = frame["pts_ns"] is not None and frame["pts_ns"] > record["request_mono_ns"]
            frame["latency_ms"] = (frame["arrived_mono_ns"] - record["request_mono_ns"]) / 1e6
        record["frames"] = frames
    portal.close(session)
    return finish(record, picker_before)


def finish(record, picker_before):
    record["picker_invocations"] = picker_invocations() - picker_before
    print(json.dumps(record, default=str))
    return 0


def revoke(args):
    bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
    bus.call_sync(PERMISSION_STORE_BUS, PERMISSION_STORE_PATH, PERMISSION_STORE_BUS, "Delete",
                  GLib.Variant("(ss)", ("screencast", args.token)), None, Gio.DBusCallFlags.NONE, -1, None)
    print(json.dumps({"revoked": args.token}))
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    run_parser = sub.add_parser("run")
    run_parser.add_argument("--label", required=True)
    run_parser.add_argument("--persist-mode", type=int, default=2)
    run_parser.add_argument("--restore-token")
    run_parser.add_argument("--frames", type=int, default=3)
    run_parser.add_argument("--deadline-s", type=float, default=5.0)
    revoke_parser = sub.add_parser("revoke")
    revoke_parser.add_argument("--token", required=True)
    args = parser.parse_args()
    refuse_outside_vm()
    return run(args) if args.command == "run" else revoke(args)


if __name__ == "__main__":
    sys.exit(main())
