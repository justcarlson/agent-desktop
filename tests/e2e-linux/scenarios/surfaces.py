from __future__ import annotations

from scenarios.headed_actions import find_ref as qualified_ref

import json

from runner.apps import launch, stop
from runner.legs import implementation
from runner.verdict import LegContext
from oracle.atspi_raw import app_root, poll

CALCULATOR = "Calculator"


def find_ref(tree: dict | None, *, name: str | None = None, role: str | None = None) -> str | None:
    return qualified_ref(tree, name=name, role=role)


def surface_ids(surfaces: list[dict]) -> dict[str, list[str]]:
    grouped: dict[str, list[str]] = {}
    for surface in surfaces:
        grouped.setdefault(surface["id"], []).append(surface["type"])
    return grouped


def batch_failed(envelope) -> str:
    return str((envelope.error or {}).get("code", ""))


def _open_about(tier) -> None:
    matches = app_root(tier, "gnome-calculator").find(name="Main Menu")
    assert matches, "Calculator Main Menu missing from the raw tree"
    matches[0].do_action("click")


def _surfaces(ctx: LegContext, app: str) -> list[dict]:
    envelope = ctx.product(["list-surfaces", "--app", app])
    assert envelope.ok, envelope.raw
    return (envelope.data or {}).get("surfaces", [])


@implementation("surfaces.batch_seed_opener_about")
def batch_seed_opener_about(ctx: LegContext) -> None:
    app = launch(ctx.tier, CALCULATOR)
    snapshot = ctx.product(["snapshot", "--app", CALCULATOR, "-i"])
    assert snapshot.ok, snapshot.raw
    ref = find_ref((snapshot.data or {}).get("tree"), name="Main Menu")
    assert ref, "Main Menu has no ref"
    batch = json.dumps(
        [
            {"command": "click", "args": {"ref": ref}},
            {"command": "wait", "args": {"event": "surface-appeared", "timeout": 8000}},
        ]
    )
    ctx.before("atspi.surfaces", len(_surfaces(ctx, CALCULATOR)))
    envelope = ctx.product(["batch", batch])
    assert envelope.ok, envelope.raw
    entries = (envelope.data or {}).get("results") or envelope.data or []
    assert entries, envelope.raw
    wait_entry = entries[-1]
    assert wait_entry.get("ok") is True, envelope.raw
    event = (wait_entry.get("data") or {}).get("event") or {}
    assert event.get("kind") == "surface_appeared", envelope.raw
    assert event.get("window_id"), envelope.raw
    grouped = surface_ids(_surfaces(ctx, CALCULATOR))
    matches = [ids for ids, kinds in grouped.items() if ids == event["window_id"]]
    ctx.after("atspi.surfaces", len(grouped))
    assert matches, f"the appeared surface {event['window_id']} is not listed"
    stop(app)


@implementation("surfaces.baseline_after_opener_fails")
def baseline_after_opener_fails(ctx: LegContext) -> None:
    app = launch(ctx.tier, CALCULATOR)
    _open_about(ctx.tier)
    ctx.before("atspi.surfaces", len(_surfaces(ctx, CALCULATOR)))
    batch = json.dumps(
        [{"command": "wait", "args": {"event": "surface-appeared", "timeout": 1500}}]
    )
    envelope = ctx.product(["batch", batch])
    ctx.after("atspi.surfaces", len(_surfaces(ctx, CALCULATOR)))
    assert not envelope.ok, envelope.raw
    assert batch_failed(envelope) == "TIMEOUT", envelope.raw
    stop(app)


def window_and_sheet_dedupe_one_id(ctx: LegContext) -> None:
    app = launch(ctx.tier, CALCULATOR)
    ctx.before("atspi.surfaces", len(_surfaces(ctx, CALCULATOR)))
    _open_about(ctx.tier)
    surfaces = poll(
        lambda: [
            item
            for item in _surfaces(ctx, CALCULATOR)
            if item["type"] in ("window", "sheet")
        ],
        5.0,
    )
    grouped = surface_ids(surfaces)
    ctx.after("atspi.surfaces", len(grouped))
    assert grouped, "no window or sheet surfaced"
    assert len(grouped) == 1, f"the dialog did not dedupe to one id: {grouped!r}"
    kinds = next(iter(grouped.values()))
    assert "window" in kinds and "sheet" in kinds, kinds
    stop(app)


def files_properties_surface_scope(ctx: LegContext) -> None:
    files = launch(ctx.tier, "Files")
    envelope = ctx.product(["list-windows", "--app", "Files"])
    assert envelope.ok and envelope.data, envelope.raw
    window_id = envelope.data[0]["id"]
    ctx.before("atspi.surfaces", len(_surfaces(ctx, "Files")))
    ctx.product(["press", "alt+Return", "--window-id", window_id])
    observed = poll(
        lambda: [item for item in _surfaces(ctx, "Files") if item["type"] == "sheet"],
        5.0,
    )
    ctx.after("atspi.surfaces", len(_surfaces(ctx, "Files")))
    assert observed, "Properties did not appear as a sheet"
    listing = ctx.product(["list-surfaces", "--app", "Files"])
    assert listing.ok, listing.raw
    pid = (listing.data or {}).get("pid")
    windows = ctx.product(["list-windows", "--app", "Files"])
    owners = {item["pid"] for item in (windows.data or [])}
    assert pid in owners, f"surface pid {pid} is not a Files owner {owners!r}"
    stop(files)
