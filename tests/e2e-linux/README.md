# Linux live end-to-end harness

## Run

Build the CLI and its adjacent clipboard owner on Linux, then run one tier:

```sh
export CARGO_INCREMENTAL=0
cargo build --locked --release -p agent-desktop -p agent-desktop-linux --bins
AGENT_DESKTOP_E2E_BIN="${CARGO_TARGET_DIR:-target}/release/agent-desktop" \
  bash tests/e2e-linux/run.sh --tier x11-openbox
```

Use `target` in place of `$CARGO_TARGET_DIR` when the variable is unset.
The runner stages and hashes both executables together. Keep the helper beside
an externally supplied binary. Omit `--tier` to run all tiers sequentially.
Set `AGENT_DESKTOP_E2E_SUITE` to a fresh directory to choose the evidence location.
On the shared validation host, put it under `$HOME/ad-wt` and enter only through
`scripts/remote-linux.sh -c`; bound each call with `AGENT_DESKTOP_REMOTE_TIMEOUT`.
The runner starts its own private sessions and takes the interaction lease.

On the shared host, run build and live validation as separate bounded calls
from this checkout. Timeouts are whole seconds; the first bounds execution,
the second bounds waiting for the shared box lock. Each call below is under
15 minutes including lock wait. The single-leg run creates its own fresh suite:

```sh
AGENT_DESKTOP_REMOTE_TIMEOUT=720 AGENT_DESKTOP_REMOTE_LOCK_TIMEOUT=60 \
  bash scripts/remote-linux.sh -c 'export CARGO_INCREMENTAL=0 && cargo build --locked --release -p agent-desktop -p agent-desktop-linux --bins'
AGENT_DESKTOP_REMOTE_TIMEOUT=720 AGENT_DESKTOP_REMOTE_LOCK_TIMEOUT=60 \
  bash scripts/remote-linux.sh -c '
    suite=$(mktemp -d "$HOME/ad-wt/e2e-docs.XXXXXX")
    AGENT_DESKTOP_E2E_SUITE="$suite" \
    AGENT_DESKTOP_E2E_BIN="$CARGO_TARGET_DIR/release/agent-desktop" \
      bash tests/e2e-linux/run.sh --tier x11-openbox --leg clipboard.text_roundtrip
  '
```

A lock-wait expiry is infrastructure contention before any test runs. Keep
failed suites for diagnosis; a successful suite is moved to Trash by the runner.
Use another fresh suite for the sole diagnostic rerun. Do not reuse a suite
with a different binary image. `run.sh` adds the extracted tool directory to
PATH; Qt launches through the venv path below without activating it in the shell.

Prerequisites include Python 3 with PyGObject, the GNOME apps, Xvfb, DBus,
AT-SPI, openbox, xdotool, wmctrl, xclip, xwd, wl-clipboard, gnome-shell, and
recoverable `trash` cleanup. Extracted tools may live under
`~/.local/ad-tools/root/usr/bin`; openbox defaults to
`~/ad-wt/wm-pkgs/root` (`AGENT_DESKTOP_WM_ROOT` overrides it).
Qt legs use `~/.local/ad-tools/qtvenv`.
A missing prerequisite exits 2 before any leg runs.

## Tiers

- `--tier x11-openbox`: private Xvfb and openbox.
- `--tier x11-mutter`: private Xvfb and the private mutter shell wrapper.
- `--tier wayland`: private bus and a health-gated headless GNOME Wayland shell.

Each tier runs the headless and headed modes registered for that leg. A headed
mode on a headless compositor means synthetic input inside that private session.
It does not mean input to the user's desktop.

## Legs

Use `--leg <id>` with the same tier to reproduce a single registered leg:

```sh
AGENT_DESKTOP_E2E_BIN="${CARGO_TARGET_DIR:-target}/release/agent-desktop" \
  bash tests/e2e-linux/run.sh --tier x11-openbox --leg clipboard.text_roundtrip
```

