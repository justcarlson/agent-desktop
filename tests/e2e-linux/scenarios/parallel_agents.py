"""Runner machinery for the four-agent legs: the lease phase, the gate, the lock sampler and the agent children.

The runner holds the harness interaction lease through run.sh. A lease phase
unlocks that descriptor so agent children can take the lease themselves, and
locks it again after they all exit. Inside the phase a gate, a fresh
descriptor on the same lock file, holds the lease so the children queue. A
sampler thread reads /proc/locks and each child's descriptor table about
every millisecond, so enqueue, acquire and release are observed, not
assumed. Barriers wait on those observations; the order of service comes
from stopping the waiters that are not next with SIGSTOP on their pidfds.
"""

from __future__ import annotations

import contextlib
import fcntl
import json
import os
import signal
import subprocess
import threading
import time
from dataclasses import dataclass, field
from pathlib import Path

from runner import product
from runner.helper import nodes, require
from runner.product import Envelope
from scenarios import parallel_oracle as oracle
from scenarios.parallel_oracle import Hold, Span

LEASE_FD_ENV = product.LEASE_FD_ENV
LOCK_PATH = Path("/tmp") / f"agent-desktop-{os.geteuid()}" / "interaction.lock"
SAMPLE_S = 0.001
RELOCK_S = 60.0
BARRIER_S = 30.0
AGENT_TIMEOUT_MS = "30000"
EXIT_S = 40.0
EFFECT_S = 5.0


def _relock(fd: int, seconds: float) -> None:
    deadline = time.monotonic() + seconds
    while True:
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            return
        except BlockingIOError:
            if time.monotonic() >= deadline:
                raise AssertionError(f"the runner could not take its interaction lease back in {seconds:.0f} s")
            time.sleep(SAMPLE_S)


@contextlib.contextmanager
def released_lease():
    """The harness lease unlocked for the children, and locked again once they are gone."""
    raw_fd = os.environ.get(LEASE_FD_ENV)
    require(raw_fd is not None, "the four-agent legs need the interaction lease run.sh holds")
    fd = int(raw_fd)
    fcntl.flock(fd, fcntl.LOCK_UN)
    os.environ.pop(LEASE_FD_ENV)
    try:
        yield
    finally:
        try:
            _relock(fd, RELOCK_S)
        finally:
            os.environ[LEASE_FD_ENV] = raw_fd


class Gate:
    """A runner-held lease on a descriptor of its own, so the children queue behind it."""

    def __init__(self) -> None:
        self.fd = os.open(LOCK_PATH, os.O_RDWR | os.O_CLOEXEC | os.O_NOFOLLOW)
        self.held = False

    def take(self, seconds: float = BARRIER_S) -> None:
        _relock(self.fd, seconds)
        self.held = True

    def give(self) -> None:
        fcntl.flock(self.fd, fcntl.LOCK_UN)
        self.held = False

    def close(self) -> None:
        if self.held:
            self.give()
        os.close(self.fd)


def _opens_lock(pid: int) -> bool:
    directory = Path("/proc") / str(pid) / "fd"
    try:
        for entry in os.listdir(directory):
            with contextlib.suppress(OSError):
                if os.readlink(directory / entry) == str(LOCK_PATH):
                    return True
    except OSError:
        return False
    return False


class LockSampler:
    """Every change in who holds the lease and which watched pids have the lock file open."""

    def __init__(self) -> None:
        self.inode = os.stat(LOCK_PATH).st_ino
        self.samples: list[tuple[float, frozenset, frozenset]] = []
        self.watched: dict[int, str] = {}
        self._state: tuple[frozenset, frozenset] = (frozenset(), frozenset())
        self._lock = threading.Lock()
        self._stop = threading.Event()
        self._thread = threading.Thread(target=self._run, daemon=True)

    def __enter__(self) -> "LockSampler":
        self._thread.start()
        return self

    def __exit__(self, *_exc) -> None:
        self._stop.set()
        self._thread.join(5.0)

    def watch(self, pid: int, name: str) -> None:
        with self._lock:
            self.watched[pid] = name

    def _read(self) -> tuple[frozenset, frozenset]:
        holders = frozenset(oracle.lock_holders(Path("/proc/locks").read_text(), self.inode))
        with self._lock:
            pids = list(self.watched)
        return holders, frozenset(pid for pid in pids if _opens_lock(pid))

    def _run(self) -> None:
        while not self._stop.is_set():
            moment = time.monotonic()
            state = self._read()
            with self._lock:
                if state != self._state or not self.samples:
                    self.samples.append((moment, *state))
                    self._state = state
            time.sleep(SAMPLE_S)

    def state(self) -> tuple[frozenset, frozenset]:
        with self._lock:
            return self._state

    def wait_for(self, check, label: str, seconds: float = BARRIER_S) -> None:
        deadline = time.monotonic() + seconds
        while not check(*self.state()):
            if time.monotonic() >= deadline:
                raise AssertionError(f"barrier {label} not observed in {seconds:.0f} s; holders and queue "
                                     f"{self.state()}")
            time.sleep(SAMPLE_S)

    def events(self) -> dict[int, dict]:
        with self._lock:
            return oracle.lease_events(list(self.samples))

    def holds(self, names: dict[int, str]) -> list[Hold]:
        with self._lock:
            return oracle.hold_intervals(list(self.samples), names)


