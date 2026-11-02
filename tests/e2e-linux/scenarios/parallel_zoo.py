"""The four-app setup the four-agent legs share on the attached `hyprland` tier.

Agent A clicks a GTK3 zoo, B types into a GTK4 zoo, D clicks a Qt zoo and,
in the headless leg, a second GTK3 zoo stands in for C. Each app logs its
effects with monotonic times to its own file. A headed setup moves the
three dispatching apps onto the shown workspace and gives the bystander
focus; every setup closes its special workspaces, gives focus and the
cursor back to the bystander and stops its apps on the way out.
"""

from __future__ import annotations

import contextlib
import fcntl
import os
import time
from dataclasses import dataclass
from pathlib import Path

from oracle import atspi_raw as raw, proc
from runner import apps, attached_session
from runner.helper import require
from runner.product import LEASE_FD_ENV
from runner.readiness import wait_ready
from runner.verdict import LegContext
from scenarios import parallel_agents as agents
from scenarios.hyprland_close import _register, _release
from scenarios.hyprland_common import bystander, desktop, main_instance, shared
from scenarios.hyprland_cursor import ended, records
from scenarios.hyprland_cursor_agents import started_multi
from scenarios.hyprland_focus_controls import cursor, move, query, restore_desktop
from scenarios.hyprland_keys_common import give_focus, keyboards, toplevel
from scenarios.hyprland_pointer_pad import x_display

BUTTON, ENTRY = "zoo-button", "zoo-entry"
ACTIVATE, CHANGED = "activate", "changed"
ZOOS = {"A": "Zoo GTK3", "B": "Zoo GTK4", "C": "Zoo GTK3", "D": "Zoo Qt"}
LAUNCH_S, RESPONSIVE_S, FOCUS_S = 20.0, 3.0, 3.0


@dataclass
class Zoo:
    agent: str
    app: apps.OwnedApp
    log: Path

    def events(self, widget: str, event: str) -> list[dict]:
        return agents.app_events(self.log, widget, event)


def _zoo_command(tier, app: str, env: dict[str, str]) -> list[str]:
    """The zoo's argv; the Qt zoo runs on the instance's XWayland display, where headed ref clicks reach it."""
    script = Path(apps.__file__).resolve().parents[1] / "apps" / f"{apps.ZOOS[app]}.py"
    if app == "Zoo Qt":
        instance = main_instance(tier)
        display = raw.poll(lambda: x_display(instance), LAUNCH_S)
        require(display, f"no XWayland display names the compositor {instance['pid']}")
        env.update(DISPLAY=display, QT_ACCESSIBILITY="1", QT_LINUX_ACCESSIBILITY_ALWAYS_ON="1",
                   QT_QPA_PLATFORM="xcb")
        return [os.environ.get("AGENT_DESKTOP_QT_PYTHON", "python3"), str(script)]
    env["GTK_USE_PORTAL"] = "0"
    return ["python3", str(script)]


def _new_owner(tier, names, before: set) -> apps.OwnedApp | None:
    try:
        found = [item for item in apps._owners(tier, names=names)
                 if item.windows and (item.pid, item.start_time) not in before]
    except TimeoutError:
        return None
    require(len(found) <= 1, f"one launch gave several owners: {found}")
    return found[0] if found else None


def _tail(path: Path) -> str:
    return path.read_text(errors="replace")[-600:] if path.exists() else "no output"


def _launch(tier, agent: str) -> Zoo:
    """The agent's zoo, started through the compositor with its own output file and registered to the run."""
    app = ZOOS[agent]
    names = apps.ALIASES[app]
    log = tier.runtime_dir / f"parallel-{agent.lower()}.jsonl"
    out = tier.runtime_dir / f"parallel-{agent.lower()}.out"
    log.unlink(missing_ok=True)
    out.unlink(missing_ok=True)
    with raw.bounded_until(time.monotonic() + LAUNCH_S):
        before = {(item.pid, item.start_time) for item in apps._owners(tier, names=names)}
    env = dict(tier.env)
    env.pop("AT_SPI_BUS_ADDRESS", None)
    attached_session.spawn(tier, [*_zoo_command(tier, app, env), str(log)], env, out)
    deadline = time.monotonic() + LAUNCH_S
    with raw.bounded_until(deadline + RESPONSIVE_S):
        owner = raw.poll(lambda: _new_owner(tier, names, before), LAUNCH_S)
    require(owner, f"{app} for agent {agent} registered no window in {LAUNCH_S:.0f} s; "
                   f"run processes: {[name for _, _, name in proc.members(tier.runtime_dir)]}; output: {_tail(out)}")
    owned = _register(tier, owner)
    try:
        wait_ready(tier, owned)
    except Exception:
        _release(tier, owned)
        raise
    return Zoo(agent, owned, log)