`runner/legs.py` is the inventory of IDs, modes, requirements, and oracles.
Pass `--leg id1,id2,id3` to run a bounded batch in one private tier session.
Every requested ID must be registered for the selected tier; a mismatch exits 2.
A pass requires independent before/after observations, restoration where state
changed, and the same private process membership after cleanup. A refusal leg
must assert its expected error and absence of effect.
Unimplemented, blocked, outstanding, failed, and unavailable-tier outcomes exit 1.
Outstanding surface legs still need their owning product surface to land.

To list registered IDs, modes and dispositions for a tier without starting apps:

```sh
python3 -I - <<'PYLIST'
import sys
sys.path.insert(0, "tests/e2e-linux")
from runner.legs import REGISTRY
for leg in REGISTRY:
    if "x11-openbox" in leg.tiers:
        print(leg.id, ",".join(leg.modes), leg.disposition, leg.reason)
PYLIST
```

Change the tier string to `x11-mutter` or `wayland` for those inventories.
A single ID executes all its registered modes; it is not a mode selector.

## KDE manual matrix (3.15)

The Live Tiers above name GNOME only. A Plasma 5.24+ session is not hosted by the
runner, so the same surface is walked by hand on a real Plasma desktop and stays
owned by 3.15 until that walk is recorded. The table is an acceptance checklist, not copy/paste shell commands; the
receiving 3.15 run must choose concrete apps/refs and record its setup. Use the
`agent-desktop` executable prefix for every product command. Pointer, drag and
notification mutation commands require global `--headed`; notification actions
also require the list fingerprint documented by the shipped command reference.
The `_NET_ACTIVE_WINDOW` oracle applies to X11 only; Wayland uses independent
raw window ACTIVE/focus reads and exact documented refusal outcomes.

One row per cell; a cell that cannot run
on the session is written down with its reason, never left blank.

| Area | Steps | Expected shape | Record |
| --- | --- | --- | --- |
| Snapshot | `snapshot --app <app> -i` on a Plasma window | refs allocated, `complete: true` | pass / finding with the raw tree |
| Semantic click | `click <ref>` on a Plasma button | the button's own effect, read back from the app | pass / finding |
| Wheel and drag | `mouse-wheel` and `drag` over a Plasma window | pointer moves and the app reacts | pass / finding |
| Focus | `focus-window` on a covered Plasma window | `_NET_ACTIVE_WINDOW` names the target | outcome set, not a single value |
| Notifications | `list-notifications` after `notify-send` | the sender's summary appears once | pass / finding |
| Dismiss | `dismiss-notification 1 --expected-app <app>` | the banner is gone and no other banner moved | pass / finding |
| Tray | `snapshot --surface system-tray` with an SNI item registered | the item is listed and `click` reaches `Activate` | pass / finding |
| Menu surface | open a Plasma menu, `snapshot --surface menu` | item count equals the raw showing count | pass / finding |
| Wayland | repeat Snapshot, Semantic click and Focus under Plasma Wayland | focus refusals match the landed X11/Wayland decision | pass / finding |

## Reading a failure

The printed report groups every outcome by tier and mode. Failed runs retain
`report.txt` and per-leg JSON evidence in the printed suite directory. Evidence
includes product argv, response envelopes, oracle reads, and process membership.
Read `report.txt` first, then pretty-print the matching
`evidence/<tier>/<mode>/<leg-id>.json` with `python3 -m json.tool`. Its
`exception_traceback`, `products`, `reads`, and `owned_baseline`/`owned_final`
show whether failure occurred before dispatch, at the effect check, or during
restoration. A matching deterministic product failure needs the same command,
preconditions and independent effect/no-effect reads on the diagnostic rerun.
A raw-bus, launch, timing or service-membership failure before product dispatch
is a fixture/environment failure; preserve its evidence when retiring it.

Rerun a failed leg at most once to distinguish a deterministic product defect
from timing, load, session setup, or environment behavior. Delete an unstable
leg or replace it with a deterministic fake or an observed-state wait. Record
the reason in a `test:` commit. Keep unmeasured surface acceptance
outstanding with its explicit reason. Fix product code only for a deterministic
product defect, especially wrong-target input or false delivery evidence.

