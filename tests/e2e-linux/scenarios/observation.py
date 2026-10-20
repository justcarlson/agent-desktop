import dataclasses
import contextlib
import json
import subprocess
import threading
from uuid import uuid4

from oracle import atspi_raw as raw
from oracle.proc import members
from runner import apps
from runner.helper import (activate, changed, command, display, managed, nodes,
                           one, ref, require, root, snapshot, trash)
from runner.legs import implementation
from runner.product import run_product
from runner.verdict import LegContext


def refs(data):
    return {item["ref_id"] for item in nodes(data) if isinstance(item.get("ref_id"), str)}


@implementation("observation.qualified_refs_survive_snapshots")
def qualified_refs_survive_snapshots(ctx: LegContext) -> None:
    with managed(ctx, "Calculator") as app:
        target = ref(ctx, app, role="button", name="7")
        tree = root(ctx, app)
        ctx.before("atspi", display(tree))
        with managed(ctx, "Files", (str(ctx.tier.runtime_dir),)) as other:
            snapshot(ctx, other)
            command(ctx, ["click", target], app)
            changed(lambda: display(tree), "7")
            ctx.after("atspi", display(tree))
        activate(one(tree, role=43, name="Undo"))
        changed(lambda: display(tree), "")
        ctx.restored("atspi", display(tree))


@implementation("observation.stale_ref_after_app_killed")
def stale_ref_after_app_killed(ctx: LegContext) -> None:
    baseline = members(ctx.tier.runtime_dir)
    ctx.before("atspi", [item.bus for item in raw.registry(ctx.tier)])
    app = apps.launch(ctx.tier, "Calculator")
    try:
        target = ref(ctx, app, role="button", name="7")
        apps.stop(app)
        after = [item.bus for item in raw.registry(ctx.tier)]
        require(app.owner_names[0] not in after, "dead owner remains registered")
        command(ctx, ["click", target, "--timeout-ms", "0"],
                code=("STALE_REF", "ELEMENT_NOT_FOUND"))
        require([item.bus for item in raw.registry(ctx.tier)] == after)
        ctx.after("atspi", after)
    finally:
        apps.stop(app)
        changed(lambda: members(ctx.tier.runtime_dir), baseline)
        ctx.restored("atspi", [item.bus for item in raw.registry(ctx.tier)])


@implementation("observation.selector_wait_resolves")
def selector_wait_resolves(ctx: LegContext) -> None:
    with managed(ctx, "Calculator") as app:
        tree = root(ctx, app)
        digit = one(tree, role=43, name="7")
        ctx.before("atspi", digit.name)
        response = command(ctx, ["snapshot", "-i", "--wait-for", "button:7",
                                 "--wait-timeout", "2000"], app)
        require(any(item.get("name") == digit.name for item in nodes(response.data)))
        command(ctx, ["snapshot", "-i", "--wait-for", "button:absent-desktop-target",
                      "--wait-timeout", "0"], app, code="TIMEOUT")
        ctx.after("atspi", digit.name)
        require(digit.name == "7")
        ctx.restored("atspi", digit.name)


@implementation("observation.enabled_state_canonical_and_raw")
def enabled_state_canonical_and_raw(ctx: LegContext) -> None:
    with managed(ctx, "Calculator") as app:
        tree = root(ctx, app)
        undo = one(tree, role=43, name="Undo")
        target = ref(ctx, app, role="button", name="Undo")
        ctx.before("atspi", "SENSITIVE" in undo.states)
        require("SENSITIVE" not in undo.states)
        require(command(ctx, ["is", target, "--property", "enabled"], app).data["result"] is False)
        try:
            activate(one(tree, role=43, name="7"))
            changed(lambda: "SENSITIVE" in undo.states, True)
            require(command(ctx, ["is", target, "--property", "enabled"], app).data["result"] is True)
            states = command(ctx, ["get", target, "--property", "states"], app).data["value"]
            require("enabled" in states and "disabled" not in states)
            ctx.after("atspi", "SENSITIVE" in undo.states)
        finally:
            activate(undo)
            changed(lambda: "SENSITIVE" in undo.states, False)
            ctx.restored("atspi", "SENSITIVE" in undo.states)


@implementation("observation.drilldown_root_replaces_subtree")
def drilldown_root_replaces_subtree(ctx: LegContext) -> None:
    with managed(ctx, "Files", (str(ctx.tier.runtime_dir),)) as app:
        tree = root(ctx, app)
        before = [node.name for node in tree.walk()]
        ctx.before("atspi", before)
        skeleton = snapshot(ctx, app, "--skeleton")
        candidates = [item for item in nodes(skeleton.get("tree"))
                      if item.get("children_count", 0) and item.get("ref_id")]
        require(candidates, "skeleton has no truncated ref-able container")
        target = candidates[0]["ref_id"]
        saved = ctx.tier.state_root / "snapshots" / skeleton["snapshot_id"] / "refmap.json"
        original = json.loads(saved.read_text())
        drilled = snapshot(ctx, app, "--root", target)
        require(drilled["snapshot_id"] == skeleton["snapshot_id"])
        require(any(item.get("children") for item in nodes(drilled.get("tree"))))
        updated = json.loads(saved.read_text())
        root_entry = original["inner"]["@" + target.split(":")[1]]
        prefix = root_entry.get("path", [])
        siblings = 0
        for key, entry in original["inner"].items():
            if entry.get("path", [])[:len(prefix)] != prefix:
                siblings += 1
                require(updated["inner"].get(key) == entry, "drill-down replaced a sibling ref")
        require(siblings, "drill-down never checked a sibling ref")
        after = [node.name for node in tree.walk()]
        require(after == before)
        ctx.after("atspi", after)
        ctx.restored("atspi", after)


