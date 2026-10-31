"""Window identity on the attached `hyprland` tier.

`list-windows` lists toplevels with no accessibility tree by compositor
evidence alone, and an accessible window once, under its frame id. A
`focus-window` that cannot bind its window to exactly one live toplevel
refuses before anything is sent. Each listed field is read back from
`j/clients`; the desktop is polled while the product works.
"""

from __future__ import annotations

from oracle import atspi_raw as raw
from runner import apps
from runner.helper import managed, require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance
from scenarios.hyprland_desktop import boundary_problems, keeps_focus
from scenarios.hyprland_focus import LAUNCH_SPECIAL, SECOND_SPECIAL, refusal
from scenarios.hyprland_windows import (clients_of, compositor_prefix, cursor, foreign_id, frame_sizes,
                                        listed, move, restore_desktop, terminal)

REF_KEYS = ("ref", "ref_id", "refs", "snapshot_id")


def compositor_entry_problems(entry: dict, client: dict, prefix: str) -> list[str]:
    """A compositor-only window: its id, process and geometry are the toplevel's, and it has no refs."""
    label = f"listed {client.get('class')}"
    expected = {
        "accessible": False,
        "pid": client.get("pid"),
        "title": client.get("title"),
        "app_name": client.get("class"),
        "visible": bool(client.get("mapped")) and not client.get("hidden"),
        "bounds": {"x": float(client["at"][0]), "y": float(client["at"][1]),
                   "width": float(client["size"][0]), "height": float(client["size"][1])},
    }
    problems = [f"{label}: {key} is {entry.get(key)!r}, the compositor says {value!r}"
                for key, value in expected.items() if entry.get(key) != value]
    if not str(entry.get("id", "")).startswith(prefix):
        problems.append(f"{label}: id {entry.get('id')!r} does not start with {prefix!r}")
    if not entry.get("process_instance"):
        problems.append(f"{label}: no process_instance to tell a restarted process apart")
    if "minimized" in entry:
        problems.append(f"{label}: reports minimized, which the compositor does not expose")
    problems += [f"{label}: carries {key}" for key in REF_KEYS if key in entry]
    return problems


