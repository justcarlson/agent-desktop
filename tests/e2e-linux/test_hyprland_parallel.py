from __future__ import annotations

import dataclasses
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from runner import legs
from runner.product import Envelope
from scenarios import parallel_oracle as oracle
from scenarios.parallel_oracle import Hold, Span

LEG_IDS = ("parallel.four_agents_serialized_dispatch", "parallel.four_agents_three_interleavings",
           "parallel.physical_lock_timeout", "parallel.cancel_waiter_and_holder")


def envelope(ok: bool, error: dict | None, exit_code: int, data: dict | None = None) -> Envelope:
    return Envelope(ok=ok, data=data, error=error, raw="{}", elapsed_ms=5.0, argv=[], exit_code=exit_code)


def serial() -> list[Span]:
    """Four agents queued at once behind a gate held until 1.0 s, served A, B, C, D."""
    return [
        Span("A", enqueue=0.10, acquire=1.00, release=1.20, effects=(1.15,), contention=40),
        Span("B", enqueue=0.11, acquire=1.21, release=1.40, effects=(1.30,), contention=60),
        Span("C", enqueue=0.12, acquire=1.41, release=1.60, effects=(1.55,), contention=80),
        Span("D", enqueue=0.13, acquire=1.61, release=1.80, effects=(1.70,), contention=100),
    ]


GATE = [Hold("gate", 0.0, 0.99)]


class ParallelLegRegistryTests(unittest.TestCase):
    def test_every_leg_is_registered_on_hyprland_with_an_implementation(self):
        for leg_id in LEG_IDS:
            leg = legs.leg_by_id(leg_id)
            self.assertEqual(leg.tiers, legs.HYPRLAND_ONLY)
            self.assertIn("proc", leg.oracles)
            self.assertIn("applog", leg.oracles)
            self.assertIn(leg_id, legs.implementations())
        self.assertEqual(legs.leg_by_id(LEG_IDS[0]).modes, legs.HEADLESS)
        for leg_id in LEG_IDS[1:]:
            self.assertEqual(legs.leg_by_id(leg_id).modes, legs.HEADED)
        self.assertEqual(legs.leg_by_id("parallel.physical_lock_timeout").disposition, "asserted-refusal")
        self.assertTrue(legs.leg_by_id("parallel.physical_lock_timeout").reason)


class DispatchIntervalTests(unittest.TestCase):
    def test_a_serial_timeline_has_no_problems(self):
        self.assertEqual(oracle.timeline_problems(serial(), GATE, "ABCD"), [])

    def test_the_interval_is_the_lease_hold(self):
        span = Span("A", enqueue=0.0, acquire=1.0, release=1.2, effects=(1.1, 1.5), contention=0)
        self.assertEqual(oracle.dispatch_interval(span), (1.0, 1.2))
        self.assertIsNone(oracle.dispatch_interval(Span("B", enqueue=0.0, acquire=None, release=None)))

    def test_two_overlapping_holds_are_found(self):
        spans = serial()
        spans[1] = Span("B", enqueue=0.11, acquire=1.10, release=1.40, effects=(1.30,), contention=60)
        found = oracle.overlap_problems(spans)
        self.assertEqual(len(found), 1)
        self.assertIn("A", found[0])
        self.assertIn("B", found[0])

    def test_a_toolkit_effect_after_release_is_a_lag_not_an_overlap(self):
        spans = serial()
        spans[0] = Span("A", enqueue=0.10, acquire=1.00, release=1.20, effects=(1.45,), contention=40)
        spans[1] = Span("B", enqueue=0.11, acquire=1.21, release=1.40, effects=(1.30,), contention=60)
        self.assertEqual(oracle.overlap_problems(spans), [])
        self.assertEqual(oracle.effect_problems(spans), [])
        self.assertEqual({name: round(lag) for name, lag in oracle.effect_lags_ms(spans).items()}, {"A": 250})

    def test_an_effect_before_its_own_acquire_still_fails(self):
        spans = serial()
        spans[1] = Span("B", enqueue=0.11, acquire=1.21, release=1.40, effects=(1.15,), contention=60)
        self.assertTrue(any("before its acquire" in item for item in oracle.effect_problems(spans)))

    def test_touching_intervals_do_not_overlap(self):
        spans = [Span("A", enqueue=0.0, acquire=1.0, release=2.0, contention=0),
                 Span("B", enqueue=0.0, acquire=2.0, release=3.0, contention=0)]
        self.assertEqual(oracle.overlap_problems(spans), [])

    def test_an_unobserved_acquire_is_a_problem_not_a_pass(self):
        spans = serial()
        spans[2] = Span("C", enqueue=0.12, acquire=None, release=None, effects=(1.55,), contention=80)
        self.assertTrue(any("C" in problem for problem in oracle.overlap_problems(spans)))


class EffectTests(unittest.TestCase):
    def test_each_agent_needs_exactly_one_effect_after_its_acquire(self):
        self.assertEqual(oracle.effect_problems(serial()), [])
        spans = serial()
        spans[0] = Span("A", enqueue=0.10, acquire=1.00, release=1.20, effects=(), contention=40)
        spans[1] = Span("B", enqueue=0.11, acquire=1.21, release=1.40, effects=(1.30, 1.31), contention=60)
        spans[2] = Span("C", enqueue=0.12, acquire=1.41, release=1.60, effects=(1.39,), contention=80)
        found = oracle.effect_problems(spans)
        self.assertEqual(len(found), 3)
        self.assertEqual([problem.split(":")[0] for problem in found], ["A", "B", "C"])

    def test_an_expected_count_of_zero_needs_no_effect(self):
        span = Span("W", enqueue=0.1, acquire=None, release=None, effects=(), contention=None)
        self.assertEqual(oracle.effect_problems([span], expected=0), [])
        late = Span("W", enqueue=0.1, acquire=None, release=None, effects=(2.0,), contention=None)
        self.assertEqual(len(oracle.effect_problems([late], expected=0)), 1)



