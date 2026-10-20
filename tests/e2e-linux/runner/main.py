from __future__ import annotations

import argparse
import importlib.util
import json
import os
import signal
import sys
import tempfile
import time
from pathlib import Path

HARNESS_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(HARNESS_ROOT))

TIER_CHOICES = ("x11-openbox", "x11-mutter", "wayland")
TIER_DEADLINE_S = 90.0
LEASE_FD_ENV = "AGENT_DESKTOP_INTERACTION_LEASE_FD"
SUITE_ENV = "AGENT_DESKTOP_E2E_SUITE"
LOCK_ROOT_ENV = "AGENT_DESKTOP_E2E_LOCK_ROOT"

_LEASE_FD: int | None = None


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(prog="run.sh")
    parser.add_argument("--tier", choices=TIER_CHOICES)
    parser.add_argument("--leg")
    parser.add_argument("--lying-actions", action="store_true")
    return parser.parse_args(argv)


def _load_helper(name: str):
    spec = importlib.util.spec_from_file_location(
        name, HARNESS_ROOT.parent / "e2e" / f"{name}.py"
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _acquire_lease(env: dict[str, str]) -> str | None:
    global _LEASE_FD
    interaction_lock = _load_helper("interaction_lock")
    lock_root = env.get(LOCK_ROOT_ENV)
    if lock_root:
        fd = interaction_lock.acquire(root=lock_root)
        if fd is None:
            return "the interaction lease is already held"
        _LEASE_FD = fd
        return None
    raw_fd = env.get(LEASE_FD_ENV)
    if raw_fd is None:
        return "run through run.sh so the interaction lease is held"
    try:
        interaction_lock.verify_inherited(int(raw_fd))
    except (ValueError, RuntimeError, OSError) as error:
        return f"the inherited interaction lease is not valid: {error}"
    return None


def _sigterm(_signum, _frame):
    raise SystemExit(143)


def _write_lying_binary(suite_dir: Path) -> Path:
    path = Path(suite_dir) / "bin" / "agent-desktop"
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text('#!/bin/sh\nprintf \'{"ok":true,"data":{"headless":true}}\\n\'\n')
    path.chmod(0o500)
    return path


def _evidence_path(suite_dir: Path, tier: str, mode: str, leg_id: str) -> Path:
    return Path(suite_dir) / "evidence" / tier / mode / f"{leg_id}.json"


def _write_evidence(path: Path, ctx, leg_id: str, tier: str, mode: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    record = {
        "leg": leg_id,
        "tier": tier,
        "mode": mode,
        "reads": ctx.reads,
        "findings": ctx.findings,
        "owned_baseline": sorted(ctx.baseline.members),
        "owned_final": sorted(ctx.owned().members),
        "products": [
            {
                "argv": envelope.argv,
                "ok": envelope.ok,
                "error": envelope.error,
                "elapsed_ms": envelope.elapsed_ms,
                "raw": envelope.raw[:4000],
            }
            for envelope in ctx.products
        ],
    }
    path.write_text(json.dumps(record, indent=2, default=repr))


def _run_leg(impl, leg, tier_obj, tier_name: str, mode: str, suite_dir: Path, verdict, report):
    from runner.verdict import LegContext

    ctx = LegContext(tier_obj, mode, leg)
    started = time.perf_counter()
    outcome = report.PASS
    detail = ""
    try:
        impl(ctx)
        failures = verdict.evaluate(ctx)
        if failures:
            outcome = report.FAIL
            detail = "; ".join(failures)
    except NotImplementedError as error:
        outcome = report.BLOCKED
        detail = str(error)
    except AssertionError as error:
        outcome = report.FAIL
        detail = str(error)
    except Exception as error:
        outcome = report.ERROR
        detail = f"{type(error).__name__}: {error}"
    elapsed = (time.perf_counter() - started) * 1000.0
    try:
        _write_evidence(
            _evidence_path(suite_dir, tier_name, mode, leg.id), ctx, leg.id, tier_name, mode
        )
    except OSError as error:
        ctx.finding(f"evidence could not be written: {error}")
    return report.LegOutcome(
        leg_id=leg.id,
        tier=tier_name,
        mode=mode,
        outcome=outcome,
        detail=detail,
        findings=tuple(ctx.findings),
        elapsed_ms=elapsed,
    )


def _run_tier(tier_name: str, legs_for_tier, impls, suite_dir: Path, verify_binary: bool):
    from runner import report, session, stage, verdict

    outcomes = []
    try:
        with session.start_tier(tier_name, TIER_DEADLINE_S) as tier_obj:
            for leg in legs_for_tier:
                impl = impls.get(leg.id)
                for mode in leg.modes:
                    if impl is None:
                        outcomes.append(
                            report.LegOutcome(
                                leg_id=leg.id,
                                tier=tier_name,
                                mode=mode,
                                outcome=report.UNIMPLEMENTED,
                            )
                        )
                        continue
                    outcomes.append(
                        _run_leg(impl, leg, tier_obj, tier_name, mode, suite_dir, verdict, report)
                    )
                if (
                    verify_binary
                    and impl is not None
                    and not stage.verify_staged(
                        Path(os.environ["AGENT_DESKTOP_E2E_BIN"]), suite_dir
                    )
                ):
                    outcomes.append(
                        report.LegOutcome(
                            leg_id="__staged_binary__",
                            tier=tier_name,
                            mode=leg.modes[-1],
                            outcome=report.ERROR,
                            detail="the staged binary changed during the run",
                        )
                    )
                    break
    except session.TierUnavailable as error:
        for leg in legs_for_tier:
            for mode in leg.modes:
                outcomes.append(
                    report.LegOutcome(
                        leg_id=leg.id,
                        tier=tier_name,
                        mode=mode,
                        outcome=report.TIER_UNAVAILABLE,
                        detail=str(error),
                    )
                )
    except session.TeardownError as error:
        outcomes.append(
            report.LegOutcome(
                leg_id="__teardown__",
                tier=tier_name,
                mode="teardown",
                outcome=report.ERROR,
                detail=str(error),
            )
        )
    return outcomes


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    import scenarios
    from runner import gate, legs, report, session, stage, verdict

    env = dict(os.environ)
    lease_problem = _acquire_lease(env)
    if lease_problem:
        print(f"gate: {lease_problem}", file=sys.stderr)
        return report.exit_code(0, True)
    tiers = (args.tier,) if args.tier else TIER_CHOICES
    missing = gate.check_prerequisites(tiers)
    if missing:
        for item in missing:
            print(f"gate: {item}", file=sys.stderr)
        return report.exit_code(0, True)

    signal.signal(signal.SIGTERM, _sigterm)

    suite_dir = Path(env.get(SUITE_ENV) or tempfile.mkdtemp(prefix="ad-e2e-suite-"))
    suite_dir.mkdir(parents=True, exist_ok=True)
    os.environ[SUITE_ENV] = str(suite_dir)

    if args.lying_actions:
        binary = _write_lying_binary(suite_dir)
    else:
        source = Path(
            env.get("AGENT_DESKTOP_E2E_BIN_SRC")
            or env.get("AGENT_DESKTOP_E2E_BIN")
            or session.HARNESS_ROOT.parent.parent / "target" / "release" / "agent-desktop"
        )
        try:
            binary = stage.stage_binary(source, suite_dir)
        except (OSError, RuntimeError, stage.StagedBinaryModified) as error:
            print(f"gate: {error}", file=sys.stderr)
            return report.exit_code(0, True)
    os.environ["AGENT_DESKTOP_E2E_BIN"] = str(binary)

    session_base = Path(
        env.get("AGENT_DESKTOP_SESSION_BASE") or str(Path.home() / "ad-wt")
    )
    session.sweep_strays(session_base)

    impls = legs.implementations()
    if args.leg:
        try:
            legs.leg_by_id(args.leg)
        except KeyError:
            print(f"gate: unknown leg {args.leg}", file=sys.stderr)
            return report.exit_code(0, True)
        selected_ids = {args.leg}
    else:
        selected_ids = {leg.id for leg in legs.REGISTRY}

    outcomes = []
    for tier_name in tiers:
        legs_for_tier = [
            leg for leg in legs.REGISTRY if leg.id in selected_ids and tier_name in leg.tiers
        ]
        if not legs_for_tier:
            continue
        outcomes.extend(
            _run_tier(
                tier_name, legs_for_tier, impls, suite_dir, not args.lying_actions
            )
        )

    report_path = Path(suite_dir) / "report.txt"
    text = report.render(outcomes)
    try:
        report_path.write_text(text + "\n")
    except OSError:
        pass
    print(text)

    failures = report.count_failures(outcomes)
    if failures == 0:
        if not session._remove_tree(suite_dir):
            print(f"warn: the suite directory was retained: {suite_dir}", file=sys.stderr)
    else:
        print(f"evidence retained at {suite_dir}", file=sys.stderr)
    return report.exit_code(failures, False)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
