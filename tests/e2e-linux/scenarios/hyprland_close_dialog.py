"""`close-app` on the attached `hyprland` tier while a dialog outside the app's window group is open.

The app is a GTK4 application window plus a `Gtk.Dialog` that is never added to the
application, so the two toplevels carry unrelated app ids. A close request to the dialog
answers it for the user and one to the window ends the app with the dialog open, so
`close-app` must send neither. Every result is read back independently: `j/clients` shows
both toplevels still mapped, `/proc` shows the process still running, the app's own log shows
no close request arrived, and the other apps' toplevels, the bystander's focus, the cursor and
the workspaces are compared before and after. The command's own `ok` is never the evidence.
"""

from __future__ import annotations

import contextlib
import shutil
import time
from pathlib import Path

from oracle import atspi_raw as raw
from runner import apps, attached_session
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_close import (_register, _release, clients, clients_of, exited, kept_state,
                                      listed_name, private_interpreter)
from scenarios.hyprland_common import Sampler, bystander, desktop, main_instance
from scenarios.hyprland_desktop import keeps_focus

HOST_SCRIPT = Path(__file__).resolve().parents[1] / "apps" / "dialog_host.py"
DIALOG_CLASS = "ad-dialog-host"
HOST_NAMES = ("dialog host", DIALOG_CLASS)
HOST_EXECUTABLE = "ad-dialog-host"
HOST_SETTLE_S = 10.0
HOLD_S = 3.0


def _host_owner(tier):
    found = [app for app in apps._owners(tier, names=HOST_NAMES) if len(app.windows) == 2]
    return found[0] if len(found) == 1 else None


def _mapped(tier, pid: int) -> list[dict] | None:
    own = clients_of(tier, pid)
    return own if len(own) == 2 and all(item.get("mapped") for item in own) else None


@contextlib.contextmanager
def dialog_host(tier, mode: str):
    """The host window and its unrelated dialog in one process. Yields the app, its two clients
    and the path of its close-request log."""
    require(not apps._owners(tier, names=HOST_NAMES), "a dialog host from an earlier leg still runs")
    env = dict(tier.env)
    env.pop("AT_SPI_BUS_ADDRESS", None)
    env["GTK_USE_PORTAL"] = "0"
    python = shutil.which("python3", path=env.get("PATH"))
    require(python, "no python3 on the session PATH")
    interpreter = private_interpreter(tier.runtime_dir, Path(python).resolve(), HOST_EXECUTABLE)
    log = tier.runtime_dir / f"dialog-host-{mode}.requests"
    log.write_text("")
    attached_session.spawn(tier, [str(interpreter), str(HOST_SCRIPT), mode, str(log)], env,
                           tier.runtime_dir / f"dialog-host-{mode}.log", toplevels=(DIALOG_CLASS, 1))
    with raw.bounded_until(time.monotonic() + HOST_SETTLE_S):
        owner = raw.poll(lambda: _host_owner(tier), HOST_SETTLE_S)
    require(owner, "the dialog host never registered its window and its dialog")
    owned = _register(tier, owner)
    try:
        found = raw.poll(lambda: _mapped(tier, owned.pid), 5.0)
        require(found, f"the dialog host has toplevels {clients_of(tier, owned.pid)}")
        require(len({item.get("class") for item in found}) == 2,
                f"the dialog shares the window's app id: {[item.get('class') for item in found]}")
        yield owned, found, log
    finally:
        _release(tier, owned)


def hold_problems(before: list[dict], after: list[dict], gone: bool, requests: str,
                  kept: tuple[list, list]) -> list[str]:
    """Both toplevels stay mapped at their addresses, the process runs, no close request arrived,
    and every other app's toplevels stay."""
    problems = []
    addresses = lambda found: sorted(item.get("address") for item in found if item.get("mapped"))
    if addresses(after) != addresses(before):
        problems.append(f"the app's toplevels went from {addresses(before)} to {addresses(after)}")
    if gone:
        problems.append("the process exited")
    if requests.strip():
        problems.append(f"the app received close requests: {requests.split()}")
    if kept[0] != kept[1]:
        problems.append(f"other apps' toplevels changed from {kept[0]} to {kept[1]}")
    return problems


def pending_report_problems(envelope) -> list[str]:
    """The report must say a dialog holds the close and that nothing was sent."""
    error = envelope.error or {}
    details = error.get("details") or {}
    delivery = (error.get("disposition") or {}).get("delivery")
    if (not envelope.ok and error.get("code") == "ACTION_FAILED" and details.get("cause") == "dialog_pending"
            and details.get("dialogs") == 1 and delivery == "not_delivered"):
        return []
    return [f"the report does not hold the close for the dialog: {envelope.raw[:400]}"]


def _hold(ctx: LegContext, mode: str) -> None:
    tier, main = ctx.tier, main_instance(ctx.tier)
    read = lambda: desktop(tier, main)
    holder = bystander(tier)
    ctx.before("hyprland.desktop", read())
    with dialog_host(tier, mode) as (host, found, log):
        name = listed_name(ctx.product(["list-apps"]), host.pid)
        others = {holder["pid"]}
        kept_before = kept_state(clients(tier), others)
        with Sampler({"desktop": read}) as sampler:
            envelope = ctx.product(["close-app", name])
            raw.poll(lambda: exited(host.pid, host.start_time) or log.read_text().strip(), HOLD_S)
        after = clients_of(tier, host.pid)
        problems = hold_problems(found, after, exited(host.pid, host.start_time), log.read_text(),
                                 (kept_before, kept_state(clients(tier), others)))
        problems += pending_report_problems(envelope) + sampler.changes + keeps_focus(tier, read())
        ctx.after("hyprland.desktop", read())
        require(not problems, f"{mode}: " + "; ".join(problems))
    ctx.restored("hyprland.desktop", read())


@implementation("lifecycle.hyprland_close_app_holds_for_a_parented_unrelated_dialog")
def close_app_holds_for_a_parented_unrelated_dialog(ctx: LegContext) -> None:
    _hold(ctx, "parented")


@implementation("lifecycle.hyprland_close_app_holds_for_an_unparented_unrelated_dialog")
def close_app_holds_for_an_unparented_unrelated_dialog(ctx: LegContext) -> None:
    _hold(ctx, "unparented")