For a bounded negative control, use the documented remote-run form with a
fresh suite and payload `bash tests/e2e-linux/run.sh --tier x11-openbox --leg
state.text_editor_semantic_type --lying-actions`. This is an expected failure.
Run `--lying-actions` as a negative control: its fake program always says
`ok:true`, and independent effect checks must still fail. Send SIGTERM to the
runner's own PID to test cancellation; cleanup must leave no private members.
Identify that PID and its start token in your private runtime's `harness-owner`
marker, verify it is your `runner/main.py` process, and signal only that PID.
The background shell/interaction-lock wrapper PID is not the runner PID.
After it exits, verify that runtime directory is gone and no process environment
references it. Cancellation is a separate live check; the negative-control
command alone does not prove cleanup.
The next run sweeps abandoned sessions whose recorded runner has exited.

The portable self-tests need no Linux session:

```sh
python3 -I -m unittest discover -s tests/e2e-linux -p 'test_*.py'
python3 -I -m unittest discover -s tests/e2e -p 'test_*.py'
```

## Ref-action contracts

The `ref_actions.*` legs observe and restore native application state:

| Action | Independent effect |
| --- | --- |
| Focus | Qt Text entry gains `FOCUSED`; the prior button regains it on restoration. |
| Double-click / triple-click | Text Editor selects a word / line, read through AT-SPI Text selection offsets. |
| Right-click | Text Editor exposes a showing `Context menu` and its menu-item children. |
| Hover | The existing Qt probe's native status tip appears and clears when the pointer leaves. |
| Expand / collapse | Calculator's Main Menu changes `EXPANDED` and showing menu children together. |
| Scroll | Settings Hot Corner leaves the viewport and Dynamic Workspaces enters it. |
| Scroll-to | Settings reveals the offscreen monitor checkbox without changing its checked state. |
| Drag | Text Editor gains a nonempty text selection. |

Headless focus, double-click, triple-click, hover, scroll and drag are separate
`asserted-refusal` rows requiring `POLICY_DENIED`, no app effect and unchanged
pointer/focus. Right-click, disclosure and scroll-to have semantic success legs
in both modes. These replacements do not establish Files tree disclosure or
300-file realization; those original real-app requirements remain unproved.

Raw pointer delivery legs also read the actual Calculator digit, Text Editor
selection or scrollbar position, then assert out-of-screen refusal has no app
effect and restore the original state. Fabricated reports and pointer movement
alone cannot pass. Text Editor is seeded in memory through raw EditableText;
GTK4's unsupported selection-removal and character-geometry methods are not
used. Selection cleanup sends Right only with the exact owned window active.

The abandoned Files targets lacked the expected expandable row and caused
private indexing-service churn. File-backed editor startup and the inaccessible
Qt tooltip were replaced by the deterministic targets above. Desktop readiness
waits for the private transient extractor to exit before taking a leg baseline,
with a finite deadline; it does not extend accessibility-call timeouts.
Launch discovery reads frame children only for the requested application's
aliases or executable name, while retaining private-runtime and process-generation
checks. A stalled bystander or a newly mapped Mutter decoration cannot substitute
for the requested owner.

The ref-action refresh measures only its selected cells:

| Tier | Headless effects | Headless asserted refusals | Headed effects | Raw headed delivery legs |
| --- | ---: | ---: | ---: | ---: |
| Openbox X11 | 4 | 6 | 10 | 3 |
| Mutter X11 | 0 | 0 | 0 | 0 |

The owner rule removes all 23 touched Mutter mode cells after bounded diagnostic
retries: all ten ref actions in both modes and the three raw headed delivery
legs. Bounded raw AT-SPI calls timed out, including all eight cells in the final
selected batch. Earlier positive samples did not reproduce reliably and are
not retained live passes. Openbox exercises their shared scenario contracts;
the complete Mutter ref-action and raw-delivery matrix remains unproved.
Other harness rows were not rerun.

