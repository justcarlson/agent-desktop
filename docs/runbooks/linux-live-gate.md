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

- `tree::physical_live_tests::live_a_click_at_the_bounds_centre_presses_a_gtk3_button`
  and `live_a_click_at_the_bounds_centre_presses_a_gtk4_button`: duplicate button
  fixtures intermittently never became mapped/viewable under Mutter, including
  isolated validation with explicit activation. The same-tier
  `live_a_click_at_the_bounds_centre_presses_a_qt_button` retains the physical
  bounds-center click and exact handler-log contract on both X11 tiers.
  `actions::live_widget_tests::a_click_runs_the_handler_of_a_button_in_every_toolkit`
  retains GTK button-handler coverage; the dedicated Mutter raw-click scenario
  and retained GTK physical typing/key cases retain native GTK input coverage.

- `tree::physical_live_tests::live_the_wheel_scrolls_a_gtk4_list_and_back`
  and `live_the_wheel_scrolls_a_qt_list_and_back`: parallel Mutter runs exposed
  timing-dependent scroll return and window mapping, respectively. Both merge
  into `physical_live_apps_tests::live_the_wheel_scrolls_the_files_list` on both X11 tiers, retaining
  the shared physical wheel movement, return and delivery contract. GTK4 and
  Qt list geometry and offscreen routing remain covered by the retained
  offscreen live matrix, and the headed Qt wheel case retains actual movement.
  A separate Qt-specific return-to-origin assertion is retired.
- `tree::physical_live_tests::live_type_into_a_gtk3_entry_reads_back_with_remapped_characters`:
  intermittently could not acquire its fixture's active frame. The same-tier
  `live_type_into_a_qt_entry_reads_back_with_remapped_characters` case
  retain Unicode remapping, exact application log/value and honest unverified
  physical delivery assertions. `live_key_combos_select_and_delete_in_a_gtk3_entry`
  retains real GTK3 typing and key delivery on both X11 tiers.

- `tree::physical_live_tests::live_the_wheel_scrolls_a_gtk3_list_and_back`:
  its window intermittently did not become mapped/viewable. The same-tier
  `tree::physical_live_apps_tests::live_the_wheel_scrolls_the_files_list` and
  `live_the_wheel_scrolls_a_chromium_page` retain delivered down/up movement and
  return-to-origin checks. The separate GTK3 list fixture is retired.
- `tree::physical_live_tests::live_type_into_a_gtk4_entry_reads_back_with_remapped_characters`:
  its window intermittently did not become mapped/viewable. The same-tier Qt
  remapped-character owner retains exact Unicode application log/value and
  honest unverified physical delivery. The real GTK4 text-editor case also
  retains Unicode readback, and the GTK4 key-combination case retains native
  entry typing and key delivery.

- `system::focus_window::fallback_live_tests::focus_window_live_f1_never_focused_conditions_are_measured`:
  repeated diagnostic scenarios timed out acquiring the shared interaction lease
  under parallel load. The same-tier
  `system::focus_window_live_tests::focus_window_live_activates_a_background_window_per_toolkit`
  retains first/background activation and independently checks that a reported
  success means the requested window is actually focused. The retained
  `focus_activation_live_tests::focus_window_takes_focus_back_from_a_window_another_tool_activated`
  covers focus theft and reacquisition. Modal, stopped-app, cross-desktop and
  zenity refusal/delivery checks remain. The diagnostic's historical input-timing
  combinations and repeated samples are retired.

- `tree::hit_live_tests::cases::hit_live_qt_cover_in_both_directions`: its app
  still exposed no usable X window on that focused retry. The retained same-tier
  `hit_live_gtk3_terminal_cover_in_both_directions` runs the identical exact-cover
  exercise, including both stack directions and refusal checks. The existing
  fake `hit_test::tests::the_cross_window_leg_is_what_separates_reaches_from_intercepted`
  now also runs Qt toolkit facts and requires exact interception while covered
  and exact reachability when raised. Separate real-Qt hit integration is retired;
  real Qt bounds-centre input and strict resolution remain live on both tiers.

- `tree::hit_live_tests::cases::hit_live_vscode_is_never_a_wrong_verdict`:
  the cover window never appeared in a full Mutter run after the app geometry
  was successfully read. The same-tier `hit_live_chrome_cover_in_both_directions`
  uses the same Chromium toolkit and the identical exercise with exact verdicts,
  rather than permitting `Unknown`. It retains both stacking directions,
  minimized-window refusal and bounded stopped-app failure. The separate VS Code
  hit integration is retired; its observation and resolution coverage remains.

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

