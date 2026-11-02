"""Probe: does a headed `type @ref` take focus back after a thief steals it on Hyprland?

Run only inside an Omarchy VM session (test/omarchy-vm run), never on a host:

    python3 -I probes/linux/hyprland-theft-reclaim.py ~/out/theft-probe [agent-desktop binary]

It repeats the steal of `input.hyprland_focus_theft_before_first_key` with the product binary. A
GTK 3 window with a button and an entry (native Wayland) and a `foot` thief are started. Per round
the thief takes focus, the product runs `--headed type <entry ref> x`, and the probe gives focus back
to the thief as soon as `j/activewindow` names the GTK window, as the leg does. It records every
`activewindowv2` and `urgent` event from the instance's event socket with its time since the product
started, the product's answer and the entry text. Rounds run with `misc:focus_on_activate` as
configured, then with it false; the option is set back after. Results go to `<out>/results.json`.
"""

from __future__ import annotations

import functools
import json
import os
import socket
import subprocess
import sys
import threading
import time
from pathlib import Path

ROUNDS = 4
FIXTURE_TITLE = "AD Theft Probe"
THIEF_TITLE = "ad-theft-thief"
SETTLE_S = 0.7


def fixture() -> None:
    os.environ["GDK_BACKEND"] = "wayland"
    os.environ["GTK_USE_PORTAL"] = "0"
    import gi
    gi.require_version("Gtk", "3.0")
    from gi.repository import Gtk
    window = Gtk.Window(title=FIXTURE_TITLE)
    box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
    box.add(Gtk.Button(label="probe-button"))
    entry = Gtk.Entry()
    entry.get_accessible().set_name("probe-entry")
    box.add(entry)
    window.add(box)
    window.set_default_size(400, 200)
    window.connect("destroy", Gtk.main_quit)
    window.show_all()
    Gtk.main()


def hyprctl(*args: str) -> str:
    return subprocess.run(["hyprctl", *args], capture_output=True, text=True, timeout=5).stdout


@functools.cache
def lua() -> bool:
    return hyprctl("eval", "return 1").strip() == "ok"


def focus(address: str) -> str:
    if lua():
        return hyprctl("dispatch", f'hl.dsp.focus({{ window = "address:{address}" }})').strip()
    return hyprctl("dispatch", "focuswindow", f"address:{address}").strip()


def spawn(argv: str) -> str:
    if lua():
        return hyprctl("dispatch", f'hl.dsp.exec_cmd("{argv}")').strip()
    return hyprctl("dispatch", "exec", argv).strip()


def option() -> str:
    return hyprctl("-j", "getoption", "misc:focus_on_activate").strip()


def set_option(on: bool) -> str:
    word = "true" if on else "false"
    if lua():
        return hyprctl("eval", f"hl.config({{ misc = {{ focus_on_activate = {word} }} }})").strip()
    return hyprctl("keyword", "misc:focus_on_activate", word).strip()


def option_on(text: str) -> bool:
    found = json.loads(text or "{}")
    value = next((found[key] for key in ("bool", "int") if key in found), None)
    return value is True or value == 1


def active() -> dict:
    try:
        return json.loads(hyprctl("-j", "activewindow") or "{}")
    except json.JSONDecodeError:
        return {}


def client(title: str) -> dict | None:
    for item in json.loads(hyprctl("-j", "clients") or "[]"):
        if item.get("title") == title or item.get("initialTitle") == title:
            return item
    return None


def wait_for(read, timeout: float):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        value = read()
        if value:
            return value
        time.sleep(0.05)
    return None


class Events:
    """Every line from the instance's event socket, stamped with time.monotonic()."""

    def __init__(self) -> None:
        signature = os.environ["HYPRLAND_INSTANCE_SIGNATURE"]
        runtime = Path(os.environ.get("XDG_RUNTIME_DIR", "/run/user/1000"))
        self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.sock.connect(str(runtime / "hypr" / signature / ".socket2.sock"))
        self.lines: list[tuple[float, str]] = []
        self.lock = threading.Lock()
        threading.Thread(target=self._read, daemon=True).start()

    def _read(self) -> None:
        buffer = b""
        while True:
            chunk = self.sock.recv(65536)
            if not chunk:
                return
            buffer += chunk
            *whole, buffer = buffer.split(b"\n")
            now = time.monotonic()
            with self.lock:
                self.lines.extend((now, line.decode(errors="replace")) for line in whole)

    def since(self, start: float, names: dict[str, str]) -> list[str]:
        with self.lock:
            found = [(at, line) for at, line in self.lines if at >= start]
        rows = []
        for at, line in found:
            kind, _, value = line.partition(">>")
            if kind not in ("activewindowv2", "urgent"):
                continue
            rows.append(f"{round((at - start) * 1000, 1)}ms {kind} {names.get(value.lower(), value)}")
        return rows


def bare(address: str) -> str:
    return address.lower().removeprefix("0x")


def product(binary: str, *argv: str) -> dict:
    result = subprocess.run([binary, *argv], capture_output=True, text=True, timeout=30)
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        return {"unparsed": result.stdout[-400:], "stderr": result.stderr[-400:]}


