"""Lease timeouts among agent processes on the attached `hyprland` tier.

The runner's gate holds the per-user interaction lease. A queued agent
waits behind it with no lease token or descriptor of the runner's; headed
contenders must return `TIMEOUT`, kind `lock_timeout` on `interaction.lock`,
and send nothing. B is a `type` with a 1500 ms timeout; D is a
`mouse-click --xy` with its default 5000 ms budget, because a headed ref
click on Hyprland refuses on its receiver check before it waits on the
lease. Two threads of one process acting
through the C ABI must give the second a `TIMEOUT`, kind
`interaction_process_lock_timeout`. Once the gate releases the lease each
queued delivery lands exactly once and no timed-out contender sends late
input. Each app's log, `/proc/locks` and the compositor's keyboards are the
evidence.
"""

from __future__ import annotations

import contextlib
import json
import os
import select
import subprocess
import sys
import time
from pathlib import Path

from runner import product
from runner.helper import nodes, require
from runner.legs import implementation
from runner.product import Envelope
from runner.verdict import LegContext
from scenarios import parallel_agents as agents
from scenarios import parallel_oracle as oracle
from scenarios import parallel_zoo as zoo
from scenarios.hyprland_cursor import live_button

CONTEND_MS = "1500"
ENVIRON_S = 2.0
FFI_HOLDER_MS, FFI_CONTENDER_MS = "30000", "1000"
FFI_ENV = "AGENT_DESKTOP_E2E_FFI_LIB"
FFI_LIBRARY = "libagent_desktop_ffi.so"
FFI_CHILD = Path(__file__).resolve().parents[1] / "apps" / "ffi_lease_child.py"


def ffi_library() -> Path:
    """The C ABI library AGENT_DESKTOP_E2E_FFI_LIB names, or the one beside the product binary."""
    named = os.environ.get(FFI_ENV)
    candidates = [Path(named)] if named else []
    for key in ("AGENT_DESKTOP_E2E_BIN", "AGENT_DESKTOP_E2E_BIN_SRC"):
        if os.environ.get(key):
            candidates.append(Path(os.environ[key]).with_name(FFI_LIBRARY))
    for path in candidates:
        if path.is_file():
            return path
    raise NotImplementedError(f"the same-process companion needs {FFI_LIBRARY}; looked at {candidates}")


def session_refs(ctx: LegContext, launched, session: str) -> dict[str, str]:
    return {"A": agents.zoo_ref(ctx, session, launched["A"].app, zoo.BUTTON, None),
            "B": agents.zoo_ref(ctx, session, launched["B"].app, zoo.ENTRY, None),
            "D": agents.zoo_ref(ctx, session, launched["D"].app, zoo.BUTTON, None)}


def _plain_ref(ctx: LegContext, app) -> str:
    found = ctx.product(["find", "--exact", "--limit", "0", "--name", zoo.BUTTON], window_id=app.windows[0],
                        headed=False)
    require(found.ok, f"find {zoo.BUTTON}: {found.raw[:400]}")
    refs = {item["ref_id"] for item in nodes(found.data) if isinstance(item.get("ref_id"), str)}
    require(len(refs) == 1, f"expected one ref named {zoo.BUTTON}, found {refs}")
    return refs.pop()


def agent_argv(name: str, ref: str, timeout: str = agents.AGENT_TIMEOUT_MS, text: str = "q") -> list[str]:
    if name == "B":
        return ["type", ref, text, "--timeout-ms", timeout]
    return ["click", ref, "--timeout-ms", timeout]


def contender_argv(name: str, refs: dict, targets: dict) -> list[str]:
    """B types with a short timeout; D clicks by coordinates, the headed click that waits on the lease."""
    if name == "D":
        return agents.xy_click_argv(targets["D"])
    return agent_argv(name, refs[name], CONTEND_MS)


def effects(launched, name: str, since: float) -> tuple[float, ...]:
    widget, event = (zoo.ENTRY, zoo.CHANGED) if name == "B" else (zoo.BUTTON, zoo.ACTIVATE)
    return agents.effect_times(launched[name].log, widget, event, since)


def strays(launched, allowed: dict[str, list[float]]) -> list[str]:
    return [problem for item in launched.values() for problem in zoo.stray_events(item, allowed)]


def _environ(pid: int) -> list[bytes]:
    return (Path("/proc") / str(pid) / "environ").read_bytes().split(b"\0")