@implementation("identity.hyprland_compositor_only_listed")
def compositor_only_listed(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder = bystander(tier)
    with managed(ctx, "Calculator") as app, terminal(tier, "ad-e2e-listed") as (_, foot):
        ctx.before("hyprland.desktop", read())
        ctx.before("atspi.frames", sorted(app.windows))
        prefix = compositor_prefix(tier)
        with Sampler({"desktop": read}) as sampler:
            windows = ctx.product(["list-windows"])
            filtered = ctx.product(["list-windows", "--app", "ad-e2e-listed"])
            own = listed(windows, pid=foot["pid"])
            problems = [] if len(own) == 1 else [f"the terminal is listed {len(own)} times: {own}"]
            problems += [problem for entry in own[:1]
                         for problem in compositor_entry_problems(entry, foot, prefix)]
            if [item.get("id") for item in listed(filtered)] != [item.get("id") for item in own[:1]]:
                problems.append(f"--app ad-e2e-listed lists {filtered.raw[:400]}")
            calculator = listed(windows, pid=app.pid)
            if (sorted(item.get("id") for item in calculator) != sorted(app.windows)
                    or not all(item.get("accessible") is True for item in calculator)):
                problems.append(f"Calculator is not listed once per AT-SPI frame: {calculator}")
            problems += _class_filter(ctx, app)
            if len(listed(windows, pid=holder["pid"])) != len(clients_of(tier, holder["pid"])):
                problems.append("the bystander is not listed once per toplevel")
            if own:
                problems += _no_tree(ctx, own[0]["id"])
            ctx.after("atspi.frames", sorted(apps.owners_named(tier, "Calculator")[0].windows))
        ctx.after("hyprland.desktop", read())
        problems += sampler.changes + keeps_focus(tier, read())
        ctx.restored("atspi.frames", sorted(apps.owners_named(tier, "Calculator")[0].windows))
        require(not problems, "; ".join(problems))
    ctx.restored("hyprland.desktop", read())


def _class_filter(ctx, app) -> list[str]:
    """Filtering by an accessible app's compositor class never shows its toplevel as compositor-only."""
    classes = {item.get("class") for item in clients_of(ctx.tier, app.pid)}
    require(len(classes) == 1, f"Calculator has toplevel classes {classes}")
    name = classes.pop()
    envelope = ctx.product(["list-windows", "--app", name])
    if not envelope.ok:
        return boundary_problems(f"list-windows --app {name}", envelope, ("APP_NOT_FOUND",))
    shown = [item for item in listed(envelope, pid=app.pid) if item.get("accessible") is not True]
    return [f"--app {name} lists Calculator as compositor-only: {shown}"] if shown else []


def _no_tree(ctx, window_id: str) -> list[str]:
    problems = []
    for argv in (["snapshot", "-i", "--window-id", window_id],
                 ["find", "--window-id", window_id, "--role", "button"]):
        envelope = ctx.product(argv)
        problems += boundary_problems(argv[0], envelope, ("INVALID_ARGS",))
        if "no accessibility tree" not in str((envelope.error or {}).get("message")):
            problems.append(f"{argv[0]}: the refusal does not say the window has no accessibility tree")
    return problems


def _twins(tier) -> apps.OwnedApp | None:
    found = apps.owners_named(tier, "Twin Windows")
    return found[0] if len(found) == 1 and len(found[0].windows) == 2 else None


@implementation("identity.hyprland_unbindable_refused")
def unbindable_refused(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder, point = bystander(tier), cursor(tier)
    frames = lambda: sorted(window for owner in apps.owners_named(tier, "Twin Windows")
                            for window in owner.windows)
    ctx.before("hyprland.desktop", read())
    ctx.before("atspi.twin_frames", frames())
    try:
        with managed(ctx, "Twin Windows") as launched:
            twins = raw.poll(lambda: _twins(tier), 5.0)
            require(twins and twins.pid == launched.pid, "the twin windows never both registered")
            addresses = sorted(item["address"] for item in clients_of(tier, twins.pid))
            require(len(addresses) == 2, f"the twin process has {len(addresses)} toplevels")
            _set_apart(tier, twins, addresses, holder, point)
            with Sampler({"desktop": read}) as sampler:
                problems = _ambiguous(ctx, twins.windows[0], addresses)
                problems += _gone_and_restarted(ctx)
            ctx.after("hyprland.desktop", read())
            ctx.after("atspi.twin_frames", frames())
            problems += sampler.changes + keeps_focus(tier, read())
            require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("atspi.twin_frames", frames())
    ctx.restored("hyprland.desktop", read())


def _set_apart(tier, twins: apps.OwnedApp, addresses: list[str], holder: dict, point: dict) -> None:
    """Each twin alone on its own hidden special workspace, so both have one title and one size."""
    for address, workspace in zip(addresses, (LAUNCH_SPECIAL, SECOND_SPECIAL)):
        move(tier, address, workspace)
    restore_desktop(tier, holder, point)

    def alike() -> bool:
        sizes = {tuple(item["size"]) for item in clients_of(tier, twins.pid)}
        if len(sizes) != 1:
            return False
        width, height = next(iter(sizes))
        return all(abs(w - width) <= 1 and abs(h - height) <= 1 for w, h in frame_sizes(tier, twins))

    require(raw.poll(alike, 5.0), f"the twins never shared one size: {clients_of(tier, twins.pid)}")


def _ambiguous(ctx, window_id: str, addresses: list[str]) -> list[str]:
    envelope = ctx.product(["focus-window", "--window-id", window_id])
    problems = refusal("twin", envelope, "ACTION_FAILED", "ambiguous_window")
    candidates = ((envelope.error or {}).get("details") or {}).get("candidates") or []
    if sorted(item.get("address") for item in candidates) != addresses:
        problems.append(f"twin: candidates {candidates} are not the twin toplevels {addresses}")
    return problems


def _gone_and_restarted(ctx) -> list[str]:
    tier = ctx.tier
    with terminal(tier, "ad-e2e-restart") as (_, first):
        old = [item["id"] for item in listed(ctx.product(["list-windows"]), pid=first["pid"])]
    require(len(old) == 1, f"the first terminal was listed {len(old)} times")
    problems = boundary_problems("closed toplevel", ctx.product(["focus-window", "--window-id", old[0]]),
                                 ("WINDOW_NOT_FOUND",))
    with terminal(tier, "ad-e2e-restart") as (_, second):
        new = [item["id"] for item in listed(ctx.product(["list-windows"]), pid=second["pid"])]
        if len(new) != 1 or new[0] == old[0]:
            problems.append(f"the restarted terminal is listed as {new}, the old id was {old[0]}")
        problems += boundary_problems("restarted process",
                                      ctx.product(["focus-window", "--window-id", old[0]]),
                                      ("WINDOW_NOT_FOUND",))
        if new:
            problems += boundary_problems("other compositor instance",
                                          ctx.product(["focus-window", "--window-id", foreign_id(new[0])]),
                                          ("WINDOW_NOT_FOUND",))
    return problems
