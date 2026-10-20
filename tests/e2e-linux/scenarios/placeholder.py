from oracle.atspi_raw import ACCESSIBLE
from runner.helper import command, managed, nodes, require, root
from runner.legs import implementation
from runner.verdict import LegContext


def placeholder_read(ctx, app, expected):
    fields = [node for node in root(ctx, app).walk() if node.role in (61, 79)
              and "SHOWING" in node.states]
    require(fields, "search field is absent")
    evidence = [(node.name, node.get_property(ACCESSIBLE, "Description"), node.attributes())
                for node in fields]
    if expected:
        require(any(attrs.get("placeholder-text", attrs.get("placeholder")) == expected
                    for _, _, attrs in evidence), "raw placeholder disagrees")
    else:
        require(all(not attrs.get("placeholder-text", attrs.get("placeholder"))
                    for _, _, attrs in evidence))
    ctx.before("atspi", evidence)
    data = command(ctx, ["snapshot", "-i"], app).data
    candidates = [item for item in nodes(data.get("tree")) if item.get("role") == "textfield"]
    require(candidates, "search field omitted from product snapshot")
    if expected:
        selected = [item for item in candidates if item.get("placeholder") == expected]
        require(len(selected) == 1 and not selected[0].get("description"))
    else:
        require(all(not item.get("placeholder") for item in candidates))
    after = [(node.name, node.get_property(ACCESSIBLE, "Description"), node.attributes())
             for node in fields]
    require(after == evidence)
    ctx.after("atspi", after)
    ctx.restored("atspi", after)


@implementation("placeholder.settings_apps_search")
def settings_apps_search(ctx: LegContext) -> None:
    with managed(ctx, "Settings", ("applications",)) as app:
        placeholder_read(ctx, app, "Search apps")


@implementation("placeholder.files_search_none")
def files_search_none(ctx: LegContext) -> None:
    with managed(ctx, "Files", (str(ctx.tier.runtime_dir),)) as app:
        tree = root(ctx, app)
        switches = tree.find(role=62, name="Search Current Folder")
        require(len(switches) == 1)
        from runner.helper import activate, changed
        activate(switches[0])
        changed(lambda: "PRESSED" in switches[0].states, True)
        try:
            placeholder_read(ctx, app, None)
        finally:
            activate(switches[0])
            changed(lambda: "PRESSED" in switches[0].states, False)
