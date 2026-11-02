"""Focus stolen before the first key-down of a headed `type` on the attached `hyprland` tier.

The thief, the key log app, holds focus when the leg starts the product.
A headed `type` first focuses its target, the zoo's entry window, so the leg
waits until the zoo is active and then gives focus back to the thief at once.
The product's focus gate runs before every key-down, so the type must refuse
with `not_delivered` and no key: the zoo's entry stays as it was and the thief
logs no key at all. When the product sends keys before the steal lands, the
attempt is a late steal, not the case under test, and the leg tries again, up
to three times. The thief must log no key in any attempt.
"""

from __future__ import annotations

import time

from oracle import atspi_raw as raw
from oracle import hyprland
from runner import product
from runner.helper import require
from runner.legs import implementation
from runner.verdict import LegContext
from scenarios.hyprland_common import desktop, main_instance, shared
from scenarios.hyprland_focus_controls import active, focus_argv, move
from scenarios.hyprland_keys_cancel import finished, running_product
from scenarios.hyprland_keys_common import give_focus, key_log, keyboards, presses, toplevel
from scenarios.hyprland_keys_controls import ENTRY, focused_zoo, zoo_ref, zoo_text

STOLEN = "stolen"
ATTEMPTS = 3
TYPE_TIMEOUT_MS = "5000"
REFUSAL_CODES = ("ACTION_FAILED", "STALE_REF")
LATE = ("stopped_after_keys", "delivered")
STEAL_WAIT_S = 10.0


def theft_outcome(envelope) -> str:
    """Names how a type whose focus was stolen ended."""
    if envelope is None:
        return "other"
    if envelope.ok:
        return "delivered"
    error = envelope.error or {}
    details = error.get("details") or {}
    delivery = (error.get("disposition") or {}).get("delivery")
    if delivery == "delivery_uncertain":
        return "stopped_after_keys"
    if (delivery == "not_delivered" and error.get("code") in REFUSAL_CODES
            and details.get("physical_delivery_started") in (None, False)):
        return "refused_before_first_key"
    return "other"


def late_steal(outcome: str) -> bool:
    """Keys left before the steal landed, so the attempt did not test a steal before the first key."""
    return outcome in LATE


def steal_once_active(read_active, target: str, steal, timeout_s: float = STEAL_WAIT_S) -> bool:
    """Calls `steal` as soon as `read_active` names the target, the product's own focus step done."""
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        if read_active() == target:
            steal()
            return True
        time.sleep(0.001)
    return False


def _steal_argv(tier, address: str) -> list[str]:
    main = main_instance(tier)
    return focus_argv(address, hyprland.uses_lua(shared(tier), main["signature"]))


def _attempt(ctx: LegContext, ref: str, target: str, thief: str, steal: list[str]):
    tier = ctx.tier
    main = main_instance(tier)
    give_focus(tier, thief)
    running = running_product(tier, ["type", ref, STOLEN, "--timeout-ms", TYPE_TIMEOUT_MS])
    sent: list[str] = []
    seen = steal_once_active(lambda: active(tier).get("address"), target,
                             lambda: sent.append(hyprland.control(shared(tier), main["signature"], steal)))
    code, out = finished(running, 20.0)
    parsed = product._parse_envelope(out)
    envelope = None if parsed is None else product.Envelope(
        ok=parsed["ok"], data=parsed.get("data"), error=parsed.get("error"), raw=out, elapsed_ms=0.0,
        argv=running.args, exit_code=code)
    stole = seen and sent == ["ok"] and raw.poll(lambda: active(tier).get("address") == thief, 3.0)
    return envelope, bool(seen), bool(stole), out


@implementation("input.hyprland_focus_theft_before_first_key")
def focus_theft_before_first_key(ctx: LegContext) -> None:
    tier = ctx.tier
    before = keyboards(tier)
    ctx.before("hyprland.keyboards", before)
    with focused_zoo(ctx) as zoo, key_log(tier, "thief") as (thief_app, log):
        shown = desktop(tier, main_instance(tier))["workspace"]
        thief = toplevel(tier, thief_app)["address"]
        target = toplevel(tier, zoo.app)["address"]
        move(tier, thief, shown)
        ref = zoo_ref(ctx, zoo, ENTRY)
        steal = _steal_argv(tier, thief)
        ctx.before("applog.thief_presses", len(presses(log)))
        problems, outcomes = [], []
        for _ in range(ATTEMPTS):
            base = zoo_text(tier, zoo, ENTRY)
            envelope, seen, stole, out = _attempt(ctx, ref, target, thief, steal)
            outcome = theft_outcome(envelope)
            outcomes.append(outcome)
            if not seen:
                problems.append(f"the product never focused the zoo: {active(tier)}; it answered {out[:300]}")
                break
            if not stole:
                problems.append(f"the thief never took focus back: {active(tier)}")
            if outcome == "refused_before_first_key":
                if zoo_text(tier, zoo, ENTRY) != base:
                    problems.append(f"the refused type changed the entry from {base!r} to "
                                    f"{zoo_text(tier, zoo, ENTRY)!r}")
                break
            if not late_steal(outcome):
                problems.append(f"the stolen type ended {outcome}: {out[:300]}")
                break
            ctx.finding(f"a late steal: the product sent keys first: {out[:300]}")
        if "refused_before_first_key" not in outcomes:
            problems.append(f"no attempt refused before the first key: {outcomes}")
        if presses(log):
            problems.append(f"the thief received keys: {presses(log)}")
        ctx.after("applog.thief_presses", len(presses(log)))
        ctx.after("hyprland.keyboards", keyboards(tier))
        if keyboards(tier) != before:
            problems.append(f"the keyboards changed: {before} to {keyboards(tier)}")
        ctx.restored("applog.thief_presses", len(presses(log)))
        require(not problems, "; ".join(problems))
    ctx.restored("hyprland.keyboards", keyboards(tier))
