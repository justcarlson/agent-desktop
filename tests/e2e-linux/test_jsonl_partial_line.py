from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from scenarios import hyprland_pointer_pad, hyprland_window_app, ref_contract
from scenarios import toolkit_qt
from apps import sni_item

RECORD = '{"widget": "w", "event": "e", "at": 1.0}'
PARTIAL = RECORD + '\n{"widget": "w", "ev'


class PartialLineTests(unittest.TestCase):
    """An app may be mid-write when a leg reads its log; only lines up to the last newline are records."""

    def read(self, reader, text: str = PARTIAL):
        with tempfile.TemporaryDirectory() as folder:
            log = Path(folder) / "app.log"
            log.write_text(text)
            return reader(log)

    def test_each_reader_skips_a_record_still_being_written(self):
        record = {"widget": "w", "event": "e", "at": 1.0}
        readers = {
            "ref_contract.events": (ref_contract.events, [record]),
            "toolkit_qt.events": (toolkit_qt.events, ["e"]),
            "hyprland_window_app.events": (hyprland_window_app.events, [record]),
            "hyprland_pointer_pad.events": (hyprland_pointer_pad.events, [record]),
        }
        for name, (reader, expected) in readers.items():
            with self.subTest(name):
                self.assertEqual(self.read(reader), expected)
                self.assertEqual(self.read(reader, RECORD + "\n"), expected)

    def test_the_tray_reader_skips_a_call_still_being_written(self):
        call = '{"call": "activate", "args": {"x": 1}}'
        expected = [("activate", {"x": 1})]
        self.assertEqual(self.read(sni_item.read_log, call + '\n{"call": "act'), expected)
        self.assertEqual(self.read(sni_item.read_log, call + "\n"), expected)


if __name__ == "__main__":
    unittest.main()
