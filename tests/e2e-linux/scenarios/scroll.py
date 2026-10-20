from uuid import uuid4

from oracle.atspi_raw import poll
from runner.helper import (changed, command, managed, one, ref, require, root,
                           stale_twin, trash)
from runner.legs import implementation
from runner.verdict import LegContext


def realised(tree):
    return sorted({node.name for node in tree.walk() if node.name.startswith("file")
                   and node.name.endswith(".txt")})


def scroll_target(bar):
    maximum = float(bar.get_property("org.a11y.atspi.Value", "MaximumValue"))
    minimum = float(bar.get_property("org.a11y.atspi.Value", "MinimumValue"))
    require(maximum > minimum, "scrollbar has no range")
    return minimum + (maximum - minimum) * 0.98


def raw_value(bar, value):
    from gi.repository import GLib
    bar.invoke("org.freedesktop.DBus.Properties", "Set", "(ssv)",
               ("org.a11y.atspi.Value", "CurrentValue", GLib.Variant("d", value)))


@implementation("scroll.files_300_last_item")
def files_300_last_item(ctx: LegContext) -> None:
    directory = ctx.tier.runtime_dir / ("scroll-" + uuid4().hex)
    directory.mkdir()
    for index in range(300):
        (directory / f"file{index:03}.txt").write_text(str(index))
    try:
        with managed(ctx, "Files", (str(directory),)) as app:
            tree = root(ctx, app)
            bar = one(tree, role=48)
            target = ref(ctx, app, role="scrollbar")
            initial = bar.value()
            before = realised(tree)
            require("file299.txt" not in before and 0 < len(before) < 300)
            try:
                raw_value(bar, scroll_target(bar))
                require(poll(lambda: bar.value() != initial, 2))
                wanted = bar.value()
            finally:
                raw_value(bar, initial)
                changed(bar.value, initial)
                changed(lambda: realised(tree), before, seconds=4, floor=2)
            ctx.before("atspi", {"value": initial, "files": before})
            try:
                command(ctx, ["set-value", target, str(wanted)], app)
                require(poll(lambda: "file299.txt" in realised(tree), 4, floor_s=2),
                        "last file did not become realised")
                command(ctx, ["find", "--name", "file299.txt", "--exact"], app)
                require(bar.value() != initial)
                ctx.after("atspi", {"value": bar.value(), "files": realised(tree)})
                stale_twin(ctx, app, target, lambda: realised(tree), ("set-value", "0"))
            finally:
                command(ctx, ["set-value", target, str(initial)], app)
                changed(bar.value, initial)
                changed(lambda: realised(tree), before, seconds=4, floor=2)
                ctx.restored("atspi", {"value": bar.value(), "files": realised(tree)})
    finally:
        trash(ctx, directory)