## Removed live checks

The validation refresh removes stock-app and timing assumptions from the live
inventory. Pure helper tests remain executable against fakes; they do not count
as live passes. Core fake-adapter tests cover exactly-once dispatch, disabled
target refusal, and snapshot ref-map replacement.

| Removed checks | Reason | Retained coverage |
| --- | --- | --- |
| Undo and delayed-enable | GTK action coalescing and wall-clock timing | Core fake dispatch/wait and disabled-target tests |
| Files drilldown, disclosure, scrolling, Calculator history | Runtime directory churn or absent version-specific roles | Ref-map, disclosure, scroll planning and zero-geometry unit tests |
| Electron | Private launcher never registers a window | No Electron live claim |
| Headed click/toggle and generic pointer gestures | Numeric-name/tab counters or incomplete raw state do not prove the effect; pointer restoration missing | Deterministic geometry and delivery-report tests; dedicated pointer delivery legs |
| Headed combo success and Super/Win | Unsupported semantic selection and explicit key policy refusal | Select refusal/no-effect leg |
| Redundant headed value write | Boolean restoration compared with backing-key string | Backing-key write/reopen/restore leg |
| Search/replace and unfocused typing | Static-name/first-field matching and longest-text toast are invalid oracles | Fake text/input contracts; direct document typing leg |
| Capture equality | Exact idle/restoration equality is blink-sensitive; covered/minimized windows have explicit refusal outcomes | Correct TrueColor XWD decoder, one live window comparison, and fake capture/restoration tests |
| Duplicate-window cleanup, close-app restoration, second-app modal | Ambiguous cleanup or missing restoration | Identity/signal and delivery unit tests |
| Wayland input, focus, capture and clipboard checks | No keyboard seat, raw query timeouts, invalid CLI/oracle assumptions | Fail-closed focus/list-window live legs; deterministic input and clipboard tests |
| Event/dialog and panel checks | Late private service churn, GTK About ordering, omitted app scope, wrong event-name expectation, or missing restoration | Fake signal/focus contracts and retained quiet/transition legs |
| Separate unreachable-bus fixture | Private oracle-bus startup depends on environment | Permission mapping unit tests |

## Validation repairs

These are harness defects fixed during the rebase validation. Each named test
fails when its repair is removed and passes after restoration.

| Reproduction | Cause and disposition |
| --- | --- |
| `test_pixels_decode.py`, `test_truecolor_uses_masks_even_when_xwd_contains_color_records` | Fixed: TrueColor pixels use channel masks even when XWD contains color records. The retained `capture.window_pixels_match_xwd` leg passes under openbox. |
| `test_scenarios_c_contract.py`, `test_outside_screen_probes_reach_argument_validation` and `test_a_parser_refusal_is_not_an_outside_screen_measurement` | Fixed: negative flag values reach runtime bounds validation; a parser error cannot count as delivery evidence. |
| `test_oracle_apps.py`, `test_owner_scan_ignores_retired_bus_names_but_preserves_other_errors` | Replaced the stale-owner liveness assumption with a fake registry contract and observed-window polling. Other bus errors still propagate. |
| `test_session_contract.py`, `test_signal_rejects_reused_pid_and_changed_runtime` and `test_shell_cleanup_revalidates_generation_and_exact_runtime`, and `test_tracked_pid_cleanup_does_not_signal_a_foreign_runtime` | Fixed: cleanup carries and revalidates process generations and exact private runtime membership. |
| `test_runner_contract.py`, `test_reused_suite_rejects_a_changed_source_image` | Fixed: a reused evidence directory rejects a changed CLI or helper source image. |
| `test_clipboard_roundtrip.py`, `test_x11_setter_runs_in_the_foreground` | Fixed: xclip uses `-quiet` so the owned selection process remains in the foreground and can be reaped. |
| `test_capture_scenarios.py`, `test_window_oracle_excludes_independently_reported_client_shadows` | Fixed: crop XWD by independently read GTK frame extents; Mutter XWD was 822 by 642 while the content PNG was 700 by 520. |
| `test_occlusion_scenarios.py`, `test_cover_position_is_observed_inside_its_client_shadow` | Fixed: compare a moved cover's visible origin and bounds, excluding its client shadow. |

