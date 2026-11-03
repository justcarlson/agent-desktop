"""Record terminal input bytes after raw mode is ready."""
import os
from pathlib import Path
import sys
import termios
import tty

log = Path(sys.argv[1])
ready = Path(sys.argv[2])
fd = sys.stdin.fileno()
original = termios.tcgetattr(fd)
try:
    tty.setraw(fd)
    with log.open('wb', buffering=0) as output:
        ready.touch()
        while True:
            data = os.read(fd, 4096)
            if not data:
                break
            output.write(data)
finally:
    termios.tcsetattr(fd, termios.TCSANOW, original)
