"""Paints the terminal it runs in one colour and logs what it painted.

It runs inside foot and sets the background with OSC 11, so the whole
surface is that colour. `static RRGGBB LOG` paints once and waits.
`ticker PERIOD_S LOG` paints tick `n` as `tick_colour(n)` every period. After
each paint it writes `"<n> <CLOCK_MONOTONIC ns>"` to LOG, so a capture's
colour names the tick it shows and the log says when that tick was written.
"""

import sys
import time

HIDE_CURSOR_AND_CLEAR = "\033[?25l\033[2J"


def tick_colour(tick: int) -> str:
    return f"{tick % 256:02x}{(tick // 256) % 256:02x}40"


def paint(colour: str) -> None:
    sys.stdout.write(f"\033]11;#{colour}\007{HIDE_CURSOR_AND_CLEAR}")
    sys.stdout.flush()


def main(argv: list[str]) -> None:
    mode, value, log = argv[1], argv[2], argv[3]
    with open(log, "a", buffering=1) as out:
        if mode == "static":
            paint(value)
            out.write(f"0 {time.monotonic_ns()}\n")
            while True:
                time.sleep(3600)
        period = float(value)
        tick = 1
        while True:
            paint(tick_colour(tick))
            out.write(f"{tick} {time.monotonic_ns()}\n")
            tick += 1
            time.sleep(period)


if __name__ == "__main__":
    main(sys.argv)
