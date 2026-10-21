from __future__ import annotations

import importlib
import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))
REPO = ROOT.parent.parent

from runner import gate, legs, main, report
from runner.legs import REGISTRY, Leg


def contract_violations(registry, implementations, phases_text):
    problems = []
    ids = {leg.id for leg in registry}
    for leg_id in sorted(ids - set(implementations)):
        problems.append(f"no implementation registered for {leg_id}")
    for leg_id in sorted(set(implementations) - ids):
        problems.append(f"implementation registered for unregistered leg {leg_id}")
    for leg in registry:
        if leg.disposition == "owner-decision" and leg.id not in phases_text:
            problems.append(
                f"owner-decision leg {leg.id} is not named in the product contract"
            )
    return problems


def make_binary(root: Path, body: str) -> Path:
    path = Path(root) / "fake-agent-desktop"
    path.write_text(f"#!/bin/sh\n{body}\n")
    path.chmod(0o700)
    helper = root / "agent-desktop-linux-clipboard-owner"
    helper.write_text("#!/bin/sh\necho clipboard-owner\n")
    helper.chmod(0o700)
    return path


class RegistryContractTests(unittest.TestCase):
    def test_every_registry_leg_has_an_implementation(self):
        importlib.import_module("scenarios")
        problems = contract_violations(
            REGISTRY, legs.implementations(), (REPO / "docs" / "phases.md").read_text()
        )
        self.assertEqual(problems, [])

    def test_unimplemented_record_is_a_violation(self):
        problems = contract_violations(REGISTRY[:1], {}, "")
        self.assertEqual(len(problems), 1)
        self.assertIn("no implementation", problems[0])

    def test_unregistered_implementation_is_a_violation(self):
        problems = contract_violations(
            REGISTRY[:1], {REGISTRY[0].id: lambda ctx: None, "phantom.id": lambda ctx: None}, ""
        )
        self.assertTrue(any("unregistered leg phantom.id" in p for p in problems))

    def test_record_with_no_requirement_fails_construction(self):
        with self.assertRaises(ValueError):
            Leg(
                id="x.y",
                requirement="  ",
                target="t",
                tiers=("x11-openbox",),
                modes=("headless",),
                oracles=("atspi",),
                disposition="pass",
            )

    def test_owner_decision_must_be_named_in_the_product_contract(self):
        leg = Leg(
            id="owner.case",
            requirement="r",
            target="t",
            tiers=("x11-openbox",),
            modes=("headless",),
            oracles=("atspi",),
            disposition="owner-decision",
            reason="the owner chose this",
        )
        impls = {"owner.case": lambda ctx: None}
        self.assertEqual(contract_violations((), impls, ""), [
            "implementation registered for unregistered leg owner.case"
        ])
        problems = contract_violations((leg,), impls, "")
        self.assertEqual(len(problems), 1)
        self.assertIn("owner.case", problems[0])
        phases_text = f"the ledger records {leg.id} as an owner decision"
        self.assertEqual(contract_violations((leg,), impls, phases_text), [])


