"""R9 and R10 on the attached `hyprland` tier: the user's desktop stays as it was.

Every leg reads the session compositor's active window, cursor, workspace and
monitors before, during and after the product works, with a bystander window
the runner did not launch holding focus. Effects are read on the app's own
accessibility tree.
"""

from __future__ import annotations

from oracle.proc import members
from runner import second_instance
from runner.helper import activate, changed, display, managed, one, ref, require, root
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance, shared
from scenarios.wayland_outputs import display_problems, expected_from_monitors, monitors

NOT_DELIVERED = {"delivery": "not_delivered", "retry": "safe"}


def keeps_focus(tier, state: dict) -> list[str]:
    holder = bystander(tier)
    active = state["active_window"]
    if active.get("address") != holder["address"] or active.get("pid") != holder["pid"]:
        return [f"the bystander lost focus to {active}"]
    return []


def boundary_problems(label: str, envelope, codes: tuple[str, ...], *, physical: bool = False,
                      cause: str | None = None) -> list[str]:
    error = envelope.error or {}
    details = error.get("details") or {}
    problems = []
    if envelope.ok or error.get("code") not in codes:
        problems.append(f"{label}: expected {codes}, got ok={envelope.ok} code={error.get('code')!r}")
    if envelope.exit_code != 1:
        problems.append(f"{label}: a structured error must exit 1, got {envelope.exit_code}")
    if error.get("disposition") != NOT_DELIVERED:
        problems.append(f"{label}: disposition is {error.get('disposition')}, not {NOT_DELIVERED}")
    if physical and details.get("physical_delivery_started") is not False:
        problems.append(f"{label}: physical_delivery_started is {details.get('physical_delivery_started')!r}")
    if cause is not None and details.get("cause") != cause:
        problems.append(f"{label}: cause is {details.get('cause')!r}, not {cause!r}")
    return problems


def _next_display(before: str) -> str:
    return "7" if before in ("", "0") else before + "7"


def _undo(tree, before: str) -> str:
    if display(tree) != before:
        activate(one(tree, role=43, name="Undo"))
    return changed(lambda: display(tree), before)


def semantic_click_preserves_desktop(ctx: LegContext) -> None:
    main = main_instance(ctx.tier)
    read = lambda: desktop(ctx.tier, main)
    with managed(ctx, "Calculator") as app, managed(ctx, "Text Editor"):
        tree = root(ctx, app)
        seven = ref(ctx, app, role="button", name="7")
        before = display(tree)
        state = read()
        require(not keeps_focus(ctx.tier, state), "; ".join(keeps_focus(ctx.tier, state)))
        ctx.before("atspi.display", before)
        ctx.before("hyprland.desktop", state)
        try:
            with Sampler({"desktop": read}) as sampler:
                clicked = ctx.product(["click", seven])
                require(clicked.ok, f"the semantic click failed: {clicked.raw}")
                ctx.after("atspi.display", changed(lambda: display(tree), _next_display(before)))
            ctx.after("hyprland.desktop", read())
            require(not sampler.changes, "a semantic click moved the desktop: " + "; ".join(sampler.changes))
        finally:
            restored = _undo(tree, before)
        ctx.restored("atspi.display", restored)
        ctx.restored("hyprland.desktop", read())


@implementation("non_interference.headless_route_preserves_desktop")
def headless_route_preserves_desktop(ctx: LegContext) -> None:
    main = main_instance(ctx.tier)
    read = lambda: desktop(ctx.tier, main)
    with managed(ctx, "Calculator") as app:
        tree = root(ctx, app)
        before = display(tree)
        ctx.before("atspi.display", before)
        ctx.before("hyprland.desktop", read())
        problems = []
        try:
            with Sampler({"desktop": read}) as sampler:
                problems += _read_only_commands(ctx, app, main)
                seven = ref(ctx, app, role="button", name="7")
                clicked = ctx.product(["click", seven])
                require(clicked.ok, f"the semantic click failed: {clicked.raw}")
                ctx.after("atspi.display", changed(lambda: display(tree), _next_display(before)))
            ctx.after("hyprland.desktop", read())
            problems += sampler.changes + keeps_focus(ctx.tier, read())
            require(not problems, "; ".join(problems))
        finally:
            restored = _undo(tree, before)
        ctx.restored("atspi.display", restored)
        ctx.restored("hyprland.desktop", read())


