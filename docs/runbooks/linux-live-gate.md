# Linux live gate

## Purpose

Run the entire Linux library inventory, including ignored live tests, with at
most three private desktops active at once. Each libtest process still uses
`--test-threads=1`. The gate fails on any test failure, session failure, timeout,
missing result, duplicate result or unexpected ignored result.

The attended `--owner` registry is separate and remains unimplemented. Its
`--dry-run` lists the owner-window legs; it does not certify the private suite
or authorize an agent to touch the owner's desktop.

## Preconditions

Use the Linux box through `bash scripts/remote-linux.sh -c '...'`. The helper's
box lock covers all concurrent shards. Do not overlap calls from one worktree.
The packaged openbox and desktop fixture apps must already be installed. Keep
`HOME` pointing at the box user's home so the packaged toolkit tools are found.
The runner builds the CLI and Linux library test executable once, then freezes
the latter in its output directory. Later chunks verify source and executable
hashes and reuse it without rebuilding or taking cargo's lock per module.

## Owner window

`bash scripts/linux-live-gate.sh --dry-run` prints the 24 attended legs.
`--owner` still refuses to claim execution of their unimplemented bodies.
Neither option starts the private suite.

## Legs

Default: `bash scripts/linux-live-gate.sh --jobs 3`. For shared-box calls below
15 minutes, split the same gate into three chunks with one output directory:

```bash
AGENT_DESKTOP_REMOTE_TIMEOUT=840 AGENT_DESKTOP_REMOTE_LOCK_WAIT=15 bash scripts/remote-linux.sh -c 'export CARGO_INCREMENTAL=0 && timeout 800 bash scripts/linux-live-gate.sh --tier openbox --output "$HOME/ad-wt/live-gate-fast"'
AGENT_DESKTOP_REMOTE_TIMEOUT=840 AGENT_DESKTOP_REMOTE_LOCK_WAIT=15 bash scripts/remote-linux.sh -c 'export CARGO_INCREMENTAL=0 && timeout 800 bash scripts/linux-live-gate.sh --tier mutter --output "$HOME/ad-wt/live-gate-fast"'
AGENT_DESKTOP_REMOTE_TIMEOUT=840 AGENT_DESKTOP_REMOTE_LOCK_WAIT=15 bash scripts/remote-linux.sh -c 'export CARGO_INCREMENTAL=0 && timeout 800 bash scripts/linux-live-gate.sh --tier wayland --output "$HOME/ad-wt/live-gate-fast"'
```

A busy shared lock returns after 15 seconds; retry that chunk once the box is
available. This leaves the tier deadline inside the call budget.

Use a fresh output directory after any source change. Existing tier results
are never overwritten. `--jobs 1` runs the same selection serially; `--list`
builds and records the complete partition without opening a desktop.

Ordinary ignored X11 cases run on both openbox and mutter. Non-ignored unit
cases run once, with the openbox tier. The native Wayland input, clipboard and
portal cases boot their own compositor and must inherit no `WAYLAND_DISPLAY`;
they run on the Wayland tier inside the common private bus/runtime envelope.
The Wayland cursor refusals use the external private Wayland shell helper.
The physical mutter fixture boots its own X11 shell in a nested private
session and runs on the mutter tier. Tests that replace a window manager run
in a nested session so they cannot damage the remaining shard's environment.

Modules stay together to avoid concurrent access to their fixture state. The
portal module is split into independent case batches because each case boots
its own compositor. Workers execute those batches serially within each shard.
On X11, the global product interaction lease intentionally spans private displays.
One worker therefore runs mutations and unit tests (including real lease tests);
the other workers run explicitly reviewed read-only modules. Unknown modules
default to the writer. This avoids cross-shard lease timeouts without bypassing,
relocating or weakening the product lock. Native Wayland fixtures use independent
portal sessions/local tokens and remain distributed across all three workers.

## Recording results

`manifest.json` pins the test executable, source hash, every test name, ignored
status, tier assignment and shard membership. Per-module logs and JSON receipts
record exact names, outcomes and elapsed time. Each tier has `summary.json` with
counts, wall time and summed module time. Summed module time is an observed
serial-work estimate for this run, **not** an old-version wall-time measurement.

After the last chunk, the gate writes the merged `summary.json` and fails unless
all three tiers are complete and green. To recheck existing receipts without
running tests:

```bash
bash scripts/linux-live-gate.sh --merge --output "$HOME/ad-wt/live-gate-fast"
```

## Safety rules

Each worker owns a display, session bus, AT-SPI bus, 0700 runtime directory,
XDG config/data/cache, temporary directory and `AGENT_DESKTOP_HOME`. Display
selection is locked while Xvfb starts. Mutter startup maps an owned primer,
waits for its map event and for Activities Overview to close, then removes the
primer before tests start. The inherited live display, authority
and accessibility overrides are cleared before any desktop app can launch.
Cleanup only signals owned child/session PIDs. It never restarts the live
shell or accessibility bus. Cleanup removes each worker's temporary runtime directory under `~/ad-wt`.

The Python boundary checks need no display:
`python3 scripts/test-linux-live-gate.py`.