def find(tree, match) -> list:
    found = []
    if isinstance(tree, dict):
        if match(tree):
            found.append(tree)
        for value in tree.values():
            found.extend(find(value, match))
    elif isinstance(tree, list):
        for value in tree:
            found.extend(find(value, match))
    return found


def entry_ref(binary: str, pid: int) -> str | None:
    listed = product(binary, "list-windows")
    windows = find(listed.get("data"), lambda item: item.get("pid") == pid and "id" in item)
    if not windows:
        return None
    snapshot = product(binary, "snapshot", "-i", "--window-id", windows[0]["id"])
    refs = find(snapshot.get("data"), lambda item: item.get("name") == "probe-entry" and "ref_id" in item)
    return refs[0]["ref_id"] if refs else None


def entry_text(pid: int) -> str | None:
    import gi
    gi.require_version("Atspi", "2.0")
    from gi.repository import Atspi
    desktop = Atspi.get_desktop(0)
    for index in range(desktop.get_child_count()):
        app = desktop.get_child_at_index(index)
        if app is None or app.get_process_id() != pid:
            continue
        stack = [app]
        while stack:
            node = stack.pop()
            if node.get_name() == "probe-entry":
                return Atspi.Text.get_text(node, 0, -1)
            stack.extend(node.get_child_at_index(i) for i in range(node.get_child_count()))
    return None


def summary(envelope: dict) -> dict:
    error = envelope.get("error") or {}
    return {"ok": envelope.get("ok"), "code": error.get("code"),
            "delivery": ((error.get("disposition") or envelope.get("data", {}).get("disposition") or {})
                         .get("delivery")),
            "check": (error.get("details") or {}).get("check"),
            "started": (error.get("details") or {}).get("physical_delivery_started"),
            "message": error.get("message")}


def one_round(binary: str, ref: str, target: dict, thief: dict, events: Events, pid: int) -> dict:
    names = {bare(target["address"]): "target", bare(thief["address"]): "thief"}
    focus(thief["address"])
    settled = wait_for(lambda: active().get("address") == thief["address"], 3.0)
    before = entry_text(pid)
    start = time.monotonic()
    running = subprocess.Popen([binary, "--headed", "type", ref, "x", "--timeout-ms", "5000"],
                               stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    stole_at = None
    while time.monotonic() - start < 10.0 and running.poll() is None:
        if active().get("address") == target["address"]:
            focus(thief["address"])
            stole_at = round((time.monotonic() - start) * 1000, 1)
            break
        time.sleep(0.001)
    out, _ = running.communicate(timeout=20)
    ended_at = round((time.monotonic() - start) * 1000, 1)
    time.sleep(SETTLE_S)
    try:
        envelope = json.loads(out)
    except json.JSONDecodeError:
        envelope = {"unparsed": out[-400:]}
    return {"thief_active_before": bool(settled), "steal_sent_ms": stole_at, "product_ended_ms": ended_at,
            "product": summary(envelope), "active_after": names.get(bare(active().get("address", "")), "other"),
            "entry_before": before, "entry_after": entry_text(pid), "events": events.since(start, names)}


def run(out: Path, binary: str) -> int:
    out.mkdir(parents=True, exist_ok=True)
    original = option()
    result = {"version": hyprctl("version").splitlines()[:1], "lua": lua(), "focus_on_activate": original,
              "binary": binary, "configured": [], "focus_on_activate_off": []}
    script = Path(__file__).resolve()
    proc = subprocess.Popen([sys.executable, "-I", str(script), "--fixture"],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    spawn(f"foot --title {THIEF_TITLE}")
    events = Events()
    try:
        target = wait_for(lambda: client(FIXTURE_TITLE), 15.0)
        thief = wait_for(lambda: client(THIEF_TITLE), 15.0)
        ref = wait_for(lambda: entry_ref(binary, proc.pid), 15.0)
        result["setup"] = {"target": bool(target), "thief": bool(thief), "ref": ref}
        if not (target and thief and ref):
            return finish(out, result)
        for _ in range(ROUNDS):
            result["configured"].append(one_round(binary, ref, target, thief, events, proc.pid))
        result["set_off_answer"] = set_option(False)
        result["option_while_off"] = option()
        for _ in range(ROUNDS):
            result["focus_on_activate_off"].append(one_round(binary, ref, target, thief, events, proc.pid))
    finally:
        result["restore_answer"] = set_option(option_on(original))
        result["option_after"] = option()
        proc.terminate()
        thief_now = client(THIEF_TITLE)
        if thief_now:
            os.kill(thief_now["pid"], 15)
    return finish(out, result)


def finish(out: Path, result: dict) -> int:
    (out / "results.json").write_text(json.dumps(result, indent=2))
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    if sys.argv[1:] == ["--fixture"]:
        fixture()
    else:
        default = str(Path.home() / "src" / "agent-desktop-bin" / "agent-desktop")
        sys.exit(run(Path(sys.argv[1] if len(sys.argv) > 1 else "probe-out"),
                     sys.argv[2] if len(sys.argv) > 2 else default))
