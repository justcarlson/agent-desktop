"""R1 on the upstream `wayland` tier: Hyprland-like state never moves the route."""

from __future__ import annotations

from pathlib import Path

from oracle.proc import members
from runner import socket_fixture
from runner.helper import activate, changed, display, managed, one, ref, require, root
from runner.legs import implementation
from runner.verdict import LegContext

STALE = "ad_e2e_stale_route"
FAKE = "ad_e2e_answer_route"


def _fake_instances(runtime: Path) -> socket_fixture.Listener:
    stale = runtime / "hypr" / STALE
    stale.mkdir(parents=True, mode=0o700)
    (stale / ".socket.sock").write_bytes(b"")
    (runtime / "hypr" / FAKE).mkdir(mode=0o700)
    return socket_fixture.Listener(runtime / "hypr" / FAKE / ".socket.sock", socket_fixture.ANSWER)


def _remove_fakes(runtime: Path) -> None:
    for name in (STALE, FAKE):
        directory = runtime / "hypr" / name
        (directory / ".socket.sock").unlink(missing_ok=True)
        if directory.exists():
            directory.rmdir()
    if (runtime / "hypr").exists() and not any((runtime / "hypr").iterdir()):
        (runtime / "hypr").rmdir()


def hyprland_like_envs(env: dict[str, str]) -> dict[str, dict[str, str]]:
    return {
        "a named instance that is gone": dict(env, HYPRLAND_INSTANCE_SIGNATURE=STALE),
        "an answering instance that serves no display": dict(env),
        "both": dict(env, HYPRLAND_INSTANCE_SIGNATURE=STALE, XDG_CURRENT_DESKTOP="Hyprland"),
    }


@implementation("session.other_wayland_route_unchanged")
def other_wayland_route_unchanged(ctx: LegContext) -> None:
    runtime = Path(ctx.tier.runtime_dir)
    with managed(ctx, "Calculator") as app:
        tree = root(ctx, app)
        seven = ref(ctx, app, role="button", name="7")
        baseline = ctx.product(["list-displays"], headed=False)
        require(baseline.ok, f"the wayland route lists no displays: {baseline.raw}")
        before = display(tree)
        ctx.before("atspi.display", before)
        ctx.before("proc.members", members(runtime))
        listener = _fake_instances(runtime)
        try:
            problems = []
            for label, env in hyprland_like_envs(ctx.tier.env).items():
                listed = ctx.product(["list-displays"], env=env, headed=False)
                if not listed.ok or listed.data != baseline.data:
                    problems.append(f"{label}: list-displays became {listed.raw}")
            clicked = ctx.product(["click", seven], env=hyprland_like_envs(ctx.tier.env)["both"], headed=False)
            require(clicked.ok, f"a semantic click under Hyprland-like state failed: {clicked.raw}")
            expected = "7" if before in ("", "0") else before + "7"
            ctx.after("atspi.display", changed(lambda: display(tree), expected))
            ctx.after("proc.members", members(runtime))
            require(not problems, "; ".join(problems))
        finally:
            listener.close()
            _remove_fakes(runtime)
            activate(one(tree, role=43, name="Undo"))
        ctx.restored("atspi.display", changed(lambda: display(tree), before))
        ctx.restored("proc.members", members(runtime))
