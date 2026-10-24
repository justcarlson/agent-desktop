"""Snapshot promises checked against widget events and independent AT-SPI reads."""

import contextlib
import json
from uuid import uuid4

from oracle.atspi_raw import poll
from runner.helper import managed, nodes, one, require, root, snapshot


ACTIONS = {"Click": "click", "Toggle": "toggle", "Check": "check",
           "Uncheck": "uncheck", "TypeText": "type", "SetValue": "set-value",
           "Select": "select", "Expand": "expand", "Collapse": "collapse"}
BLOCKED = {"hidden", "offscreen", "disabled"}
OPENERS = {"zoo-open-dialog": "Zoo Dialog", "zoo-open-chooser": "Zoo Save",
           "zoo-open-message": "Zoo Message",
           "zoo-open-toplevel-button": "Zoo Top Button"}


def expected_outcomes(node):
    """Only the snapshot decides which commands are promised and may refuse."""
    actions = [ACTIONS[a] for a in node.get("available_actions", ()) if a in ACTIONS]
    states = set(node.get("states", ()))
    expected = "land"
    if states & BLOCKED:
        actions = actions[:1]
        expected = ("land-or-refuse" if node.get("name") == "zoo-offscreen"
                    and states & BLOCKED == {"offscreen"} else "refuse")
    return [(action, expected) for action in actions]


def row_verdict(expected, *, refused, log_effect, raw_effect, log_unchanged, raw_unchanged):
    landed = log_effect and raw_effect
    clean_refusal = refused and log_unchanged and raw_unchanged
    return "pass" if ((expected == "land" and landed)
                      or (expected == "refuse" and clean_refusal)
                      or (expected == "land-or-refuse" and (landed or clean_refusal))) else "fail"