def child_environ(agent: agents.Agent, seconds: float = ENVIRON_S) -> list[bytes] | None:
    """The environment the child's own exec installed, marked by its agent id. A read that lands before the
    exec shows the runner's environment instead, which still holds the lease variable it unset."""
    marker = f"{oracle.AGENT_ENV}={agent.name}".encode()
    limit = time.monotonic() + seconds
    while True:
        with contextlib.suppress(OSError):
            environ = _environ(agent.pid)
            if marker in environ:
                return environ
        if time.monotonic() > limit:
            return None
        time.sleep(0.005)


def inherited_problems(agent: agents.Agent) -> list[str]:
    """The child's environment carries no lease token of the runner's."""
    environ = child_environ(agent)
    if environ is None:
        return [f"{agent.name} never showed its own environment ({oracle.AGENT_ENV}={agent.name})"]
    prefixes = tuple(prefix.encode() for prefix in oracle.LEASE_PREFIXES)
    found = [item.decode(errors="replace").partition("=")[0] for item in environ if item.startswith(prefixes)]
    return [f"{agent.name} inherited lease variables {found}"] if found else []


def delivery_problems(sampler, pid: int, effects_seen: tuple[float, ...], envelope) -> list[str]:
    """The serialized leg's checks on one queued delivery: an observed hold, no effect before it, none
    past the lag bound after it, and, where an envelope exists, a reported wait behind the gate."""
    seen = sampler.events().get(pid, {})
    span = oracle.Span("A", seen.get("enqueue"), seen.get("acquire"), seen.get("release"), effects_seen,
                       oracle.reported_contention(envelope))
    found = oracle.overlap_problems([span]) + oracle.early_effect_problems([span]) + oracle.lag_problems([span])
    if envelope is not None:
        gate = [hold for hold in sampler.holds({os.getpid(): "runner", pid: "A"}) if hold.holder == "runner"]
        found += oracle.queueing_problems([span], oracle.holds_of([span], gate))
    return found


def _contender_problems(tier, launched, sampler, refs, targets, session: str, since: float) -> list[str]:
    problems = []
    for name in "DB":
        contender = agents.spawn(tier, sampler, name, contender_argv(name, refs, targets), session=session,
                                 headed=True)
        with agents.reaped([contender]):
            problems += inherited_problems(contender)
            envelope = contender.finish()
        if envelope is None:
            problems.append(f"contender {name} exited {contender.exit_code} with no envelope")
            continue
        problems += [f"contender {name}: {item}" for item in oracle.lock_timeout_problems(envelope, "lock_timeout")]
        path = ((envelope.error or {}).get("details") or {}).get("path")
        if path != "interaction.lock":
            problems.append(f"contender {name} timed out on {path!r}, not the interaction lease")
        if "acquire" in sampler.events().get(contender.pid, {}):
            problems.append(f"contender {name} held the lease")
        if effects(launched, name, since):
            problems.append(f"contender {name} reached its app while the lease was held")
    return problems


def lease_phase_problems(tier, gate, sampler, launched, refs, targets, session: str, allowed) -> list[str]:
    """A queued waiter, two timed-out contenders, then the waiter's delivery after the gate lets go."""
    since = time.monotonic()
    waiter = agents.spawn(tier, sampler, "A", agent_argv("A", refs["A"]), session=session)
    with agents.reaped([waiter]):
        agents.queued(sampler, [waiter], "the waiter queues behind the gate")
        problems = inherited_problems(waiter)
        problems += _contender_problems(tier, launched, sampler, refs, targets, session, since)
        contended = time.monotonic()
        agents.turn(gate, sampler, waiter, [waiter])
        waiter.finish()
    problems += agents.outcome_problems(waiter)
    agents.settle(lambda: effects(launched, "A", since))
    delivered = effects(launched, "A", since)
    problems += [f"the queued agent: {item}" for item in delivery_problems(sampler, waiter.pid, delivered,
                                                                            waiter.envelope)]
    allowed.setdefault("A", []).extend(delivered)
    if len(delivered) != 1:
        problems.append(f"the queued agent delivered {len(delivered)} clicks after release, not 1")
    elif delivered[0] < contended:
        problems.append("the queued agent's click landed while the contenders were timing out")
    return problems