class AppLogTests(unittest.TestCase):
    def test_an_effect_before_its_own_hold_fails_for_every_agent_including_a_typist(self):
        self.assertEqual(oracle.early_effect_problems(serial()), [])
        spans = serial()
        spans[1] = Span("B", enqueue=0.11, acquire=1.21, release=1.40, effects=(1.30, 1.15), contention=60)
        found = oracle.early_effect_problems(spans)
        self.assertEqual(len(found), 1)
        self.assertTrue(found[0].startswith("B: an effect at 1.1500"), found)

    def test_an_effect_at_its_own_acquire_is_inside_the_hold(self):
        spans = [Span("A", enqueue=0.1, acquire=1.0, release=1.2, effects=(1.0,), contention=0)]
        self.assertEqual(oracle.early_effect_problems(spans), [])

    def test_a_lag_within_the_bound_passes_and_one_past_it_fails(self):
        bound = oracle.LAG_BOUND_MS / 1000.0
        within = [Span("B", enqueue=0.1, acquire=1.0, release=1.2, effects=(1.2 + bound - 0.001,), contention=0)]
        past = [Span("B", enqueue=0.1, acquire=1.0, release=1.2, effects=(1.2 + bound + 0.001,), contention=0)]
        self.assertEqual(oracle.lag_problems(within), [])
        found = oracle.lag_problems(past)
        self.assertEqual(len(found), 1)
        self.assertIn(f"past the {oracle.LAG_BOUND_MS:.0f} ms bound", found[0])
        self.assertTrue(any("bound" in problem for problem in oracle.timeline_problems(past, [], "B")))

    def test_each_app_logs_its_effects_in_its_agents_hold_order(self):
        rounds = [Span("A", enqueue=0.1, acquire=1.0, release=1.2, effects=(1.1,), contention=0),
                  Span("A", enqueue=2.0, acquire=2.1, release=2.3, effects=(2.2,), contention=0),
                  Span("B", enqueue=0.1, acquire=1.3, release=1.5, effects=(1.6,), contention=1)]
        self.assertEqual(oracle.effect_order_problems(rounds), [])
        swapped = [rounds[0], dataclasses.replace(rounds[1], effects=(1.05,)), rounds[2]]
        found = oracle.effect_order_problems(swapped)
        self.assertEqual(len(found), 1)
        self.assertTrue(found[0].startswith("A:"), found)

    def test_an_effect_that_lands_after_the_next_hold_of_its_agent_is_out_of_order(self):
        rounds = [Span("D", enqueue=0.1, acquire=1.0, release=1.2, effects=(2.15,), contention=0),
                  Span("D", enqueue=2.0, acquire=2.1, release=2.3, effects=(2.2,), contention=0)]
        self.assertEqual(len(oracle.effect_order_problems(rounds)), 1)


class QueueingTests(unittest.TestCase):
    def test_a_wait_behind_another_holder_must_be_reported(self):
        spans = serial()
        spans[1] = Span("B", enqueue=0.11, acquire=1.21, release=1.40, effects=(1.30,), contention=0)
        found = oracle.queueing_problems(spans, oracle.holds_of(spans, GATE))
        self.assertEqual(len(found), 1)
        self.assertTrue(found[0].startswith("B"))

    def test_reported_contention_with_no_holder_in_the_wait_is_a_problem(self):
        spans = [Span("A", enqueue=1.0, acquire=1.001, release=1.2, effects=(1.1,), contention=3)]
        self.assertEqual(len(oracle.queueing_problems(spans, [])), 1)

    def test_an_unwaited_uncontended_acquire_is_fine(self):
        spans = [Span("A", enqueue=1.0, acquire=1.001, release=1.2, effects=(1.1,), contention=0)]
        self.assertEqual(oracle.queueing_problems(spans, []), [])

    def test_a_missing_envelope_report_is_a_problem(self):
        spans = serial()
        spans[3] = Span("D", enqueue=0.13, acquire=1.61, release=1.80, effects=(1.70,), contention=None)
        found = oracle.queueing_problems(spans, oracle.holds_of(spans, GATE))
        self.assertEqual(len(found), 1)
        self.assertTrue(found[0].startswith("D"))

    def test_holds_merge_agent_leases_and_gate_holds(self):
        found = oracle.holds_of(serial(), GATE)
        self.assertEqual([hold.holder for hold in found], ["gate", "A", "B", "C", "D"])
        self.assertEqual(found[1], Hold("A", 1.00, 1.20))


class OrderAndWaitTests(unittest.TestCase):
    def test_the_acquire_order_must_match(self):
        self.assertEqual(oracle.order_problems(serial(), "ABCD"), [])
        self.assertEqual(len(oracle.order_problems(serial(), "BACD")), 1)
        self.assertEqual(oracle.acquire_order(serial()), "ABCD")

    def test_waits_are_acquire_minus_enqueue_in_milliseconds(self):
        self.assertAlmostEqual(oracle.wait_ms(serial()[0]), 900.0)
        self.assertIsNone(oracle.wait_ms(Span("A", enqueue=None, acquire=1.0, release=1.2)))

    def test_percentiles_use_the_nearest_rank(self):
        values = [float(value) for value in range(1, 21)]
        self.assertEqual(oracle.percentile(values, 0.5), 10.0)
        self.assertEqual(oracle.percentile(values, 0.95), 19.0)
        self.assertEqual(oracle.percentile([7.0], 0.95), 7.0)
        self.assertIsNone(oracle.percentile([], 0.5))


