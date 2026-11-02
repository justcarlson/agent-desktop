"""Probe: what does a Hyprland window and its own app see after maximize, restore and an output move?

Run only inside an Omarchy VM session (test/omarchy-vm run), never on a host:

    python3 -I probes/linux/hyprland-window-restore.py ~/out/restore-probe

The probe starts a nested Hyprland from a Lua config inside the VM session, one output, and the
window-leg sizer (`tests/e2e-linux/apps/hyprland_window_sizer.py`) on it, floating and focused.
The sizer logs every size and toplevel state the compositor sends it, which is the app's own view.
Each read takes `j/clients` (place, size, `fullscreen`, `fullscreenClient`, monitor), `j/monitors`
and the sizer's last `state` and `configure` lines.

Cases, two rounds each:
- `state`: maximize with `fullscreen{mode="maximized",action="set"}`, restore with
  `fullscreen_state{internal=0,client=0,action="set"}`, the product's restore request.
- `mode`: the same maximize, restore with `fullscreen{mode="maximized",action="unset"}`.
- `float-moved`: the floating window, not maximized, while its output moves to (-1024, -640) at
  scale 1.25, then back.
- `state-moved` and `mode-moved`: maximize, move the output, restore with the case's request, read,
  then put the output back.

Results go to `<out>/results.jsonl` and a summary to stdout. The nested Hyprland and the sizer are
the probe's own children and are stopped by their pids.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tests" / "e2e-linux"))

from oracle import hyprland  # noqa: E402
from runner import second_instance  # noqa: E402

SIZER = ROOT / "tests" / "e2e-linux" / "apps" / "hyprland_window_sizer.py"
ROUNDS = 2
SETTLE = 1.5
MOVED = ("-1024x-640", 1.25)
CASES = ("state", "mode", "float-moved", "state-moved", "mode-moved", "record-moved")
TAGS = ("agent-desktop-restore", "+agent-desktop-max-x-50y-20w500h400", "agent-desktop-max:1,2,3,4",
        "-agent-desktop-max-x-50y-20w500h400", "-agent-desktop-restore")
LEGACY_MONITOR = ("WAYLAND-1,1280x800@60,-1024x-640,1.25", "WAYLAND-1,1280x800@60,-1024x-640,1",
                  "WAYLAND-1,preferred,-1024x-640,1.25")


def poll(read, seconds: float):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        found = read()
        if found:
            return found
        time.sleep(0.1)
    return None


class Nested:
    def __init__(self, out: Path, lua: bool = True):
        self.runtime = Path(os.environ["XDG_RUNTIME_DIR"])
        config = out / ("nested.lua" if lua else "legacy.conf")
        config.write_text(second_instance.config_text(lua))
        before = set(hyprland.instances(self.runtime))
        env = {key: value for key, value in os.environ.items() if key not in second_instance.DROPPED}
        self.process = subprocess.Popen(["Hyprland", "-c", str(config)], env=env,
                                        stdout=open(out / f"{config.stem}.log", "w"), stderr=subprocess.STDOUT)
        self.signature = poll(lambda: next(iter(set(hyprland.instances(self.runtime)) - before), None), 20.0)
        if not self.signature:
            raise RuntimeError("the nested Hyprland did not answer")
        self.wayland = poll(lambda: second_instance.wayland_of(self.runtime, self.process.pid), 20.0)
        self.lua = self.control(["eval", "return 1"]) == "ok"
        if not self.wayland or self.lua != lua:
            self.stop()
            raise RuntimeError(f"the nested Hyprland serves Wayland {self.wayland}, Lua {self.lua}")

    def control(self, argv: list[str]) -> str:
        return hyprland.control(self.runtime, self.signature, argv)

    def query(self, name: str):
        return hyprland.query(self.runtime, self.signature, name)

    def stop(self) -> None:
        self.process.terminate()
        self.process.wait(10)


def last(log: Path, kind: str) -> dict | None:
    found = [json.loads(line) for line in log.read_text().splitlines() if f'"ev": "{kind}"' in line]
    return found[-1] if found else None


def read(nested: Nested, address: str, log: Path) -> dict:
    time.sleep(SETTLE)
    client = next(item for item in nested.query("clients") if item["address"] == address)
    monitor = next(item for item in nested.query("monitors") if item["id"] == client["monitor"])
    return {"at": client["at"], "size": client["size"], "fullscreen": client["fullscreen"],
            "fullscreenClient": client.get("fullscreenClient"), "floating": client["floating"],
            "monitor": [monitor["name"], monitor["x"], monitor["y"], monitor["scale"]],
            "app_state": last(log, "state"), "app_size": last(log, "configure")}


def window(address: str) -> str:
    return f'window="address:{address}"'


def maximize(nested: Nested, address: str) -> str:
    return nested.control(["dispatch", f'hl.dsp.window.fullscreen({{mode="maximized",action="set",{window(address)}}})'])


def restore(nested: Nested, address: str, how: str) -> str:
    if how == "state":
        return nested.control(["dispatch", "hl.dsp.window.fullscreen_state("
                               f'{{internal=0,client=0,action="set",{window(address)}}})'])
    return nested.control(["dispatch", f'hl.dsp.window.fullscreen({{mode="maximized",action="unset",{window(address)}}})'])


def move_output(nested: Nested, name: str, position: str, scale: float) -> str:
    return nested.control(["eval", f'hl.monitor({{ output = "{name}", mode = "{second_instance.MODE}", '
                                   f'position = "{position}", scale = {scale:g} }})'])


def tag_syntax(nested: Nested, address: str) -> list[dict]:
    """Each tag request's reply and the window's tags after it, then clear_tags."""
    found = []
    for tag in TAGS:
        reply = nested.control(["dispatch", f'hl.dsp.window.tag({{tag="{tag}",{window(address)}}})'])
        found.append({"tag": tag, "reply": reply, "tags": tags_of(nested, address)})
    reply = nested.control(["dispatch", f'hl.dsp.window.clear_tags({{{window(address)}}})'])
    return found + [{"tag": "clear_tags", "reply": reply, "tags": tags_of(nested, address)}]


def tags_of(nested: Nested, address: str):
    time.sleep(0.3)
    return next(item for item in nested.query("clients") if item["address"] == address).get("tags")


def legacy_monitor(out: Path) -> dict:
    """The legacy-config nested instance's answer to each `keyword monitor` move, and its outputs after."""
    try:
        nested = Nested(out, lua=False)
    except RuntimeError as error:
        return {"started": str(error)}
    try:
        found = {"started": "legacy", "monitors_before": outputs(nested)}
        legacy_rules(nested, found)
        found["create_reply"] = nested.control(["output", "create", "headless", second_instance.OUTPUT])
        found["output_reply"] = nested.control(second_instance.output_command(False))
        time.sleep(SETTLE)
        found["two_outputs"] = outputs(nested)
        legacy_rules(nested, found, "two outputs ")
        return found
    finally:
        nested.stop()