Earlier lane runs retired Mutter specimens after raw accessibility bus timeouts,
service membership churn, Qt startup/teardown or long-text restoration timing.
The current inventory restores eleven event variants after isolating unrelated
shell owners and adding a WM/AT-SPI responsiveness gate. The four ref gestures
and canonical Qt headed-click contract run on both X11 tiers. A restored row is
not a measured pass until its current receipt records one. The Files Properties opener did not require proof of its modal, so its
leg was deleted on both X11 tiers. A newly launched window may already have
focus; the unconditional never-focused refusal expectation was deleted.

A historical Mutter batch passed 32 cases and retired nine lifecycle/event
variants after raw accessibility-bus timeouts; it does not validate this head. Lifecycle resize/restoration uses
visible bounds after subtracting independently read GTK client shadows;
`test_resize_and_restore_use_visible_bounds_inside_client_shadows` proves
the request, observed effect, and restoration against a fake X11 client.

Native test deletions follow the same rule. The ambient-command clipboard
lookup test was not hermetic. Chromium action, capture, close, launch, and menu
fixtures failed before a usable private window existed; the launch process
died with signal 5 and the product reported the crash honestly. Their
deterministic routing, capture, close, and launch ownership tests remain.

The one corrected Mutter resize rerun never reached its leg: desktop warm-up
reported an owned process leaving its runtime directory. That Mutter variant
is removed rather than retried again. The deterministic visible-geometry
contract and measured Openbox resize leg remain.

The Wayland-without-input native test required a Wayland session inside the
mandatory X11 all-ignored gate. That incompatible session-precondition check
was deleted; deterministic signal timeout/baseline contracts and the private
Wayland harness refusal cases remain.

Historical pre-review lane measurements (not a receipt for the integrated head):

| Tier | Passed | Failed | Outstanding |
| --- | ---: | ---: | ---: |
| Openbox X11 | 88 | 0 | 4 |
| Mutter X11 | 58 | 0 | 32 |
| Headless Wayland | 3 | 0 | 0 |

The 36 outstanding instances need the menu/popover, shell, tray or
notification surfaces on 3.14. The refresh removes 192 of the original
377 mode instances under the owner rule. Removed checks are not passes.
Wayland retains refusal/observation checks; no headed seat-input coverage
is claimed. See the removal reasons above and the `test:` commit body.

Native environment-dependent checks removed in this refresh:

| Test | Reason |
| --- | --- |
| `missing_tools_without_a_display_name_the_package` | Fake lookup still executes an ambient host clipboard command. |
| `a_chromium_page_takes_a_click_and_a_check` | Private Chrome startup fails before an action; its one diagnostic retry fails too. |
| `grab_live_a_chromium_window_is_captured_on_its_client_unit` | Private Chrome PID never lists a window; its one diagnostic retry fails too. |
| `close_live_chromium_graceful` | Private Chrome PID never maps a top-level window before close. |
| `live_chrome_launches_as_chromium_with_a_private_profile` | Private launcher dies with signal 5; product reports the crash honestly. |
| `menu_wait_live_chrome_menu_opens_and_closes_by_escape` | No private Chrome window within the existing 40-second observed-state wait. |
| `signal_live_chrome_menu_is_not_window_opened_and_two_windows_diff_correctly` | Same private Chrome startup failure. |
| `live_the_wheel_scrolls_a_chromium_page` | Shares the unusable private Chrome fixture. |
| `offscreen_live_chrome_page_realises_an_offscreen_button` | Shares the unusable private Chrome fixture. |
| `hit_live_chrome_cover_in_both_directions` | Shares the unusable private Chrome fixture. |
| `signal_live_wayland_without_input_times_out_with_baseline_counts` | Wayland-only precondition in the mandatory X11 all-ignored gate. |
| `qt_live_rows_after_a_resort_resolve_to_their_own_text_or_are_refused_never_swapped` | Every replaced row correctly becomes stale; requiring a successful old ref contradicts the refusal contract. |