class NegativeControlTests(unittest.TestCase):
    def test_an_injected_overlap_of_two_dispatches_fails_the_oracle(self):
        injected = oracle.with_injected_overlap(serial())
        self.assertNotEqual(injected, serial())
        problems = oracle.timeline_problems(injected, GATE, "ABCD")
        self.assertTrue(any("overlap" in problem for problem in problems), problems)

    def test_an_unreported_wait_fails_the_oracle(self):
        hidden = oracle.with_unreported_wait(serial())
        problems = oracle.timeline_problems(hidden, GATE, "ABCD")
        self.assertTrue(any("contention" in problem for problem in problems), problems)

    def test_an_effect_inside_another_agents_hold_fails_the_oracle(self):
        moved = oracle.with_effect_in_foreign_hold(serial())
        self.assertEqual(moved[1].effects, (1.10,))
        problems = oracle.timeline_problems(moved, GATE, "ABCD")
        self.assertTrue(any(problem.startswith("B: an effect at 1.1000") for problem in problems), problems)

    def test_a_blind_effect_check_fails_the_controls(self):
        original = oracle.early_effect_problems
        try:
            oracle.early_effect_problems = lambda spans: []
            found = oracle.control_problems(serial(), GATE, "ABCD")
        finally:
            oracle.early_effect_problems = original
        self.assertEqual(found, ["an effect inside another agent's hold passed the oracle"])

    def test_the_controls_pass_only_when_the_oracle_catches_both(self):
        self.assertEqual(oracle.control_problems(serial(), GATE, "ABCD"), [])

    def test_a_blind_oracle_fails_the_controls(self):
        original = oracle.overlap_problems
        try:
            oracle.overlap_problems = lambda spans: []
            found = oracle.control_problems(serial(), GATE, "ABCD")
        finally:
            oracle.overlap_problems = original
        self.assertEqual(len(found), 1)
        self.assertIn("overlap", found[0])

    def test_a_timeline_of_one_span_cannot_inject_an_overlap(self):
        with self.assertRaises(ValueError):
            oracle.with_injected_overlap(serial()[:1])


class LockSampleTests(unittest.TestCase):
    LOCKS = ("1: FLOCK  ADVISORY  WRITE 4242 fd:01:777 0 EOF\n"
             "2: FLOCK  ADVISORY  WRITE 99 fd:01:778 0 EOF\n"
             "3: -> FLOCK  ADVISORY  WRITE 5151 fd:01:777 0 EOF\n"
             "4: POSIX  ADVISORY  READ 31 00:1a:777 0 EOF\n")

    def test_holders_come_from_flock_lines_on_the_lock_inode(self):
        self.assertEqual(oracle.lock_holders(self.LOCKS, 777), {4242})
        self.assertEqual(oracle.lock_holders(self.LOCKS, 1), set())

    def test_lease_events_come_from_samples(self):
        samples = [
            (0.0, set(), set()),
            (0.1, set(), {10, 11}),
            (0.2, {10}, {10, 11}),
            (0.3, set(), {11}),
            (0.4, {11}, {11}),
            (0.5, set(), set()),
        ]
        found = oracle.lease_events(samples)
        self.assertEqual(found[10], {"enqueue": 0.1, "acquire": 0.2, "release": 0.3})
        self.assertEqual(found[11], {"enqueue": 0.1, "acquire": 0.4, "release": 0.5})

    def test_a_second_hold_by_one_pid_is_kept_as_a_problem_sign(self):
        samples = [(0.0, {10}, {10}), (0.1, set(), {10}), (0.2, {10}, {10}), (0.3, set(), set())]
        self.assertEqual(oracle.lease_events(samples)[10]["holds"], 2)


    def test_hold_intervals_name_every_holder_and_keep_open_holds(self):
        samples = [(0.0, {1}, set()), (0.5, set(), {10}), (0.6, {10}, {10}), (0.9, {7}, set())]
        found = oracle.hold_intervals(samples, {1: "runner", 10: "A"})
        self.assertEqual(found[:2], [Hold("runner", 0.0, 0.5), Hold("A", 0.6, 0.9)])
        self.assertEqual(found[2].holder, "pid 7")
        self.assertEqual(found[2].end, float("inf"))