def until(check, label: str, seconds: float = BARRIER_S) -> None:
    """Polls `check` about every millisecond; a barrier that is not observed in time fails the leg."""
    deadline = time.monotonic() + seconds
    while not check():
        if time.monotonic() >= deadline:
            raise AssertionError(f"barrier {label} not observed in {seconds:.0f} s")
        time.sleep(SAMPLE_S)


def settle(check, seconds: float = EFFECT_S) -> bool:
    """Polls `check` until it holds or `seconds` pass. An app's handler may run after its agent has
    released the lease and exited, so effects are read once they land; a missing one stays missing."""
    deadline = time.monotonic() + seconds
    while not check():
        if time.monotonic() >= deadline:
            return False
        time.sleep(SAMPLE_S)
    return True


def _start_ticks(pid: int) -> int:
    stat = (Path("/proc") / str(pid) / "stat").read_text()
    return int(stat[stat.rindex(")") + 2:].split()[19])


@dataclass
class Agent:
    """One agent child process, signalled only through its pidfd."""

    name: str
    process: subprocess.Popen
    started: int
    pidfd: int
    argv: list[str] = field(default_factory=list)
    envelope: Envelope | None = None
    exit_code: int | None = None

    @property
    def pid(self) -> int:
        return self.process.pid

    def signal(self, number: int) -> None:
        if self.pidfd >= 0 and self.process.poll() is None:
            signal.pidfd_send_signal(self.pidfd, number)

    def finish(self, seconds: float = EXIT_S) -> Envelope | None:
        stdout, stderr = self.process.communicate(timeout=seconds)
        self.exit_code = self.process.returncode
        os.close(self.pidfd)
        self.pidfd = -1
        parsed = product._parse_envelope(stdout)
        if parsed is not None:
            self.envelope = Envelope(ok=parsed["ok"], data=parsed.get("data"), error=parsed.get("error"),
                                     raw=stdout, elapsed_ms=0.0, argv=self.argv, exit_code=self.exit_code,
                                     stderr=stderr)
        return self.envelope

    def kill(self) -> None:
        if self.pidfd >= 0 and self.process.poll() is None:
            signal.pidfd_send_signal(self.pidfd, signal.SIGCONT)
            signal.pidfd_send_signal(self.pidfd, signal.SIGKILL)
        if self.process.poll() is None:
            self.process.wait(5.0)
        if self.pidfd >= 0:
            os.close(self.pidfd)
            self.pidfd = -1


def agent_env(tier, name: str) -> dict[str, str]:
    env = dict(tier.env)
    env.setdefault("HOME", str(Path.home()))
    env.setdefault("LANG", "C.UTF-8")
    return oracle.child_env(env, name)


def spawn(tier, sampler: LockSampler | None, name: str, argv: list[str], *, session: str,
          headed: bool = False) -> Agent:
    """An agent child in the session under its own agent id, with no inherited lease."""
    full = [product._binary_path(), "--session", session, "--agent-id", name, *(["--headed"] if headed else []),
            *argv]
    process = subprocess.Popen(full, env=agent_env(tier, name), stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                               stdin=subprocess.DEVNULL, text=True, close_fds=True)
    pidfd = os.pidfd_open(process.pid)
    agent = Agent(name, process, _start_ticks(process.pid), pidfd, full)
    if sampler is not None:
        sampler.watch(process.pid, name)
    return agent


@contextlib.contextmanager
def reaped(agents: list[Agent]):
    """Every agent child gone on the way out, by its own pidfd."""
    try:
        yield agents
    finally:
        for agent in agents:
            agent.kill()


