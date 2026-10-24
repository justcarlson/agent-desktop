import unittest

from scenarios.ref_contract import expected_outcomes, row_verdict


class RefContractTests(unittest.TestCase):
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