class RefusalTests(unittest.TestCase):
    def test_a_lock_timeout_is_timeout_exit_one_with_the_kind(self):
        error = {"code": "TIMEOUT", "details": {"kind": "lock_timeout", "path": "interaction.lock",
                                                 "contention_count": 90},
                 "disposition": {"delivery": "not_delivered", "retry": "safe"}}
        self.assertEqual(oracle.lock_timeout_problems(envelope(False, error, 1), "lock_timeout"), [])
        self.assertEqual(len(oracle.lock_timeout_problems(envelope(True, None, 0), "lock_timeout")), 1)
        other = {**error, "details": {**error["details"], "kind": "interaction_process_lock_timeout"}}
        self.assertEqual(len(oracle.lock_timeout_problems(envelope(False, other, 1), "lock_timeout")), 1)
        delivered = {**error, "disposition": {"delivery": "delivered_unverified", "retry": "unsafe"}}
        self.assertEqual(len(oracle.lock_timeout_problems(envelope(False, delivered, 1), "lock_timeout")), 1)
        uncontended = {**error, "details": {**error["details"], "contention_count": 0}}
        self.assertEqual(len(oracle.lock_timeout_problems(envelope(False, uncontended, 1), "lock_timeout")), 1)

    def test_a_partial_delivery_never_reports_a_safe_retry(self):
        self.assertEqual(oracle.partial_retry_problems(None), [])
        safe = {"code": "ACTION_FAILED", "disposition": {"delivery": "delivery_uncertain", "retry": "safe"}}
        self.assertEqual(len(oracle.partial_retry_problems(envelope(False, safe, 1))), 1)
        unsafe = {"code": "ACTION_FAILED", "disposition": {"delivery": "delivery_uncertain", "retry": "unsafe"}}
        self.assertEqual(oracle.partial_retry_problems(envelope(False, unsafe, 1)), [])
        self.assertEqual(len(oracle.partial_retry_problems(envelope(True, None, 0))), 1)

    def test_reported_contention_reads_auto_wait(self):
        data = {"details": {"auto_wait": {"lease_contention_count": 7, "lease_hold_ms": 30}}}
        self.assertEqual(oracle.reported_contention(envelope(True, None, 0, data)), 7)
        self.assertIsNone(oracle.reported_contention(envelope(True, None, 0, {})))
        self.assertIsNone(oracle.reported_contention(None))


class ChildEnvironmentTests(unittest.TestCase):
    def test_children_inherit_no_lease_and_get_their_agent_id(self):
        source = {"PATH": "/usr/bin", "AGENT_DESKTOP_INTERACTION_LEASE_FD": "9",
                  "AGENT_DESKTOP_INTERACTION_LEASE": "token", "AGENT_DESKTOP_AGENT_ID": "harness"}
        found = oracle.child_env(source, "agent-b")
        self.assertEqual(found["PATH"], "/usr/bin")
        self.assertNotIn("AGENT_DESKTOP_INTERACTION_LEASE_FD", found)
        self.assertNotIn("AGENT_DESKTOP_INTERACTION_LEASE", found)
        self.assertEqual(found["AGENT_DESKTOP_AGENT_ID"], "agent-b")



class FfiChildTests(unittest.TestCase):
    def test_a_live_c_abi_child_is_not_read_to_end_before_it_gets_its_line(self):
        import subprocess
        import threading
        from types import SimpleNamespace
        from unittest.mock import patch

        from scenarios import hyprland_parallel_faults as faults

        started, popen = [], subprocess.Popen

        def blocked_on_stdin(*_args, **kwargs):
            child = popen([sys.executable, "-I", "-c", "import sys; sys.stdin.readline()"], **kwargs)
            started.append(child)
            return child

        sampler = SimpleNamespace(watch=lambda *_args: None, state=lambda: (frozenset(), frozenset(
            child.pid for child in started)))
        outcome = []

        def run() -> None:
            try:
                faults.same_process_problems(SimpleNamespace(env={}), None, sampler, {}, "@s1:e1", {})
            except AssertionError as error:
                outcome.append(str(error))

        with patch.object(faults, "ffi_library", return_value=Path("/private/lib.so")), \
                patch.object(faults.agents, "agent_env", return_value={}), \
                patch.object(faults.subprocess, "Popen", side_effect=blocked_on_stdin):
            worker = threading.Thread(target=run, daemon=True)
            worker.start()
            worker.join(10.0)
            hung = worker.is_alive()
            for child in started:
                child.kill()
            worker.join(10.0)
        self.assertFalse(hung, "the runner blocked reading a live child's stderr")
        self.assertEqual(len(outcome), 1)
        self.assertIn("closed its output early", outcome[0])


class FailureReportTests(unittest.TestCase):
    def test_a_refusal_names_its_occluder_past_the_passing_checks(self):
        import json
        from types import SimpleNamespace

        from scenarios import parallel_agents as agents

        checks = [{"check": "visible", "status": "pass"},
                  {"check": "stable", "status": "pass", "stability": {"samples": [[1, 2, 3, 4]] * 40}},
                  {"check": "receives_events", "status": "fail", "reason": "occluded by layer",
                   "occluder": {"role": "layer", "name": "waybar", "bounds": {"x": 0, "y": 0, "width": 1280}}}]
        error = {"code": "ACTION_FAILED", "message": "Target is not actionable: receives_events (occluded by layer)",
                 "suggestion": "x" * 200, "details": {"actionable": False, "checks": checks},
                 "disposition": {"delivery": "not_delivered", "retry": "safe"}}
        raw = json.dumps({"version": "2.5", "ok": False, "command": "click", "error": error})
        refusal = Envelope(ok=False, data=None, error=error, raw=raw, elapsed_ms=5.0, argv=[], exit_code=1)
        problems = agents.outcome_problems(SimpleNamespace(name="D", envelope=refusal, exit_code=1))
        self.assertEqual(len(problems), 1)
        self.assertIn('"name": "waybar"', problems[0])
        self.assertIn("not_delivered", problems[0])
        self.assertNotIn("samples", problems[0])

    def test_the_layer_stack_lists_each_surface_by_level(self):
        from unittest.mock import patch

        from scenarios import parallel_zoo as zoo

        layers = {"Virtual-1": {"levels": {"3": [{"namespace": "agent-desktop-cursor", "pid": 41, "x": 0, "y": 0,
                                                  "w": 1280, "h": 800}],
                                           "2": [{"namespace": "waybar", "pid": 9, "x": 0, "y": 0, "w": 1280,
                                                  "h": 26}]}}}
        with patch.object(zoo, "query", return_value=layers):
            found = zoo.layer_stack(None)
        self.assertEqual(found, [("Virtual-1", "2", "waybar", 9, (0, 0, 1280, 26)),
                                 ("Virtual-1", "3", "agent-desktop-cursor", 41, (0, 0, 1280, 800))])