The shared Chromium fixture removals preserve deterministic Chromium tests;
they make no claim about the unexercised live wheel, offscreen or cover effects.

Live VS Code Chromium checks for stale refs after process exit and stable
web-content paths after a second file both passed in the native gate.

The performance search-miss probe clears the retained object before looking for
an absent name. Its old fake regression manufactured a named retained-object
ref, changed its identity, and demanded successful resolution anyway. On the
shipped resolver the sole diagnostic rerun again refused that unverifiable
identity with `resolution_identity_unknown` and `not_delivered`. The invalid
fake is deleted; no product code is changed. Canonical resolver tests retain
swapped-widget identity, replacement refusal, complete-search miss and equal-twin
ambiguity (`resolve_refusal_tests` and `resolve_tests`).

The two twin fixtures now use the same search-probe helper, clearing exact
object tokens before testing size or ambiguity. Canonical native resolver tests check that equal candidates remain ambiguous. The Qt resort fixture was
deleted: every replaced row correctly returned StaleRef, but the fixture
incorrectly demanded at least one success. Deterministic replacement-refusal
and reordered-row tests remain; no wrong-target result was observed.

Historical pre-review validation gates: 2,506 retained Linux library tests
pass across bounded private-session chunks. Twelve native checks are removed
for the reasons above. The four-package all-target Clippy gate passes with
warnings denied. Mac harness self-tests pass 271 with one skip; shared e2e
self-tests pass 58. All 64 new tests have failed under an intentional break
and passed after restoration. Formatting, Rust size, source-reference policy,
findings validation and secret scans pass. Headless/headed live passes are
69/19 on Openbox, 48/10 on Mutter, and 3/0 on headless Wayland.


## Integration review decisions

All 76 review OWNER rows are resolved in
[the decision table](../../docs/plans/3.12-integration-decisions.md): 23 live
contracts, one exact default-timeout refusal, 50 required 3.15 hardening cells,
and two removed invalid contracts (HTML setting and the Qt choice target). The separate unsupported HTML-import live leg is also retired, outside these 76 decision rows. The receiving scope lists every
transferred row in `docs/phases.md` §3.15. No transferred cell counts as a pass.
Surface/modal/shell/notification/tray live acceptance remains outstanding on the landed product. Registration is not a measured pass.

Capture, quiet events, identity queries, failed-launch/stale-ref queries,
lifecycle and non-interference now record independent final reads. Duplicate
non-interference after records are removed. Qt evidence consistently reads the
actual log and process members, including log removal after app teardown.
Transient window events match the independently observed frame and owner PID;
registry-derived reads are labeled AT-SPI. Controlled Qt activation
and seatless Wayland semantic-click tests
reject lying successes in portable tests and have been invert-checked.

Two integration-only structural tests are retired: the source AST pairing
scanner could not see cross-module context-manager observations, and the lane
source restriction rejected legitimate implementations after lane integration.
Runtime verdict tests and executable registry coverage retain the actual
contracts. The assertion requiring an unlanded surface dependency is also removed: the rebased product includes those surfaces, while unmeasured acceptance remains nonzero. The owner-retirement splitter test is removed with that obsolete
mechanism; there are no executable OWNER rows to split.

## Integration validation receipt

The rebased release CLI and adjacent clipboard helper build successfully. The
four-package all-target Clippy gate passes with warnings denied. The library
run exposed the obsolete fake described above; its sole retry reproduced the
invalid success expectation. Final library validation follows its deletion.
Live validation of this inventory is in progress; no pending cell counts as a
pass. Historical lane numbers above do not certify this head.