def events(path):
    return [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []


def since_reset(path):
    records = events(path)
    start = max((i + 1 for i, e in enumerate(records)
                 if e.get("widget") == "zoo" and e.get("event") == "reset"), default=0)
    return records[start:]


def raw_state(tree):
    # Paths identify duplicates; read interfaces directly, without the product.
    return tuple(sorted((node.path, node.name, node.role, tuple(sorted(node.states)),
                         node.text(), node.value()) for node in tree.walk()))


def reset(path):
    before = len(events(path))
    path.with_name(path.name + ".reset").touch()
    require(poll(lambda: any(e.get("widget") == "zoo" and e.get("event") == "reset"
                             for e in events(path)[before:]), 4), "fixture reset not acknowledged")


def ref_nodes(data):
    return [node for node in nodes(data.get("tree", data)) if node.get("ref_id")]


def named(data, name):
    matches = [node for node in ref_nodes(data) if node.get("name") == name]
    require(len(matches) == 1, f"expected one snapshot ref for {name}: {matches}")
    return matches[0]


def fresh(ctx, app, original, occurrence=0):
    matches = [node for node in ref_nodes(snapshot(ctx, app))
               if (node.get("name"), node.get("role")) ==
               (original.get("name"), original.get("role"))]
    require(len(matches) > occurrence, f"ref disappeared after reset: {original}")
    return matches[occurrence]


def act(ctx, node, action, value=None):
    argv = [action, node["ref_id"]]
    if value is not None:
        argv.append(str(value))
    return ctx.product(argv, headed=False)


@contextlib.contextmanager
def zoo(ctx):
    # Supply the log argument explicitly instead of depending on a launch default.
    toolkit = ctx.leg.id.split("zoo_", 1)[1].removesuffix("_windows")
    label = {"gtk3": "GTK3", "gtk4": "GTK4", "qt": "Qt"}[toolkit]
    path = ctx.tier.runtime_dir / f"ref-contract-{toolkit}-{uuid4().hex}.log"
    with managed(ctx, "Zoo " + label, (str(path),)) as app:
        reset(path)
        yield app, path


def matching_event(records, widget, event, value=None, *, any_value=False):
    return any(e.get("widget") == widget and e.get("event") == event
               and (any_value or e.get("value") == value) for e in records)


def selected(tree, name):
    target = one(tree, name=name, role=37 if name.startswith("zoo-tab-") else None)
    return bool(target.states & {"SELECTED", "CHECKED"})


def effect(tree, node, action, value, before, records):
    """Return separate log/raw evidence; never consume an action response."""
    name = node.get("name")
    target = one(tree, name=name, role=37 if name and name.startswith("zoo-tab-") else None)
    states = target.states
    if action == "type" or (action == "set-value" and target.value() is None):
        if name == "zoo-password":
            length = target.get_property("org.a11y.atspi.Text", "CharacterCount")
            return (matching_event(records, name, "changed", len(value)), length == len(value))
        return matching_event(records, name, "changed", value), target.text() == value
    if action == "set-value":
        return matching_event(records, name, "changed", float(value)), target.value() == float(value)
    if name in ("zoo-toggle", "zoo-check", "zoo-switch", "zoo-expander"):
        bit = "EXPANDED" if "EXPANDABLE" in states else "CHECKED"
        want = (action in ("check", "expand") if action in ("check", "uncheck", "expand", "collapse")
                else bit not in dict((r[0], r[3]) for r in before)[target.path])
        event = "expanded" if name == "zoo-expander" else "toggled"
        return matching_event(records, name, event, want), (bit in states) == want
    if name == "zoo-combo":
        texts = [n.text() for n in target.walk()]
        names = [n.name for n in target.walk() if n.states & {"SELECTED", "CHECKED"}]
        return matching_event(records, name, "changed", value), value in texts or value in names
    if action == "select" and value and value.startswith(("zoo-row-", "zoo-tab-", "zoo-radio-")):
        event = "row" if value.startswith("zoo-row-") else "page" if value.startswith("zoo-tab-") else "selected"
        return matching_event(records, value, event, value), selected(tree, value)
    if name and name.startswith(("zoo-radio-", "zoo-tab-", "zoo-row-")):
        event = "selected" if name.startswith("zoo-radio-") else "page" if name.startswith("zoo-tab-") else "row"
        return matching_event(records, name, event, name), selected(tree, name)
    if name == "zoo-menubutton":
        showing = [n for n in tree.walk() if n.name == "zoo-menu-item" and "SHOWING" in n.states]
        return not records, bool(showing) == (action != "collapse")
    if name in OPENERS:
        # Openers have no event in the fixture contract; their window is the effect.
        return not records, raw_window(tree, OPENERS[name]) is not None
    return (matching_event(records, name, "activate", any_value=True),
            "ENABLED" in states and "SHOWING" in states)


def prepare(ctx, app, tree, node, action):
    """Make idempotent actions observable, then remint the target ref."""
    name = node.get("name")
    if set(node.get("states", ())) & BLOCKED:
        return node
    opposite = {"check": "uncheck", "uncheck": "check", "expand": "collapse", "collapse": "expand"}
    if action in opposite:
        act(ctx, node, opposite[action])
        if name == "zoo-menubutton":
            require(poll(lambda: any(n.name == "zoo-menu-item" and "SHOWING" in n.states
                                     for n in tree.walk()) == (action == "collapse"), 3))
            return named(snapshot(ctx, app), name)
        bit = "EXPANDED" if "EXPANDABLE" in one(tree, name=name).states else "CHECKED"
        require(poll(lambda: (bit in one(tree, name=name).states) == (action in ("uncheck", "collapse")), 3),
                f"could not prepare {action} on {name}")
        return named(snapshot(ctx, app), name)
    if name in ("zoo-radio-a", "zoo-radio-b", "zoo-tab-a", "zoo-tab-b",
                "zoo-row-1", "zoo-row-2", "zoo-row-3") and selected(tree, name):
        if name.startswith("zoo-row-"):
            other = "zoo-row-2" if name == "zoo-row-1" else "zoo-row-1"
        else:
            other = name[:-1] + ("b" if name.endswith("a") else "a")
        act(ctx, named(snapshot(ctx, app), other), "click")
        require(poll(lambda: selected(tree, other) and not selected(tree, name), 3))
        return named(snapshot(ctx, app), name)
    return node


def print_rows(ctx, rows):
    columns = ("name", "role", "states", "action", "expected", "observed", "verdict")
    table = " | ".join(columns) + "\n" + "\n".join(
        " | ".join(str(row.get(key, "")) for key in columns) for row in rows)
    print(table, flush=True)
    ctx.finding(table)


def refs(ctx):
    rows = []
    try:
        with zoo(ctx) as (app, path):
            tree = root(ctx, app)
            initial = ref_nodes(snapshot(ctx, app))
            require(initial, "fixture snapshot has no refs")
            occurrences = {}
            for original in initial:
                key = original.get("name"), original.get("role")
                occurrence = occurrences.get(key, 0)
                occurrences[key] = occurrence + 1
                row = dict(name=key[0], role=key[1], states=original.get("states", []),
                           action=[], expected=[], observed=[], verdict="pass")
                rows.append(row)
                outcomes = expected_outcomes(original)
                if not outcomes:
                    if key[0] == "zoo-link" and ctx.leg.id == "ref_contract.zoo_qt":
                        row.update(expected="skip", observed="Qt link has no advertised primary action")
                    else:
                        row.update(verdict="fail", observed="no observable primary action advertised")
                    continue
                for action, _ in outcomes:
                    channel = f"ref{len(rows)}.{action}"
                    try:
                        baseline = raw_state(tree)
                        ctx.before("atspi." + channel, baseline)
                        ctx.before("applog." + channel, since_reset(path))
                        node = prepare(ctx, app, tree, fresh(ctx, app, original, occurrence), action)
                        expected = dict(expected_outcomes(node)).get(action)
                        require(expected is not None, f"{action} disappeared from refreshed snapshot")
                        row["action"].append(action)
                        row["expected"].append(expected)
                        value = (f"marker-{uuid4().hex}" if action == "type" or
                                 (action == "set-value" and node.get("name") in
                                  ("zoo-entry", "zoo-password", "zoo-text")) else
                                 "7" if action == "set-value" else
                                 ("Charlie" if node.get("value") == "Bravo" else "Bravo")
                                 if action == "select" else None)
                        if action == "select" and node.get("name") != "zoo-combo":
                            children = [n for n in one(tree, name=node.get("name")).walk()
                                        if (n.name.startswith(("zoo-row-", "zoo-radio-")) or
                                            (n.name.startswith("zoo-tab-") and n.role == 37))
                                        and not n.states & {"SELECTED", "CHECKED"}]
                            require(children, "Select ref has no different contract choice")
                            value = children[0].name
                        before, log_before = raw_state(tree), events(path)
                        response = act(ctx, node, action, value)

                        def landed():
                            return all(effect(tree, node, action, value, before,
                                              events(path)[len(log_before):]))

                        poll(landed, 2, floor_s=0.25)
                        after, log_after = raw_state(tree), events(path)
                        log_effect, raw_effect = effect(tree, node, action, value, before,
                                                        log_after[len(log_before):])
                        if node.get("name") == "zoo-menubutton" and action != "collapse" and raw_effect:
                            menu = ctx.product(["snapshot", "-i", "--app", app.name, "--surface", "menu"], headed=False).data
                            act(ctx, named(menu, "zoo-menu-item"), "click")
                            log_effect = bool(poll(lambda: matching_event(events(path)[len(log_before):],
                                                                        "zoo-menu-item", "activate", any_value=True), 3))
                            log_after = events(path)
                        refused = (response.exit_code == 1 and bool(response.error)
                                   and not response.error.get("code", "").startswith("HARNESS_"))
                        verdict = row_verdict(expected, refused=refused, log_effect=log_effect,
                                              raw_effect=raw_effect, log_unchanged=log_after == log_before,
                                              raw_unchanged=after == before)
                        row["observed"].append(dict(action=action, refused=refused,
                                                    log=log_effect, raw=raw_effect))
                        if verdict == "fail":
                            row["verdict"] = "fail"
                        ctx.after("atspi." + channel, after)
                        ctx.after("applog." + channel, since_reset(path))
                    except Exception as error:
                        row["verdict"] = "fail"
                        row["observed"].append(f"{action}: {error}")
                    finally:
                        reset(path)
                        snapshot(ctx, app)
                    if "atspi." + channel in ctx.reads:
                        restored = raw_state(tree)
                        ctx.restored("atspi." + channel, restored)
                        ctx.restored("applog." + channel, since_reset(path))
                        if restored != baseline or since_reset(path):
                            row["verdict"] = "fail"
                            row["observed"].append("reset did not restore the baseline")
    finally:
        print_rows(ctx, rows)
    require(all(row["verdict"] == "pass" for row in rows), "ref-contract table has failures")


def window_state(tree):
    return tuple(sorted((n.name, n.role, "MODAL" in n.states)
                        for n in tree.children() if "SHOWING" in n.states))


def raw_window(tree, title):
    # A top-level widget can expose its accessible name instead of its WM title.
    names = (title, "zoo-top-button") if title == "Zoo Top Button" else (title,)
    matches = [n for n in tree.children() if n.name in names and "SHOWING" in n.states]
    require(len(matches) <= 1, f"multiple raw windows for {title}")
    return matches[0] if matches else None


def surface_snapshot(ctx, app, title, modal):
    argv = ["snapshot", "-i", "--app", app.name]
    if modal:
        argv += ["--surface", "sheet"]
    else:
        from runner.apps import window_id
        argv += ["--window-id", window_id(raw_window(root(ctx, app), title))]
    response = ctx.product(argv, headed=False)
    require(response.data and response.data.get("complete") is True,
            f"incomplete {title} surface: {response.raw}")
    data = response.data
    names = (title, "zoo-top-button") if title == "Zoo Top Button" else (title,)
    require(any(n.get("name") in names for n in nodes(data.get("tree", data))),
            f"snapshot did not reach {title}")
    return data


def window_listed(ctx, app, title):
    response = ctx.product(["list-windows", "--app", app.name], headed=False)
    return any(n.get("title") == title for n in nodes(response.data))


def dialog(ctx, app, tree, path, data):
    marker = "dialog-" + uuid4().hex
    start = len(events(path))
    act(ctx, named(data, "zoo-dialog-entry"), "type", marker)
    require(poll(lambda: matching_event(events(path)[start:], "zoo-dialog-entry", "changed", marker)
                 and one(tree, name="zoo-dialog-entry").text() == marker, 3),
            "dialog entry did not receive text in both oracles")
    data = surface_snapshot(ctx, app, "Zoo Dialog", True)
    act(ctx, named(data, "zoo-dialog-ok"), "click")
    require(poll(lambda: matching_event(events(path)[start:], "zoo-dialog-ok", "activate", any_value=True)
                 and not any(n.name == "Zoo Dialog" and "SHOWING" in n.states
                             for n in tree.children()), 3), "dialog OK did not activate and close")


def chooser(ctx, app, tree, path, data, saved):
    # Native chooser labels differ; the snapshot must expose exactly one name field.
    fields = [n for n in ref_nodes(data) if "TypeText" in n.get("available_actions", ())
              and n.get("name", "").strip().casefold().rstrip(":") in
              ("name", "file name", "filename")]
    require(len(fields) == 1, f"expected one save-name field: {fields}")
    act(ctx, fields[0], "type", str(saved))
    require(poll(lambda: one(tree, name=fields[0].get("name")).text() == str(saved), 3),
            "chooser name field did not receive the runtime path")
    data = surface_snapshot(ctx, app, "Zoo Save", True)
    buttons = [n for n in ref_nodes(data) if n.get("name", "").replace("_", "").casefold() == "save"
               and "Click" in n.get("available_actions", ())]
    require(len(buttons) == 1, f"expected one Save button: {buttons}")
    start = len(events(path))
    act(ctx, buttons[0], "click")
    require(poll(lambda: matching_event(events(path)[start:], "zoo-open-chooser", "saved", str(saved))
                 and saved.is_file(), 4), "save produced neither the contracted event nor file")


def windows(ctx):
    rows = []
    try:
        with zoo(ctx) as (app, path):
            tree = root(ctx, app)
            openers = list(OPENERS) if ctx.leg.id.startswith("ref_contract.zoo_qt") else list(OPENERS)[:2]
            for index, opener in enumerate(openers):
                title = OPENERS[opener]
                row = dict(name=opener, role="button", states=[], action="click/window workflow",
                           expected="land", observed="", verdict="fail")
                rows.append(row)
                channel = f"window{index}"
                saved = ctx.tier.runtime_dir / f"zoo-save-{uuid4().hex}.txt"
                ctx.before("atspi." + channel, window_state(tree))
                ctx.before("applog." + channel, since_reset(path))
                try:
                    node = named(snapshot(ctx, app), opener)
                    row.update(role=node.get("role"), states=node.get("states", []))
                    require(raw_window(tree, title) is None, f"{title} already open before click")
                    act(ctx, node, "click")
                    require(poll(lambda: raw_window(tree, title), 4), f"no raw {title} window")
                    require(poll(lambda: window_listed(ctx, app, title), 3), f"list-windows missed {title}")
                    modal = opener != "zoo-open-toplevel-button"
                    if modal:
                        require("MODAL" in raw_window(tree, title).states, f"{title} is not raw MODAL")
                    data = surface_snapshot(ctx, app, title, modal)
                    if opener == "zoo-open-dialog":
                        dialog(ctx, app, tree, path, data)
                    elif opener == "zoo-open-chooser":
                        chooser(ctx, app, tree, path, data, saved)
                    else:
                        name = "OK" if opener == "zoo-open-message" else "zoo-top-button"
                        start = len(events(path))
                        act(ctx, named(data, name), "click")
                        if name == "OK":
                            require(poll(lambda: not any(n.name == title and "SHOWING" in n.states
                                                         for n in tree.children()), 3))
                        else:
                            require(poll(lambda: matching_event(events(path)[start:], name, "activate", any_value=True)
                                         and "SHOWING" in one(tree, name=name).states, 3))
                    row.update(observed=f"raw {title}, listed title, surface and effects verified", verdict="pass")
                except Exception as error:
                    row["observed"] = str(error)
                finally:
                    ctx.after("atspi." + channel, window_state(tree))
                    ctx.after("applog." + channel, since_reset(path))
                    reset(path)
                    snapshot(ctx, app)
                    restored = window_state(tree)
                    ctx.restored("atspi." + channel, restored)
                    ctx.restored("applog." + channel, since_reset(path))
                    before = ctx.reads["atspi." + channel]["before"][0]
                    if restored != before or since_reset(path):
                        row.update(verdict="fail", observed="reset did not restore the window baseline")
    finally:
        print_rows(ctx, rows)
    require(all(row["verdict"] == "pass" for row in rows), "window-contract table has failures")
