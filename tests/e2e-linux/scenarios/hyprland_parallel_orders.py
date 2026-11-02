"""The three orders and the headed click overlap pass of the four-agent leg.

A is a headless click on a GTK3 zoo, B a headed `type` into a GTK4 zoo, C
a `screenshot --screen` that takes no lease, and D a headed `mouse-click
--xy` at the centre of a Qt zoo button. On Hyprland a headed ref click
refuses before any event, so D clicks by coordinates and the Qt zoo's log is
the evidence; a companion check asserts that D's headed ref click refuses.
A, B and D start together behind the gate and the gate hands the
lease to one of them at a time while the others stay stopped. C starts
inside a holder's turn; where an order needs its frame to wait, C is
stopped once its hide holder file appears and resumed at the next barrier.
In pass 3, D is stopped only once its virtual pointer appears, which it opens
after its hide has finished. Every barrier is an observed event: a
`/proc/locks` holder, an open lock descriptor, a holder file, a compositor
device or an exit, never a sleep.
"""

from __future__ import annotations

import signal
import time
from dataclasses import dataclass, field

from oracle import hyprland, pixels
from runner.helper import require
from runner.verdict import LegContext
from scenarios import parallel_agents as agents
from scenarios import parallel_oracle as oracle
from scenarios import parallel_zoo as zoo
from scenarios.hyprland_capture_common import capture_path
from scenarios.hyprland_capture_screen import grim
from scenarios.hyprland_common import main_instance, shared
from scenarios.hyprland_cursor import arrow_box, live_button, monitor_at, record_of
from scenarios.hyprland_cursor_agents import surfaces_of
from scenarios.hyprland_cursor_hide import arrow_shown, holders_in, registry_of
from scenarios.hyprland_focus_controls import restore_desktop

LETTERS = {"order 1": "w", "order 2": "x", "order 3": "y", "pass 3": "z"}
HOLDER_S = 10.0


@dataclass
class Setup:
    ctx: LegContext
    launched: dict
    session: str
    holder: dict
    point: dict
    refs: dict
    targets: dict
    monitor: dict
    index: int
    renderer: dict | None = None
    positives: list = field(default_factory=list)

    def note_renderer(self) -> list[str]:
        """The session renderer starts at its first headed present; the first record seen is the one
        every later read must match."""
        now = record_of(shared(self.ctx.tier), self.session)
        if self.renderer is None:
            self.renderer = now
            return []
        if not now or now.get("pid") != self.renderer.get("pid"):
            return [f"the session renderer changed from {self.renderer} to {now}"]
        return []

    def final_problems(self) -> list[str]:
        problems = [] if self.positives else ["no C capture ran while grim showed a drawn agent cursor"]
        if self.renderer is None:
            return problems + [f"no headed agent started a renderer for the session {self.session}"]
        return problems + self.note_renderer()


def prepared(ctx: LegContext, launched: dict, session: str, holder: dict, point: dict) -> Setup:
    tier, main = ctx.tier, main_instance(ctx.tier)
    refs, targets = {}, {}
    for name in "AD":
        refs[name], targets[name] = live_button(ctx, launched[name].app, session, zoo.BUTTON)
    refs["B"] = agents.zoo_ref(ctx, session, launched["B"].app, zoo.ENTRY, None)
    monitors = raw_monitors(tier, main)
    monitor = monitor_at(monitors, targets["D"])
    listed = ctx.product(["list-displays"], headed=False)
    ids = [item.get("id") for item in listed.data or []] if listed.ok else []
    require(monitor["name"] in ids, f"list-displays does not list {monitor['name']}: {listed.raw[:400]}")
    return Setup(ctx, launched, session, holder, point, refs, targets, monitor, ids.index(monitor["name"]))


def raw_monitors(tier, main: dict) -> list[dict]:
    return hyprland.query(shared(tier), main["signature"], "monitors", pid=main["pid"])