class GateTests(unittest.TestCase):
    def test_gate_fails_off_linux(self):
        missing = gate.check_prerequisites(platform="darwin")
        self.assertEqual(missing, ["a linux host is required for the live harness"])

    def test_missing_tool_is_named(self):
        with tempfile.TemporaryDirectory() as root:
            env = {
                "AGENT_DESKTOP_SESSION_BASE": str(Path(root) / "sessions"),
                "AGENT_DESKTOP_E2E_BIN": str(make_binary(Path(root), "exit 0")),
                "AGENT_DESKTOP_WM_ROOT": str(Path(root) / "wm"),
            }
            (Path(env["AGENT_DESKTOP_WM_ROOT"]) / "usr" / "bin").mkdir(parents=True)
            (Path(env["AGENT_DESKTOP_WM_ROOT"]) / "usr" / "bin" / "openbox").write_text("x")
            missing = gate.check_prerequisites(
                ("x11-openbox",),
                which=lambda tool: None,
                env=env,
                extra_paths=(),
                gi_probe=lambda: None,
                platform="linux",
            )
        self.assertTrue(any("missing tool: Xvfb" in m for m in missing))
        self.assertTrue(any("xdotool" in m for m in missing))

    def test_all_tools_found_passes(self):
        with tempfile.TemporaryDirectory() as root:
            wm = Path(root) / "wm" / "usr" / "bin"
            wm.mkdir(parents=True)
            (wm / "openbox").write_text("x")
            env = {
                "AGENT_DESKTOP_SESSION_BASE": str(Path(root) / "sessions"),
                "AGENT_DESKTOP_E2E_BIN": str(make_binary(Path(root), "exit 0")),
                "AGENT_DESKTOP_WM_ROOT": str(Path(root) / "wm"),
            }
            for key in ("AGENT_DESKTOP_E2E_BIN", "AGENT_DESKTOP_E2E_BIN_SRC"):
                with self.subTest(key=key):
                    selected = {name: value for name, value in env.items()
                                if name != "AGENT_DESKTOP_E2E_BIN"}
                    selected[key] = env["AGENT_DESKTOP_E2E_BIN"]
                    missing = gate.check_prerequisites(
                        ("x11-openbox",),
                        which=lambda tool: f"/usr/bin/{tool}",
                        env=selected,
                        extra_paths=(),
                        gi_probe=lambda: None,
                        platform="linux",
                    )
                    self.assertEqual(missing, [])

    def test_gate_failure_maps_to_exit_two(self):
        self.assertEqual(report.exit_code(0, True), 2)

    def test_run_without_session_exits_two(self):
        with tempfile.TemporaryDirectory() as root:
            with mock.patch.dict(
                os.environ, {"AGENT_DESKTOP_E2E_LOCK_ROOT": root}
            ):
                code = main.main(["--tier", "x11-openbox"])
        self.assertEqual(code, 2)

    def test_incompatible_leg_and_tier_is_rejected_before_startup(self):
        with mock.patch.object(main, "_acquire_lease", return_value=None), \
            mock.patch.object(gate, "check_prerequisites", side_effect=AssertionError("invalid selection reached startup")):
            self.assertEqual(main.main([
                "--tier", "wayland", "--leg", "state.terminal_copy_follows_selection"]), 2)


class ReportTests(unittest.TestCase):
    def test_exit_code_mapping(self):
        self.assertEqual(report.exit_code(0, False), 0)
        self.assertEqual(report.exit_code(3, False), 1)
        self.assertEqual(report.exit_code(0, True), 2)
        self.assertEqual(report.exit_code(3, True), 2)

    def test_failure_counts(self):
        outcomes = [
            report.LegOutcome("a", "x11-openbox", "headless", report.PASS),
            report.LegOutcome("b", "x11-openbox", "headless", report.BLOCKED),
            report.LegOutcome("c", "x11-openbox", "headless", report.FAIL),
            report.LegOutcome("d", "x11-openbox", "headless", report.ERROR),
            report.LegOutcome("e", "x11-openbox", "headless", report.UNIMPLEMENTED),
            report.LegOutcome("f", "x11-openbox", "headless", report.TIER_UNAVAILABLE),
            report.LegOutcome("g", "x11-openbox", "headless", report.OUTSTANDING),
        ]
        self.assertEqual(report.count_failures(outcomes), 6)

    def test_only_passed_or_asserted_refusal_legs_exit_zero(self):
        accepted = [
            report.LegOutcome("a", "x11-openbox", "headless", report.PASS),
            report.LegOutcome("b", "x11-openbox", "headless", report.REFUSED),
        ]
        self.assertEqual(report.count_failures(accepted), 0)
        self.assertEqual(report.exit_code(report.count_failures(accepted), False), 0)
        for rejected in (report.OWNER, report.BLOCKED, report.OUTSTANDING, report.FAIL):
            outcomes = accepted + [
                report.LegOutcome("z", "x11-openbox", "headless", rejected)
            ]
            self.assertEqual(report.exit_code(report.count_failures(outcomes), False), 1)

    def test_render_groups_by_tier_and_mode(self):
        outcomes = [
            report.LegOutcome("a", "x11-openbox", "headless", report.PASS),
            report.LegOutcome("b", "wayland", "headed", report.FAIL, detail="no read"),
        ]
        text = report.render(outcomes)
        self.assertIn("tier x11-openbox", text)
        self.assertIn("tier wayland", text)
        self.assertIn("headed", text)
        self.assertIn("a", text)
        self.assertIn("summary:", text)

    def test_parse_args_flags(self):
        args = main.parse_args(["--tier", "wayland", "--leg", "x.y", "--lying-actions"])
        self.assertEqual(args.tier, "wayland")
        self.assertEqual(args.leg, "x.y")
        self.assertTrue(args.lying_actions)


