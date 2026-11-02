"""Probe: does AT-SPI `Component.GrabFocus` on a GTK 3 entry activate its inactive window on Hyprland?

Run only inside an Omarchy VM session (test/omarchy-vm run), never on a host:

    python3 -I probes/linux/hyprland-grabfocus-activation.py ~/out/grabfocus-probe

A headed `type @ref` on Wayland does not ask core to activate the window. It calls
`Component.GrabFocus` on the target and waits for the target's `focused` state. GTK 3 answers
`GrabFocus` with `gtk_widget_grab_focus` and `gtk_window_present`, and on Wayland the present is an
activation request the compositor may refuse. This probe measures what the live Hyprland does with
that request.

It starts a GTK 3 window with one entry (native Wayland) and a `foot` thief. Per round it focuses the
thief through the compositor, calls `GrabFocus` on the entry, and polls `j/activewindow` and the
entry's `focused` state for two seconds. A control round then focuses the GTK window through the
compositor's own focus dispatcher. It records the Hyprland version, the config dialect,
`misc:focus_on_activate` and the compositor log lines that mention activation. Results go to
`<out>/results.json` and a summary to stdout.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROUNDS = 4
POLL_S = 2.0
FIXTURE_TITLE = "AD GrabFocus Probe"
THIEF_TITLE = "ad-grabfocus-thief"


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


def entry_node(pid: int):
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
                return node
            stack.extend(node.get_child_at_index(i) for i in range(node.get_child_count()))
    return None


def focused(node) -> bool:
    import gi
    gi.require_version("Atspi", "2.0")
    from gi.repository import Atspi
    return node.get_state_set().contains(Atspi.StateType.FOCUSED)


def watch(target: str, node) -> dict:
    start = time.monotonic()
    first_active = first_focused = None
    while time.monotonic() - start < POLL_S:
        now = round((time.monotonic() - start) * 1000, 1)
        if first_active is None and active().get("address") == target:
            first_active = now
        if first_focused is None and focused(node):
            first_focused = now
        time.sleep(0.02)
    return {"target_active_ms": first_active, "entry_focused_ms": first_focused,
            "active_after": active().get("title")}


def log_lines() -> list[str]:
    signature = os.environ.get("HYPRLAND_INSTANCE_SIGNATURE", "")
    runtime = Path(os.environ.get("XDG_RUNTIME_DIR", "/run/user/1000"))
    path = runtime / "hypr" / signature / "hyprland.log"
    if not path.is_file():
        return [f"no log at {path}"]
    words = ("activat", "focus_on_activate", "steal")
    return [line for line in path.read_text(errors="replace").splitlines()
            if any(word in line.lower() for word in words)][-60:]


def run(out: Path) -> int:
    out.mkdir(parents=True, exist_ok=True)
    result = {"version": hyprctl("version").splitlines()[:1], "lua": lua(),
              "focus_on_activate": hyprctl("-j", "getoption", "misc:focus_on_activate").strip(),
              "rounds": [], "control": None}
    script = Path(__file__).resolve()
    proc = subprocess.Popen([sys.executable, "-I", str(script), "--fixture"],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    spawn(f"foot --title {THIEF_TITLE}")
    try:
        target = wait_for(lambda: client(FIXTURE_TITLE), 15.0)
        thief = wait_for(lambda: client(THIEF_TITLE), 15.0)
        node = wait_for(lambda: entry_node(proc.pid), 15.0)
        result["fixture"] = target and {k: target.get(k) for k in ("address", "xwayland", "pid")}
        if not (target and thief and node):
            result["error"] = f"setup: target {bool(target)} thief {bool(thief)} entry {bool(node)}"
            return finish(out, result)
        for _ in range(ROUNDS):
            focus(thief["address"])
            settled = wait_for(lambda: active().get("address") == thief["address"], 3.0)
            before = focused(node)
            started = time.monotonic()
            grabbed = node.grab_focus()
            row = {"thief_active_before": bool(settled), "entry_focused_before": before,
                   "grab_focus_returned": grabbed,
                   "grab_focus_ms": round((time.monotonic() - started) * 1000, 1)}
            row.update(watch(target["address"], node))
            result["rounds"].append(row)
        focus(thief["address"])
        wait_for(lambda: active().get("address") == thief["address"], 3.0)
        answer = focus(target["address"])
        result["control"] = {"dispatch_answer": answer, **watch(target["address"], node)}
    finally:
        proc.terminate()
        thief_now = client(THIEF_TITLE)
        if thief_now:
            os.kill(thief_now["pid"], 15)
    result["log"] = log_lines()
    return finish(out, result)


def finish(out: Path, result: dict) -> int:
    (out / "results.json").write_text(json.dumps(result, indent=2))
    print(json.dumps({k: v for k, v in result.items() if k != "log"}, indent=2))
    print(f"log lines: {len(result.get('log', []))}")
    return 0


if __name__ == "__main__":
    if sys.argv[1:] == ["--fixture"]:
        fixture()
    else:
        sys.exit(run(Path(sys.argv[1] if len(sys.argv) > 1 else "probe-out")))
