import ctypes
import pathlib
import re
import unittest

from ffi_live import EXECUTE_ARGS, EXECUTE_TIMEOUT_ARGS, Action, ref_for

HEADER = pathlib.Path(__file__).resolve().parents[2] / "crates/ffi/include/agent_desktop.h"


def header_parameter_count(name):
    match = re.search(rf"\b{name}\(([^)]*)\);", HEADER.read_text())
    if match is None:
        raise AssertionError(f"{name} is not declared in {HEADER}")
    return len(match.group(1).split(","))


class FfiLiveContractTests(unittest.TestCase):
    def test_action_layout_matches_pinned_c_abi(self):
        self.assertEqual(ctypes.sizeof(Action), 96)

    def test_execute_bindings_match_the_committed_header(self):
        self.assertEqual(len(EXECUTE_ARGS), header_parameter_count("ad_execute_by_ref"))
        self.assertEqual(len(EXECUTE_TIMEOUT_ARGS),
                         header_parameter_count("ad_execute_by_ref_timeout"))

    def test_ref_lookup_reads_typed_native_identifier(self):
        envelope = {
            "data": {
                "tree": {
                    "role": "window",
                    "children": [{
                        "role": "button",
                        "native_id": {"kind": "AX_IDENTIFIER", "value": "delayed-button"},
                        "ref_id": "@snapshot:e3",
                    }],
                }
            }
        }
        self.assertEqual(ref_for(envelope, "delayed-button"), "@snapshot:e3")


if __name__ == "__main__":
    unittest.main()