@contextlib.contextmanager
def zoos(tier, names: str):
    """One zoo per agent letter in `names`, stopped on the way out."""
    launched: dict[str, Zoo] = {}
    try:
        for name in names:
            launched[name] = _launch(tier, name)
        yield launched
    finally:
        for zoo in launched.values():
            _release(tier, zoo.app)


def lease_state() -> str:
    """Whether the runner's own lease descriptor holds the per-user interaction lease."""
    raw_fd = os.environ.get(LEASE_FD_ENV)
    if raw_fd is None:
        return "released"
    probe = os.open(agents.LOCK_PATH, os.O_RDWR | os.O_CLOEXEC | os.O_NOFOLLOW)
    try:
        fcntl.flock(probe, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        return "held"
    else:
        fcntl.flock(probe, fcntl.LOCK_UN)
        return "free"
    finally:
        os.close(probe)


def desktop_of(tier) -> dict:
    return desktop(tier, main_instance(tier))


def stray_events(zoo: Zoo, allowed: dict[str, list[float]]) -> list[str]:
    """Logged effects that no recorded command accounts for."""
    found = []
    for item in agents.app_events(zoo.log, BUTTON, ACTIVATE) + agents.app_events(zoo.log, ENTRY, CHANGED):
        if item["at"] not in allowed.get(zoo.agent, []):
            found.append(f"{zoo.agent}: unaccounted {item['widget']} {item['event']} at {item['at']:.4f}")
    return found


@contextlib.contextmanager
def multi_session(ctx: LegContext):
    """A `--cursor --multi-agent` session, ended on the way out."""
    session = started_multi(ctx)
    try:
        yield session
    finally:
        closing = ended(ctx, session)
        require(closing.ok, f"the session did not end: {closing.raw[:400]}")


@contextlib.contextmanager
def shown_desktop(ctx: LegContext, launched: dict[str, Zoo], names: str):
    """The apps in `names` on the shown workspace, with the bystander focused before and after."""
    tier = ctx.tier
    holder, point = bystander(tier), cursor(tier)
    shown = desktop_of(tier)["workspace"]
    try:
        for name in names:
            move(tier, toplevel(tier, launched[name].app)["address"], shown)
        restore_desktop(tier, holder, point)
        yield holder, point
    finally:
        restore_desktop(tier, holder, point)


def common_reads(ctx: LegContext, stage: str) -> None:
    getattr(ctx, stage)("proc.lease", lease_state())
    getattr(ctx, stage)("hyprland.keyboards", keyboards(ctx.tier))


def focus_entry(tier, zoo: Zoo) -> None:
    """Gives the GTK4 zoo's window focus and its entry the toolkit focus. GTK4 answers
    `Component.GrabFocus` with NotSupported (FINDINGS L4-16, L7-8), so a headed `type` can only
    prove focus on an entry that already holds it."""
    give_focus(tier, toplevel(tier, zoo.app)["address"])
    request, consumed = Path(str(zoo.log) + ".focus-entry"), Path(str(zoo.log) + ".focus-entry-consumed")
    consumed.unlink(missing_ok=True)
    request.touch()
    require(raw.poll(consumed.exists, FOCUS_S), f"the GTK4 zoo did not take the entry focus in {FOCUS_S} s")


def layer_stack(tier) -> list[tuple]:
    """Every layer surface the compositor maps: monitor, level, namespace, pid and rectangle."""
    return [(monitor, level, item.get("namespace"), item.get("pid"),
             (item.get("x"), item.get("y"), item.get("w"), item.get("h")))
            for monitor, info in query(tier, "layers").items()
            for level, items in sorted((info.get("levels") or {}).items()) for item in items]


def note_layers(ctx: LegContext, problems: list[str]) -> None:
    """Records the layer stack and the renderer records while the session lives, when an agent was
    refused as occluded, so the report names what covered the target."""
    if any("receives_events" in item for item in problems):
        ctx.finding(f"layers after the occluded refusal: {layer_stack(ctx.tier)}; "
                    f"renderer records: {records(shared(ctx.tier))}")


def entry_text(tier, zoo: Zoo) -> str | None:
    found = [item.text() for item in raw.app_root(tier, zoo.app.owner_names[0]).find(name=ENTRY)]
    return found[0] if len(found) == 1 else None