class RendererRecordTests(unittest.TestCase):
    def setup_of(self, records):
        from types import SimpleNamespace
        from unittest.mock import patch

        from scenarios import hyprland_parallel_orders as orders

        setup = orders.Setup(SimpleNamespace(tier=None), {}, "run-1", {}, {}, {}, {}, {}, 0)
        setup.positives.append((1.0, 1.0))
        reads = iter(records)
        patcher = patch.multiple(orders, record_of=lambda *_args: next(reads), shared=lambda _tier: None)
        patcher.start()
        self.addCleanup(patcher.stop)
        return setup

    def test_a_setup_has_no_renderer_until_a_headed_agent_starts_one(self):
        setup = self.setup_of([{"pid": 7}, {"pid": 7}])
        self.assertEqual(setup.note_renderer(), [])
        self.assertEqual(setup.final_problems(), [])

    def test_a_replaced_renderer_fails(self):
        setup = self.setup_of([{"pid": 7}, {"pid": 8}])
        setup.note_renderer()
        self.assertIn("the session renderer changed", setup.final_problems()[0])

    def test_no_renderer_after_the_headed_agents_fails(self):
        setup = self.setup_of([None, None])
        setup.note_renderer()
        self.assertIn("no headed agent started", setup.final_problems()[0])


class LeftKeyboardTests(unittest.TestCase):
    PHYSICAL = ("at-translated-set-2-keyboard", True, "us", "English (US)", 0)
    POWER = ("power-button", False, "us", "English (US)", 0)
    IME = ("hl-virtual-keyboard-fcitx5", True, "us", "English (US)", 0)

    def problems(self, before, after):
        from scenarios import parallel_zoo as zoo

        return zoo.left_keyboard_problems(before, after)

    def test_the_input_method_keyboard_arriving_and_taking_main_is_not_left_input(self):
        before = [self.PHYSICAL, self.POWER]
        after = [(*self.PHYSICAL[:1], False, *self.PHYSICAL[2:]), self.IME, self.POWER]
        self.assertEqual(self.problems(before, after), [])

    def test_a_virtual_keyboard_left_by_another_client_fails(self):
        left = ("hl-virtual-keyboard-agent-desktop", False, "us", "English (US)", 0)
        problems = self.problems([self.PHYSICAL, self.POWER], [self.PHYSICAL, left, self.POWER])
        self.assertEqual(len(problems), 1)
        self.assertIn("hl-virtual-keyboard-agent-desktop", problems[0])

    def test_a_changed_layout_group_fails(self):
        switched = (*self.PHYSICAL[:4], 1)
        problems = self.problems([self.PHYSICAL, self.POWER], [switched, self.POWER])
        self.assertEqual(len(problems), 1)
        self.assertIn("at-translated-set-2-keyboard", problems[0])

    def test_a_lost_keyboard_fails(self):
        self.assertEqual(len(self.problems([self.PHYSICAL, self.POWER], [self.PHYSICAL])), 1)


class HeadedClickTests(unittest.TestCase):
    REFUSAL = {"code": "ACTION_FAILED",
               "details": {"kind": "receiver_evidence", "check": "hyprland_stack", "cause": "receiver_unproven",
                           "physical_delivery_started": False},
               "disposition": {"delivery": "not_delivered", "retry": "safe"}}

    def test_d_clicks_the_centre_of_its_target_by_coordinates(self):
        from scenarios import parallel_agents as agents

        self.assertEqual(agents.xy_click_argv((482.5, 53.4)), ["mouse-click", "--xy", "482,53"])
        self.assertEqual(agents.xy_click_argv((640.6, 300.5)), ["mouse-click", "--xy", "641,300"])

    def test_the_orders_give_d_a_coordinate_click_and_a_a_ref_click(self):
        from types import SimpleNamespace

        from scenarios import hyprland_parallel_orders as orders

        setup = SimpleNamespace(ctx=SimpleNamespace(tier=None), refs={"A": "@s1:e1", "D": "@s1:e2"},
                                targets={"A": (10.0, 20.0), "D": (482.5, 53.4)})
        schedule = orders.Schedule(setup, None, None, "order 1")
        self.assertEqual(schedule.argv("D"), ["mouse-click", "--xy", "482,53"])
        self.assertEqual(schedule.argv("A")[:2], ["click", "@s1:e1"])

    def test_d_coordinate_click_envelope_reports_its_lease_contention(self):
        data = {"action": "mouse_click", "count": 1, "x": 482, "y": 53,
                "details": {"delivered_events": 3, "receiver_proof": False,
                            "auto_wait": {"lease_contention_count": 2, "lease_hold_ms": 41}},
                "disposition": {"delivery": "delivered_unverified", "retry": "unsafe"},
                "target": {"kind": "pointer", "point": {"x": 482, "y": 53}}}
        contention = oracle.reported_contention(envelope(True, None, 0, data))
        self.assertEqual(contention, 2)
        spans = serial()
        spans[3] = dataclasses.replace(spans[3], contention=contention)
        self.assertEqual(oracle.queueing_problems(spans, oracle.holds_of(spans, GATE)), [])
        unreported = {**data, "details": {"delivered_events": 3, "receiver_proof": False}}
        spans[3] = dataclasses.replace(spans[3], contention=oracle.reported_contention(
            envelope(True, None, 0, unreported)))
        found = oracle.queueing_problems(spans, oracle.holds_of(spans, GATE))
        self.assertEqual(found, ["D: the envelope reports no lease contention count"])

    def test_the_cancel_stages_give_d_a_coordinate_click(self):
        from scenarios import hyprland_parallel_cancel as cancel

        refs, targets = {"A": "@s1:e1", "B": "@s1:e3", "D": "@s1:e2"}, {"D": (482.5, 53.4)}
        self.assertEqual(cancel.argv_of("D", refs, targets), ["mouse-click", "--xy", "482,53"])
        self.assertEqual(cancel.argv_of("A", refs, targets)[:2], ["click", "@s1:e1"])

    def test_the_lock_timeout_contenders_wait_on_the_lease_before_any_receiver_check(self):
        from scenarios import hyprland_parallel_faults as faults

        refs, targets = {"B": "@s1:e3", "D": "@s1:e2"}, {"D": (482.5, 53.4)}
        self.assertEqual(faults.contender_argv("D", refs, targets), ["mouse-click", "--xy", "482,53"])
        self.assertEqual(faults.contender_argv("B", refs, targets),
                         ["type", "@s1:e3", "q", "--timeout-ms", faults.CONTEND_MS])

    def test_a_headed_ref_click_must_refuse_as_unproven_before_any_event(self):
        self.assertEqual(oracle.ref_refusal_problems(envelope(False, self.REFUSAL, 1)), [])
        self.assertEqual(len(oracle.ref_refusal_problems(envelope(True, None, 0))), 1)
        self.assertEqual(len(oracle.ref_refusal_problems(None)), 1)
        for key, value in (("cause", "layer_cover"), ("physical_delivery_started", True),
                           ("kind", "lock_timeout")):
            changed = {**self.REFUSAL, "details": {**self.REFUSAL["details"], key: value}}
            self.assertEqual(len(oracle.ref_refusal_problems(envelope(False, changed, 1))), 1, key)
        delivered = {**self.REFUSAL, "disposition": {"delivery": "delivered_unverified", "retry": "unsafe"}}
        self.assertEqual(len(oracle.ref_refusal_problems(envelope(False, delivered, 1))), 1)
        timeout = {**self.REFUSAL, "code": "TIMEOUT"}
        self.assertEqual(len(oracle.ref_refusal_problems(envelope(False, timeout, 1))), 1)
        self.assertEqual(len(oracle.ref_refusal_problems(envelope(False, self.REFUSAL, 2))), 1)


