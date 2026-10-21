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

The [static cost inventory](linux-live-costs.md) records all 247 original
ignored cases, direct sleeps, fixture profiles, launches and deadlines. Its
original roster remains the baseline even when a test is explicitly retired.

## Owner window

`bash scripts/linux-live-gate.sh --dry-run` prints the 24 attended legs.
`--owner` still refuses to claim execution of their unimplemented bodies.
Neither option starts the private suite.

## Legs

Default: `bash scripts/linux-live-gate.sh --jobs 3`. For shared-box calls below
15 minutes, split the same gate into three chunks with one output directory:

```bash
bash scripts/remote-linux.sh -c 'export CARGO_INCREMENTAL=0 && timeout 840 bash scripts/linux-live-gate.sh --tier openbox --output "$HOME/ad-wt/live-gate-fast"'
bash scripts/remote-linux.sh -c 'export CARGO_INCREMENTAL=0 && timeout 840 bash scripts/linux-live-gate.sh --tier mutter --output "$HOME/ad-wt/live-gate-fast"'
bash scripts/remote-linux.sh -c 'export CARGO_INCREMENTAL=0 && timeout 840 bash scripts/linux-live-gate.sh --tier wayland --output "$HOME/ad-wt/live-gate-fast"'
```

Use a fresh output directory after any source change. Existing tier results
are never overwritten. `--jobs 1` runs the same selection serially; `--list`
builds and records the complete partition without opening a desktop.

Ordinary ignored X11 cases run on both openbox and mutter. Non-ignored unit
cases run once, with the openbox tier. The native Wayland input, clipboard and
portal cases boot their own compositor and must inherit no `WAYLAND_DISPLAY`;
they run on the Wayland tier inside the common private bus/runtime envelope.
The Wayland cursor refusals and no-input signal case use the external private
Wayland shell helper; the signal case deliberately omits input devices.
The physical mutter fixture boots its own X11 shell in a nested private
session and runs on the mutter tier. Tests that replace a window manager run
in a nested session so they cannot damage the remaining shard's environment.

Modules stay together to avoid concurrent access to their fixture state. The
portal module is split into independent case batches because each case boots
its own compositor. Workers execute those batches serially within each shard.

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

The libtest list is the runtime authority. Every original test must remain in
it, or appear in the disposition list with its retained contract owner. A case
that returns early because its own applicability guard declines is still a
libtest pass; the gate does not turn that into proof of the guarded behavior.

### Retired cases

- `tree::perf_live_tests::perf_live_costs_min_of_seven_with_the_warm_up_discarded`:
  removed redundant repeated timing samples and an incoherent missing-name ref
  that retained the original object token. Same-tier live owners remain:
  `resolve_live_every_snapshot_ref_resolves_to_the_node_it_was_taken_from`,
  `qt_live_moved_twin_refs_stay_distinct_and_shared_text_only_evidence_is_ambiguous`,
  `live_read_every_projection_agrees_with_a_second_read_on_each_toolkit`,
  `find_live_returns_matches_and_refs_that_resolve_back_to_their_nodes`, and
  `walker_live_real_apps`. The deterministic search-miss owner is
  `tree::resolve::tests::a_unique_match_resolves_and_no_match_is_stale_only_when_everything_was_read`;
  retained-object replacement refusal remains in
  `tree::resolve::refusal_tests::a_retained_object_ref_refuses_a_replacement_at_the_old_rectangle`.

- `tree::resolve_twin_live_tests::resolve_live_files_view_options_nodes_each_resolve_to_their_own_node`:
  one retry passed after the parallel run returned incomplete AT-SPI evidence
  for a Nautilus menubutton. The retained same-tier
  `resolve_live_every_snapshot_ref_resolves_to_the_node_it_was_taken_from`
  checks every stable Files ref with and without bounds against a second read.
  The moved-Qt-twin owner above retains live size disambiguation and ambiguity.

- `tree::resolve_twin_live_tests::resolve_live_twin_buttons_are_told_apart_by_path_and_by_stored_size_and_stay_ambiguous_without`:
  merged into the moved-Qt-twin owner above on both X11 tiers. The old negative
  fixture kept a unique retained-object token and therefore could not be
  ambiguous. The replacement explicitly tests retained identity, shared text
  plus size, and shared text alone returning `AMBIGUOUS_TARGET` with two candidates.

### Replaced cases

- `tree::qt_live_tests::qt_live_twins_resolve_to_their_own_button_after_the_window_moves`
  becomes `qt_live_moved_twin_refs_stay_distinct_and_shared_text_only_evidence_is_ambiguous`.
  Its shared-text fixture removes object/native identifiers before asking for
  ambiguity. The real moved-window checks and independent width reads remain.
- `tree::qt_live_tests::qt_live_rows_after_a_resort_resolve_to_their_own_text_or_are_refused_never_swapped`
  becomes `qt_live_recreated_rows_refuse_old_refs_and_fresh_refs_resolve_to_their_own_text`.
  The fixture destroys and recreates its rows, so requiring an old ref to
  succeed contradicted the identity contract. The replacement waits for the
  reversed replacement rows, independently checks each fresh ref's text, and
  requires every destroyed row ref to return `STALE_REF`. This replaces a
  fixed 600 ms sleep with a bounded condition wait.

### Replaced fixture assumptions in retained cases

The eight initial Mutter failures all passed their sole isolated rerun. Keep
both Chrome menu cases: their Escape helper is routed by the private AT-SPI
registry, so the Mutter helper now updates bus activation `DISPLAY` before
starting the shell or primer. The registry must never inherit the outer Xvfb
display while the applications use Mutter's display.

The three Unicode typing and three key-combination cases in
`tree::physical_live_tests` now establish the fixture's viewable active window,
click the entry, and wait for its frame to report active instead of sleeping
300 ms. The separate Mutter raw-click/Overview case still proves activation
by a raw click. The three wheel cases wait for available bounds, actual
movement and return, with stable observations after the requested condition;
an unchanged pre-event sample is no longer accepted as completed movement.
`grab_live_describes_the_window_and_names_each_refusal` compares root coordinates
with an independent X11 query instead of assuming the requested origin survives
the WM; exact size, map state, parent and every refusal assertion remain.

The GTK menu-grab fixture maps both windows before a file handshake requests
the popup. Its readiness flag follows the popup call and mapped-state check;
the host also waits for the subject's inactive AT-SPI frame before testing the
refusal and unverified global delivery. This replaces the 600 ms popup timer,
1.2 s / 800 ms setup sleeps and 500 ms log sleep. Button and wheel fixtures use
the same explicit active-window setup; the button still receives exactly one
click, and wheel origin is sampled after setup. Private GNOME animations are
disabled so window-map transitions cannot move a fixture underneath its input.

## Safety rules

Each worker owns a display, session bus, AT-SPI bus, 0700 runtime directory,
XDG config/data/cache, temporary directory and `AGENT_DESKTOP_HOME`. Display
selection is locked while Xvfb starts. Mutter startup maps an owned primer,
waits for its map event and for Activities Overview to close, then removes the
primer before tests start. The inherited live display, authority
and accessibility overrides are cleared before any desktop app can launch.
Cleanup only signals owned child/session PIDs. It never restarts the live
shell or accessibility bus. Runtime directories are moved to Trash when the
box has `trash`; otherwise they are retained under `~/ad-wt`.

The Python boundary checks need no display:
`python3 scripts/test-linux-live-gate.py`.
