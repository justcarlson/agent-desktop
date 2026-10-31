"""Window identity on the attached `hyprland` tier.

`list-windows` lists an accessible window once, under its frame id, and a
toplevel with no accessibility tree once, under a `w-hypr-*` id of this
compositor. A window operation binds its frame to exactly one live toplevel
before it acts: same-titled twins are told apart by size or refused as
ambiguous, and a closed window is not found. Every listed field and every
binding is read back from the compositor's `j/clients`, the data
`hyprctl clients -j` prints, and the desktop is polled while the product works.
"""

from __future__ import annotations

from oracle import atspi_raw as raw
from runner import apps
from runner.helper import managed, require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance
from scenarios.hyprland_desktop import boundary_problems, keeps_focus
from scenarios.hyprland_windows import (SPECIAL, clients_of, compositor_prefix, cursor, foreign_id,
                                        frame_sizes, listed, move, restore_desktop, terminal)

REF_KEYS = ("ref", "ref_id", "refs", "snapshot_id")
TWIN_SPECIALS = (SPECIAL + "ad_e2e_twin_a", SPECIAL + "ad_e2e_twin_b")
SIZE_TOLERANCE = 1.0
NO_TREE = "no accessibility tree"


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


def listed_once_problems(label: str, entries: list[dict], frames: list[str], toplevels: int) -> list[str]:
    """Each AT-SPI frame is listed once and accessible, and no toplevel of the process is listed again."""
    problems = []
    ids = sorted(str(item.get("id")) for item in entries)
    if ids != sorted(frames):
        problems.append(f"{label}: listed {ids}, the AT-SPI frames are {sorted(frames)}")
    hidden = [item.get("id") for item in entries if item.get("accessible") is not True]
    if hidden:
        problems.append(f"{label}: listed as compositor-only too: {hidden}")
    if toplevels != len(frames):
        problems.append(f"{label}: the compositor has {toplevels} toplevels for {len(frames)} frames")
    return problems


def native_problems(clients: list[dict]) -> list[str]:
    """Twins under Xwayland could be minimized; native Wayland twins show only the binding."""
    return [f"{item.get('address')} runs under Xwayland" for item in clients if item.get("xwayland") is not False]


def _within(frame, size) -> bool:
    return all(abs(float(a) - float(b)) <= SIZE_TOLERANCE for a, b in zip(frame, size))


def same_size(clients: list[dict], frames: list[tuple]) -> bool:
    """Both toplevels have one size, and every frame reports it."""
    sizes = {tuple(item["size"]) for item in clients}
    if len(clients) != 2 or len(sizes) != 1 or len(frames) != 2:
        return False
    size = next(iter(sizes))
    return all(_within(frame, size) for frame in frames)


def distinct_sizes(clients: list[dict], frames: list[tuple]) -> bool:
    """The toplevels differ by more than twice the tolerance, and each frame matches its own toplevel."""
    if len(clients) != 2 or len(frames) != 2:
        return False
    first, second = (tuple(item["size"]) for item in clients)
    if all(abs(float(a) - float(b)) <= 2 * SIZE_TOLERANCE for a, b in zip(first, second)):
        return False
    matches = [[index for index, item in enumerate(clients) if _within(frame, tuple(item["size"]))]
               for frame in frames]
    return sorted(match[0] for match in matches if len(match) == 1) == [0, 1]


def candidate_problems(label: str, envelope, expected: dict[str, str]) -> list[str]:
    """AMBIGUOUS_TARGET names every twin toplevel with its workspace."""
    problems = boundary_problems(label, envelope, ("AMBIGUOUS_TARGET",))
    candidates = ((envelope.error or {}).get("details") or {}).get("candidates")
    if not isinstance(candidates, list):
        return problems + [f"{label}: details.candidates is {candidates!r}"]
    named = {str(item.get("address")): item.get("workspace") for item in candidates if isinstance(item, dict)}
    if len(candidates) != len(expected) or named != expected:
        problems.append(f"{label}: candidates {candidates} are not the twin toplevels {expected}")
    return problems


def twin_state(clients: list[dict]) -> list[tuple]:
    """What a refused window operation must leave alone on each twin."""
    return sorted((item.get("address"), (item.get("workspace") or {}).get("name"), tuple(item.get("at") or ()),
                   tuple(item.get("size") or ()), item.get("mapped"), item.get("hidden"),
                   item.get("fullscreen"), item.get("floating"))
                  for item in clients)


def _twins(tier) -> apps.OwnedApp | None:
    found = apps.owners_named(tier, "Twin Windows")
    return found[0] if len(found) == 1 and len(found[0].windows) == 2 else None


def _twin_app(ctx, launched) -> tuple[apps.OwnedApp, list[dict]]:
    twins = raw.poll(lambda: _twins(ctx.tier), 5.0)
    require(twins and twins.pid == launched.pid, "the twin windows never both registered")
    found = clients_of(ctx.tier, twins.pid)
    require(len(found) == 2, f"the twin process has {len(found)} toplevels")
    problems = native_problems(found)
    require(not problems, "; ".join(problems))
    return twins, found