class QueuedDeliveryTests(unittest.TestCase):
    """The lock timeout leg's queued deliveries meet the serialized leg's timing and contention checks."""

    CONTENDED = {"details": {"auto_wait": {"lease_contention_count": 4, "lease_hold_ms": 30}}}

    def sampler(self, pid_of, base: float):
        from types import SimpleNamespace

        return SimpleNamespace(
            events=lambda: {pid_of(): {"enqueue": base + 1.0, "acquire": base + 50.0, "release": base + 50.2}},
            holds=lambda _names: [Hold("runner", base, base + 50.0)],
            watch=lambda *_args: None, state=lambda: (frozenset(), frozenset({pid_of()})))

    def waiter_problems(self, lag_s: float, data: dict | None) -> list[str]:
        import time
        from types import SimpleNamespace
        from unittest.mock import patch

        from scenarios import hyprland_parallel_faults as faults

        base = time.monotonic()
        waiter = SimpleNamespace(name="A", pid=4242, exit_code=0, envelope=envelope(True, None, 0, data),
                                 kill=lambda: None, finish=lambda: waiter.envelope)
        with patch.object(faults.agents, "spawn", return_value=waiter), \
                patch.object(faults.agents, "queued"), patch.object(faults.agents, "turn"), \
                patch.object(faults, "inherited_problems", return_value=[]), \
                patch.object(faults, "_contender_problems", return_value=[]), \
                patch.object(faults, "effects", return_value=(base + 50.2 + lag_s,)):
            return faults.lease_phase_problems(None, None, self.sampler(lambda: 4242, base), {}, {"A": "@s1:e1"},
                                               {}, "run-1", {})

    def test_a_timely_contended_waiter_click_passes(self):
        self.assertEqual(self.waiter_problems(0.1, self.CONTENDED), [])

    def test_a_waiter_click_past_the_lag_bound_fails(self):
        found = self.waiter_problems(2.0, self.CONTENDED)
        self.assertTrue(any("past the 1000 ms bound" in item for item in found), found)

    def test_a_waiter_envelope_with_no_contention_count_fails(self):
        found = self.waiter_problems(0.1, {"details": {}})
        self.assertTrue(any("no lease contention count" in item for item in found), found)

    def holder_problems(self, lag_s: float, data: dict) -> list[str]:
        import json
        import subprocess
        import time
        from types import SimpleNamespace
        from unittest.mock import patch

        from scenarios import hyprland_parallel_faults as faults

        timeout = {"version": "2.5", "ok": False, "command": "execute_by_ref",
                   "error": {"code": "TIMEOUT", "details": {"kind": "interaction_process_lock_timeout",
                                                           "contention_count": 3},
                             "disposition": {"delivery": "not_delivered", "retry": "safe"}}}
        delivered = {"version": "2.5", "ok": True, "command": "execute_by_ref", "data": data}
        lines = [json.dumps({"thread": "contender", "code": 1, "envelope": json.dumps(timeout)}),
                 json.dumps({"thread": "holder", "code": 0, "envelope": json.dumps(delivered)})]
        script = f"import sys; sys.stdin.readline(); print({lines[0]!r}, flush=True); print({lines[1]!r})"
        started, popen, base = [], subprocess.Popen, time.monotonic()

        def child(*_args, **kwargs):
            started.append(popen([sys.executable, "-I", "-c", script], **kwargs))
            return started[-1]

        given = []
        gate = SimpleNamespace(give=lambda: given.append(True), take=lambda: None)
        landed = lambda *_args: (base + 50.2 + lag_s,) if given else ()
        with patch.object(faults, "ffi_library", return_value=Path("/private/lib.so")), \
                patch.object(faults.agents, "agent_env", return_value={}), \
                patch.object(faults.subprocess, "Popen", side_effect=child), \
                patch.object(faults, "effects", side_effect=landed):
            return faults.same_process_problems(SimpleNamespace(env={}), gate,
                                                self.sampler(lambda: started[0].pid, base), {}, "@s1:e1", {})

    def test_a_timely_contended_c_abi_holder_click_passes(self):
        self.assertEqual(self.holder_problems(0.1, self.CONTENDED), [])

    def test_a_c_abi_holder_click_past_the_lag_bound_fails(self):
        found = self.holder_problems(2.0, self.CONTENDED)
        self.assertTrue(any("past the 1000 ms bound" in item for item in found), found)

    def test_a_c_abi_holder_envelope_with_no_contention_count_fails(self):
        found = self.holder_problems(0.1, {"details": {}})
        self.assertTrue(any("no lease contention count" in item for item in found), found)


