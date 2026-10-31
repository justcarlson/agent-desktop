"""Cancelling a ready runner on the attached `hyprland` tier.

The leg starts a second runner on the same session. That child starts a second
Hyprland instance and Calculator in its own slice, says so, and waits. The
parent cancels it through its readiness receipt, then checks that everything
the child recorded is gone and that the session it attached to is untouched.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from oracle.proc import members
from runner import apps, attached, attached_session, cancel, second_instance
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import bystander, desktop, main_instance, record

LEG = "session.cancel_cleanup_recorded_generations"
CHILD_ENV = "AGENT_DESKTOP_E2E_CANCEL_CHILD"
LEASE_FD_ENV = "AGENT_DESKTOP_INTERACTION_LEASE_FD"
DROPPED = ("AGENT_DESKTOP_E2E_LOCK_ROOT", "AGENT_DESKTOP_E2E_BIN")
RUNNER = Path(cancel.__file__).with_name("main.py").resolve()
READY_S = 120.0
CLEANUP_S = 60.0
CANCELLED = 128 + 15


def child_env(source: dict[str, str], suite: Path, ready: Path) -> tuple[dict[str, str], int]:
    """The child's environment and the lease descriptor it inherits."""
    raw_fd = source.get(LEASE_FD_ENV)
    if raw_fd is None:
        raise AssertionError("the cancel leg needs the interaction lease that run.sh holds")
    env = {key: value for key, value in source.items() if key not in DROPPED}
    env["AGENT_DESKTOP_E2E_BIN_SRC"] = source.get("AGENT_DESKTOP_E2E_BIN_SRC") or source["AGENT_DESKTOP_E2E_BIN"]
    env.update({"AGENT_DESKTOP_E2E_SUITE": str(suite), CHILD_ENV: str(ready)})
    return env, int(raw_fd)


def _hold(ctx: LegContext, ready: Path) -> None:
    nested = second_instance.ensure(ctx.tier)
    app = apps.launch(ctx.tier, "Calculator")
    found = attached.load(ctx.tier.runtime_dir)
    facts = {"app": [app.pid, app.start_time], "instance": [nested["pid"], nested["start"]],
             "signature": nested["signature"], "state_root": str(ctx.tier.state_root),
             "members": [list(item[:2]) for item in attached.members(found)]}
    temporary = ready.with_suffix(".tmp")
    temporary.write_text(json.dumps(facts) + "\n")
    temporary.replace(ready)
    while True:
        time.sleep(1)


def aftermath(ready: dict, child_record: dict, parent_record: dict, proc_root: Path = Path("/proc")) -> list[str]:
    """What a cancelled child left behind, and what of the session it took with it."""
    alive = lambda pair: attached_session._alive(pair[0], pair[1], proc_root)
    shared = Path(parent_record["shared_runtime"])
    problems = list(attached_session.remaining(child_record, proc_root))
    problems += attached_session.dead_survivors(child_record, proc_root)
    for label, pair in (("Calculator", ready["app"]), ("second instance", ready["instance"])):
        if alive(pair):
            problems.append(f"the child's {label} ({pair[0]}) survived the cancel")
    problems += [f"a recorded generation survived: {pair[0]}" for pair in ready["members"] if alive(pair)]
    if os.path.lexists(shared / "hypr" / ready["signature"]):
        problems.append(f"the child's instance directory survived: {ready['signature']}")
    if os.path.lexists(ready["state_root"]):
        problems.append(f"the child's state root survived: {ready['state_root']}")
    if not shared.is_dir():
        problems.append(f"the shared runtime directory {shared} is gone")
    if not alive(parent_record["compositor"]):
        problems.append("the session compositor did not survive the cancel")
    holder = parent_record.get("bystander") or {}
    if holder and not alive((holder["pid"], holder["start"])):
        problems.append("the bystander did not survive the cancel")
    return problems


def _wait_ready(child: subprocess.Popen, ready: Path, receipt: Path, log: Path) -> dict:
    deadline = time.monotonic() + READY_S
    while not (ready.is_file() and receipt.is_file()):
        if child.poll() is not None:
            raise AssertionError(f"the child runner exited {child.returncode} before it was ready: "
                                 f"{log.read_text(errors='replace')[-2000:]}")
        if time.monotonic() >= deadline:
            raise AssertionError(f"the child runner was not ready in {READY_S:.0f} s")
        time.sleep(0.2)
    return json.loads(ready.read_text())


def _observe(ctx: LegContext, stage: str, main: dict) -> None:
    getattr(ctx, stage)("hyprland.desktop", desktop(ctx.tier, main))
    getattr(ctx, stage)("proc.members", members(ctx.tier.runtime_dir))


def _cancel(ctx: LegContext) -> None:
    main = main_instance(ctx.tier)
    bystander(ctx.tier)
    suite = Path(tempfile.mkdtemp(prefix="cancel-child-", dir=os.environ["AGENT_DESKTOP_E2E_SUITE"]))
    ready, receipt, log = suite / "child.ready.json", suite / "hyprland.ready.json", suite / "child.log"
    env, lease = child_env(dict(os.environ), suite, ready)
    _observe(ctx, "before", main)
    with log.open("wb") as output:
        child = subprocess.Popen([sys.executable, "-I", str(RUNNER), "--tier", "hyprland", "--leg", LEG],
                                 env=env, pass_fds=(lease,), stdout=output, stderr=subprocess.STDOUT)
    try:
        facts = _wait_ready(child, ready, receipt, log)
        alive = lambda pair: attached_session._alive(pair[0], pair[1], Path("/proc"))
        require(alive(facts["app"]) and alive(facts["instance"]),
                f"the child's Calculator and second instance were not running when it was cancelled: {facts}")
        stamped = json.loads(receipt.read_text())
        cancel.terminate(stamped)
        code = child.wait(CLEANUP_S)
        mirror = Path(stamped["attached"])
        cancel.check_attached_cleanup(Path(stamped["runtime"]), mirror, CLEANUP_S)
        problems = aftermath(facts, json.loads(mirror.read_text()), record(ctx.tier))
        if code != CANCELLED:
            problems.append(f"the cancelled runner exited {code}, not {CANCELLED}")
        _observe(ctx, "after", main)
        require(not problems, "; ".join(problems))
    finally:
        if child.poll() is None:
            child.terminate()
            child.wait(CLEANUP_S)
    _observe(ctx, "restored", main)


@implementation("session.cancel_cleanup_recorded_generations")
def cancel_cleanup_recorded_generations(ctx: LegContext) -> None:
    ready = os.environ.get(CHILD_ENV)
    if ready:
        _hold(ctx, Path(ready))
    _cancel(ctx)
