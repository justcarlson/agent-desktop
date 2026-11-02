"""A bound pair reads and acts on its own Hyprland instance only.

The runner's second Hyprland runs a sentinel foot terminal. `list-windows`
under each pair must list compositor-only toplevels of that instance alone,
under ids that instance issued. The sentinel's id given to the session pair is
not found there, and both desktops stay as they were. A minimize and restore of
the sentinel through the second pair must show in the second instance's
`j/clients` and leave the session instance's `j/clients` and desktop as they
were. A minimize and restore of a runner-owned Zoo GTK3 window through the
session pair must show in the session's `j/clients` and leave the second
instance's desktop and stable `j/clients` fields as they were. Each unchanged
side is polled while the other side acts.
"""

from __future__ import annotations

from pathlib import Path

from oracle import hyprland, proc
from runner import second_instance
from runner.helper import managed, require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance, shared
from scenarios.hyprland_desktop import boundary_problems
from scenarios.hyprland_focus_controls import (client, clients_of, cursor, move, restore_desktop, spare_workspaces,
                                               workspaces_in_use)
from scenarios.hyprland_window_checks import MINIMIZED, operated, placement, unchanged, window_argv

APP = "Zoo GTK3"


def pair_of(tier, instance: dict) -> dict[str, str]:
    """The product's full environment bound to `instance`."""
    return dict(tier.env, WAYLAND_DISPLAY=instance["wayland"], HYPRLAND_INSTANCE_SIGNATURE=instance["signature"])


def id_prefix(instance: dict, start: int) -> str:
    return f"w-hypr-{instance['pid']}-{start}-"


def own_listing_problems(label: str, envelope, prefix: str, own: set[int], required: set[int],
                         forbidden: set[int]) -> list[str]:
    """Every compositor-only entry is a toplevel of the bound instance, under an id it issued; each
    `required` pid is listed and no `forbidden` pid is."""
    if not envelope.ok or not isinstance(envelope.data, list):
        return [f"{label}: list-windows failed: {envelope.raw[:400]}"]
    problems = []
    for entry in (item for item in envelope.data if item.get("accessible") is False):
        if not str(entry.get("id", "")).startswith(prefix):
            problems.append(f"{label}: {entry.get('id')!r} was not issued by the bound instance ({prefix})")
        if entry.get("pid") not in own:
            problems.append(f"{label}: lists pid {entry.get('pid')} ({entry.get('title')!r}), "
                            f"which has no toplevel on the bound instance")
    pids = {item.get("pid") for item in envelope.data}
    problems += [f"{label}: pid {pid} is not listed" for pid in sorted(required - pids)]
    problems += [f"{label}: lists pid {pid} of the other instance" for pid in sorted(forbidden & pids)]
    return problems


def clients_state(clients: list[dict]) -> list[tuple]:
    """What an operation on another instance must leave alone in this one's `j/clients`."""
    return sorted((item.get("address"), item.get("pid"), item.get("mapped"), item.get("hidden"),
                   tuple(sorted((key, str(value)) for key, value in placement(item).items())))
                  for item in clients)


def _start(pid: int) -> int:
    return proc.stat_fields((Path("/proc") / str(pid) / "stat").read_text())[0]


def _clients(tier, instance: dict) -> list[dict]:
    return hyprland.query(shared(tier), instance["signature"], "clients", pid=instance["pid"])


def _sentinel_reader(tier, nested: dict, address: str):
    def read() -> dict:
        found = [item for item in _clients(tier, nested) if item.get("address") == address]
        require(len(found) == 1, f"the sentinel {address} has {len(found)} toplevels")
        return placement(found[0])
    return read


def _listings(ctx: LegContext, main: dict, nested: dict, sentinel: int) -> tuple[list[str], str | None]:
    tier = ctx.tier
    main_pids = {item.get("pid") for item in _clients(tier, main)}
    nested_pids = {item.get("pid") for item in _clients(tier, nested)}
    require(sentinel in nested_pids, f"the sentinel {sentinel} has no toplevel on the second instance")
    on_main = ctx.product(["list-windows"], env=pair_of(tier, main), headed=False)
    problems = own_listing_problems("the session pair", on_main, id_prefix(main, _start(main["pid"])), main_pids,
                                    {bystander(tier)["pid"]}, {sentinel})
    on_second = ctx.product(["list-windows"], env=pair_of(tier, nested), headed=False)
    problems += own_listing_problems("the second pair", on_second, id_prefix(nested, _start(nested["pid"])),
                                     nested_pids, {sentinel}, set())
    ids = [item.get("id") for item in (on_second.data or []) if item.get("pid") == sentinel]
    if len(ids) != 1:
        problems.append(f"the second pair lists the sentinel {len(ids)} times: {ids}")
    return problems, (ids[0] if len(ids) == 1 else None)