class PauseBarrierTests(unittest.TestCase):
    """Pass 3 stops D only once its hide has finished, so the 500 ms hide deadline never spans the pause."""

    BEFORE = ("0x1", "at-translated-set-2-mouse")
    OPENED = ("0x2", "unknown-device")

    class Stopped(Exception):
        pass

    def pause(self, reads: list, exited: bool = False) -> list:
        import signal
        from types import SimpleNamespace
        from unittest.mock import patch

        from scenarios import hyprland_parallel_orders as orders

        log = []

        def devices(_tier):
            log.append(reads.pop(0) if len(reads) > 1 else reads[0])
            return log[-1]

        def send(number):
            log.append(number)
            if number == signal.SIGSTOP:
                raise self.Stopped()

        refusal = envelope(False, {"code": "TIMEOUT", "details": {"kind": "cursor_hide_timeout"}}, 1)
        d = SimpleNamespace(name="D", pid=7, signal=send, envelope=refusal, exit_code=1, finish=lambda: refusal,
                            process=SimpleNamespace(poll=lambda: 1 if exited else None))
        setup = SimpleNamespace(ctx=SimpleNamespace(tier=None), session="run-1")
        schedule = orders.Schedule(setup, None, None, "pass 3")
        schedule.agents["D"] = d
        schedule.pointers = {self.BEFORE}
        with patch.multiple(orders, registry_of=lambda *_args: None, holders_in=lambda _registry: [{"pid": 7}],
                            shared=lambda _tier: None), \
                patch.object(orders, "pointer_devices", side_effect=devices, create=True):
            try:
                schedule.pause_d()
            except self.Stopped:
                pass
        return log

    def test_d_stops_only_once_its_pointer_shows_the_hide_finished(self):
        import signal

        log = self.pause([{self.BEFORE}, {self.BEFORE}, {self.BEFORE, self.OPENED}])
        self.assertEqual(log[-1], signal.SIGSTOP)
        self.assertGreaterEqual(len(log), 4, log)
        self.assertEqual(log[-2], {self.BEFORE, self.OPENED})

    def test_d_exiting_before_its_pointer_opens_reports_its_envelope(self):
        with self.assertRaises(AssertionError) as raised:
            self.pause([{self.BEFORE}], exited=True)
        self.assertIn("cursor_hide_timeout", str(raised.exception))


class KeyboardVerdictTests(unittest.TestCase):
    """The final keyboard verdict applies the same allowed changes as the cancelled-agent check."""

    PHYSICAL = LeftKeyboardTests.PHYSICAL
    POWER = LeftKeyboardTests.POWER
    IME = LeftKeyboardTests.IME

    def verdict(self, reads: list[list[tuple]]) -> list[str]:
        from unittest.mock import patch

        from runner import verdict
        from scenarios import parallel_zoo as zoo

        ctx = verdict.LegContext(None, "headed", member_reader=lambda _tier: frozenset())
        with patch.object(zoo, "keyboards", side_effect=reads), patch.object(zoo, "lease_state",
                                                                               return_value="released"):
            for stage in ("before", "after", "restored"):
                zoo.common_reads(ctx, stage)
        return [item for item in verdict.evaluate(ctx) if "keyboards" in item]

    def test_the_input_method_keyboard_arriving_and_taking_main_passes(self):
        moved = [(*self.PHYSICAL[:1], False, *self.PHYSICAL[2:]), self.IME, self.POWER]
        self.assertEqual(self.verdict([[self.PHYSICAL, self.POWER], moved, moved]), [])

    def test_a_lost_keyboard_still_fails(self):
        self.assertEqual(len(self.verdict([[self.PHYSICAL, self.POWER], [self.PHYSICAL], [self.PHYSICAL]])), 1)

    def test_a_changed_layout_still_fails(self):
        switched = [(*self.PHYSICAL[:4], 1), self.POWER]
        self.assertEqual(len(self.verdict([[self.PHYSICAL, self.POWER], switched, switched])), 1)

    def test_a_new_keyboard_no_input_method_owns_still_fails(self):
        left = ("hl-virtual-keyboard-agent-desktop", False, "us", "English (US)", 0)
        after = [self.PHYSICAL, left, self.POWER]
        self.assertEqual(len(self.verdict([[self.PHYSICAL, self.POWER], after, after])), 1)