def no_tree_problems(label: str, envelope) -> list[str]:
    """A read of a compositor-only window refuses as INVALID_ARGS, exit 1, naming the missing tree."""
    error = envelope.error or {}
    problems = []
    if envelope.ok or error.get("code") != "INVALID_ARGS":
        problems.append(f"{label}: expected INVALID_ARGS, got ok={envelope.ok} code={error.get('code')!r}")
    if envelope.exit_code != 1:
        problems.append(f"{label}: a structured error must exit 1, got {envelope.exit_code}")
    if NO_TREE not in str(error.get("message")):
        problems.append(f"{label}: the refusal does not say the window has no accessibility tree")
    return problems


def _no_tree(ctx, window_id: str) -> list[str]:
    return [problem for argv in (["snapshot", "-i", "--window-id", window_id],
                                 ["find", "--window-id", window_id, "--role", "button"])
            for problem in no_tree_problems(argv[0], ctx.product(argv))]


def _class_filter(ctx, app) -> list[str]:
    """Filtering by an accessible app's compositor class never shows its toplevel as compositor-only."""
    classes = {item.get("class") for item in clients_of(ctx.tier, app.pid)}
    require(len(classes) == 1, f"{app.name} has toplevel classes {classes}")
    name = classes.pop()
    envelope = ctx.product(["list-windows", "--app", name])
    if not envelope.ok:
        return boundary_problems(f"list-windows --app {name}", envelope, ("APP_NOT_FOUND",))
    shown = [item for item in listed(envelope, pid=app.pid) if item.get("accessible") is not True]
    return [f"--app {name} lists {app.name} as compositor-only: {shown}"] if shown else []