def serve(gate: Gate, sampler: LockSampler, order: list[Agent], before_each=None, after_each=None) -> None:
    """Hands the gate's lease to each agent in turn while the others stay stopped."""
    require(gate.held, "the gate must hold the lease before it serves")
    for index, agent in enumerate(order):
        waiting = [other for other in order[index + 1:] if other.process.poll() is None]
        for other in waiting:
            other.signal(signal.SIGSTOP)
        try:
            if before_each:
                before_each(agent)
            gate.give()
            sampler.wait_for(lambda holders, _queued: agent.pid in holders, f"{agent.name} acquires")
            sampler.wait_for(lambda holders, _queued: agent.pid not in holders, f"{agent.name} releases")
            gate.take()
            if after_each:
                after_each(agent)
        finally:
            for other in waiting:
                other.signal(signal.SIGCONT)


def turn(gate: Gate, sampler: LockSampler, agent: Agent, others: list[Agent], during=None) -> None:
    """Hands the gate's lease to `agent` alone; `during` runs once it holds, and every other live
    agent stays stopped until the gate holds the lease again."""
    require(gate.held, "the gate must hold the lease before a turn")
    stopped = [other for other in others if other is not agent and other.process.poll() is None]
    for other in stopped:
        other.signal(signal.SIGSTOP)
    try:
        gate.give()
        sampler.wait_for(lambda holders, _queued: agent.pid in holders, f"{agent.name} acquires")
        if during:
            during()
        sampler.wait_for(lambda holders, _queued: agent.pid not in holders, f"{agent.name} releases")
        gate.take()
    finally:
        for other in stopped:
            other.signal(signal.SIGCONT)


def queued(sampler: LockSampler, agents: list[Agent], label: str) -> None:
    pids = {agent.pid for agent in agents}
    sampler.wait_for(lambda _holders, waiting: pids <= waiting, label)


def app_events(log: Path, widget: str, event: str) -> list[dict]:
    if not log.exists():
        return []
    found = [json.loads(line) for line in log.read_text().rpartition("\n")[0].splitlines() if line.strip()]
    return [item for item in found if item.get("widget") == widget and item.get("event") == event]


def effect_times(log: Path, widget: str, event: str, since: float) -> tuple[float, ...]:
    return tuple(item["at"] for item in app_events(log, widget, event) if item["at"] >= since)


def span_of(agent: Agent, events: dict[int, dict], effects: tuple[float, ...]) -> Span:
    seen = events.get(agent.pid, {})
    return Span(agent.name, seen.get("enqueue"), seen.get("acquire"), seen.get("release"), effects,
                oracle.reported_contention(agent.envelope))


def holder_names(agents: list[Agent]) -> dict[int, str]:
    return {os.getpid(): "runner", **{agent.pid: agent.name for agent in agents}}


def xy_click_argv(point: tuple[float, float]) -> list[str]:
    """A headed click at a target's centre by coordinates. On Hyprland a headed ref click refuses
    before any event, because the compositor cannot prove which surface takes the pointer;
    `mouse-click --xy` reports `receiver_proof: false`, and the app's log is the evidence."""
    return ["mouse-click", "--xy", f"{round(point[0])},{round(point[1])}"]


def zoo_ref(ctx, session: str, app, name: str, env: dict[str, str]) -> str:
    """The one qualified ref named `name` in the zoo window, found in the session."""
    found = ctx.product(["--session", session, "find", "--exact", "--limit", "0", "--name", name],
                        env=env, window_id=app.windows[0], headed=False)
    require(found.ok, f"find {name}: {found.raw[:400]}")
    refs = {item["ref_id"] for item in nodes(found.data) if isinstance(item.get("ref_id"), str)}
    require(len(refs) == 1, f"expected one ref named {name} in {app.name}, found {refs}")
    return refs.pop()


def outcome_problems(agent: Agent) -> list[str]:
    envelope = agent.envelope
    if envelope is None:
        return [f"{agent.name}: exited {agent.exit_code} with no envelope"]
    if not envelope.ok or agent.exit_code != 0:
        return [f"{agent.name}: exited {agent.exit_code}: {failure_summary(envelope)}"]
    return []


def failure_summary(envelope: Envelope) -> str:
    """The error without its suggestion or passing checks, so a refusal keeps the check and occluder it names."""
    if not envelope.error:
        return envelope.raw[:400]
    error = {key: value for key, value in envelope.error.items() if key != "suggestion"}
    details = error.get("details")
    if isinstance(details, dict) and isinstance(details.get("checks"), list):
        failed = [check for check in details["checks"] if not isinstance(check, dict) or check.get("status") != "pass"]
        error["details"] = {**details, "checks": failed}
    return json.dumps(error, sort_keys=True)[:1200]
