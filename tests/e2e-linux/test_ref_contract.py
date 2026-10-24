import unittest
from unittest.mock import patch
from pathlib import Path
from tempfile import TemporaryDirectory
from types import SimpleNamespace
import json

from scenarios.ref_contract import expected_outcomes, row_verdict, fresh, prepare
from scenarios.ref_contract_identity import identify, raw_target
from oracle.atspi_raw import RawNode


class RefContractTests(unittest.TestCase):
    def test_prepare_selects_another_option_using_its_advertised_action(self):
        node = {"name": "zoo-row-1", "role": "option", "available_actions": ["Select"]}
        other = dict(node, name="zoo-row-2", ref_id="@s1:e2")
        with patch("scenarios.ref_contract.selected", return_value=True), \
                patch("scenarios.ref_contract.snapshot"), \
                patch("scenarios.ref_contract.named", return_value=other), \
                patch("scenarios.ref_contract.act") as act, \
                patch("scenarios.ref_contract.poll", return_value=True), \
                patch("scenarios.ref_contract.fresh", return_value=node):
            prepare(None, None, None, node, "select")
        act.assert_called_once_with(None, other, "select", "zoo-row-2")

    def test_raw_text_uses_character_count_for_toolkits_without_negative_end(self):
        node = RawNode(None, ":1.5", "/entry")
        with patch.object(node, "interfaces", return_value=["org.a11y.atspi.Text"]), \
                patch.object(node, "get_property", return_value=6), \
                patch.object(node, "invoke", side_effect=lambda interface, method, signature, args:
                             ("marker" if args == (0, 6) else "",)):
            self.assertEqual(node.text(), "marker")

    def test_raw_mapping_distinguishes_unnamed_and_duplicate_named_nodes(self):
        class Node:
            bus = ":1.5"

            def __init__(self, path, name, children=()):
                self.path, self.name, self._children = path, name, children

            def children(self):
                return self._children

        controls = [Node("/button", "same"), Node("/label", "same"),
                    Node("/bar_a", ""), Node("/bar_b", "")]
        tree = Node("/root", "", [Node("/frame", "window", controls)])
        with TemporaryDirectory() as directory:
            root = Path(directory)
            saved = root / "snapshots" / "s1"
            saved.mkdir(parents=True)
            (saved / "refmap.json").write_text(json.dumps({"inner": {
                "@e1": {"source_window_id": "w-1.5-frame", "path": [0]},
                "@e2": {"source_window_id": "w-1.5-frame", "path": [3]},
            }}))
            data = identify(root, {"tree": {"children": [
                {"ref_id": "@s1:e1", "name": "same"}, {"ref_id": "@s1:e2"}]}})
            first, second = data["tree"]["children"]
            self.assertIs(raw_target(tree, first), controls[0])
            self.assertIs(raw_target(tree, second), controls[3])

    def test_fresh_ref_uses_address_when_value_name_changes(self):
        original = {"ref_id": "@s1:e1", "role": "tablist", "name": "tab-a",
                    "_identity": ("window", (0, 3))}
        refreshed = dict(original, ref_id="@s2:e2", name="tab-b")
        other = dict(original, ref_id="@s2:e1", _identity=("window", (0, 2)))
        with patch("scenarios.ref_contract.snapshot", return_value={"tree": {
                "children": [other, refreshed]}}):
            self.assertIs(fresh(SimpleNamespace(), None, original), refreshed)

    def test_snapshot_promises_and_blocked_primary_action(self):
        node = {"name": "zoo-entry", "available_actions":
                ["SetFocus", "TypeText", "SetValue", "RightClick", "ScrollTo"]}
        self.assertEqual(expected_outcomes(node), [("type", "land"), ("set-value", "land")])
        for state in ("hidden", "offscreen", "disabled"):
            with self.subTest(state=state):
                self.assertEqual(expected_outcomes(dict(node, states=[state])), [("type", "refuse")])
        self.assertEqual(expected_outcomes({"role": "button"}), [])

    def test_all_observable_advertised_actions_are_preserved(self):
        node = {"available_actions": ["Click", "Toggle", "Check", "Uncheck", "TypeText",
                                      "SetValue", "Select", "Expand", "Collapse"]}
        self.assertEqual(expected_outcomes(node), [(a, "land") for a in
                         ("click", "toggle", "check", "uncheck", "type", "set-value",
                          "select", "expand", "collapse")])

    def test_offscreen_exception_does_not_override_hidden_or_disabled(self):
        for states, expected in (([], "land"), (["offscreen"], "land-or-refuse"),
                                 (["hidden", "offscreen"], "refuse"),
                                 (["disabled", "offscreen"], "refuse")):
            with self.subTest(states=states):
                self.assertEqual(expected_outcomes({"name": "zoo-offscreen", "states": states,
                                                   "available_actions": ["Click"]}), [("click", expected)])

    def test_landing_needs_both_oracles_and_refusal_needs_no_effect(self):
        # expected, refusal, log effect, raw effect, unchanged log/raw, verdict
        cases = [
            ("land", False, True, True, False, False, "pass"),
            ("land", False, True, False, False, True, "fail"),
            ("land", False, False, True, True, False, "fail"),
            ("land", False, False, False, True, True, "fail"),  # lying success
            ("land", True, False, False, True, True, "fail"),   # unmarked refusal
            ("refuse", True, False, False, True, True, "pass"),
            ("refuse", False, False, False, True, True, "fail"),
            ("refuse", True, False, False, False, True, "fail"),  # unexpected log
            ("refuse", True, False, False, True, False, "fail"),  # state drift
            ("refuse", False, True, True, False, False, "fail"),
            ("land-or-refuse", False, True, True, False, False, "pass"),
            ("land-or-refuse", True, False, False, True, True, "pass"),
            ("land-or-refuse", False, False, False, True, True, "fail"),
            ("land-or-refuse", True, False, False, True, False, "fail"),
        ]
        for expected, refused, log, raw, log_same, raw_same, verdict in cases:
            with self.subTest(case=(expected, refused, log, raw, log_same, raw_same)):
                self.assertEqual(row_verdict(expected, refused=refused, log_effect=log,
                                              raw_effect=raw, log_unchanged=log_same,
                                              raw_unchanged=raw_same), verdict)


if __name__ == "__main__":
    unittest.main()