class SourceRuleTests(unittest.TestCase):
    def test_remote_lock_wait_can_be_bounded_separately_from_execution(self):
        with tempfile.TemporaryDirectory() as directory:
            for name, body in (("ssh", 'printf "%s\\n" "$*"'), ("rsync", "exit 0")):
                path = Path(directory) / name
                path.write_text("#!/bin/sh\n" + body + "\n")
                path.chmod(0o700)
            env = dict(os.environ, PATH=directory + os.pathsep + os.environ["PATH"],
                       AGENT_DESKTOP_REMOTE_TIMEOUT="780",
                       AGENT_DESKTOP_REMOTE_LOCK_TIMEOUT="30")
            result = subprocess.run(["bash", str(REPO / "scripts/remote-linux.sh"),
                                     "-c", "true"], env=env, capture_output=True, text=True,
                                    timeout=10, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("flock -o -w 30", result.stdout)
        self.assertIn("timeout -k 30 780", result.stdout)

    def test_session_and_remote_cleanup_is_recoverable(self):
        permanent_delete = re.compile(r"(?m)(?:^|[;&|(){]|trap ['\"])\s*rm(?:\s|$)|\bfind\b[^\n]*\s-delete\b")
        paths = [REPO / "scripts" / name for name in (
            "private-session.sh", "private-wayland-shell.sh", "remote-linux.sh")]
        self.assertEqual([path.name for path in paths if permanent_delete.search(path.read_text())], [])

    def test_no_shell_file_uses_rm(self):
        permanent_delete = re.compile(r"(?m)(?:^|[;&|(){])\s*rm(?:\s|$)")
        offenders = [
            path.name
            for path in sorted(ROOT.glob("*.sh"))
            if permanent_delete.search(path.read_text())
        ]
        self.assertEqual(offenders, [])

    def test_rm_scan_covers_e2e_linux(self):
        contract = (REPO / "tests" / "e2e" / "test_harness_contract.py").read_text()
        self.assertIn("e2e-linux", contract)

    def test_runner_files_stay_under_the_line_cap(self):
        over = [
            path.name
            for path in ROOT.rglob("*.py")
            if len(path.read_text().splitlines()) >= 400
        ]
        self.assertEqual(over, [])


class WorkflowTests(unittest.TestCase):
    def test_live_workflow_fails_closed(self):
        workflow = (REPO / ".github" / "workflows" / "linux-e2e.yml").read_text()
        self.assertIn("workflow_dispatch", workflow)
        self.assertIn("LINUX_E2E_RUNNER", workflow)
        self.assertNotIn("continue-on-error", workflow)
        self.assertIn("tests/e2e-linux/run.sh", workflow)

    def test_hosted_lane_runs_the_self_tests(self):
        ci = (REPO / ".github" / "workflows" / "ci.yml").read_text()
        self.assertIn("discover -s tests/e2e-linux", ci)
        self.assertIn("tests/e2e-linux/run.sh", ci)
        self.assertIn("AGENT_DESKTOP_E2E_SKIP_LIVE", ci)


if __name__ == "__main__":
    unittest.main()