class CancelStageOracleTests(unittest.TestCase):
    """The cancel leg's next agents report their wait, and its cancelled agents' effects stay in their holds."""

    CONTENDED = {"details": {"auto_wait": {"lease_contention_count": 4, "lease_hold_ms": 30}}}

    def setUp(self):
        import time

        self.base = time.monotonic()

    def agent(self, name: str, pid: int, data: dict | None = None, cancelled: bool = False):
        import signal
        from types import SimpleNamespace

        reply = None if cancelled else envelope(True, None, 0, data)
        return SimpleNamespace(name=name, pid=pid, exit_code=-signal.SIGTERM if cancelled else 0, envelope=reply,
                               finish=lambda: reply, kill=lambda: None, signal=lambda _number: None,
                               process=SimpleNamespace(poll=lambda: None))

    def sampler(self, pids: list[int]):
        from types import SimpleNamespace

        base = self.base
        events = {pid: {"enqueue": base + 1.0, "acquire": base + 50.0 + index, "release": base + 50.2 + index}
                  for index, pid in enumerate(pids)}
        return SimpleNamespace(events=lambda: events, holds=lambda _names: [Hold("runner", base, base + 50.0)])

    def next_problems(self, data: dict | None) -> list[str]:
        from unittest.mock import patch

        from scenarios import hyprland_parallel_cancel as cancel

        after = self.agent("D", 8, data)
        with patch.object(cancel.agents, "turn"), patch.object(cancel, "effects", return_value=(self.base + 50.1,)):
            return cancel._next_proceeds(None, self.sampler([8]), {}, after, self.base, {})

    def test_a_contended_next_agent_passes(self):
        self.assertEqual(self.next_problems(self.CONTENDED), [])

    def test_a_next_agent_with_no_contention_count_fails(self):
        found = self.next_problems({"details": {}})
        self.assertTrue(any("no lease contention count" in item for item in found), found)

    def test_a_next_agent_that_reports_no_wait_behind_the_gate_fails(self):
        found = self.next_problems({"details": {"auto_wait": {"lease_contention_count": 0}}})
        self.assertTrue(any("reports lease contention 0" in item for item in found), found)

    def acquired_problems(self, landed: tuple[float, ...]) -> list[str]:
        from unittest.mock import patch

        from scenarios import hyprland_parallel_cancel as cancel

        holder, after = self.agent("D", 7, cancelled=True), self.agent("A", 8, self.CONTENDED)
        allowed = {}
        with patch.object(cancel.agents, "spawn", side_effect=[holder, after]), \
                patch.object(cancel.agents, "queued"), \
                patch.object(cancel.agents, "turn", side_effect=lambda *args, during=None: during()), \
                patch.object(cancel, "_next_proceeds", return_value=[]), \
                patch.object(cancel, "effects", return_value=landed):
            found = cancel.acquired_stage(None, None, self.sampler([7, 8]), {}, {"A": "@s1:e1"}, {"D": (1.0, 1.0)},
                                          "run-1", allowed)
        self.assertEqual(allowed.get("D"), list(landed))
        return found

    def test_a_cancelled_holder_click_inside_its_hold_passes(self):
        self.assertEqual(self.acquired_problems((self.base + 50.1,)), [])

    def test_a_cancelled_holder_click_past_the_lag_bound_fails(self):
        found = self.acquired_problems((self.base + 52.2,))
        self.assertTrue(any("past the 1000 ms bound" in item for item in found), found)

    def test_a_cancelled_holder_click_before_its_hold_fails(self):
        found = self.acquired_problems((self.base + 20.0,))
        self.assertTrue(any("before its acquire" in item for item in found), found)

    def partial_problems(self, landed: tuple[float, ...]) -> list[str]:
        from types import SimpleNamespace
        from unittest.mock import patch

        from scenarios import hyprland_parallel_cancel as cancel

        typist, after = self.agent("B", 7, cancelled=True), self.agent("A", 8, self.CONTENDED)
        ctx = SimpleNamespace(tier=None, finding=lambda _text: None)
        allowed = {}
        with patch.object(cancel.agents, "spawn", side_effect=[typist, after]), \
                patch.object(cancel.agents, "queued"), patch.object(cancel.agents, "turn"), \
                patch.object(cancel, "keyboards", return_value=[]), \
                patch.object(cancel.zoo, "focus_entry"), patch.object(cancel.zoo, "entry_text", return_value="abc"), \
                patch.object(cancel, "_next_proceeds", return_value=[]), \
                patch.object(cancel, "effects", return_value=landed):
            refs = {"A": "@s1:e1", "B": "@s1:e3"}
            found = cancel.partial_stage(ctx, None, self.sampler([7, 8]), {"B": "entry"}, refs, "run-1", allowed)
        self.assertEqual(allowed.get("B"), list(landed))
        return found

    def test_a_cancelled_typists_changes_inside_its_hold_pass(self):
        self.assertEqual(self.partial_problems((self.base + 50.05, self.base + 50.1)), [])

    def test_a_cancelled_typists_change_past_the_lag_bound_fails(self):
        found = self.partial_problems((self.base + 50.1, self.base + 52.2))
        self.assertTrue(any("past the 1000 ms bound" in item for item in found), found)

    def test_a_cancelled_typists_change_before_its_hold_fails(self):
        found = self.partial_problems((self.base + 20.0, self.base + 50.1))
        self.assertTrue(any("before its acquire" in item for item in found), found)


if __name__ == "__main__":
    unittest.main()
