#!/usr/bin/env python3
"""Check probes/linux/FINDINGS.md: row shape, vocabularies, unique ids, deferral closure, phases.md citations, and no box identity."""
import argparse
import os
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STACKS = {"dbus", "gi-atspi", "tool"}
SCOPES = {"api-contract", "app", "box"}
VERDICTS = {"CONFIRMS", "CONTRADICTS", "NEW-EDGE", "DEFERRED"}
ROW = re.compile(r"^\|\s*(L\d+-\d+)\s*\|")
ID = re.compile(r"\bL\d+-\d+\b")
CLOSURE = re.compile(r"closure:\s*3\.(?:1[0-5]|[0-9])(?:\.\d+)?(?!\d)")
IDENTITY = [
    (re.compile(r"/home/\w+"), "home path"),
    (re.compile(r"(?<![\d.])100\.(?:6[4-9]|[7-9]\d|1[01]\d|12[0-7])\.\d{1,3}\.\d{1,3}(?![\d])"), "tailnet address"),
    (re.compile(r"\.ts\.net\b"), "tailnet host name"),
    (re.compile(r"(?<![0-9a-fA-F])[0-9a-fA-F]{32}(?![0-9a-fA-F])"), "machine-id style token"),
]


def split_cells(row):
    cells, current, in_code, escaped = [], [], False, False
    for char in row.strip():
        if escaped:
            current.append(char)
            escaped = False
        elif char == "\\":
            current.append(char)
            escaped = True
        elif char == "`":
            in_code = not in_code
            current.append(char)
        elif char == "|" and not in_code:
            cells.append("".join(current).strip())
            current = []
        else:
            current.append(char)
    cells.append("".join(current).strip())
    return cells[1:-1]


def check_row(cells):
    if len(cells) != 8:
        return [f"expected 8 cells, found {len(cells)}"]
    _, _, stack, scope, _, _, verdict, action = cells
    problems = []
    if not stack or set(stack.split(" + ")) - STACKS:
        problems.append(f"stack {stack!r} is not dbus, gi-atspi, tool or a ' + ' combination")
    if scope not in SCOPES:
        problems.append(f"scope {scope!r} is not api-contract, app or box")
    word = verdict.split()[0] if verdict.split() else ""
    if word not in VERDICTS:
        problems.append(f"verdict {verdict!r} is not CONFIRMS, CONTRADICTS, NEW-EDGE or DEFERRED")
    if word == "DEFERRED" and not ("needs:" in action and CLOSURE.search(action)):
        problems.append("DEFERRED action needs 'needs:' and 'closure: 3.<n>'")
    return problems


def identity_problems(text):
    patterns = list(IDENTITY)
    for token in filter(None, (t.strip() for t in os.environ.get("AGENT_DESKTOP_BOX_IDENTITY", "").split(","))):
        patterns.append((re.compile(re.escape(token), re.I), "configured box identity token"))
    return [f"box identity ({label}) present" for pattern, label in patterns if pattern.search(text)]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--ledger", type=Path, default=ROOT / "probes/linux/FINDINGS.md")
    parser.add_argument("--phases", type=Path, default=ROOT / "docs/phases.md")
    parser.add_argument("--sub-phase")
    args = parser.parse_args()
    if not args.ledger.exists():
        print("check-linux-findings: no Linux ledger, nothing to check")
        return 0
    text = args.ledger.read_text()
    rows = [(ROW.match(line).group(1), split_cells(line)) for line in text.splitlines() if ROW.match(line)]
    if args.sub_phase:
        named = re.compile(re.escape(args.sub_phase) + r"(?!\d|\.\d)")
        for row_id, cells in rows:
            if len(cells) == 8 and named.search(cells[7]):
                print(f"{row_id}\t{cells[6]}\t{cells[7]}")
        return 0
    violations, seen = [], set()
    for row_id, cells in rows:
        if row_id in seen:
            violations.append((row_id, "duplicate id"))
        seen.add(row_id)
        violations += [(row_id, p) for p in check_row(cells)]
    if args.phases.exists():
        cited = sorted(set(ID.findall(args.phases.read_text())) - seen)
        violations += [(i, f"cited in {args.phases.name} but missing from the ledger") for i in cited]
    violations += [(args.ledger.name, p) for p in identity_problems(text)]
    for where, problem in violations:
        print(f"check-linux-findings: {where}: {problem}")
    if violations:
        return 1
    print(f"check-linux-findings: OK, {len(rows)} rows")
    return 0


if __name__ == "__main__":
    sys.exit(main())
