"""`close-app` on the attached `hyprland` tier.

`close-app` sends the compositor's close request to every toplevel of the
process. Every result is read back independently: `j/clients` shows which
toplevels remain, `/proc` shows whether the process exited, and the other
apps' toplevels, the bystander's focus, the cursor and the workspaces are
compared before and after. The command's own `ok` is never the evidence.
"""

from __future__ import annotations

import contextlib
import shutil
import time
from pathlib import Path

from oracle import atspi_raw as raw, hyprland, proc
from runner import apps, attached_session
from runner.apps import OwnedApp
from runner.helper import managed, require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance, shared
from scenarios.hyprland_desktop import boundary_problems, keeps_focus

PAIR_SCRIPT = Path(__file__).resolve().parents[1] / "apps" / "close_pair.py"
PAIR_CLASS = "close_pair.py"
PAIR_NAMES = ("close pair", PAIR_CLASS)
PAIR_EXECUTABLE = "ad-close-pair"
PAIR_SETTLE_S = 10.0
CLOSE_SETTLE_S = 5.0


def clients(tier) -> list[dict]:
    main = main_instance(tier)
    return hyprland.query(shared(tier), main["signature"], "clients", pid=main["pid"])


def clients_of(tier, pid: int) -> list[dict]:
    return [item for item in clients(tier) if item.get("pid") == pid]


def kept_state(found: list[dict], pids: set[int]) -> list[tuple]:
    """What closing another app must leave alone: each toplevel, its process, workspace and mapping.
    Geometry is left out because the compositor retiles a workspace when a window leaves it."""
    return sorted((item.get("address"), item.get("pid"), (item.get("workspace") or {}).get("name"),
                   item.get("mapped"))
                  for item in found if item.get("pid") in pids)


def exited(pid: int, start: int, proc_root: Path = Path("/proc")) -> bool:
    """The process generation is gone or a zombie, read from /proc."""
    try:
        text = (proc_root / str(pid) / "stat").read_text()
    except (FileNotFoundError, ProcessLookupError):
        return True
    fields = proc.stat_fields(text)
    return fields[0] != start or text[text.rindex(")") + 2:].split()[0] == "Z"


def listed_name(envelope, pid: int) -> str:
    """The name list-apps gives the process; the leg uses it as input, never as evidence."""
    data = envelope.data if envelope.ok else None
    found = [item.get("name") for item in (data or {}).get("apps", ()) if item.get("pid") == pid]
    require(len(found) == 1 and found[0], f"list-apps lists pid {pid} as {found}: {envelope.raw[:400]}")
    named = [item for item in data["apps"] if str(item.get("name")).casefold() == found[0].casefold()]
    require(len(named) == 1, f"list-apps gives {len(named)} apps the name {found[0]!r}")
    return found[0]


def close_problems(label: str, remaining: list[dict], gone: bool, kept: tuple[list, list]) -> list[str]:
    """Every toplevel of the app is gone, its process exited, and every other app's toplevels stay."""
    problems = []
    if remaining:
        problems.append(f"{label}: the compositor still lists {[item.get('address') for item in remaining]}")
    if not gone:
        problems.append(f"{label}: the process is still running")
    before, after = kept
    if before != after:
        problems.append(f"{label}: other apps' toplevels changed from {before} to {after}")
    return problems


def report_problems(label: str, envelope) -> list[str]:
    """The report must claim what the compositor and /proc showed: a verified close."""
    delivery = ((envelope.data or {}).get("disposition") or {}).get("delivery")
    if envelope.ok and delivery == "delivered_verified":
        return []
    return [f"{label}: the report does not claim the close it made: {envelope.raw[:400]}"]


def _new_client(tier, app_id: str, known: set[str]) -> dict | None:
    found = [item for item in clients(tier) if item.get("class") == app_id and item.get("address") not in known]
    require(len(found) <= 1, f"several new {app_id} clients appeared: {found}")
    return found[0] if found and found[0].get("mapped") else None


def _register(tier, owned: OwnedApp) -> OwnedApp:
    require(proc.owned(tier, owned.pid, owned.start_time, {}), f"{owned.name} is not in the run's slice")
    apps._TIERS[owned.pid, owned.start_time] = tier
    return owned


def _release(tier, owned: OwnedApp) -> None:
    apps.stop(owned)
    apps._TIERS.pop((owned.pid, owned.start_time), None)


@contextlib.contextmanager
def terminal(tier, app_id: str):
    """A foot toplevel with no accessibility tree, in the run's slice. Yields it and its client."""
    hyprland.lua_string(app_id)
    known = {item.get("address") for item in clients(tier)}
    argv = ["foot", "--app-id", app_id, "--title", app_id, "sleep", "infinity"]
    attached_session.spawn(tier, argv, dict(tier.env), tier.runtime_dir / "close-terminal.log")
    found = raw.poll(lambda: _new_client(tier, app_id, known), 10.0)
    require(found, f"the {app_id} terminal never mapped")
    start = proc.stat_fields((Path("/proc") / str(found["pid"]) / "stat").read_text())[0]
    owned = _register(tier, OwnedApp(app_id, found["pid"], start, [], [], ""))
    try:
        yield owned, found
    finally:
        _release(tier, owned)


