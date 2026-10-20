from oracle import atspi_raw as raw, proc
from runner.helper import (command, delayed, managed, nodes, ref, require, root,
                           snapshot)
from runner.legs import implementation
from runner.verdict import LegContext


def frame_alive(ctx, app):
    owners = [node for node in raw.registry(ctx.tier) if node.bus in app.owner_names]
    require(len(owners) == 1, "VS Code frame owner vanished")
    require(proc.facts(app.pid)[0] == app.start_time, "VS Code process generation changed")
    from runner.apps import window_id
    windows = [window_id(node) for node in owners[0].children() if node.role in (16, 23, 69)]
    require(set(app.windows).issubset(windows), "VS Code frame vanished")
    return windows


def dropdown(tree):
    return [node.name for node in tree.find(role=35) if "SHOWING" in node.states
            and node.name not in ("File", "Edit", "Selection", "View", "Go", "More", "Run", "Terminal", "Help")]


@implementation("toolkit_electron.frame_stays_registered")
def frame_stays_registered(ctx: LegContext) -> None:
    with managed(ctx, "VS Code") as app:
        initial = frame_alive(ctx, app)
        ctx.before("atspi", initial)
        require(raw.poll(lambda: bool(frame_alive(ctx, app)), 33, floor_s=32))
        after = frame_alive(ctx, app)
        require(after == initial)
        ctx.after("atspi", after)
        ctx.restored("atspi", after)


@implementation("toolkit_electron.refs_and_completeness")
def refs_and_completeness(ctx: LegContext) -> None:
    with managed(ctx, "VS Code") as app:
        initial = frame_alive(ctx, app)
        ctx.before("atspi", initial)
        data = snapshot(ctx, app)
        require(data["ref_count"] > 20)
        require(any(item.get("name") == "File" for item in nodes(data.get("tree"))))
        require(frame_alive(ctx, app) == initial)
        ctx.after("atspi", frame_alive(ctx, app))
        ctx.restored("atspi", frame_alive(ctx, app))


@implementation("toolkit_electron.menu_bar_by_keyboard")
def menu_bar_by_keyboard(ctx: LegContext) -> None:
    with managed(ctx, "VS Code") as app:
        tree = root(ctx, app)
        frame_alive(ctx, app)
        require(not dropdown(tree))
        ctx.before("atspi", dropdown(tree))
        try:
            command(ctx, ["press", "alt+f"], app)
            require(raw.poll(lambda: dropdown(tree), 3), "keyboard did not open the File dropdown")
            bar = tree.find(role=34)
            require(bar and "SHOWING" in bar[0].states)
            ctx.after("atspi", dropdown(tree))
            frame_alive(ctx, app)
        finally:
            command(ctx, ["press", "escape"], app)
            require(raw.poll(lambda: not dropdown(tree), 2))
            frame_alive(ctx, app)
            ctx.restored("atspi", dropdown(tree))


@implementation("toolkit_electron.wait_menu_both_directions")
def wait_menu_both_directions(ctx: LegContext) -> None:
    with managed(ctx, "VS Code") as app:
        tree = root(ctx, app)
        target = ref(ctx, app, name="File", surface="menubar")
        require(not dropdown(tree))
        ctx.before("atspi", dropdown(tree))
        trigger = None

        def wait(opened, timeout=2000, code=None):
            frame_alive(ctx, app)
            return command(ctx, ["wait", "--menu" if opened else "--menu-closed",
                                 "--app", app.name, "--timeout", str(timeout)], code=code)

        try:
            wait(True, 200, "TIMEOUT")
            require(not dropdown(tree))
            trigger = delayed(0.4, lambda: command(ctx, ["click", target], app, headed=True))
            wait(True)
            trigger.join()
            require(raw.poll(lambda: dropdown(tree), 2))
            wait(False, 200, "TIMEOUT")
            require(dropdown(tree))
            ctx.after("atspi", dropdown(tree))
            trigger = delayed(0.4, lambda: command(ctx, ["press", "escape"], app, headed=True))
            wait(False)
            trigger.join()
            require(raw.poll(lambda: not dropdown(tree), 2))
            frame_alive(ctx, app)
        finally:
            if trigger:
                trigger.cancel()
            if dropdown(tree):
                command(ctx, ["press", "escape"], app, headed=True)
            require(raw.poll(lambda: not dropdown(tree), 2))
            ctx.restored("atspi", dropdown(tree))
