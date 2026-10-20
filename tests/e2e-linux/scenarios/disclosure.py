from uuid import uuid4

from oracle import gsettings
from runner.helper import (changed, command, managed, nodes, require, root,
                           snapshot, stale_twin, trash)
from runner.legs import implementation
from runner.verdict import LegContext


def expanded_state(tree, label):
    matches = [node for node in tree.walk() if node.name == label and "EXPANDABLE" in node.states]
    require(len(matches) == 1, "folder disclosure row missing or ambiguous")
    row = matches[0]
    children = [node.name for node in tree.walk() if node.name == "leaf.txt" and "SHOWING" in node.states]
    return "EXPANDED" in row.states, bool(children)


@implementation("disclosure.files_tree_expand_collapse")
def files_tree_expand_collapse(ctx: LegContext) -> None:
    directory = ctx.tier.runtime_dir / ("tree-" + uuid4().hex)
    (directory / "folder").mkdir(parents=True)
    (directory / "folder/leaf.txt").write_text("desktop tree")
    settings = (("org.gnome.nautilus.list-view", "use-tree-view", "true"),
                ("org.gnome.nautilus.preferences", "default-folder-viewer", "'list-view'"))
    initial = {(schema, key): gsettings.get(ctx.tier, schema, key) for schema, key, _ in settings}
    ctx.before("gsettings", initial)
    try:
        for schema, key, value in settings:
            gsettings.set(ctx.tier, schema, key, value)
        with managed(ctx, "Files", (str(directory),)) as app:
            tree = root(ctx, app)
            changed(lambda: expanded_state(tree, "folder"), (False, False))
            data = snapshot(ctx, app)
            candidates = [item for item in nodes(data.get("tree")) if item.get("name") == "folder"
                          and item.get("ref_id") and
                          any(action in ("Expand", "expand") for action in item.get("available_actions", []))]
            require(len(candidates) == 1, "no unique expandable folder ref")
            target = candidates[0]["ref_id"]
            ctx.before("atspi", (False, False))
            try:
                command(ctx, ["expand", target], app)
                changed(lambda: expanded_state(tree, "folder"), (True, True), seconds=3)
                ctx.after("atspi", expanded_state(tree, "folder"))
                stale_twin(ctx, app, target, lambda: expanded_state(tree, "folder"), ("expand",))
            finally:
                command(ctx, ["collapse", target], app)
                changed(lambda: expanded_state(tree, "folder"), (False, False))
                ctx.restored("atspi", expanded_state(tree, "folder"))
    finally:
        for (schema, key), value in initial.items():
            gsettings.set(ctx.tier, schema, key, value)
        restored = {(schema, key): gsettings.get(ctx.tier, schema, key) for schema, key in initial}
        require(restored == initial)
        ctx.after("gsettings", restored)
        ctx.restored("gsettings", restored)
        trash(ctx, directory)
