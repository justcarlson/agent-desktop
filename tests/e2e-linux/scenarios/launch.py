from __future__ import annotations

from runner.apps import launch, stop
from runner.legs import implementation
from runner.verdict import LegContext
from oracle.atspi_raw import app_root
from oracle.proc import members

CALCULATOR = "Calculator"
TEXT_EDITOR = "Text Editor"
TERMINAL = "Terminal"


def app_names(data) -> list[str]:
    return sorted(item["name"] for item in ((data or {}).get("apps") or []))


def error_code(envelope) -> str:
    return str((envelope.error or {}).get("code", ""))


@implementation("launch.launch_calculator_attributed")
def launch_calculator_attributed(ctx: LegContext) -> None:
    before = members(ctx.tier.runtime_dir)
    ctx.before("proc.members", len(before))
    envelope = ctx.product(["launch", CALCULATOR])
    assert envelope.ok, envelope.raw
    launched = (envelope.data or {}).get("pid")
    assert isinstance(launched, int), envelope.raw
    after = members(ctx.tier.runtime_dir)
    ctx.after("proc.members", len(after))
    assert any(member[0] == launched for member in after), "the launched pid is not a member"
    window = ctx.product(["list-windows", "--app", CALCULATOR])
    assert window.ok and window.data, window.raw
    assert app_root(ctx.tier, "gnome-calculator")


@implementation("launch.launch_text_editor_new_window")
def launch_text_editor_new_window(ctx: LegContext) -> None:
    first = launch(ctx.tier, TEXT_EDITOR)
    windows = ctx.product(["list-windows", "--app", TEXT_EDITOR])
    assert windows.ok and windows.data, windows.raw
    existing = {item["id"] for item in windows.data}
    ctx.before("atspi.window_ids", sorted(existing))
    envelope = ctx.product(["launch", TEXT_EDITOR, "--arg=--new-window"])
    assert envelope.ok, envelope.raw
    after = ctx.product(["list-windows", "--app", TEXT_EDITOR])
    assert after.ok and after.data, after.raw
    fresh = {item["id"] for item in after.data} - existing
    ctx.after("atspi.window_ids", sorted({item["id"] for item in after.data}))
    assert fresh, "a second launch added no window"
    stop(first)


@implementation("launch.launch_terminal_owner")
def launch_terminal_owner(ctx: LegContext) -> None:
    ctx.before("proc.terminal_owner", None)
    envelope = ctx.product(["launch", TERMINAL])
    assert envelope.ok, envelope.raw
    owners = [member for member in members(ctx.tier.runtime_dir) if "gnome-terminal" in member[2]]
    ctx.after("proc.terminal_owner", [owner[2] for owner in owners])
    assert owners, "no gnome-terminal-server owner in the runtime directory"
    assert app_root(ctx.tier, "gnome-terminal-server")


@implementation("launch.list_apps_alias")
def list_apps_alias(ctx: LegContext) -> None:
    ctx.before("proc.apps", [])
    envelope = ctx.product(["list-apps", "--app", CALCULATOR])
    assert envelope.ok, envelope.raw
    names = app_names(envelope.data)
    ctx.after("proc.apps", names)
    assert names, "list-apps returned no Calculator"
    other = ctx.product(["list-apps", "--app", "calculator"])
    assert other.ok, other.raw
    assert "complete" not in (envelope.data or {}) or envelope.data.get("complete") is not False


@implementation("launch.launch_suggestion")
def launch_suggestion(ctx: LegContext) -> None:
    ctx.before("proc.apps", [])
    envelope = ctx.product(["launch", "No Such Application For Harness"])
    ctx.after("proc.apps", [])
    assert not envelope.ok, envelope.raw
    assert error_code(envelope) == "APP_NOT_FOUND", envelope.raw
    suggestion = (envelope.error or {}).get("suggestion")
    assert suggestion, "the APP_NOT_FOUND envelope carries no suggestion"


@implementation("launch.recovery_strategy_on_error")
def recovery_strategy_on_error(ctx: LegContext) -> None:
    app = launch(ctx.tier, CALCULATOR)
    snapshot = ctx.product(["snapshot", "--app", CALCULATOR, "-i"])
    assert snapshot.ok, snapshot.raw
    ref = _first_ref((snapshot.data or {}).get("tree"))
    assert ref, "the snapshot allocated no ref"
    stop(app)
    ctx.before("proc.calculator", "running")
    envelope = ctx.product(["click", ref])
    ctx.after("proc.calculator", "gone")
    assert not envelope.ok, envelope.raw
    assert error_code(envelope) in ("STALE_REF", "ELEMENT_NOT_FOUND"), envelope.raw
    error = envelope.error or {}
    disposition = error.get("disposition") or {}
    if disposition.get("retry") == "safe":
        assert (error.get("recovery") or {}).get("strategy"), envelope.raw


def _first_ref(tree: dict | None) -> str | None:
    if not tree:
        return None
    stack = [tree]
    while stack:
        node = stack.pop()
        if node.get("ref"):
            return node["ref"]
        stack.extend(reversed(node.get("children") or []))
    return None