def _pair_owner(tier) -> OwnedApp | None:
    found = [app for app in apps._owners(tier, names=PAIR_NAMES) if len(app.windows) == 2]
    return found[0] if len(found) == 1 else None


def _mapped_pair(tier, pid: int) -> list[dict] | None:
    own = clients_of(tier, pid)
    return own if len(own) == 2 and all(item.get("mapped") for item in own) else None


def pair_interpreter(work: Path, python: Path) -> Path:
    """A private copy of the Python interpreter, named for the close pair alone.

    list-apps names a process by its executable, so a pair run by the shared interpreter shares
    its name with every other Python program in the session, and `close-app` cannot name it
    alone. The `pyvenv.cfg` beside the copy points it at the original interpreter's standard
    library and system packages, as a venv does."""
    folder = work / "close-pair-python"
    (folder / "bin").mkdir(mode=0o700, parents=True, exist_ok=True)
    (folder / "pyvenv.cfg").write_text(f"home = {python.parent}\ninclude-system-site-packages = true\n")
    target = folder / "bin" / PAIR_EXECUTABLE
    if not target.exists():
        shutil.copy2(python, target)
    return target


@contextlib.contextmanager
def close_pair(tier):
    """One process with two native Wayland windows. Yields the app and its two clients."""
    require(not apps._owners(tier, names=PAIR_NAMES), "a close pair from an earlier leg still runs")
    env = dict(tier.env)
    env.pop("AT_SPI_BUS_ADDRESS", None)
    env["GTK_USE_PORTAL"] = "0"
    python = shutil.which("python3", path=env.get("PATH"))
    require(python, "no python3 on the session PATH")
    interpreter = pair_interpreter(tier.runtime_dir, Path(python).resolve())
    attached_session.spawn(tier, [str(interpreter), str(PAIR_SCRIPT)], env, tier.runtime_dir / "close-pair.log",
                           toplevels=(PAIR_CLASS, 2))
    with raw.bounded_until(time.monotonic() + PAIR_SETTLE_S):
        owner = raw.poll(lambda: _pair_owner(tier), PAIR_SETTLE_S)
    require(owner, "the close pair never registered two frames")
    owned = _register(tier, owner)
    try:
        found = raw.poll(lambda: _mapped_pair(tier, owned.pid), 5.0)
        require(found, f"the close pair has toplevels {clients_of(tier, owned.pid)}")
        yield owned, found
    finally:
        _release(tier, owned)


def _close(ctx: LegContext, label: str, name: str, app: OwnedApp, others: set[int]) -> list[str]:
    """Close the app by name, then read the compositor and /proc, not the envelope. The closed app
    is gone by the restore point, so it is checked here and not recorded as an oracle."""
    tier = ctx.tier
    kept_before = kept_state(clients(tier), others)
    require(clients_of(tier, app.pid), f"{label}: {app.name} has no toplevel to close")
    envelope = ctx.product(["close-app", name])
    raw.poll(lambda: not clients_of(tier, app.pid) and exited(app.pid, app.start_time), CLOSE_SETTLE_S)
    remaining, gone = clients_of(tier, app.pid), exited(app.pid, app.start_time)
    kept_after = kept_state(clients(tier), others)
    problems = close_problems(label, remaining, gone, (kept_before, kept_after))
    return problems + report_problems(label, envelope)


@implementation("lifecycle.hyprland_close_app_every_window")
def close_app_every_window(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder = bystander(tier)
    ctx.before("hyprland.desktop", read())
    with managed(ctx, "Calculator") as calculator, terminal(tier, "ad-e2e-close-keep") as (foot, _):
        with close_pair(tier) as (pair, _):
            name = listed_name(ctx.product(["list-apps"]), pair.pid)
            others = {calculator.pid, foot.pid, holder["pid"]}
            require(len(kept_state(clients(tier), others)) >= 3, "the other apps have no toplevels to keep")
            with Sampler({"desktop": read}) as sampler:
                problems = _close(ctx, "close", name, pair, others)
            ctx.after("hyprland.desktop", read())
            problems += sampler.changes + keeps_focus(tier, read())
            require(not problems, "; ".join(problems))
    ctx.restored("hyprland.desktop", read())


@implementation("lifecycle.hyprland_close_app_gone_app_not_found")
def close_app_gone_app_not_found(ctx: LegContext) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder = bystander(tier)
    ctx.before("hyprland.desktop", read())
    with terminal(tier, "ad-e2e-close-stay") as (foot, _), managed(ctx, "Calculator") as calculator:
        name = listed_name(ctx.product(["list-apps"]), calculator.pid)
        others = {foot.pid, holder["pid"]}
        problems = _close(ctx, "first close", name, calculator, others)
        require(not problems, "; ".join(problems))
        kept_before = kept_state(clients(tier), others)
        with Sampler({"desktop": read}) as sampler:
            again = ctx.product(["close-app", name])
        kept_after = kept_state(clients(tier), others)
        problems = boundary_problems("second close-app", again, ("APP_NOT_FOUND",))
        problems += [] if kept_after == kept_before else [f"other toplevels changed: {kept_before} to {kept_after}"]
        problems += sampler.changes + keeps_focus(tier, read())
        ctx.after("hyprland.desktop", read())
        require(not problems, "; ".join(problems))
    ctx.restored("hyprland.desktop", read())