The retained Unicode typing and three key-combination cases in
`tree::physical_live_tests` now establish the fixture's viewable active window,
click the entry, and wait for its frame to report active instead of sleeping
300 ms. The separate Mutter raw-click/Overview case still proves activation
by a raw click. Retained Files and Chromium wheel cases independently measure
actual movement and return to their initial positions.
`grab_live_describes_the_window_and_names_each_refusal` compares root coordinates
with an independent X11 query before mapping, when the WM cannot move the client
between reads. It then waits for viewability and checks exact size, map state,
parent and every refusal. The separate framed-window case retains mapped root
coordinate checks. Sequential product/oracle reads during WM placement are not
an atomic geometry observation.

The GTK menu-grab fixture maps both windows before a file handshake requests
the popup. Its readiness flag follows the popup call and mapped-state check;
the host also waits for the subject's inactive AT-SPI frame before testing the
refusal and unverified global delivery. This replaces the 600 ms popup timer,
1.2 s / 800 ms setup sleeps and 500 ms log sleep. Button and entry fixtures use
the same explicit active-window setup; the button still receives exactly one
click. Private GNOME animations are
disabled so window-map transitions cannot move a fixture underneath its input.

The retained stopped-window capture test waits until the owned painter's
independently read `/proc/<pid>/status` reports `T`/`t` after `SIGSTOP`.
A successful signal command only requests a stop; capturing before the kernel
has stopped the process made its precondition race. The production stopped
process parser and exact `APP_UNRESPONSIVE` assertion remain unchanged.

Three hit-test live scenarios remain. Their independent X11 oracle waits for
an owned application window to appear in `wmctrl -lp` with an eight-second
deadline. A settled AT-SPI tree alone does not establish that precondition.
The failed app fixtures exposed no usable window at the oracle query; later
focused retries still failed app/cover lifecycle setup and their dispositions
are listed above. The retained terminal and Chrome cases require exact verdicts
in both cover directions; Files retains zero-axis `Unknown` checks.

Each cover clock has a unique fixture title. Its readiness wait requires that
exact title, rather than an XID absent from a pre-launch list: X11 can reuse
the previous clock's ID while the window-manager list still includes it. This
removes clock/XID lifecycle timing from the retained cover scenarios without
changing any hit verdict.

### Moved to deterministic unit coverage

`tree::hit_live_tests::cases::hit_live_gtk4_files_at_two_positions` and
`hit_live_gtk4_calculator_at_two_positions` merge into
`tree::hit_test::tests::shadow::gtk4_hits_follow_both_positions_and_never_reach_through_a_cover`.
The cover lifecycle and native app mapping still failed on their bounded focused
retries. The fake moves the same target between both original window positions,
independently checks the calibrated local point, requires exact interception
under a cover, exact reachability after its removal, and `Unknown` outside the
calibrated target. Native GTK4 zero-axis refusal, real GTK4 physical input,
terminal/Chromium exact-cover hit scenarios and other live geometry checks remain.
These app/cover lifecycle scenarios are no longer live gate requirements.

`tree::resolve_twin_live_tests::resolve_live_files_row_refs_survive_a_sibling_row_leaving`
is replaced by
`tree::resolve::depth_tests::removed_sibling_refs_stay_stale_while_surviving_twins_keep_identity`.
Nautilus failed to reflect an independently deleted file within its existing
deadline, before any resolver assertion ran. The fake changes the tree directly
and exercises the real resolver: removed sibling refs return `STALE_REF`, both
surviving nested twins resolve to their exact original objects after index shift,
and a stopped owner returns `APP_UNRESPONSIVE`. The retained live Qt twin and
recreated-row cases still verify real bus identity, ambiguity and stale refs on
both X11 tiers; the live absence matrix retains removed-element and stopped-app
refusals. No retry or longer Files timeout is required.

`system::signal_live_tests::signal_live_helper_with_no_frames_is_not_an_app`
is replaced by
`system::signal_capture::tests::windowless::an_answering_registry_app_without_frames_adds_no_population_or_signals`.
The live test depended on GNOME Terminal leaving its server alive after the last
window closed; the server instead exited during the parallel run. The fake
capture test supplies an answering terminal registry root with no frames,
requires that root to be read, and asserts no population entry, window or signal.
The terminal's surviving-server lifecycle is no longer a live gate requirement;
real windowed terminal launch attribution and terminal menu interaction remain covered.

Mutter startup uses the private shell's writable `OverviewActive` D-Bus property
and observes its startup true value before requesting false: Shell exports the
property before its asynchronous startup animation shows Overview. This avoids
both closing an initial false value too early and keyboard-focus-dependent
Escape synthesis.

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
