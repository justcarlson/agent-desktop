from __future__ import annotations

import json
import os
import socket
import sys
import tempfile
import threading
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))

from oracle import hyprland

ANSWERS = {
    b"j/activewindow": {"address": "0x1", "pid": 7, "class": "foot", "title": "user@host:~",
                        "initialTitle": "bystander", "size": [1, 1]},
    b"j/cursorpos": {"x": 640, "y": 400},
    b"j/activeworkspace": {"name": "1", "id": 1},
    b"j/monitors": [{"name": "Virtual-1", "focused": True, "activeWorkspace": {"name": "1"},
                     "specialWorkspace": {"name": ""}}],
}


class FakeInstance:
    def __init__(self, runtime: Path, signature: str):
        self.path = hyprland.socket_path(runtime, signature)
        self.path.parent.mkdir(parents=True)
        self.server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.server.bind(str(self.path))
        self.server.listen(8)
        self.requests = []
        self.thread = threading.Thread(target=self.serve, daemon=True)
        self.thread.start()

    def serve(self):
        while True:
            try:
                connection, _ = self.server.accept()
            except OSError:
                return
            with connection:
                data = connection.recv(4096)
                if not data:
                    continue
                self.requests.append(data)
                try:
                    connection.sendall(json.dumps(ANSWERS.get(data, {})).encode())
                except BrokenPipeError:
                    continue

    def close(self):
        self.server.close()


class HyprlandOracleTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.runtime = Path(directory.name)
        self.fake = FakeInstance(self.runtime, "sig_1")
        self.addCleanup(self.fake.close)

    def test_desktop_reads_focus_cursor_workspace_and_monitors(self):
        state = hyprland.desktop(self.runtime, "sig_1", pid=os.getpid())
        self.assertEqual(state["active_window"],
                         {"address": "0x1", "pid": 7, "class": "foot", "initialTitle": "bystander"})
        self.assertEqual(state["cursor"], {"x": 640, "y": 400})
        self.assertEqual(state["workspace"], "1")
        self.assertEqual(state["monitors"], [("Virtual-1", True, "1", "")])

    def test_a_shell_retitling_the_focused_window_is_not_drift(self):
        before = hyprland.desktop(self.runtime, "sig_1", pid=os.getpid())
        retitled = dict(ANSWERS[b"j/activewindow"], title="user@host:/tmp")
        with mock.patch.dict(ANSWERS, {b"j/activewindow": retitled}):
            after = hyprland.desktop(self.runtime, "sig_1", pid=os.getpid())
        self.assertEqual(before, after)

    def test_a_listener_from_another_process_does_not_answer_for_the_compositor(self):
        with self.assertRaisesRegex(RuntimeError, "expected 1"):
            hyprland.query(self.runtime, "sig_1", "cursorpos", pid=1)
        self.assertEqual(self.fake.requests, [])

    def test_signatures_cannot_leave_the_instance_directory(self):
        for bad in ("..", "a/b", "", "sig 1"):
            with self.subTest(signature=bad), self.assertRaises(ValueError):
                hyprland.socket_path(self.runtime, bad)

    def test_listening_distinguishes_a_live_socket_from_a_leftover_path(self):
        self.assertTrue(hyprland.listening(self.fake.path))
        leftover = self.runtime / "wayland-9"
        leftover.touch()
        self.assertFalse(hyprland.listening(leftover))

    def test_silent_exec_targets_a_special_workspace_without_focus(self):
        with mock.patch.object(hyprland, "control", side_effect=["ok", "ok"]) as control:
            hyprland.exec_silently(self.runtime, "sig_1", "/bin/sh /tmp/rt.x/launch-1.sh", "ad_e2e")
        self.assertEqual(control.call_args_list[1].args[2], [
            "dispatch",
            'hl.dsp.exec_cmd("/bin/sh /tmp/rt.x/launch-1.sh", { workspace = "special:ad_e2e silent" })',
        ])

    def test_silent_exec_uses_the_legacy_rule_without_lua(self):
        with mock.patch.object(hyprland, "control", side_effect=["unknown request", "ok"]) as control:
            hyprland.exec_silently(self.runtime, "sig_1", "/bin/sh /tmp/x.sh", "ad_e2e")
        self.assertEqual(control.call_args_list[1].args[2],
                         ["dispatch", "exec", "[workspace special:ad_e2e silent] /bin/sh /tmp/x.sh"])

    def test_commands_that_could_escape_the_lua_string_are_refused(self):
        for bad in ('a"b', "a\\b", "a\nb", "a;b", "$(x)"):
            with self.subTest(command=bad), mock.patch.object(hyprland, "control") as control:
                with self.assertRaises(ValueError):
                    hyprland.exec_silently(self.runtime, "sig_1", bad, "ad_e2e")
                control.assert_not_called()


    def test_the_class_rule_places_exactly_one_class_silently_and_can_be_disabled(self):
        self.assertEqual(hyprland.class_rule_code("ad_e2e_place", "twin_windows.py", "ad_e2e"),
                         'hl.window_rule({ name = "ad_e2e_place", enabled = true, '
                         'match = { class = "^twin_windows[.]py$" }, workspace = "special:ad_e2e silent" })')
        self.assertEqual(hyprland.disable_rule_code("ad_e2e_place"),
                         'hl.window_rule({ name = "ad_e2e_place", enabled = false })')

    def test_class_rules_that_could_escape_the_lua_string_are_refused(self):
        for name, app_class in (("ad", 'a"b'), ("ad", "a\\b"), ("ad", "a b"), ("a-d", "x"), ("a\"", "x")):
            with self.subTest(name=name, app_class=app_class), self.assertRaises(ValueError):
                hyprland.class_rule_code(name, app_class, "ad_e2e")

    def test_a_class_rule_is_evaluated_and_its_refusal_raises(self):
        with mock.patch.object(hyprland, "control", side_effect=["ok", "ok"]) as control:
            hyprland.place_class(self.runtime, "sig_1", "ad_e2e_place", "twin_windows.py", "ad_e2e")
        self.assertEqual(control.call_args_list[1].args[2][0], "eval")
        with mock.patch.object(hyprland, "control", side_effect=["ok", "error: unknown field"]):
            with self.assertRaisesRegex(RuntimeError, "unknown field"):
                hyprland.disable_rule(self.runtime, "sig_1", "ad_e2e_place")

    def test_a_legacy_config_gets_no_class_rule_it_could_not_remove(self):
        with mock.patch.object(hyprland, "control", side_effect=["unknown request"]) as control:
            with self.assertRaisesRegex(RuntimeError, "legacy"):
                hyprland.place_class(self.runtime, "sig_1", "ad_e2e_place", "twin_windows.py", "ad_e2e")
        self.assertEqual(control.call_count, 1)


if __name__ == "__main__":
    unittest.main()
