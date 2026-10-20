from oracle.atspi_raw import poll
from runner.helper import (activate, changed, command, delayed, display, managed,
                           one, ref, require, root, stale_twin)
from runner.legs import implementation
from runner.verdict import LegContext


def timeout_verdict(response, expected_ms):
    error = response.error or {}
    require(error.get("code") == "TIMEOUT", response.raw)
    require(error.get("details", {}).get("kind") == "actionability_timeout", response.raw)
    require(expected_ms * 0.85 <= response.elapsed_ms <= expected_ms + 1800,
            f"timeout outside tolerance: {response.elapsed_ms}")


def undo_run(ctx, timeout=None, *, delayed_input=False):
    with managed(ctx, "Calculator") as app:
        tree = root(ctx, app)
        undo = one(tree, role=43, name="Undo")
        digit = one(tree, role=43, name="7")
        target = ref(ctx, app, role="button", name="Undo")
        initial = display(tree)
        require(initial == "" and "SENSITIVE" not in undo.states, "Undo not initially disabled")
        ctx.before("atspi", {"display": initial, "sensitive": False})

        def input_twice():
            activate(digit)
            activate(digit)

        trigger = delayed(2.0, input_twice) if delayed_input else None
        args = ["click", target]
        if timeout is not None:
            args += ["--timeout-ms", str(timeout)]
        try:
            expected_code = "ACTION_FAILED" if timeout == 0 else (
                "TIMEOUT" if not delayed_input or timeout == 1000 else None)
            response = command(ctx, args, app, code=expected_code)
            if expected_code == "TIMEOUT":
                timeout_verdict(response, timeout or 5000)
            if timeout == 0:
                require(response.elapsed_ms < 800, "zero-timeout action waited")
                require(display(tree) == initial, "zero-timeout action changed the display")
            if trigger:
                trigger.join()
                require(trigger.fired_at is not None, "input helper did not fire")
                expected = "77" if expected_code else "7"
                changed(lambda: display(tree), expected, floor=0.3)
            else:
                require(display(tree) == initial, "disabled action changed the display")
            ctx.after("atspi", {"display": display(tree), "sensitive": "SENSITIVE" in undo.states})
            stale_twin(ctx, app, target, lambda: display(tree))
        finally:
            if trigger:
                trigger.cancel()
            for _ in range(3):
                previous = display(tree)
                if previous == initial:
                    break
                activate(undo)
                require(poll(lambda: display(tree) != previous, 1), "Undo restoration had no effect")
            changed(lambda: display(tree), initial)
            changed(lambda: "SENSITIVE" in undo.states, False)
            ctx.restored("atspi", {"display": display(tree), "sensitive": False})


@implementation("state.undo_enable_delayed_default_timeout")
def undo_enable_delayed_default_timeout(ctx: LegContext) -> None:
    undo_run(ctx, delayed_input=True)


@implementation("state.undo_enable_delayed_explicit_timeout")
def undo_enable_delayed_explicit_timeout(ctx: LegContext) -> None:
    undo_run(ctx, 5000, delayed_input=True)


@implementation("state.undo_enable_delayed_short_timeout_fails")
def undo_enable_delayed_short_timeout_fails(ctx: LegContext) -> None:
    undo_run(ctx, 1000, delayed_input=True)


@implementation("state.undo_dispatches_exactly_once")
def undo_dispatches_exactly_once(ctx: LegContext) -> None:
    undo_run(ctx, delayed_input=True)


@implementation("state.undo_disabled_explicit_timeout")
def undo_disabled_explicit_timeout(ctx: LegContext) -> None:
    undo_run(ctx, 2000)


@implementation("state.undo_disabled_default_timeout")
def undo_disabled_default_timeout(ctx: LegContext) -> None:
    undo_run(ctx)


@implementation("state.undo_zero_timeout_single_check")
def undo_zero_timeout_single_check(ctx: LegContext) -> None:
    undo_run(ctx, 0, delayed_input=True)


@implementation("state.terminal_copy_follows_selection")
def terminal_copy_follows_selection(ctx: LegContext) -> None:
    with managed(ctx, "Terminal", ("--show-menubar",)) as app:
        tree = root(ctx, app)
        terminal = one(tree, role=60)
        copy = one(tree, name="Copy", role=35)
        target = ref(ctx, app, name="Copy", role="menuitem", surface="menubar")
        interface = "org.a11y.atspi.Text"
        length = terminal.get_property(interface, "CharacterCount")
        require(length > 0 and "SENSITIVE" not in copy.states, "Copy not initially disabled")
        ctx.before("atspi", False)
        require(command(ctx, ["is", target, "--property", "enabled"], app).data["result"] is False)
        try:
            require(terminal.invoke(interface, "AddSelection", "(ii)", (0, length))[0])
            changed(lambda: "SENSITIVE" in copy.states, True)
            require(command(ctx, ["is", target, "--property", "enabled"], app).data["result"] is True)
            ctx.after("atspi", True)
        finally:
            count = terminal.invoke(interface, "GetNSelections")[0]
            for index in reversed(range(count)):
                require(terminal.invoke(interface, "RemoveSelection", "(i)", (index,))[0])
            changed(lambda: "SENSITIVE" in copy.states, False)
            ctx.restored("atspi", False)


@implementation("state.text_editor_semantic_type")
def text_editor_semantic_type(ctx: LegContext) -> None:
    with managed(ctx, "Text Editor", ("--new-window",)) as app:
        tree = root(ctx, app)
        field = one(tree, role=61)
        target = ref(ctx, app, role="textfield")
        initial = field.text()
        require(initial == "", "document must start empty")
        ctx.before("atspi", initial)
        try:
            command(ctx, ["type", target, "Desktop café"], app)
            changed(field.text, "Desktop café")
            ctx.after("atspi", field.text())
            stale_twin(ctx, app, target, field.text, ("type", "ignored"))
        finally:
            command(ctx, ["clear", target], app)
            changed(field.text, initial)
            ctx.restored("atspi", field.text())
