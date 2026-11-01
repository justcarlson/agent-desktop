"""Every cursor leg reads each oracle channel it opens before, after and restored.

The verdict needs exactly one read at each stage for every channel a leg
touches, and these legs read their channels in their own bodies, so a
channel missing a stage fails the leg after its product checks pass.
"""

import ast
import pathlib
import unittest

MODULES = ("hyprland_cursor.py", "hyprland_cursor_agents.py")
STAGES = {"before", "after", "restored"}


def stages_by_channel(function: ast.FunctionDef) -> dict[str, set[str]]:
    found: dict[str, set[str]] = {}
    for call in ast.walk(function):
        if not (isinstance(call, ast.Call) and isinstance(call.func, ast.Attribute)):
            continue
        if call.func.attr not in STAGES or getattr(call.func.value, "id", None) != "ctx":
            continue
        if call.args and isinstance(call.args[0], ast.Constant) and isinstance(call.args[0].value, str):
            found.setdefault(call.args[0].value, set()).add(call.func.attr)
    return found


def legs(path: pathlib.Path):
    for node in ast.walk(ast.parse(path.read_text())):
        if isinstance(node, ast.FunctionDef) and any(
                isinstance(mark, ast.Call) and getattr(mark.func, "id", None) == "implementation"
                for mark in node.decorator_list):
            yield node


class CursorOracleReadsTest(unittest.TestCase):
    def test_every_cursor_leg_reads_each_channel_before_after_and_restored(self):
        scenarios = pathlib.Path(__file__).parent / "scenarios"
        gaps = []
        for name in MODULES:
            for leg in legs(scenarios / name):
                for channel, stages in stages_by_channel(leg).items():
                    if stages != STAGES:
                        gaps.append(f"{name}:{leg.name} {channel} lacks {sorted(STAGES - stages)}")
        self.assertEqual(gaps, [])


if __name__ == "__main__":
    unittest.main()
