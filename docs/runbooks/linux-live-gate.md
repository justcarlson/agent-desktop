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
`HOME` pointing at the box user's home for private session and artifact storage.
Supply tool executables through `PATH` and shared libraries through `LD_LIBRARY_PATH`.
Set `AGENT_DESKTOP_QT_PYTHON` if PyQt6 uses a separate Python interpreter.
The runner builds the CLI and Linux library test executable once, then freezes
the latter in its output directory. Later chunks verify source and executable
hashes and reuse it without rebuilding or taking cargo's lock per module.

## Owner window

`bash scripts/linux-live-gate.sh --dry-run` prints the 24 attended legs.
`--owner` still refuses to claim execution of their unimplemented bodies.
Neither option starts the private suite.

## Legs

Default: `bash scripts/linux-live-gate.sh --jobs 3`. For shared-box calls below
15 minutes, prepare the frozen manifest in its own call, then split the same
gate into three tier chunks with one output directory. This keeps compilation
from consuming the first tier's execution budget:

```bash
AGENT_DESKTOP_REMOTE_TIMEOUT=840 AGENT_DESKTOP_REMOTE_LOCK_WAIT=15 bash scripts/remote-linux.sh -c 'export CARGO_INCREMENTAL=0 && timeout 800 bash scripts/linux-live-gate.sh --list --output "$HOME/ad-wt/live-gate-fast"'
AGENT_DESKTOP_REMOTE_TIMEOUT=840 AGENT_DESKTOP_REMOTE_LOCK_WAIT=15 bash scripts/remote-linux.sh -c 'export CARGO_INCREMENTAL=0 && timeout 800 bash scripts/linux-live-gate.sh --tier openbox --output "$HOME/ad-wt/live-gate-fast"'
AGENT_DESKTOP_REMOTE_TIMEOUT=840 AGENT_DESKTOP_REMOTE_LOCK_WAIT=15 bash scripts/remote-linux.sh -c 'export CARGO_INCREMENTAL=0 && timeout 800 bash scripts/linux-live-gate.sh --tier mutter --output "$HOME/ad-wt/live-gate-fast"'
AGENT_DESKTOP_REMOTE_TIMEOUT=840 AGENT_DESKTOP_REMOTE_LOCK_WAIT=15 bash scripts/remote-linux.sh -c 'export CARGO_INCREMENTAL=0 && timeout 800 bash scripts/linux-live-gate.sh --tier wayland --output "$HOME/ad-wt/live-gate-fast"'
```

A busy shared lock returns after 15 seconds; retry that chunk once the box is
available. This leaves the tier deadline inside the call budget.

Use a fresh output directory after any source change. Existing tier results
are never overwritten. `--jobs 1` runs the same selection serially; `--list`
builds and records the complete partition without opening a desktop.

For a closing gate after a measured full run, select only changed modules with
`--ignored-only --module tree::physical_live_tests --module notifications::live_tests`
(repeat `--module` for each affected module). Module prefixes match at `::`
boundaries; an unmatched module fails before opening a session. Pass the same
selection on every tier chunk and `--merge`; the manifest pins that selection.
The receipt counts only selected tests and never certifies the full inventory.

Changed owners map to these ignored live modules:

| Changed owner | Ignored live modules |
| --- | --- |
| Keyboard focus and shared focus proof | `tree::physical_live_tests`, `tree::physical_live_apps_tests`, `actions::live_widget_tests` |
| Selection and ref recovery | `actions::live_select_tests`, `actions::live_widget_tests`, `tree::navigate_live_tests` |
| Menu resolution and modal routing | `system::menu::menu_surface_live_tests`, `system::menu::menu_wait_live_tests` |
| Notification cleanup lease | `notifications::live_tests` (Mutter only) |

The modal-gate and resolver fake tests are non-ignored and run in the library
gate; menu/modal/surface E2E legs provide their live acceptance coverage.

Ordinary ignored X11 cases run on both openbox and mutter. GNOME notification
and shell-outage cases require GNOME Shell and run only on mutter. Non-ignored unit
cases run once, with the openbox tier. The native Wayland input, clipboard and
portal and shell cases (including nested test modules) boot their own compositor
and must inherit no `WAYLAND_DISPLAY`;
they run on the Wayland tier inside the common private bus/runtime envelope.
The Wayland cursor refusals use the external private Wayland shell helper.
Tests that replace a window manager run
in a nested session so they cannot damage the remaining shard's environment.
The fake RemoteDesktop service also gets a nested session bus so it cannot
collide with the tier compositor's well-known service name.

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

The Mutter shell runs in the `ad-private` session mode: the user mode without the polkit agent. Without it, the first private shell of a login becomes the polkit agent for every private session and shows colord's "Authentication Required" modal, which holds focus and swallows input.

X11 live tests wait until Mutter has destroyed a closed window's frame. The next window can reuse the same id, and Mutter would still move it to the old frame origin.

The Python boundary checks need no display:
`python3 scripts/test-linux-live-gate.py`.

## Retired environment-dependent cases

Each case below failed the full gate and received one diagnostic rerun. Their
failures occurred before product delivery; they are removed rather than counted
as passes or given longer waits.

| Retired case | Reason | Retained contract owner |
| --- | --- | --- |
| `registry_live_focus_agrees_with_the_window_manager` | A fixed openbox extraction path ignores the configured private-session root. | `focus::tests` checks exact active-window attribution; `lifecycle.focus_window_active_reads` observes native focus. |
| `focus_window_live_a_minimized_window_ends_as_measured_for_the_manager` | Both attempts timed out acquiring the machine-wide interaction lease. | `focus_window::lift_tests` covers restoration and activation; `lifecycle.openbox_lift_restored_and_active` observes the live effect. |
| `live_x11_cursor_cli_reports_rendered_and_paints_agent_positions` | Mouse-move hit lease contention, then passed its sole rerun. | `live_x11_cursor_adapter_paints_agent_positions_and_retires_resources` checks native rendering and cleanup. |
| `close_live_a_private_gnome_shell_is_refused_and_keeps_running` | Both attempts timed out acquiring the interaction lease after private shell startup. | `close::tests` covers protected identity before any send; protected-decoy and private-session-process live cases retain refusal coverage. |