def legacy_rules(nested: Nested, found: dict, prefix: str = "") -> None:
    """Each `keyword monitor` move of WAYLAND-1 and the outputs after it, then WAYLAND-1 put back."""
    for rule in LEGACY_MONITOR:
        reply = nested.control(["keyword", "monitor", rule])
        took = poll(lambda: next((item for item in outputs(nested) if item[0] == "WAYLAND-1"
                                  and item[1:3] == [-1024, -640]), None), 5.0)
        found[prefix + rule] = {"reply": reply, "took": took, "monitors": outputs(nested)}
        nested.control(["keyword", "monitor", "WAYLAND-1,1280x800@60,0x0,1"])
        time.sleep(SETTLE)


def outputs(nested: Nested) -> list:
    return [[item["name"], item["x"], item["y"], item["scale"], item["width"], item["height"]]
            for item in nested.query("monitors")]


def place(nested: Nested, address: str, at: list[int], size: list[int]) -> list[str]:
    return [nested.control(["dispatch", f'hl.dsp.window.resize({{x={size[0]},y={size[1]},{window(address)}}})']),
            nested.control(["dispatch", f'hl.dsp.window.move({{x={at[0]},y={at[1]},{window(address)}}})'])]


def record_moved(nested: Nested, address: str, log: Path, steps: dict, home: tuple) -> dict:
    """Maximize, move the output, restore, then put the window at its recorded geometry relative to
    its output; then a plain maximize and restore with no output change."""
    before, output = steps["before"], steps["before"]["monitor"]
    offset = [before["at"][0] - output[1], before["at"][1] - output[2]]
    steps["maximize_reply"] = maximize(nested, address)
    steps["move_reply"] = move_output(nested, output[0], *MOVED)
    steps["moved"] = read(nested, address, log)
    steps["restore_reply"] = restore(nested, address, "state")
    steps["restored"] = read(nested, address, log)
    moved = steps["restored"]["monitor"]
    steps["place_replies"] = place(nested, address, [moved[1] + offset[0], moved[2] + offset[1]], before["size"])
    steps["corrected"] = read(nested, address, log)
    move_output(nested, output[0], *home)
    steps["back"] = read(nested, address, log)
    maximize(nested, address)
    restore(nested, address, "state")
    steps["again"] = read(nested, address, log)
    return steps