def _read_only_commands(ctx: LegContext, app, main: dict) -> list[str]:
    problems = []
    listed = ctx.product(["list-apps"])
    if not listed.ok or app.pid not in {item.get("pid") for item in (listed.data or {}).get("apps", ())}:
        problems.append(f"list-apps does not list the launched Calculator ({app.pid})")
    windows = ctx.product(["list-windows", "--app", "Calculator"])
    if not windows.ok or {item.get("id") for item in windows.data or ()} != set(app.windows):
        problems.append(f"list-windows disagrees with the accessibility tree: {windows.raw[:400]}")
    snapshot = ctx.product(["snapshot", "-i", "--window-id", app.windows[0]])
    if not snapshot.ok or (snapshot.data or {}).get("complete") is not True:
        problems.append(f"the snapshot is not complete: {snapshot.raw[:400]}")
    problems += display_problems("list-displays", ctx.product(["list-displays"]),
                                 expected_from_monitors(monitors(shared(ctx.tier), main["signature"])))
    status = ctx.product(["status"])
    if not status.ok:
        problems.append(f"status failed: {status.raw[:400]}")
    return problems


@implementation("non_interference.headed_changes_only_target")
def headed_changes_only_target(ctx: LegContext) -> None:
    main = main_instance(ctx.tier)
    nested = second_instance.ensure(ctx.tier)
    reads = {"desktop": lambda: desktop(ctx.tier, main), "second": lambda: desktop(ctx.tier, nested)}
    ctx.before("hyprland.desktop", reads["desktop"]())
    ctx.before("hyprland.second_desktop", reads["second"]())
    with Sampler(reads) as sampler:
        with managed(ctx, "Calculator") as app:
            tree = root(ctx, app)
            seven = ref(ctx, app, role="button", name="7")
            before = display(tree)
            ctx.before("atspi.display", before)
            problems = _headed_refusals(ctx, app, seven)
            ctx.after("atspi.display", display(tree))
            problems += [] if display(tree) == before else ["a refused headed command changed Calculator"]
            ctx.restored("atspi.display", _undo(tree, before))
        ctx.after("hyprland.desktop", reads["desktop"]())
        ctx.after("hyprland.second_desktop", reads["second"]())
    problems += sampler.changes + keeps_focus(ctx.tier, reads["desktop"]())
    require(not problems, "; ".join(problems))
    ctx.restored("hyprland.desktop", reads["desktop"]())
    ctx.restored("hyprland.second_desktop", reads["second"]())


def absent_ref_problems(ctx: LegContext, absent: str) -> list[str]:
    """Headless, an absent ref is an identity refusal; headed, the Wayland pointer gate refuses first."""
    envelope = ctx.product(["click", absent, "--timeout-ms", "500"])
    if ctx.mode == "headed":
        return boundary_problems("absent ref", envelope, ("ACTION_NOT_SUPPORTED",), physical=True,
                                 cause="wayland_session")
    return boundary_problems("absent ref", envelope, ("ELEMENT_NOT_FOUND", "STALE_REF"))


def _headed_refusals(ctx: LegContext, app, seven: str) -> list[str]:
    problems = boundary_problems("headed ref click", ctx.product(["click", seven]),
                                 ("ACTION_NOT_SUPPORTED",), cause="wayland_session")
    problems += boundary_problems("focus-window", ctx.product(["focus-window", "--window-id", app.windows[0]]),
                                  ("ACTION_NOT_SUPPORTED",), cause="wayland_session")
    problems += boundary_problems("mouse-click", ctx.product(["mouse-click", "--xy", "1,1"]),
                                  ("ACTION_NOT_SUPPORTED",), physical=True, cause="wayland_session")
    return problems


@implementation("error_contract.hyprland_delivery_boundaries")
def hyprland_delivery_boundaries(ctx: LegContext) -> None:
    main = main_instance(ctx.tier)
    read = lambda: desktop(ctx.tier, main)
    with managed(ctx, "Calculator") as app:
        tree = root(ctx, app)
        seven = ref(ctx, app, role="button", name="7")
        before = display(tree)
        ctx.before("atspi.display", before)
        ctx.before("hyprland.desktop", read())
        ctx.before("proc.members", members(ctx.tier.runtime_dir))
        with Sampler({"desktop": read}) as sampler:
            problems = absent_ref_problems(ctx, seven.split(":", 1)[0] + ":e999999")
            if ctx.mode == "headed":
                problems += _headed_refusals(ctx, app, seven)
            else:
                problems += boundary_problems("headless mouse-click", ctx.product(["mouse-click", "--xy", "1,1"]),
                                              ("POLICY_DENIED",))
        ctx.after("atspi.display", display(tree))
        ctx.after("hyprland.desktop", read())
        ctx.after("proc.members", members(ctx.tier.runtime_dir))
        problems += sampler.changes + keeps_focus(ctx.tier, read())
        problems += [] if display(tree) == before else ["a refused command changed Calculator"]
        require(not problems, "; ".join(problems))
        ctx.restored("atspi.display", display(tree))
        ctx.restored("hyprland.desktop", read())
        ctx.restored("proc.members", members(ctx.tier.runtime_dir))
