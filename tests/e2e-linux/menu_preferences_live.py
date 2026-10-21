"""Run through private-session.sh, passing the agent-desktop binary as argv[1]."""

import json
import os
from pathlib import Path
import subprocess
import sys
from types import SimpleNamespace

from gi.repository import Gio

from oracle.atspi_raw import app_root, call, poll


def objects(value):
    if isinstance(value, dict):
        yield value
        for child in value.values():
            yield from objects(child)
    elif isinstance(value, list):
        for child in value:
            yield from objects(child)


def main():
    runtime = Path(os.environ["AGENT_DESKTOP_PRIVATE_SESSION"])
    assert runtime == Path(os.environ["XDG_RUNTIME_DIR"])
    assert os.environ["DISPLAY"] not in (":0", ":1", ":99")
    binary = str(Path(sys.argv[1]).resolve())

    def product(*args):
        result = subprocess.run([binary, *args], capture_output=True, text=True, timeout=20)
        envelope = json.loads(result.stdout)
        print(json.dumps({"args": args, "result": envelope}), flush=True)
        assert envelope["ok"], envelope
        return envelope

    def unique_ref(envelope, name, role=None):
        refs = [node["ref_id"] for node in objects(envelope)
                if node.get("name") == name and "ref_id" in node
                and (role is None or node.get("role") == role)]
        assert len(refs) == 1, refs
        return refs[0]

    app = subprocess.Popen(["gnome-calculator"], stdout=subprocess.DEVNULL,
                           stderr=subprocess.DEVNULL)
    try:
        address = call(Gio.bus_get_sync(Gio.BusType.SESSION, None), "org.a11y.Bus",
                       "/org/a11y/bus", "org.a11y.Bus", "GetAddress")[0]
        tier = SimpleNamespace(a11y_bus_address=address, runtime_dir=runtime)

        def registered():
            try:
                return app_root(tier, "gnome-calculator")
            except LookupError:
                return None

        root = poll(registered, 10)
        assert root is not None, "Calculator did not register"
        assert poll(lambda: root.find(name="Main Menu"), 10), "Main Menu did not appear"
        product("focus-window", "--app", "gnome-calculator")
        opener = unique_ref(product("snapshot", "--app", "gnome-calculator", "-i"),
                            "Main Menu", "switch")
        product("click", opener)
        product("wait", "--menu", "--app", "gnome-calculator", "--timeout", "5000")
        menu = product("snapshot", "--surface", "menu", "--app", "gnome-calculator", "-i")
        preferences = unique_ref(menu, "Preferences")
        found = product("find", "--surface", "menu", "--app", "gnome-calculator",
                        "--name", "Preferences")
        found_ref = unique_ref(found, "Preferences")
        if "--from-find" in sys.argv[2:]:
            preferences = found_ref
        raw_items = [node for node in root.walk() if node.role == 35]
        print(json.dumps({"raw_menu_items": [
            {"path": node.path, "name": node.name, "attributes": node.attributes(),
             "interfaces": node.interfaces()} for node in raw_items]}), flush=True)

        def dialogs():
            return [node for node in root.walk() if node.name == "Preferences"
                    and node.role in (16, 23) and "SHOWING" in node.states]

        assert not dialogs(), "Preferences was already open"
        product("click", preferences)
        appeared = poll(dialogs, 8)
        assert len(appeared) == 1, "independent AT-SPI read found no unique Preferences dialog"
        print(json.dumps({"oracle": "AT-SPI", "preferences_dialog": appeared[0].path,
                          "role": appeared[0].role, "name": appeared[0].name}), flush=True)
    finally:
        app.terminate()
        try:
            app.wait(timeout=5)
        except subprocess.TimeoutExpired:
            app.kill()
            app.wait(timeout=5)


if __name__ == "__main__":
    main()