def refused_without_agent(ctx: LegContext, setup: Setup) -> list[str]:
    """The missing agent id companion: a click with no agent id is refused and the app sees nothing."""
    log = setup.launched["D"]
    before = len(log.events(zoo.BUTTON, zoo.ACTIVATE))
    env = {key: value for key, value in ctx.tier.env.items() if key != oracle.AGENT_ENV}
    envelope = ctx.product(["--session", setup.session, "click", setup.refs["D"]], env=env)
    problems = []
    if envelope.ok or (envelope.error or {}).get("code") != "INVALID_ARGS" or envelope.exit_code != 1:
        problems.append(f"a click with no agent id was not refused with INVALID_ARGS: {envelope.raw[:400]}")
    if len(log.events(zoo.BUTTON, zoo.ACTIVATE)) != before:
        problems.append("a click with no agent id reached the Qt zoo")
    return problems


def ref_click_refused(setup: Setup, sampler) -> list[str]:
    """The headed ref click companion: D's headed `click @ref` refuses as unproven and the Qt zoo
    logs nothing."""
    since = time.monotonic()
    argv = ["click", setup.refs["D"], "--timeout-ms", agents.AGENT_TIMEOUT_MS]
    agent = agents.spawn(setup.ctx.tier, sampler, "D", argv, session=setup.session, headed=True)
    with agents.reaped([agent]):
        agent.finish()
    log = setup.launched["D"].log
    landed = agents.settle(lambda: bool(agents.effect_times(log, zoo.BUTTON, zoo.ACTIVATE, since)), 1.0)
    problems = [f"headed ref click: {item}" for item in oracle.ref_refusal_problems(agent.envelope)]
    return problems + (["headed ref click: the refused click reached the Qt zoo"] if landed else [])


def pointer_devices(tier) -> set[tuple]:
    """Every pointer device the compositor lists, virtual ones included, by address and name."""
    main = main_instance(tier)
    found = hyprland.query(shared(tier), main["signature"], "devices", pid=main["pid"]).get("mice", [])
    return {(item.get("address"), item.get("name")) for item in found}


def _tip(surface: dict) -> tuple[float, float]:
    return surface["x"] + 1, surface["y"] + 1