| Tier | Mode | Pending executable cells | Outstanding surface cells |
| --- | --- | ---: | ---: |
| Openbox X11 | headless | 104 | 10 |
| Openbox X11 | headed | 38 | 2 |
| Mutter X11 | headless | 81 | 30 |
| Mutter X11 | headed | 23 | 10 |
| Headless Wayland | headless | 5 | 0 |
| Headless Wayland | headed | 1 | 0 |

The supplied Mutter diagnosis is implemented: exclude attributed shell/decoration
processes before property reads, isolate unrelated name timeouts while retaining
plausible target failures, resolve explicit unique bus names directly, and bound
WM/AT-SPI responsiveness before baselines and after mapping. Overview is closed
only on observed true; an expired gate makes the tier unavailable. Portable
regressions in `test_runtime_readiness.py` and the mapped-launch test in
`test_oracle_apps.py` fail under intentional inversions.

Local gates pass: portable Linux harness 314 tests (one live-only skip), shared
E2E 58, Python files below 400 lines, source-reference policy, findings validator
(701 rows), and Lahfir author/committer checks. Final secret scanning uses
`7dc6500f..HEAD`. Live stranger dogfood is pending.

## Rebase resolutions

Rebased with rerere onto shipped core-delivery `7dc6500f`.

| Conflicted file | Resolution |
| --- | --- |
| `probes/linux/FINDINGS.md` | Keep both shipped L32 findings and harness L34 measurements. |
| `scripts/private-session.sh` | Keep cancellation/member sweeping, private `ad-home`, and recoverable cleanup with Trash data under the owned base. |
| `scripts/private-wayland-shell.sh` | Keep recoverable socket cleanup and the socket value shared with the payload; the portable wrapper test catches a local variable hiding it. |
| `scripts/remote-linux.sh` | Keep the single outer timeout around lock and execution, plus the separate bounded flock wait. |
| `crates/linux/src/system/capture/capture_live_tests.rs` | Preserve the upstream Qt fixture removal and the harness branch's obsolete Chromium fixture removal. |
| `crates/linux/src/system/menu_wait_live_tests.rs` | Keep upstream outcome diagnostics and remove imports for retired Chromium checks. |
| `crates/linux/src/system/signal_live_surfaces_tests.rs` | Keep upstream outcome diagnostics and the branch's retired Chromium surface checks. |
| `crates/linux/src/system/signal_live_tests.rs` | Keep upstream outcome diagnostics and remove the mismatched-session Wayland check and its unused import. |
| `tests/e2e-linux/test_runner_contract.py` | Keep tests moved into the workflow contract module; preserve behavioral lock timeout coverage there. |
| `tests/e2e-linux/runner/legs.py` | Combine the ref-action and gap inventories without losing either family. |
| `docs/phases.md` | Preserve shipped shell/notification/tray contracts and the explicit outstanding harness follow-through. |

## Current integration findings

| Repro | Observation and disposition |
| --- | --- |
| `--tier x11-openbox --leg clipboard.html_roundtrip` | Both attempts offered only text/html; auto reads no supported content. Removed this unsupported import expectation and its permissive fake HTML fallback. `formats::TEXT_TARGETS`, `targets::choose_target`, and `x11_read_tests::an_owner_without_the_wanted_target_reads_as_absent` define the intentional supported-target boundary. Text/image/lifetime/clear legs remain. |
| `--tier x11-openbox --leg gaps.display_value_select_success` | Both modes and their sole rerun find zero raw role-11 nodes named Delivery choice; no product select dispatch occurs. Removed the unavailable Qt selection leg and its mocked positive-select check. Native `select_tests`, `select_search_tests`, and the existing Qt write_ignored check retain route, identity, effect and refusal coverage. |
| `--tier x11-openbox --leg values.workspace_count_commit` | Both modes and their sole rerun return STALE_REF/resolve_no_candidate, complete=true, preparatory_scroll=may_have_changed_view, process_state=running, not_delivered/safe. The GSettings value is unchanged and restored. Retained as a deterministic product finding, owned by 3.15 scroll/identity hardening; no resolver safety check is weakened and no positive workspace-write pass is claimed. |