def _through_second(ctx: LegContext, nested: dict, sentinel_id: str, address: str) -> list[str]:
    read = _sentinel_reader(ctx.tier, nested, address)
    first, env = read(), pair_of(ctx.tier, nested)
    problems = operated(ctx, "minimize through the second pair", window_argv("minimize", sentinel_id), read,
                        {"workspace": MINIMIZED + first["workspace"]}, env=env)
    return problems + operated(ctx, "restore through the second pair", window_argv("restore", sentinel_id), read,
                               {"workspace": first["workspace"], "at": first["at"], "size": first["size"]}, env=env)


def _foreign_id(ctx: LegContext, main: dict, nested: dict, sentinel_id: str, address: str) -> list[str]:
    read = _sentinel_reader(ctx.tier, nested, address)
    before = read()
    envelope = ctx.product(window_argv("minimize", sentinel_id), env=pair_of(ctx.tier, main))
    problems = boundary_problems("the sentinel's id through the session pair", envelope, ("WINDOW_NOT_FOUND",))
    return problems + unchanged("the sentinel's id through the session pair", read, before)


def _through_session(ctx: LegContext, main: dict, app, holder: dict, point: dict) -> list[str]:
    tier = ctx.tier
    own = clients_of(tier, app.pid)
    require(len(own) == 1 and len(app.windows) == 1, f"{APP} has toplevels {own} and frames {app.windows}")
    move(tier, own[0]["address"], spare_workspaces(workspaces_in_use(tier), 1)[0])
    restore_desktop(tier, holder, point)
    read = lambda: placement(client(tier, own[0]["address"]))
    first, env = read(), pair_of(tier, main)
    problems = operated(ctx, "minimize through the session pair", window_argv("minimize", app.windows[0]), read,
                        {"workspace": MINIMIZED + first["workspace"]}, env=env)
    return problems + operated(ctx, "restore through the session pair", window_argv("restore", app.windows[0]),
                               read, {"workspace": first["workspace"], "at": first["at"], "size": first["size"]},
                               env=env)


@implementation("session.hyprland_effects_stay_on_bound_instance")
def hyprland_effects_stay_on_bound_instance(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    holder, point = bystander(tier), cursor(tier)
    try:
        try:
            nested = second_instance.ensure_sized(tier)
        except second_instance.OutputsNotReady as error:
            ctx.after("hyprland.second_not_ready", error.diagnostics)
            raise
        sentinel = nested["sentinel"][0]
        found = [item for item in _clients(tier, nested) if item.get("pid") == sentinel]
        require(len(found) == 1 and found[0].get("mapped"), f"the sentinel is not one mapped toplevel: {found}")
        session = {"session desktop": lambda: desktop(tier, main),
                   "session clients": lambda: clients_state(_clients(tier, main))}
        second = {"second desktop": lambda: desktop(tier, nested),
                  "second clients": lambda: clients_state(_clients(tier, nested))}
        ctx.before("hyprland.desktop", desktop(tier, main))
        ctx.before("hyprland.second_clients", clients_state(_clients(tier, nested)))
        with Sampler({**session, **second}) as quiet:
            problems, sentinel_id = _listings(ctx, main, nested, sentinel)
            if sentinel_id is not None:
                problems += _foreign_id(ctx, main, nested, sentinel_id, found[0]["address"])
        problems += quiet.changes
        if sentinel_id is not None:
            with Sampler(session) as sampler:
                problems += _through_second(ctx, nested, sentinel_id, found[0]["address"])
            problems += sampler.changes
        with managed(ctx, APP, (str(tier.runtime_dir / "zoo-gtk3-bound.log"),)) as app:
            ctx.before("hyprland.second_desktop", desktop(tier, nested))
            with Sampler(second) as sampler:
                problems += _through_session(ctx, main, app, holder, point)
            problems += sampler.changes
        restore_desktop(tier, holder, point)
        ctx.after("hyprland.desktop", desktop(tier, main))
        ctx.after("hyprland.second_clients", clients_state(_clients(tier, nested)))
        ctx.after("hyprland.second_desktop", desktop(tier, nested))
        require(not problems, "; ".join(problems))
        ctx.restored("hyprland.desktop", desktop(tier, main))
        ctx.restored("hyprland.second_clients", clients_state(_clients(tier, nested)))
        ctx.restored("hyprland.second_desktop", desktop(tier, nested))
    finally:
        try:
            restore_desktop(tier, holder, point)
        finally:
            second_instance.stop(tier)