class Schedule:
    """One order: its agents, its C capture and the oracle over what the sampler and logs saw."""

    def __init__(self, setup: Setup, gate, sampler, label: str):
        self.setup, self.gate, self.sampler, self.label = setup, gate, sampler, label
        self.tier = setup.ctx.tier
        self.agents: dict[str, agents.Agent] = {}
        self.capture = None
        self.c_tips: list[tuple[float, float]] = []
        self.problems: list[str] = []
        self.spans: list[oracle.Span] = []
        self.pointers: set[tuple] = set()
        self.since = 0.0

    def argv(self, name: str) -> list[str]:
        ref, timeout = self.setup.refs[name], ["--timeout-ms", agents.AGENT_TIMEOUT_MS]
        if name == "B":
            return ["type", ref, LETTERS[self.label], *timeout]
        if name == "D":
            return agents.xy_click_argv(self.setup.targets["D"])
        return ["click", ref, *timeout]

    def spawn(self, names: str) -> None:
        group = []
        for name in names:
            group.append(agents.spawn(self.tier, self.sampler, name, self.argv(name), session=self.setup.session,
                                      headed=name in "BD"))
            self.agents[name] = group[-1]
        agents.queued(self.sampler, group, f"{self.label}: {names} queue")

    def turn(self, name: str, during=None) -> None:
        agent = self.agents[name]
        before = zoo.desktop_of(self.tier) if name == "A" else None
        if name == "B":
            zoo.focus_entry(self.tier, self.setup.launched["B"])
        agents.turn(self.gate, self.sampler, agent, list(self.agents.values()), during)
        if before is not None and zoo.desktop_of(self.tier) != before:
            self.problems.append(f"{self.label}: A changed the desktop: {before} to {zoo.desktop_of(self.tier)}")

    def drawn_tips(self) -> list[tuple[float, float]]:
        """The agent cursor tips grim shows drawn on the leg's monitor right now."""
        image = grim(self.tier, self.setup.monitor["name"])
        found = []
        for surface in surfaces_of(shared(self.tier), main_instance(self.tier)):
            if surface["monitor"] == self.setup.monitor["name"] and arrow_shown(image, self._box(image, _tip(surface))):
                found.append(_tip(surface))
        return found

    def _box(self, image, tip) -> tuple[int, int, int, int]:
        monitor = self.setup.monitor
        return arrow_box((monitor["x"], monitor["y"]), tip, monitor["scale"], (image.width, image.height))

    def start_c(self, hold: bool) -> None:
        self.c_tips = self.drawn_tips()
        self.setup.positives += self.c_tips
        self.capture = capture_path(self.tier, f"parallel-c-{self.label.replace(' ', '-')}")
        argv = ["screenshot", "--screen", str(self.setup.index), str(self.capture)]
        self.agents["C"] = agents.spawn(self.tier, self.sampler, "C", argv, session=self.setup.session)
        if hold:
            pid = self.agents["C"].pid
            registry = registry_of(shared(self.tier), self.setup.session)
            agents.until(lambda: any(item.get("pid") == pid for item in holders_in(registry)),
                         f"{self.label}: C writes its hide holder", HOLDER_S)
            self.agents["C"].signal(signal.SIGSTOP)
            require(self.agents["C"].process.poll() is None and not self.capture.exists(),
                    f"{self.label}: C's frame arrived before the barrier held it")

    def finish_c(self) -> None:
        agent = self.agents["C"]
        agent.signal(signal.SIGCONT)
        agent.finish()
        self.problems += [f"{self.label}: {problem}" for problem in agents.outcome_problems(agent)]
        if not self.capture.exists():
            self.problems.append(f"{self.label}: C wrote no PNG")
            return
        image = pixels.decode_png(self.capture.read_bytes())
        self.capture.unlink()
        shown = [tip for tip in self.c_tips if arrow_shown(image, self._box(image, tip))]
        if shown:
            self.problems.append(f"{self.label}: C's PNG shows an agent cursor at {shown}")

    def pause_d(self) -> None:
        """Pass 3: D holds the lease, finishes its hide and stops before its click; A, B and C act
        meanwhile. The hide's 500 ms acknowledgement deadline runs on the wall clock, so D stops only
        once it opens its virtual pointer, which it does after the hide has finished."""
        d = self.agents["D"]
        registry = registry_of(shared(self.tier), self.setup.session)
        agents.until(lambda: any(item.get("pid") == d.pid for item in holders_in(registry)),
                     "pass 3: D writes its hide holder", HOLDER_S)
        agents.until(lambda: pointer_devices(self.tier) - self.pointers or d.process.poll() is not None,
                     "pass 3: D opens its virtual pointer after its hide", HOLDER_S)
        if d.process.poll() is not None:
            d.finish()
            raise AssertionError(f"pass 3: D ended before its hide finished: {agents.outcome_problems(d)}")
        d.signal(signal.SIGSTOP)
        if agents.effect_times(self.setup.launched["D"].log, zoo.BUTTON, zoo.ACTIVATE, self.since):
            raise AssertionError("pass 3: D clicked before the barrier stopped it")
        self.spawn("A")
        image = grim(self.tier, self.setup.monitor["name"])
        if arrow_shown(image, self._box(image, self.setup.targets["A"])):
            self.problems.append("pass 3: A's travel drew a cursor while D's holder hides them")
        self.agents["A"].signal(signal.SIGSTOP)
        self.spawn("B")
        self.agents["B"].signal(signal.SIGSTOP)
        before = zoo.desktop_of(self.tier)
        self.start_c(hold=False)
        self.finish_c()
        if zoo.desktop_of(self.tier) != before:
            self.problems.append(f"pass 3: C changed the desktop: {before} to {zoo.desktop_of(self.tier)}")
        d.signal(signal.SIGCONT)

    def run(self, steps) -> list[str]:
        restore_desktop(self.tier, self.setup.holder, self.setup.point)
        self.gate.take()
        self.since = time.monotonic()
        try:
            steps(self)
            for agent in self.agents.values():
                if agent.pidfd >= 0:
                    agent.finish()
        finally:
            for agent in self.agents.values():
                agent.kill()
        return self.verdict()

    def verdict(self) -> list[str]:
        problems = list(self.problems) + [f"{self.label}: {item}" for item in self.setup.note_renderer()]
        for name in "ABD":
            problems += [f"{self.label}: {problem}" for problem in agents.outcome_problems(self.agents[name])]
        events = self.sampler.events()
        effects = self._settled_effects()
        self.spans = [agents.span_of(self.agents[name], events, effects[name]) for name in "ABD"]
        names = agents.holder_names(list(self.agents.values()))
        holds = self.sampler.holds(names)
        gate_holds = [hold for hold in holds if hold.holder == "runner"]
        order = ORDER_OF[self.label]
        found = (oracle.overlap_problems(self.spans) + oracle.queueing_problems(self.spans, oracle.holds_of(
            self.spans, gate_holds)) + oracle.order_problems(self.spans, order)
                 + oracle.control_problems(self.spans, gate_holds, order))
        found += [f"A or D: {problem}" for problem in oracle.effect_problems(
            [span for span in self.spans if span.agent in "AD"])]
        found += oracle.early_effect_problems([span for span in self.spans if span.agent == "B"])
        found += oracle.lag_problems(self.spans)
        found += self._typed_problems(effects["B"])
        if any(hold.holder == "C" for hold in holds):
            found.append("C held the interaction lease")
        return problems + [f"{self.label}: {problem}" for problem in found]

    def _effects(self) -> dict[str, tuple[float, ...]]:
        launched = self.setup.launched
        effects = {name: agents.effect_times(launched[name].log, zoo.BUTTON, zoo.ACTIVATE, self.since)
                   for name in "AD"}
        effects["B"] = agents.effect_times(launched["B"].log, zoo.ENTRY, zoo.CHANGED, self.since)
        return effects

    def _settled_effects(self) -> dict[str, tuple[float, ...]]:
        agents.settle(lambda: all(self._effects().values()))
        return self._effects()

    def _typed_problems(self, changed) -> list[str]:
        letter = LETTERS[self.label]
        text = zoo.entry_text(self.tier, self.setup.launched["B"]) or ""
        values = [item["value"] for item in self.setup.launched["B"].events(zoo.ENTRY, zoo.CHANGED)
                  if item["at"] >= self.since]
        if not changed or text.count(letter) != 1 or values[-1:] != [text]:
            return [f"B typed {letter!r} but the entry reads {text!r} after changes {values}"]
        return []


