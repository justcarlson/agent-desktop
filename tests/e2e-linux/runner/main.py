from __future__ import annotations

import argparse
import sys
from pathlib import Path

HARNESS_ROOT = Path(__file__).resolve().parent.parent
TIER_CHOICES = ("x11-openbox", "x11-mutter", "wayland")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(prog="run.sh")
    parser.add_argument("--tier", choices=TIER_CHOICES)
    parser.add_argument("--leg")
    parser.add_argument("--lying-actions", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    parse_args(argv)
    sys.path.insert(0, str(HARNESS_ROOT))
    import scenarios
    from runner.legs import REGISTRY, implementations

    if {leg.id for leg in REGISTRY} != set(implementations()):
        return 2
    raise NotImplementedError("the leg executor")


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