def session_start(ctx):
    return command(ctx, ["session", "start"]).data["session_id"]


@implementation("observation.session_namespace_isolation")
def session_namespace_isolation(ctx: LegContext) -> None:
    with managed(ctx, "Calculator") as app:
        tree = root(ctx, app)
        ctx.before("atspi", display(tree))
        first, second = session_start(ctx), session_start(ctx)
        try:
            data = command(ctx, ["--session", first, "snapshot", "-i"], app).data
            target = next(item["ref_id"] for item in nodes(data)
                          if item.get("name") == "7" and item.get("ref_id"))
            command(ctx, ["--session", second, "get", target],
                    code=("SNAPSHOT_NOT_FOUND", "STALE_REF"))
            require(command(ctx, ["--session", first, "get", target]).data["value"] == "7")
            require(display(tree) == "")
            ctx.after("atspi", display(tree))
        finally:
            for session in (first, second):
                command(ctx, ["session", "end", session])
                trash(ctx, ctx.tier.state_root / "sessions" / session)
            ctx.restored("atspi", display(tree))


@implementation("observation.trace_segment_written")
def trace_segment_written(ctx: LegContext) -> None:
    with managed(ctx, "Calculator") as app:
        session = session_start(ctx)
        directory = ctx.tier.state_root / "sessions" / session / "trace"
        before = set(directory.glob("*.jsonl"))
        ctx.before("proc", sorted(str(path) for path in before))
        try:
            command(ctx, ["--session", session, "snapshot", "-i"], app)
            added = set(directory.glob("*.jsonl")) - before
            require(len(added) == 1, f"expected one new trace segment: {added}")
            rows = [json.loads(line) for line in added.pop().read_text().splitlines()]
            require(rows and any("snapshot" in json.dumps(row) for row in rows))
            ctx.after("proc", rows)
        finally:
            command(ctx, ["session", "end", session])
            trash(ctx, ctx.tier.state_root / "sessions" / session)
            ctx.restored("proc", [])


@contextlib.contextmanager
def unreachable_accessibility_bus(ctx):
    from gi.repository import Gio, GLib
    socket = ctx.tier.runtime_dir / ("oracle-bus-" + uuid4().hex)
    address = "unix:path=" + str(socket)
    missing = "unix:path=" + str(ctx.tier.runtime_dir / ("absent-" + uuid4().hex))
    log = ctx.tier.runtime_dir / "oracle-bus.log"
    with log.open("ab") as stream:
        daemon = subprocess.Popen(["dbus-daemon", "--session", "--nofork", "--address=" + address],
                                  env=ctx.tier.env, stdout=stream, stderr=stream,
                                  start_new_session=True)
    connection, loop, thread = None, None, None
    try:
        require(raw.poll(socket.exists, 2), "private oracle bus did not start")
        connection = raw.connect(address)
        result = raw.call(connection, "org.freedesktop.DBus", "/org/freedesktop/DBus",
                          "org.freedesktop.DBus", "RequestName", "(su)", ("org.a11y.Bus", 4))
        require(result[0] == 1)
        xml = ('<node><interface name="org.a11y.Bus"><method name="GetAddress">'
               '<arg type="s" direction="out"/></method></interface></node>')
        interface = Gio.DBusNodeInfo.new_for_xml(xml).interfaces[0]

        def answer(conn, sender, path, iface, method, params, invocation):
            invocation.return_value(GLib.Variant("(s)", (missing,)))

        registration = connection.register_object("/org/a11y/bus", interface, answer, None, None)
        loop = GLib.MainLoop()
        thread = threading.Thread(target=loop.run, daemon=True)
        thread.start()
        require(raw.call(connection, "org.a11y.Bus", "/org/a11y/bus", "org.a11y.Bus", "GetAddress")[0]
                == missing)
        env = dict(ctx.tier.env, DBUS_SESSION_BUS_ADDRESS=address)
        yield dataclasses.replace(ctx.tier, env=env, bus_address=address,
                                  a11y_bus_address=missing)
        connection.unregister_object(registration)
    finally:
        if loop:
            loop.quit()
        if thread:
            thread.join(2)
            require(not thread.is_alive(), "oracle bus thread survived")
        if connection:
            connection.close_sync(None)
        daemon.terminate()
        try:
            daemon.wait(2)
        except subprocess.TimeoutExpired:
            daemon.kill()
            daemon.wait(2)
        trash(ctx, socket)
        trash(ctx, log)


@implementation("observation.unreachable_bus_permission_error")
def unreachable_bus_permission_error(ctx: LegContext) -> None:
    baseline = [node.bus for node in raw.registry(ctx.tier)]
    ctx.before("dbus", baseline)
    with unreachable_accessibility_bus(ctx) as tier:
        response = run_product(tier, ["list-apps"], headed=False)
        error = response.error or {}
        if error.get("code") == "PLATFORM_NOT_SUPPORTED":
            raise NotImplementedError("product lacks the unavailable-accessibility-bus permission mapping")
        require(not response.ok and error.get("code") == "PERM_DENIED", response.raw)
        after = [node.bus for node in raw.registry(ctx.tier)]
        require(after == baseline, "unreachable-bus read changed the registry")
        ctx.after("dbus", after)
    ctx.restored("dbus", [node.bus for node in raw.registry(ctx.tier)])