def _ffi_line(child: subprocess.Popen, seconds: float = agents.EXIT_S) -> dict:
    ready, _, _ = select.select([child.stdout], [], [], seconds)
    require(ready, "the C ABI child printed nothing in time")
    line = child.stdout.readline()
    require(line, "the C ABI child closed its output early")
    return json.loads(line)


def _ffi_envelope(line: dict) -> Envelope:
    raw = line.get("envelope") or ""
    parsed = product._parse_envelope(raw) or {"ok": False}
    return Envelope(ok=parsed["ok"], data=parsed.get("data"), error=parsed.get("error"), raw=raw or json.dumps(line),
                    elapsed_ms=0.0, argv=[], exit_code=0 if line.get("code") == 0 else 1)


def same_process_problems(tier, gate, sampler, launched, ref: str, allowed) -> list[str]:
    """Two C ABI threads in one child: the second times out on the process lease, the first delivers."""
    since = time.monotonic()
    argv = [sys.executable, "-I", str(FFI_CHILD), str(ffi_library()), ref, FFI_HOLDER_MS, FFI_CONTENDER_MS]
    child = subprocess.Popen(argv, env=agents.agent_env(tier, "ffi"), stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                             stderr=subprocess.PIPE, text=True, close_fds=True)
    try:
        sampler.watch(child.pid, "ffi")
        agents.until(lambda: child.pid in sampler.state()[1] or child.poll() is not None,
                     "the C ABI holder thread queues")
        if child.poll() is not None:
            _, stderr = child.communicate(timeout=agents.EXIT_S)
            raise AssertionError(f"the C ABI child exited {child.returncode}: {stderr[:400]}")
        child.stdin.write("\n")
        child.stdin.flush()
        first = _ffi_line(child)
        problems = [] if first.get("thread") == "contender" else [f"the C ABI holder answered first: {first}"]
        problems += [f"same process: {item}" for item in oracle.lock_timeout_problems(
            _ffi_envelope(first), "interaction_process_lock_timeout")]
        if effects(launched, "A", since):
            problems.append("the same-process contender reached the app while the lease was held")
        gate.give()
        second = _ffi_line(child)
        child.wait(agents.EXIT_S)
        gate.take()
        holder = _ffi_envelope(second)
        if not holder.ok:
            problems.append(f"the C ABI holder did not deliver after release: {second}")
        agents.settle(lambda: effects(launched, "A", since))
        delivered = effects(launched, "A", since)
        problems += [f"the C ABI holder: {item}" for item in delivery_problems(sampler, child.pid, delivered,
                                                                                holder)]
        allowed.setdefault("A", []).extend(delivered)
        if len(delivered) != 1:
            problems.append(f"the C ABI holder delivered {len(delivered)} clicks, not 1")
        return problems
    finally:
        if child.poll() is None:
            child.kill()
            child.wait(5.0)


@implementation("parallel.physical_lock_timeout")
def physical_lock_timeout(ctx: LegContext) -> None:
    tier = ctx.tier
    library = ffi_library()
    before = zoo.desktop_of(tier)
    ctx.before("hyprland.desktop", before)
    zoo.common_reads(ctx, "before")
    problems, allowed = [], {}
    with zoo.zoos(tier, "ABD") as launched, zoo.multi_session(ctx) as session:
        with zoo.shown_desktop(ctx, launched, "ABD"):
            refs, plain = session_refs(ctx, launched, session), _plain_ref(ctx, launched["A"].app)
            targets = {"D": live_button(ctx, launched["D"].app, session, zoo.BUTTON)[1]}
            ctx.before("applog.strays", [])
            with agents.released_lease(), agents.LockSampler() as sampler:
                gate = agents.Gate()
                try:
                    gate.take()
                    problems += lease_phase_problems(tier, gate, sampler, launched, refs, targets, session, allowed)
                    problems += same_process_problems(tier, gate, sampler, launched, plain, allowed)
                finally:
                    gate.close()
            found = strays(launched, allowed)
            ctx.after("applog.strays", found)
            problems += found
            ctx.after("hyprland.desktop", zoo.desktop_of(tier))
            zoo.common_reads(ctx, "after")
            ctx.restored("applog.strays", strays(launched, allowed))
    ctx.finding(f"the same-process companion ran through {library.name}")
    zoo.common_reads(ctx, "restored")
    ctx.restored("hyprland.desktop", zoo.desktop_of(tier))
    require(not problems, "; ".join(problems))
