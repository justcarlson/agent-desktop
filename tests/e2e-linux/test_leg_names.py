"""Every name a leg module reads is bound in that module or is a builtin.

The legs run only inside a VM, so a name lost from an import surfaces there as a NameError.
This check reads the symbol tables instead, and needs no session.
"""

from __future__ import annotations

import builtins
import symtable
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent
PACKAGES = ("apps", "oracle", "runner", "scenarios")
MODULE_NAMES = {"__file__", "__name__", "__doc__", "__spec__", "__package__", "__loader__", "__builtins__",
                "__conditional_annotations__"}


def sources() -> list[Path]:
    found = [path for package in PACKAGES for path in sorted((ROOT / package).rglob("*.py"))]
    return found + sorted(ROOT.glob("*.py"))


def unbound(text: str, name: str) -> set[str]:
    top = symtable.symtable(text, name, "exec")
    bound = {symbol.get_name() for symbol in top.get_symbols()
             if symbol.is_assigned() or symbol.is_imported() or symbol.is_namespace()}
    known = bound | set(dir(builtins)) | MODULE_NAMES
    missing: set[str] = set()
    pending = [top]
    while pending:
        table = pending.pop()
        pending.extend(table.get_children())
        for symbol in table.get_symbols():
            if not symbol.is_referenced() or symbol.get_name() in known:
                continue
            if table is top or symbol.is_global():
                missing.add(symbol.get_name())
    return missing


class LegNameTests(unittest.TestCase):
    def test_every_leg_module_binds_the_names_it_reads(self):
        problems = []
        for path in sources():
            missing = unbound(path.read_text(encoding="utf-8"), str(path))
            if missing:
                problems.append(f"{path.relative_to(ROOT)}: {', '.join(sorted(missing))}")
        self.assertEqual(problems, [])

    def test_a_name_lost_from_an_import_is_caught(self):
        text = "from os import path\n\ndef leg():\n    return path, cursor_argv(1, 2)\n"
        self.assertEqual(unbound(text, "leg.py"), {"cursor_argv"})

    def test_locals_arguments_and_comprehensions_count_as_bound(self):
        text = ("import os\n\ndef leg(a, *rest, **opts):\n    b = [c for c in rest]\n"
                "    with open(a) as d:\n        pass\n    try:\n        pass\n    except OSError as e:\n"
                "        return e\n    return b, d, opts, os, print, __file__\n")
        self.assertEqual(unbound(text, "leg.py"), set())


if __name__ == "__main__":
    unittest.main()