def run_case(nested: Nested, address: str, log: Path, case: str) -> dict:
    steps = {"before": read(nested, address, log)}
    output = steps["before"]["monitor"]
    home = (f"{output[1]}x{output[2]}", float(output[3]))
    how = case.split("-")[0]
    if case == "record-moved":
        return record_moved(nested, address, log, steps, home)
    if case == "float-moved":
        steps["move_reply"] = move_output(nested, output[0], *MOVED)
        steps["moved"] = read(nested, address, log)
        move_output(nested, output[0], *home)
        steps["back"] = read(nested, address, log)
        return steps
    steps["maximize_reply"] = maximize(nested, address)
    steps["maximized"] = read(nested, address, log)
    if case.endswith("-moved"):
        steps["move_reply"] = move_output(nested, output[0], *MOVED)
        steps["moved"] = read(nested, address, log)
    steps["restore_reply"] = restore(nested, address, how)
    steps["restored"] = read(nested, address, log)
    if case.endswith("-moved"):
        move_output(nested, output[0], *home)
        steps["back"] = read(nested, address, log)
    if steps["restored"]["fullscreen"]:
        restore(nested, address, "state")
    return steps


def summary(case: str, steps: dict) -> str:
    end = steps.get("corrected") or steps.get("restored") or steps.get("moved")
    state = (end.get("app_state") or {})
    return (f"{case:12} before {steps['before']['at']} {steps['before']['size']} -> "
            f"{end['at']} {end['size']} fullscreen {end['fullscreen']}/{end['fullscreenClient']} "
            f"monitor {end['monitor'][1:]} app maximized {state.get('maximized')} "
            f"app size {[(end.get('app_size') or {}).get(k) for k in ('w', 'h')]}")


def main(out: Path) -> int:
    if not os.environ.get("OMARCHY_VM_CHANNEL"):
        raise SystemExit("hyprland-window-restore: run this inside an Omarchy VM, never on a host")
    out.mkdir(parents=True, exist_ok=True)
    nested = Nested(out)
    sizer = None
    try:
        log = out / "sizer.events"
        log.write_text("")
        env = dict(os.environ, WAYLAND_DISPLAY=nested.wayland, GDK_BACKEND="wayland")
        for key in second_instance.DROPPED:
            env.pop(key, None)
        sizer = subprocess.Popen([sys.executable, "-I", str(SIZER), str(log)], env=env,
                                 stdout=subprocess.DEVNULL, stderr=open(out / "sizer.log", "w"))
        client = poll(lambda: next((item for item in nested.query("clients")
                                    if item.get("pid") == sizer.pid and item.get("mapped")), None), 15.0)
        if not client:
            raise RuntimeError("the sizer never mapped")
        address = client["address"]
        nested.control(["dispatch", f'hl.dsp.window.float({{action="enable",{window(address)}}})'])
        nested.control(["dispatch", f'hl.dsp.focus({{{window(address)}}})'])
        with open(out / "results.jsonl", "a") as results:
            found = tag_syntax(nested, address)
            results.write(json.dumps({"case": "tag-syntax", "steps": found}) + "\n")
            for item in found:
                print(f"tag {item['tag']!r}: reply {item['reply']!r} tags {item['tags']!r}", flush=True)
            for round_index in range(ROUNDS):
                for case in CASES:
                    steps = run_case(nested, address, log, case)
                    results.write(json.dumps({"round": round_index, "case": case, "steps": steps}) + "\n")
                    print(f"r{round_index} {summary(case, steps)}", flush=True)
    finally:
        if sizer:
            sizer.terminate()
            sizer.wait(10)
        nested.stop()
    found = legacy_monitor(out)
    with open(out / "results.jsonl", "a") as results:
        results.write(json.dumps({"case": "legacy-monitor", "steps": found}) + "\n")
    print(f"legacy monitor: {json.dumps(found)}", flush=True)
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("usage: hyprland-window-restore.py <out-dir>")
    raise SystemExit(main(Path(sys.argv[1])))