def order_1(schedule: Schedule) -> None:
    schedule.spawn("ABD")
    schedule.turn("B", during=lambda: schedule.start_c(hold=True))
    schedule.turn("A")
    schedule.finish_c()
    schedule.turn("D")


def order_2(schedule: Schedule) -> None:
    schedule.spawn("ABD")
    schedule.turn("D", during=lambda: schedule.start_c(hold=True))
    schedule.turn("B", during=schedule.finish_c)
    schedule.turn("A")


def order_3(schedule: Schedule) -> None:
    schedule.spawn("ABD")

    def during() -> None:
        before = zoo.desktop_of(schedule.tier)
        schedule.start_c(hold=False)
        schedule.finish_c()
        if zoo.desktop_of(schedule.tier) != before:
            schedule.problems.append(f"order 3: C changed the desktop: {before} to {zoo.desktop_of(schedule.tier)}")
    schedule.turn("A", during=during)
    schedule.turn("D")
    schedule.turn("B")


def pass_3(schedule: Schedule) -> None:
    schedule.pointers = pointer_devices(schedule.tier)
    schedule.spawn("D")
    schedule.turn("D", during=schedule.pause_d)
    schedule.agents["A"].signal(signal.SIGCONT)
    schedule.turn("A")
    schedule.turn("B")


ORDERS = (("order 1", order_1), ("order 2", order_2), ("order 3", order_3), ("pass 3", pass_3))
ORDER_OF = {"order 1": "BAD", "order 2": "DBA", "order 3": "ADB", "pass 3": "DAB"}