@implementation("identity.hyprland_accessible_listed_once")
def accessible_listed_once(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder = bystander(tier)
    with managed(ctx, "Calculator") as calculator, managed(ctx, "Twin Windows") as launched:
        twins, _ = _twin_app(ctx, launched)
        ctx.before("hyprland.desktop", read())
        with Sampler({"desktop": read}) as sampler:
            windows = ctx.product(["list-windows"])
            problems = []
            for app in (calculator, twins):
                problems += listed_once_problems(app.name, listed(windows, pid=app.pid), app.windows,
                                                 len(clients_of(tier, app.pid)))
                problems += _class_filter(ctx, app)
            if len(listed(windows, pid=holder["pid"])) != len(clients_of(tier, holder["pid"])):
                problems.append("the bystander is not listed once per toplevel")
        ctx.after("hyprland.desktop", read())
        problems += sampler.changes + keeps_focus(tier, read())
        require(not problems, "; ".join(problems))
    ctx.restored("hyprland.desktop", read())


def _set_apart(tier, twins: apps.OwnedApp, found: list[dict], holder: dict, point: dict) -> dict[str, str]:
    """Each twin alone on its own hidden special workspace, so both have one title and one size."""
    addresses = sorted(item["address"] for item in found)
    for address, workspace in zip(addresses, TWIN_SPECIALS):
        move(tier, address, workspace)
    restore_desktop(tier, holder, point)
    require(raw.poll(lambda: same_size(clients_of(tier, twins.pid), frame_sizes(tier, twins)), 5.0),
            f"the twins never shared one size: {clients_of(tier, twins.pid)}")
    return dict(zip(addresses, TWIN_SPECIALS))


def _unchanged_twins(ctx, twins, work) -> list[str]:
    """Each phase checks the twins itself: they exit before the leg's final restored read."""
    before = twin_state(clients_of(ctx.tier, twins.pid))
    problems = work()
    after = twin_state(clients_of(ctx.tier, twins.pid))
    return problems + ([f"the twins changed: {before} became {after}"] if after != before else [])


def _ambiguous(ctx, twins, expected: dict[str, str]) -> list[str]:
    problems = []
    for window_id in twins.windows:
        envelope = ctx.product(["minimize", "--window-id", window_id])
        problems += candidate_problems(f"same-size twin {window_id}", envelope, expected)
    return problems


def _bound(ctx, twins) -> list[str]:
    """Each twin binds to one toplevel, so the operation reaches the Wayland boundary instead of a tie."""
    problems = []
    for window_id in twins.windows:
        envelope = ctx.product(["minimize", "--window-id", window_id])
        problems += boundary_problems(f"distinct-size twin {window_id}", envelope, ("ACTION_NOT_SUPPORTED",),
                                      physical=True, cause="wayland_session")
    return problems


@implementation("identity.hyprland_twins_by_size_or_ambiguous")
def twins_by_size_or_ambiguous(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder, point = bystander(tier), cursor(tier)
    ctx.before("hyprland.desktop", read())
    try:
        with managed(ctx, "Twin Windows") as launched:
            twins, found = _twin_app(ctx, launched)
            expected = _set_apart(tier, twins, found, holder, point)
            with Sampler({"desktop": read}) as sampler:
                problems = _unchanged_twins(ctx, twins, lambda: _ambiguous(ctx, twins, expected))
            problems += sampler.changes + keeps_focus(tier, read())
            with terminal(tier, "ad-e2e-twin-split") as (_, foot):
                move(tier, foot["address"], TWIN_SPECIALS[1])
                restore_desktop(tier, holder, point)
                require(raw.poll(lambda: distinct_sizes(clients_of(tier, twins.pid), frame_sizes(tier, twins)),
                                 5.0),
                        f"the twins never took distinct sizes: {clients_of(tier, twins.pid)}")
                with Sampler({"desktop": read}) as sampler:
                    problems += _unchanged_twins(ctx, twins, lambda: _bound(ctx, twins))
                problems += sampler.changes + keeps_focus(tier, read())
            ctx.after("hyprland.desktop", read())
            require(not problems, "; ".join(problems))
    finally:
        restore_desktop(tier, holder, point)
    ctx.restored("hyprland.desktop", read())


def _closed_frames(ctx) -> list[str]:
    tier = ctx.tier
    with managed(ctx, "Twin Windows") as launched:
        twins, _ = _twin_app(ctx, launched)
        listed_ids = sorted(item.get("id") for item in listed(ctx.product(["list-windows"]), pid=twins.pid))
        require(listed_ids == sorted(twins.windows), f"the twins are listed as {listed_ids}")
        old, pid = list(twins.windows), twins.pid
    require(raw.poll(lambda: not clients_of(tier, pid), 5.0), "the closed twins still have toplevels")
    return [problem for window_id in old
            for problem in boundary_problems(f"closed frame {window_id}",
                                             ctx.product(["minimize", "--window-id", window_id]),
                                             ("WINDOW_NOT_FOUND",))]


def _closed_toplevels(ctx) -> list[str]:
    tier = ctx.tier
    with terminal(tier, "ad-e2e-restart") as (_, first):
        old = [item["id"] for item in listed(ctx.product(["list-windows"]), pid=first["pid"])]
    require(len(old) == 1, f"the first terminal was listed {len(old)} times")
    require(raw.poll(lambda: not clients_of(tier, first["pid"]), 5.0), "the closed terminal still has a toplevel")
    problems = boundary_problems("closed toplevel", ctx.product(["minimize", "--window-id", old[0]]),
                                 ("WINDOW_NOT_FOUND",))
    with terminal(tier, "ad-e2e-restart") as (_, second):
        new = [item["id"] for item in listed(ctx.product(["list-windows"]), pid=second["pid"])]
        if len(new) != 1 or new[0] == old[0]:
            problems.append(f"the restarted terminal is listed as {new}, the old id was {old[0]}")
        problems += boundary_problems("restarted process", ctx.product(["minimize", "--window-id", old[0]]),
                                      ("WINDOW_NOT_FOUND",))
        if new:
            problems += boundary_problems("other compositor instance",
                                          ctx.product(["minimize", "--window-id", foreign_id(new[0])]),
                                          ("WINDOW_NOT_FOUND",))
    return problems


@implementation("identity.hyprland_closed_window_not_found")
def closed_window_not_found(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    ctx.before("hyprland.desktop", read())
    with Sampler({"desktop": read}) as sampler:
        problems = _closed_frames(ctx) + _closed_toplevels(ctx)
    ctx.after("hyprland.desktop", read())
    problems += sampler.changes + keeps_focus(tier, read())
    require(not problems, "; ".join(problems))
    ctx.restored("hyprland.desktop", read())


@implementation("identity.hyprland_compositor_only_id")
def compositor_only_id(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    with terminal(tier, "ad-e2e-listed") as (_, foot):
        ctx.before("hyprland.desktop", read())
        prefix = compositor_prefix(tier)
        with Sampler({"desktop": read}) as sampler:
            own = listed(ctx.product(["list-windows"]), pid=foot["pid"])
            problems = [] if len(own) == 1 else [f"the terminal is listed {len(own)} times: {own}"]
            problems += [problem for entry in own[:1]
                         for problem in compositor_entry_problems(entry, foot, prefix)]
            filtered = ctx.product(["list-windows", "--app", "ad-e2e-listed"])
            if [item.get("id") for item in listed(filtered)] != [item.get("id") for item in own[:1]]:
                problems.append(f"--app ad-e2e-listed lists {filtered.raw[:400]}")
            if own:
                problems += _no_tree(ctx, own[0]["id"])
        ctx.after("hyprland.desktop", read())
        problems += sampler.changes + keeps_focus(tier, read())
        require(not problems, "; ".join(problems))
    ctx.restored("hyprland.desktop", read())
