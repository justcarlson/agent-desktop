---
title: Linux Signals and Wait Parity (Sub-phase 3.11) - Plan
type: feat
date: 2026-10-18
origin: docs/phases.md
artifact_contract: ce-unified-plan/v1
product_contract_source: docs/phases.md §Phase 3 sub-phase 3.11
execution: code
---

# Linux Signals and Wait Parity (Sub-phase 3.11) - Plan

## Goal Capsule

- **Objective:** an agent on a GNOME desktop can wait for a window to open or close, an app to start or exit, focus to move, a dialog to appear or go away, and a menu to open or close, by the same `wait` envelopes as on macOS and Windows, and every `found: true` is true because the desktop was observed, never because a call returned.
- **Means:** the Linux adapter implements `capture_signal_baseline` and `wait_for_menu`, the only two seams core leaves for event waits, over 3.4's registry pass, window identity and focus decision, 3.9's process facts and liveness verdict, and the one menu detector (`system/menu_popup.rs`, `menu_surface.rs`) that the menu-surface work first wrote (KTD1-KTD16). Core already owns the diff, the poll loop, the seeded baseline, the disappearance seen-set and the timeout details.
- **Authority:** `docs/phases.md` §3.11 as corrected by 3.0-3.10, and further corrected here where the ledger or the code disproves it (R15). The pre-code probe rows `L18-1` to `L18-17` (already in `probes/linux/FINDINGS.md`) are the measurements this plan stands on; new rows from this sub-phase go in area `L33`.
- **Stop conditions:** stop if a requirement needs a change under `crates/core`; if the cut base lacks `read_registry`, `windows_of`, `read_frames`, `window_id::encode`, `process_facts.instance`, `decide_focus`, `liveness` or `process_state` (all present at 3.9 head `ba1bf9bd`); if the menu detector files (`menu_popup.rs`, `menu_reader.rs`, the detection half of `menu_surface.rs`, `tree/observe_menu.rs` `rooted` and `scan_walk`) cannot be brought from the menu-surface branch (KTD9); or if no GTK, Qt and Chromium app can run in a private session.
- **Execution profile:** step 0 (skeleton, detector import) and U1 first (U1 settles the in-window dialog question, the object-path reuse question, the Chromium menu-frame question and the stopped-versus-slow question); lane A (U2, U3 residual, U12) and lane C (U6, U7) start at once after step 0; lane B (U4, U5) codes against lane A's frozen pass interface; lane D (live legs of U9) starts with the stubs; U8 after U1, lane A and lane B; U11 after U9; U10 last, because it records the dispositions U9 and U11 produce.
- **Finishes and ships:** the implementer runs the box measurements and live verification, commits, and opens the PR stacked on `feat/linux-3.10-capture-clipboard` with `gh stack`, from branch `feat/linux-3.11-signals-wait`; nothing merges to `main`. One PR, or two stacked PRs at the U5 seam (KD4), by the measured diff.

---

## Product Contract

### Summary

Make event waits real on Linux. After this sub-phase `wait --event` answers all seven tokens (`window-opened`, `window-closed`, `app-launched`, `app-terminated`, `focus-changed`, `surface-appeared`, `surface-dismissed`) from the accessibility registry, `wait --menu` and `--menu-closed` answer from a per-toolkit open-state predicate, and every limit the desktop imposes (no `ACTIVE` state on Wayland, a menu that only a key press can close, a dialog drawn inside its window) is stated by name instead of showing up as a silent timeout. Core is unchanged. The sub-phase polls; it does not subscribe, and `watch` stays the owner's `P3-O9`.

### Problem Frame

Core's event wait is a baseline diff: it captures a `SignalBaseline` at start, re-captures every 200 ms, and diffs. It never asks the adapter what changed, only what the desktop looks like now, and it treats any capture error other than `TIMEOUT`, `ELEMENT_NOT_FOUND` or `APP_UNRESPONSIVE` as fatal (`crates/core/src/commands/wait_event.rs`). A batch also captures a baseline before the preceding command and aborts with `pre_action_baseline_failed` when that capture errors (`src/batch/execution.rs`). So the adapter's honesty is entirely in what it puts in the three lists and four booleans: an entity with no `process_instance` is dropped from the diff, a window id that renumbers is a false close plus a false open, a class marked complete when it was not is a false close of everything a slow app failed to answer, and one `INTERNAL` from a flaky bus read kills the wait or the batch.

The ledger measured the traps. GTK4 emits no `Window:Create` or `Window:Destroy`; a window opens as a child-list change on the app root and closes as `defunct` (`L7-6`), so a registry poll sees both transitions. One poll over 19 apps costs about 40 ms (`L8-1`). Two accessibles report `ACTIVE` at once (`L8-3`) and a headless Wayland session never sets it (`L13-5`). A stopped app answers AT-SPI only after 25 s (`L11-15`). `gnome-text-editor --new-window` hands off to the running process, so the new frame belongs to an old pid (`L4-26`). Action-driven changes arrive in bursts, and a GTK4 popover appears 157-331 ms after the call returns, so a 300 ms floor is a minimum, not a budget (`L7-5`, `L18-2`).

The roadmap gives 3.11 neither menu waits nor a way to open a dialog. Windows shipped `wait --menu` in its signals sub-phase, and `L18` now supplies what Linux lacked: a menu-open signal on every toolkit probed (`L18-9`), and three dialog openers reachable by AT-SPI actions alone (`L18-11` to `L18-13`). `L18` also contradicts the ledger's earlier reading that a GTK4 popover is unobservable (`L18-1` against `L4-20`, `L14-14`, `L14-8`); those three rows are not yet amended. The window and sheet shapes `L18-11` to `L18-13` found are already in 3.4's code: `is_window_shape` takes the state set and admits a `MODAL` role-39 child of the app root, and `tree::surfaces::is_sheet` is "`MODAL`, or a `SHOWING` dialog". What is left is the `SHOWING` clause on the role-39 and `MODAL` branches (an absent `SHOWING` still passes today) and the Chromium menu frame.

### Requirements

**Windows, apps, focus**

- R1. `window-opened` and `window-closed` come from one registry pass and key on (pid, process instance, window id). Every window carries a non-empty process instance and an id that is identical across polls and unique within the pid, so a survivor never renumbers into a false close plus open (`L7-6`, `L8-1`, `L8-7`).
- R2. A handoff window (a new frame under a running pid) is `window-opened` and not `app-launched` (`L4-26`). The app population is the apps that own at least one window, so a helper that answers with no frames (`gsd-*`, `ibus-*`, `L8-1`) never produces `app-launched` or `app-terminated`; an app that keeps running without a window is `app-terminated` only until core's `process_state` corroboration (3.9) suppresses it.
- R3. `focus-changed` fires from at most one focused window per capture. When the sources cannot name one (two `ACTIVE` candidates and no X11 answer), the capture marks the window class incomplete rather than guess. On a Wayland session with no input device no GTK4 window carries `ACTIVE` (`L13-5`, `L23-29`), so the event cannot occur there until a seat keyboard exists; once a RemoteDesktop session has sent its first key the focused window carries `ACTIVE`, the later-mapped window takes focus, and the event occurs (`L23-29`). The skill and `docs/phases.md` state the limit by name.

**Surfaces**

- R4. `surface-appeared` and `surface-dismissed` report a `sheet` for every dialog-like top-level of the target app: a role-16 dialog (modal or not, `L18-11`), a `MODAL` frame (`L18-12`), and a `MODAL` role-39 top-level window (`L18-13`). Keying is on identity, never on the dialog's name (`L18-13` has none).
- R5. The sheet predicate is one function used by `list_surfaces`, `snapshot --surface sheet` and the signal capture, so a surface that is announced can be rooted. A dialog is listed by `list_surfaces` as both a `window` and a `sheet` under one id (the Calculator About, `L23-15`); the capture emits it once, as a `Sheet`, and its window as `window-opened`, so a seeded diff never counts one dialog twice. This sub-phase emits `Sheet` only, and the emitted kinds are pinned by a test. `Menu` is advertised and signalled by the menu-surface work (`supported_surfaces`, `list_surfaces`, and one call into `signal_surfaces`), which rewrites that pin in the same change; until it lands `supported_surfaces` stays `Window` and `Sheet`.
- R6. A surface scan runs only when the filter names an app or a process, because core rejects unscoped surface events; an unscoped capture reports surfaces as empty and incomplete.
- R7. An in-window dialog (`L18-14`) is reported if and only if U1 shows both transitions observable by a bounded scan; otherwise its absence is a named limit in `docs/phases.md` and the skill source, not a quiet gap.

**Honesty, errors, deadlines**

- R8. Completeness is derived from what the pass read. `APP_UNRESPONSIVE` naming the app is returned only for a scoped target that is confirmed stopped, by the evidence 3.4's `unresponsive::stopped_target` uses (the root did not answer and `/proc` shows the stopped state), so core stores it as `last_error` and the timeout shows it. A scoped target that is alive but slow (a read ran out its slice, the process is not stopped) is not an error: the capture marks the window, app and surface classes incomplete and returns `Ok`, so a batch seed is not aborted by a busy Electron app. An unscoped capture with an unreadable app marks the same classes incomplete. A frame or a child list that timed out or failed for a reason other than "gone" is unread, never empty. `complete()` is never the default (`L11-15`).
- R9. Every error that leaves `capture_signal_baseline` is `TIMEOUT`, `APP_UNRESPONSIVE` (a confirmed stopped scoped target only), or the bus-unavailable `PLATFORM_NOT_SUPPORTED` that the landed bus fault mapping returns, which stays fatal. A dropped connection is forgotten so the next poll reconnects. A capture on a healthy desktop never fails a batch.
- R10. The capture honours the caller's deadline: filter by process and app before any frame read, bound every registry and frame read by an explicit short slice, and return `TIMEOUT` rather than publish a capture that finished late. The true cost is stated and measured, not "zero": `read_registry` reads the name and toolkit of every registered app that is not stopped (a stopped app's name comes from `/proc`, so it costs no slice), so a name filter pays at most one concurrent `ROOT_SLICE` (300 ms) for apps that are alive but silent (`L33-7`: 322/326/347 ms under another app's name filter, 614/620/626 ms unscoped, because an unscoped pass also spends one `CHILDREN_SLICE` on the silent app's frames); a process filter skips non-matching pids before those reads; `read_frames` is never asked for an unselected root, so frames cost at most one `CHILDREN_SLICE` plus one `FRAME_SLICE` for the selected apps.

**Menu waits**

- R11. `wait --menu` is satisfied when the target process has an open menu by a stateless, per-toolkit predicate; `--menu-closed` when it has none. The predicate is the one detector `menu_popup::in_window` and `menu_popup::top_level` already implement (a `SHOWING` role-33 or role-41 menu with at least one `SHOWING` item; top-level GTK3 context menus and Chromium menu frames; web content skipped), so `wait --menu`, `list-surfaces` and `snapshot --surface menu` cannot disagree. It covers GTK4 popover menus, GTK3 menu bars, Qt menus and Chromium menus (`L18-1` to `L18-7`, `L18-9`) and excludes the idle-true traps (Qt menu-bar items `SHOWING`, GTK3 menu items present while closed, Chromium `HAS_POPUP` on idle nodes). The GTK3 header-bar popover (`L18-5`) is not in that detector today and is added to it (KTD9, Rule B).
- R12. An already-open menu satisfies `--menu` on the first read, and an already-closed state satisfies `--menu-closed` on the first read, as on Windows. The loop polls at 50 ms, re-verifies the process before the read and again before success, absorbs a predicate timeout, consults the deadline only after a mismatch, and turns a vanished target into `STALE_REF` with `not_delivered` and process-oriented recovery.
- R13. There is no fixed settle anywhere. Waits are deadline-bounded polls at 50-100 ms; the 300 ms figure is a floor the polling cadence already clears (`L18-2`).

**Process and proof**

- R14. `crates/core` is unchanged, no shipped source names a plan, phase or decision id, and no probe script or capture enters the repository. Pure decisions take plain values and run on every host; every other requirement is proved by a real run on the GNOME box in a private session (`scripts/private-session.sh`), each test invert-verified.
- R15. `docs/phases.md` states what was built and no longer states what `L18` or the code disproved: §3.11's scope and exit criterion and §3.12's popover and dialog sentences (§3.4's window shape and sheet sentences and the `L18-1` to `L18-17` rows are already corrected and present). `probes/linux/FINDINGS.md` gets the amendments to `L4-20`, `L14-8` and `L14-14` and this sub-phase's rows in area `L33`, every ledger row naming 3.11 is disposed of, and what core would need is written as owner items.
- R16. A stranger dogfood is run and its report committed with at least one finding, each disposed of. Latency is measured min-of-seven with the warm-up discarded, and the release binary stays under 15 MiB.

### Key Decisions

- KD1. **Core stays platform-neutral.** (session-settled: user-directed — chosen over adding a per-event capability query, a `Menu` snapshot surface or a partial-class completeness to core: core is the shared surface.) Governs R14. The three things this leaves unsolved are written into `docs/phases.md` as owner items (U10), not built.
- KD2. **Tests fail for a real reason, and real runs outrank suites.** (session-settled: user-directed — chosen over broad mocked suites: probes and captures are never read by tests and never committed.) Governs R14. Real GNOME, Qt and Chromium apps open the menus and dialogs; the live harness is 3.12's.
- KD3. **All box access goes through `scripts/remote-linux.sh`, never `sudo`, and live verification runs inside `scripts/private-session.sh`.** (session-settled: user-directed — chosen over the live bus, which is broken until the owner restarts it, and over a hand-rolled session that leaks the launching display into D-Bus-activated apps, `L18-17`.) Governs R14.
- KD4. **The PR stacks on `feat/linux-3.10-capture-clipboard` through `gh stack`; nothing merges to `main`; one PR or two by measured size.** (session-settled: user-directed — chosen over a PR against `main`; the split is decided by the merged diff against the 2,000-line guideline, at the U5 seam, not before the code exists.)
- KD5. **Every `docs/phases.md` statement the evidence disproves is corrected in place, as its own unit, citing rows.** (session-settled: user-directed — chosen over leaving it for the next planner.) Governs R15.
- KD6. **The stranger dogfood is a gate.** (session-settled: user-directed — chosen over an author-run check: the operator gets only the shipped skill and the built binary.) Governs R16.
- KD7. **Poll the registry; do not subscribe.** (session-settled: user-directed — chosen over an AT-SPI match-rule subscription, which the settled bus design excludes: one shared blocking zbus connection, no tokio, no atspi crate, every wait bounded by the caller's deadline.) Governs R1, R10. The evidence is in KTD2.

### Scope Boundaries

- No `watch` command, no AT-SPI event subscription and no match rules (`P3-O9`); `--element --predicate`, `--window` and `--text` waits are core polls over 3.4 and 3.5 and need no code here; `--notification` waits belong to 3.14. No screenshot or clipboard (3.10), no lifecycle (3.9), no input (3.8), no live harness (3.12), no Linux platform skill (3.13).
- Considered and not built:
  - **A subscription for the event waits or the menu wait.** See KTD2. Evidence that would change the call: a dogfood finding that a transition shorter than a poll interval matters, or the owner adding the core `watch` command.
  - **Emitting `Menu` as a surface signal and rooting `snapshot --surface menu`.** Both belong to the menu-surface work (3.14: `Menu` in `supported_surfaces` and `list_surfaces`, `observe_menu`, menu refs, and one call into `signal_surfaces` that rewrites the kinds pin). Emitting it here without the snapshot side would repeat Windows' F13 (an announced surface that cannot be snapshotted). `wait --menu` is this sub-phase's menu contract and uses the same detector, so the two cannot disagree.
  - **Carrying last-known windows across polls for an unreadable app.** It turns the adapter into a stateful cache whose staleness is itself a false event source. A scoped wait reports the unreadable app by name instead.
  - **Detecting menus inside web content.** A page can hold an always-visible ARIA menu, so web content would make the predicate constant-true. Browser chrome menus are top-level frames and are covered.
  - **Closing a menu.** `DoAction` closes GTK4 and Qt popovers and the GTK3 header popover, but not a GTK3 menu bar or a Chromium menu (`L18-10`); closing those is a 3.8 key press, and `--menu-closed` only waits.
  - **`Collection.GetMatches` as the menu read.** Adopted only if U1 shows it answers on all five toolkits and is at least three times faster than the detector's pruned reads (`menu_reader.rs`: 2000 nodes per window, 80 per top-level candidate), and then it replaces those reads inside the detector, not beside it (KTD9).
- Deferred to follow-up work, written into `docs/phases.md` in U10: a per-event-kind capability query so an adapter can refuse `focus-changed` by name on Wayland (core); a partial-class completeness so a hung app does not blind an unscoped wait (core); core's `identity_unknown_error` answering `APP_UNRESPONSIVE` for a node that vanished during churn while the app is not stopped (core, routed from the resolution work); VS Code web-content menus (`L18-8`, 3.12). `snapshot --surface menu` is the menu-surface work's, not deferred from here.

---

## Planning Contract

### Assumptions

- 3.4 is landed and the plan reads it as it is at 3.9 head `ba1bf9bd`: `registry::read_registry(bus, deadline) -> Registry { apps, skipped }` (each `RegistryApp` has `root`, `name`, `registry_name`, `answered`, `facts.instance`, `toolkit`, and `matches(query)`); `window_ops::windows_of(bus, registry, select, deadline)` (skips `is_listable` failures and stopped pids, calls `read_frames`, then `window_listing::collect(chosen, frames)`, which zips apps with frames by position); `window_ops::list_windows` (concurrent X11 read, `place_windows`, `mark_minimized`, `focused_index` over `decide_focus`); `unresponsive::{stopped_target, scoped_windows_of, require_known_target, settle_matches}`; `window_filter::{is_window_shape(role, interfaces, states), is_bubble, is_helper_app, frame_state}`; `tree::surfaces::is_sheet`; `window_id::{encode, decode}`; `surfaces::{surfaces_of, list_surfaces}`; `window_resolve::{verify, resolve_window_strict}`; `app_ops::{info, infos, one_entry_per_process}`. U2 to U5 modify those in place and say where.
- 3.9 is landed: `liveness::liveness(pid, instance) -> Liveness { Alive, Stopped, Gone }` (a zombie and a token mismatch are `Gone`; an unreadable stat is `Alive`), `liveness::identity(pid, instance) -> Identity { Gone, Same, Replaced, Unreadable }`, `proc_stat::{parse, is_stopped, ProcState}`, `process_facts::{read, lookup, instance_token}`, and `process_state::process_state` already wired in `system/adapter.rs`. Core calls `process_state` for `app-terminated`; the menu wait calls `liveness`. The 300 ms `ROOT_SLICE` (`registry.rs`) and `CHILDREN_SLICE` and `FRAME_SLICE` (`registry_frames.rs`) are 3.4's and private; 3.9 added no probe-slice constant (its `process_state` probe reuses `read_registry`). `system/signal.rs` is 3.9's process-signal sender (SIGTERM and SIGKILL); it has nothing to do with the event signals here, so the new files keep the `signal_` prefix only for `signal_capture`, `signal_inventory`, `signal_population`, `signal_surfaces`, `signal_boundary`.
- 3.4's C2 work is landed: the window shape admits a `MODAL` role-39 child with the Component interface, `is_sheet` is "`MODAL`, or a `SHOWING` dialog", §3.4's text says so, and `L18-1` to `L18-17` are in the ledger. U3 is therefore a residual (the `SHOWING` clause if U1 item 1 supports it, the Chromium menu frame, and live proof), and U10's §3.4 and `L18` bullets are done.
- The menu detector is the menu-surface branch's work (`feat/linux-3.14-shell-notifications`, head `2bcc4c70` at this gate): `system/menu_popup.rs` (pure: `in_window`, `top_level`, `MenuFamily`), `system/menu_reader.rs` (`BusReader`: one window walk of up to 2000 nodes, top-level candidates of the app root of up to 80 nodes, web processes skip the in-window walk), `system/menu_surface.rs` (`MenuPopup`, `PopupReader`, `find_popups`, `popups_at`, `popups_of(bus, &WindowInfo, settle, deadline)`), and `tree/observe_menu.rs` (`rooted`, `scan_walk`). It is not in the stack below 3.11. Step 0 brings the detection half of those files across (KTD9); the surface half (`supported`, `open_popups` with `action_clock`, `entries`, `surfaces_of`, `observe_menu`, `menu_resolve`) stays with that branch, which drops the imported files when it rebases. This plan builds no second predicate.
- The core facts the plan relies on are unchanged: `SystemOps::capture_signal_baseline(&SignalFilter { app, process }, Deadline)` and `wait_for_menu(ProcessIdentity, open: bool, Deadline)` default to `not_supported`; `SignalBaseline { windows, apps, surfaces, completeness }`; `SignalCompleteness { windows, apps, surfaces }`; `SurfaceSignal { kind, app, pid, process_instance, id, title }`.
- The stack branch `feat/linux-3.10-capture-clipboard` exists (confirmed at this gate); the cut is from its final head, and every 3.9-head fact above is re-read at the cut.
- 3.7's `click @ref` and 3.8's `press` are how the live tests and the dogfood open and close menus and dialogs; no 3.11 code calls either. 3.7 decides in its own U1 which GTK4 popover node accepts `DoAction` (its default arm is a `click` that reports `verified: false`), so the popover openers in U9 are only as reliable as that outcome and name the node it settles. A global `press` is not focus-gated and on Wayland reports `focus_verified: false`; closers use it with the target window focused, or `press --app` through 3.9's app-targeted key dispatch.
- Core's `find_match` compares event kinds with `same_variant`, so any `SurfaceAppeared` matches whatever `surface` the CLI token defaults to; Linux emitting `Sheet` is matched by `--event surface-appeared`.
- Core's `diff_signals` skips a class unless both captures mark it complete, ranks events closed, terminated, dismissed, opened, launched, appeared, focus, and takes the first focused window as the focus.

### Key Technical Decisions

- KTD1. **Layout.** New files by the shared platform layout, one concern each, under the 400-line cap, each with its `_tests.rs`. `crates/linux/src/system/`: `signal_capture.rs` (assembly and the deadline), `signal_inventory.rs` (one pass: windows, apps, focus, per-root outcomes), `signal_population.rs` (the window-owning app rule), `signal_surfaces.rs` (sheet signals and the pinned kinds), `signal_boundary.rs` (error narrowing), `menu_wait.rs` (the loop), `menu_anchor.rs` (process to popups). The menu detector is imported, not written (KTD9): `menu_popup.rs`, `menu_reader.rs`, `menu_surface.rs` (detection half), `tree/observe_menu.rs` (`rooted`, `scan_walk`). Modified in place: `registry_frames.rs`, `registry.rs`, `window_ops.rs`, `window_filter.rs`, `window_listing.rs`, `listed_window.rs`, `surfaces.rs`, `window_resolve.rs`, `unresponsive.rs`, `focus.rs`, `tree/surfaces.rs`, `system/adapter.rs` (wiring only), `system/mod.rs` (module declarations). `ROOT_SLICE`, `CHILDREN_SLICE` and `FRAME_SLICE` become `pub(crate)`. `tree/mod.rs` needs no re-export now (the detector owns the walker call). `bus.rs` is not edited: the boundary uses `classify`'s output and the existing `SharedBus::forget`. Governs R1-R13.
- KTD2. **Poll, do not subscribe.** (Evidence: `L7-6`, `L8-1`, `L18-1`; rejected: a second-connection match-rule subscription.) Open and close are visible as a child-list delta, so a poll sees every transition a subscription would; one pass costs about 40 ms against core's 200 ms cadence, so the poll is cheap; a subscription could only maintain a cache read at poll cadence, since core's loop cannot be woken; a baseline must come from the registry anyway; and an unfiltered rule on the shared connection stalls it, because zbus queues 64 messages and a window churn emits about 209 `BoundsChanged` (`L7-9`). A blocking zbus `MessageIterator` also has no timeout, so a subscription would need a helper thread; every wait here is instead a `within(deadline, ...)` future or a sleep between bounded polls. The blocking-iterator question is moot for this sub-phase and belongs to `P3-O9`. Governs R1, R10.
- KTD3. **One coherent pass.** (Evidence: the apps and windows of two separate listings can disagree within one poll.) `signal_inventory.rs` reads the registry once, selects apps by the filter before any frame read (KTD10), reads frames once through `read_frames`, builds windows with `collect` (apps pass `is_listable`, which applies `is_helper_app`; that covers only `gnome-shell` and `mutter-x11-frames`, so `gsd-*` and `ibus-*` helpers stay out by owning no window, KTD5), derives the app population from those windows, and decides focus with the same `decide_focus` and X11 evidence `list_windows` uses (`focused_index`, `x11::start` and `evidence`). `list_windows` and `windows_of` are refactored to call the same pass function, so listing and signals cannot drift. Governs R1, R2, R3.
- KTD4. **Identity is 3.4's, restated as the contract.** A window is keyed by (pid, `process_instance`, id) where the id is 3.4's reversible encoding of the app's bus name and the frame's object path, and the instance is `process_facts.instance`. A handoff window is a new object path under an old pid, so it is `window-opened` only (`L4-26`). The one residual risk is an object path reused for a different window between two polls (a close and reopen faster than 200 ms), which U1 measures across 20 cycles on GTK4, GTK3 and Chromium; if reuse occurs the id gains the frame's accessible-name hash only if U1 shows it distinguishes, otherwise the reuse is a stated limit. An entity with no instance is not published (`process_facts` returned nothing: the app is `skipped`, as in 3.4). A restart changes both the process instance and the id (`w-1.3-...` to `w-1.6-...`, the bus unique-name counter) and the old id is `WINDOW_NOT_FOUND` (`L23-36`), so a restart is `window-closed` plus `window-opened` and `app-terminated` plus `app-launched`, never a renumbering. Governs R1.
- KTD5. **App population is "owns at least one window".** (Evidence: `L8-1` helpers are empty-but-answered; `L13-4` registry appearance trails launch by 0.3-1.3 s; Windows' window-owning population; `L20-1` a Qt process holds two registry connections and only one owns the windows.) `signal_population.rs` is a pure function from the listed windows; its `AppInfo` values are built as `app_ops::info` builds them (name, pid, `process_instance` from `facts.instance`, presentation `Foreground`) and merged by `app_ops::one_entry_per_process`, so a Qt process is one app. Consequences stated in the skill: `app-launched` and the first `window-opened` arrive on the same poll; an app with no window is never launched or terminated; a window-less resident app reads as `app-terminated` and core's `process_state` corroboration (3.9) suppresses it. Governs R2.
- KTD6. **Completeness is derived, and scoped and unscoped differ.** (Evidence: `L11-15`.) `read_frames` returns, per root, the nodes and a read outcome (`Read`, `TimedOut`). Today it drops silently at two levels: a `CHILDREN_SLICE` timeout or any `list_children` error returns an empty list, and inside `collect_frames` a frame whose `read_node` times out (`FRAME_SLICE`) or fails is filtered out, so a slow app that answers its child list but not one frame would publish as "that window closed". Both levels report `TimedOut`. There is no gone-versus-slow classification to preserve in the landed code, so lane A applies 3.5's `tree/absence.rs` (landed: `failure_of_name`, `failure`, `Failure { AppGone, Suspect, TimedOut, Other }`) to the root and frame failures. `list_children` already folds a failed read into an `AdapterError` (`Internal` for a failed field), so the classifier runs on the underlying call result where it is still available and, where only the `AdapterError` remains, "gone" means `WindowNotFound`, `AppNotFound` or `ElementNotFound`, and everything else is `TimedOut`. A failure that means the object or app is gone stays benign, empty and `Read`. Scoped (filter names an app or process): the target is `Err(APP_UNRESPONSIVE)` carrying the app name, pid and instance in `details` only when it is confirmed stopped, by `unresponsive::stopped_target(registry, select, proc_stat::is_stopped)` (the existing definition, shared with listing; its `!app.answered` and stopped-state test is the whole evidence). A scoped target that is alive but `TimedOut` is not an error: windows, apps and surfaces are incomplete and the capture is `Ok`, so core keeps polling, skips those classes in every diff, and a batch seed `[action, wait --event]` is not aborted by a busy app. A stopped scoped target does abort that seed as `pre_action_baseline_failed` (`src/batch/execution.rs` aborts on any `Err`), which the skill and the dogfood state. A process-scoped selector (`pid`, then `facts.instance`) is new, as in `list_surfaces`. Unscoped: an unread app sets `windows` and `apps` incomplete and the pass publishes the rest. Surfaces are complete only when the scan ran and every scanned root was read. Focus is indeterminate when there is at least one `ACTIVE` candidate and `decide_focus` returned none, computed from that same call so the branching has one copy; that sets `windows` incomplete for the capture. Core's `diff_focus` turns "baseline had a focused window, now none" into a window-less `focus-changed`, so publishing an undecided capture as complete would report a focus change that did not happen. Governs R3, R6, R8.
- KTD7. **The error boundary narrows everything.** `signal_boundary.rs` maps each failure to `TIMEOUT` (budget spent, dropped transport, a slow read that is not a stopped target) or `APP_UNRESPONSIVE` (a scoped target confirmed stopped, KTD6), and keeps only the bus-unavailable `PLATFORM_NOT_SUPPORTED` (`BusFault::into_error`, `retryable: false`; nothing in `crates/linux` emits `PERM_DENIED` on this path) fatal. `classify` yields `INTERNAL` for unexpected faults, which would abort the wait or the batch on one flaky read, and `read_registry` returns an already-classified `AdapterError` through `?`, so the boundary wraps `AdapterError`, not only `zbus::Error`. `SharedBus::forget` is called today only from `probe_registry`, and `read_registry` and `read_frames` hold an `AdapterError` by the time the boundary runs, so `is_connection_failure` (which takes a `zbus::Error`) is not reachable from the boundary: the boundary forgets the cached connection for every fault it narrows from `INTERNAL` (the next `SharedBus::get` reconnects lazily, so a healthy connection costs one reconnect). The narrowing is by cause, not by blanket: a programmer error in the decoder is a test failure, not a swallowed `TIMEOUT`. Governs R9.
- KTD8. **Sheets: one predicate, widened by evidence, mostly landed.** (Evidence: `L18-11`, `L18-12`, `L18-13`.) What is in 3.4's code: a role-16, 23 or 69 child with Component is a window, a `MODAL` role-39 child with Component is a window, and `is_sheet` is `MODAL` or (dialog and `SHOWING`). What this sub-phase adds, if U1 item 1 shows it is needed: a `SHOWING` clause on the role-39 branch and on `MODAL` (an absent state is already refused for an unreadable set), the app-root census that confirms role-39 children are 0 at idle on GTK3, Qt and Chromium and that the header popover's `MODAL` role-39 container is not an app-root child (otherwise opening that popover would read as `window-opened` under the widened shape), and, if U1 item 3 shows a Chromium menu popup frame passes `is_bubble`, a pure `is_menu_popup_frame` over the `RawNode` (a frame whose class and name say popup) that excludes it from windows, so opening a Chrome menu is not `window-opened`. The detector's own `menu_popup::top_level` works on a walked tree, which `collect` does not have, so it cannot be the window-side predicate. The landed tests (`surfaces_tests.rs`, `window_filter_tests.rs`) keep pinning the shared rule; `snapshot --surface sheet` already roots a non-modal dialog and `observe_first_owner` handles a non-sheet window. Governs R4, R5.
- KTD9. **The menu predicate is the detector that already exists; there is one.** (Evidence: `L18-1` to `L18-10`, `L32-9`; the menu-surface branch's `menu_popup.rs`.) 3.11 builds no `menu_open.rs` or `menu_read.rs`. The detector is stateless and structural: a menu node (role 33 or 41) that is `SHOWING` with at least one `SHOWING` item below it (not through a nested menu), outside any web document, found in a window's tree (`in_window`, with a menu-bar flag that names the GTK3 bar) and among the nameless, `SHOWING` top-level frames and windows of the app root whose whole subtree is menu content (`top_level`: GTK3 context menu is a role-69 window, Chromium menu is a role-23 frame). It excludes by structure the Qt menu-bar items (role 35 under a role-34 bar), the GTK3 menu items present while closed (no `SHOWING` item) and Chromium `HAS_POPUP` idle nodes (not items under a menu). Its read is `menu_reader::BusReader`: one walk of the window (2000 nodes; skipped for a web process) and a 80-node walk of each top-level candidate. `menu_surface::popups_of(bus, &WindowInfo, settle, deadline)` is the entry; `settle` is passed `false` here because the wait loop owns the cadence.
  - **Step 0 import.** From the menu-surface branch head, verbatim with tests: `menu_popup.rs`, `menu_reader.rs`, and from `menu_surface.rs` only `MenuPopup`, `MenuFamily` re-export, `PopupReader`, `scan`, `find_popups`, `vanished`, `popups_at` and `popups_of`; from `tree/observe_menu.rs` only `rooted` and `scan_walk`. `supported`, `open_popups` (it needs `action_clock`), `entries`, `surfaces_of`, `observe_menu` and `menu_resolve` are not imported. That branch drops the imported files when it rebases onto this one; the file split is the one cross-branch coordination of this sub-phase and is reported at the gate.
  - **Anchor.** `wait_for_menu` receives a `ProcessIdentity`, not a window. `menu_wait.rs` lists the process's windows through the shared pass (selected by pid, so a stopped or unrelated app costs nothing) and calls `popups_of` for each window, stopping at the first popup; a process with no window is read through `popups_at` on the app root. U1 item 5 measures whether one `popups_at` on the app root (it walks every window of the process in one 2000-node read) is cheaper and equal in answers, and the plan commits to the measured choice.
  - **Rule B, the GTK3 header popover (`L18-5`), is not in the detector.** `MenuFamily` has no header-popover family. Rule B: a `CHECKED` role-62 toggle in a window that also holds a `MODAL`, `SHOWING` role-39 container with `SHOWING` button descendants. It is added to `menu_popup.rs` as a `Gtk3Popover` family with its own rows (the conjunct keeps a role-39 modal top-level dialog, `L18-13`, from reading as a menu), so `wait --menu`, `list-surfaces` and `snapshot --surface menu` agree. If U1 item 1 shows the container is an app-root child, the rule is dropped and `L18-5` is recorded as a named limit.
  - `SELECTED` and tree presence are never evidence (`L18-3`). A GTK3 menu bar that is hidden opens nothing; the wait then times out with the direction message, which is true.
  - **Read strategy.** The detector's bounded reads are the default; `Collection.GetMatches` (server-side role and state match) replaces them only on the condition in Scope Boundaries, and then inside the detector.
  - Web-content menus are not detected, by name. Governs R11.
- KTD10. **Filter first, slice everything.** (Evidence: `L11-15`, `L11-16`, `L8-1`.) The `SignalFilter.process` and `app` selection runs on the registry list before `read_frames`, so a stopped or unrelated app's frames are never read when the filter names another app. The cost of `read_registry` is stated as it is: it reads each registered app's process id from the bus daemon and its facts from `/proc`; for an app that is not stopped it also reads the root's name and toolkit under `ROOT_SLICE` (300 ms, concurrent, 32 in flight), so a name filter (the name is the selector) pays at most one `ROOT_SLICE` when some alive app is silent, an unscoped pass pays that slice plus one `CHILDREN_SLICE` for the silent app's frames (`L33-7`: 0.62 s against 44 ms), and a stopped app pays none (`read_app` takes the name from `/proc` for a stopped pid). A process filter skips the name and toolkit reads of non-matching pids: that needs a select argument that `read_registry` does not have, so it gains an additive `read_registry_for(bus, pid_filter, deadline)` and `read_registry` keeps its signature and callers (`process_state`, `list_windows`, `list_surfaces`, `app_ops`). `windows_of` already drops stopped pids before reading frames. U1 item 8 and the U9 stopped-app legs measure and assert these figures. Every registry, root and frame read keeps 3.4's explicit short slices clipped to the remaining deadline, and `ensure_budget` runs before and after the pass. Governs R10.
- KTD11. **`wait_for_menu` is a port of the Windows loop, with a Linux liveness.** (Evidence: `crates/windows/src/system/wait.rs`, dogfood F6.) The windows of the target pid are resolved from the registry through the pass and re-resolved once on a root-gone failure; each iteration verifies the process (`liveness(pid, instance)` is not `Gone`; `Stopped` is `APP_UNRESPONSIVE` at once, no 25 s read), reads the detector through `popups_of(.., settle: false, ..)` clipped to the deadline, verifies the process again before returning success, absorbs a read `TIMEOUT`, consults the deadline only after a mismatch, and pauses `min(50 ms, remaining)`. The detector's `vanished` rule already turns a window that went away mid-read into "no popups"; the loop's liveness check is what turns a dead target into `STALE_REF` with `not_delivered`, details (pid, instance) and the process-oriented suggestion. The expiry message says whether the state was never read. Governs R12.
- KTD12. **Focus is the existing decision, plus an honesty rule.** `focus-changed` uses 3.4's `decide_focus` unchanged (one `ACTIVE` candidate with no X11 answer is believed; several are not; with an X11 answer the candidate must agree). The only addition is KTD6's incomplete-on-indeterminate rule (any `ACTIVE` candidate with `decide_focus` returning none: several candidates without an X11 answer, or one candidate with X11 `Ambiguous`, `NoActive` or `Unmapped`, or a frame naming a non-candidate). A Wayland session with no input device never sets `ACTIVE` (`L13-5`, `L23-29`), so there `is_focused` is false everywhere and the event cannot occur until a seat keyboard exists, after which the focused window carries `ACTIVE` and `decide_focus` believes the one candidate (Wayland has no X11 answer to contradict it); this is stated, because core's capture takes no event kind and an adapter cannot refuse one token. Governs R3.
- KTD13. **The advertised surface kinds are pinned.** `SIGNAL_SURFACE_KINDS` is `[Sheet]` (a plain window is covered by the window events, not a surface signal), asserted by a test that fails when a kind is added without the snapshot side, mirroring Windows' list. The menu-surface work adds `Menu` to this list, to `supported_surfaces` and to `list_surfaces` together and rewrites the pin in that change; this sub-phase leaves one call site obvious in `signal_surfaces.rs`. Because `list_surfaces` lists a dialog under both kinds with one id (`L23-15`), consumers that diff `list-surfaces` dedupe by id; the capture's single `Sheet` kind already does. `supported_surfaces` is unchanged here. Governs R5.
- KTD14. **Which wait kinds Linux delivers.** Core defines seven event tokens, the modes sleep, element (with predicate), window, text, menu, notification and event. Linux delivers: all seven tokens subject to the three named limits (focus on a Wayland session with no seat keyboard, in-window dialog dismissal if U1 says so, the app population rule); `--menu` and `--menu-closed`; `--element`, `--window` and `--text` through core over 3.4 and 3.5; `--notification` in 3.14. Nothing is refused with `ACTION_NOT_SUPPORTED` because core gives the adapter no way to refuse a single event token; the limits live in the core skill's `wait` docs (U10), `docs/phases.md` and the ledger; the Linux platform skill (3.13) repeats them. Governs R3, R7.
- KTD15. **No fixed settle.** (Evidence: `L18-2`: appearance 157-331 ms, disappearance 186-233 ms, dialog close 255-271 ms.) `capture_signal_baseline` is a snapshot and settles nothing; `wait_for_menu` polls at 50 ms; core's own post-action settle (50 ms steps to 400 ms) already covers the 254 ms burst. A documented caveat remains: `--menu-closed` issued between a `click` and the popover reaching the registry reads closed, and that interval was 157-331 ms in `L18-2` but 0.66 s for the Calculator Main Menu popover in a private GNOME session, where the About dialog then appeared 0.177 s after the item's action (`L23-15`, `L23-19`); U1 measures it in both directions and the skill says to wait for `--menu` first, with a deadline of at least 1 s. Governs R13.
- KTD16. **Dependencies are named, not assumed.** The plan consumes 3.4 (the modules in Assumptions), 3.9 (`liveness`, `identity`, `process_state`, `proc_stat`), 3.7 and 3.8 (live-test and dogfood openers only), 3.5's `tree/absence.rs` classifier (landed; it classifies D-Bus error names as app gone, suspect, timed out or other; it is wired into node reads only, so lane A applies it to the root `list_children` and frame `read_node` failures), 3.4's slices, `unresponsive.rs` and instance token, and the menu-surface branch's detector (KTD9). A missing one is a Goal Capsule stop condition.

### High-Level Technical Design

*This illustrates the intended approach and is directional guidance for review, not implementation specification.*

One capture pass:

```
filter(app?, process?) -> select registry apps (BEFORE any frame read)
   |-> read_registry  (once; apps + facts.instance)
   |-> read_frames    (per root: nodes + Read|TimedOut)
   |       |-> collect -> windows (shape widened, bubbles/menu-frames out)
   |       |-> focus: decide_focus(active candidates, X11 evidence)
   |       `-> sheets: is_sheet over the same nodes (scoped only)
   |-> apps = owners of >=1 window            (signal_population)
   |-> completeness: scoped target stopped        -> Err(APP_UNRESPONSIVE)
   |                 scoped target alive, TimedOut -> windows/apps/surfaces incomplete, Ok
   |                 unscoped unread app           -> windows/apps incomplete
   |                 surfaces                      -> complete only if scanned and read
   `-> boundary: every Err -> TIMEOUT | APP_UNRESPONSIVE (stopped) | (no bus) PLATFORM_NOT_SUPPORTED
```

Menu wait iteration (per 50 ms):

```
verify(pid, token) -> popups_of per window (detector, bounded) -> state == wanted ?
     |                      | timeout: absorb          yes -> verify again -> Ok
     gone -> STALE_REF      `-> deadline checked only after a mismatch -> pause 50 ms
```

Menu detector by family (the rows U6 pins; the first five are the detector's own tests, the last is new):

| Family | Open signal | Closed | Idle-true trap excluded by |
|---|---|---|---|
| GTK4 popover | `SHOWING` role-33 menu with a `SHOWING` item (`Gtk4Popover`) | no menu nodes | idle trees hold none |
| GTK3 menu bar (bar shown) | same, below a role-34 bar (`Gtk3Bar`) | items not `SHOWING` | bar items alone are not evidence |
| GTK3 context menu | nameless `SHOWING` role-69 top-level with only menu content (`Gtk3Context`) | window gone | needs menu-only subtree |
| Qt | `SHOWING` role-41 popup with a `SHOWING` item (`Qt`) | popup not `SHOWING` | role-35 under role-34 does not count |
| Chromium | nameless `SHOWING` role-23 top-level frame with only menu content (`Chromium`) | frame gone | `HAS_POPUP` is not evidence; web content pruned |
| GTK3 header popover (added) | `CHECKED` role-62 toggle plus `MODAL` `SHOWING` role-39 with `SHOWING` buttons (`Gtk3Popover`) | toggle clear | the role-39 conjunct (`L18-13`) |

### Sequencing

U1 settles the open measurements before code. Lanes A and C are independent of each other (the event-wait chain and the menu-wait chain share no file). The seam for splitting the PR is after U5 and U12 (event waits) against U6-U8 (menu wait, in-window dialogs). Decision applied: the integration step measures the merged diff and splits at that seam into two stacked PRs if the hand-written diff exceeds 2,000 lines, else ships one (KD4).

---

## Implementation Units

### U1. Measurements before code

- **Goal:** settle by measurement what the plan would otherwise assume, as new ledger rows in area `L33`. All items run before code (the item list is not trimmed); items 1 to 5 and 11 gate interfaces and run first, items 6 to 10 gate the lane that owns their number and may run while the skeleton is built.
- **Requirements:** R1, R3, R7, R8, R10, R11, R12.
- **Dependencies:** 3.4 and 3.9 landed (registry pass, liveness), `scripts/private-session.sh`, `L18` rows.
- **Files:** none in the repository; a probe script under the owner's probes directory, and its rows go to `probes/linux/FINDINGS.md` in U10 from `plans/3.11-lane-rows-*.md`.
- **Approach:** private session with a window manager, GTK4 (Calculator, Files, Text Editor), GTK3 (Terminal with `default-show-menubar` set in the private keyfile only), the Qt script, Chrome, VS Code and Firefox where available. Each item is measured at least 3 times in both directions. Items:
  1. App-root children census at idle and after opening each opener: roles, `SHOWING`/`MODAL`, Component interface, `class` attribute; confirms role-39 root children are 0 at idle on every toolkit and that the role-39 Shortcuts window has Component (KTD8); also records whether the GTK3 header popover's `MODAL` role-39 container (Rule B) is an app-root child, because if it is, opening that popover would read as `window-opened` and `surface-appeared` under the widened shape.
  2. Object-path reuse: 20 open-close-reopen cycles per toolkit with the close and reopen inside one 200 ms interval; does a reopened window reuse its path, and does a name hash distinguish it (KTD4).
  3. Does the Chromium menu popup frame (`L18-7`) pass `is_bubble`; its `class` and name; does opening a menu change the focus decision.
  4. In-window dialog (`L18-14`): where it sits (depth, ancestors), whether it loses `SHOWING` after dismissal, the cost of a bounded scan at the depth that finds it, and the identity of the opener (R7).
  5. Menu read cost through the imported detector (`popups_of` per window and `popups_at` on the app root), against `Collection.GetMatches`, per toolkit and per app, min-of-seven, with the cost at the 50 ms cadence; whether the app-root anchor and the per-window loop give the same answer (KTD9 anchor); whether the detector holds with the GTK3 bar shown at idle (no item `SHOWING`); whether the GTK3 header popover (`L18-5`) is read by it (it is not today).
  6. The closed-direction race: a `--menu-closed` poll started immediately after a GTK4 `click` reads closed for how long, and a `--menu` poll started before the click catches the open within one cadence (KTD15).
  7. Focus: two-`ACTIVE` frequency during churn; X11 with and without a window manager; on Wayland `ACTIVE` appears only once a seat keyboard exists (`L23-29`), so U1 records the capture before and after a first key and whether any other source exists (recorded, not built).
  8. A SIGSTOPped unrelated app and a SIGSTOPped target: pass duration with the filter naming another app, naming the stopped app, and unscoped; confirms the slices bound it far below 25 s (`L11-15`), and records the registry-pass cost of an unrelated stopped app (expected none: `read_app` reads `/proc`) and of an alive-but-silent app (expected one `ROOT_SLICE`) with a process filter, a name filter and no filter (KTD10).
  9. The handoff window (`gnome-text-editor --new-window`, `L4-26`): new frame, old pid, and the apps list before and after.
  10. Error table for the root `list_children` and the per-frame `read_node` on a dying app, against 3.5's `tree/absence.rs` rows (`L25`): which failures are "gone" and which are unreadable, covering only what those rows do not already measure, and what `list_children` returns after its own folding into `AdapterError`.
  11. Stopped versus slow-but-alive (R8): a target held in short SIGSTOP and SIGCONT cycles, and a busy Electron app, against `stopped_target` (`answered` false, state `T`): how often a root misses `ROOT_SLICE` while the process is not stopped, and what the pass returns in that case (windows incomplete, no `Err`).
- **Test expectation:** none -- a measurement run; its rows and dispositions land in U10.
- **Verification:** each item has a row with a verdict; any item contradicting this plan is reported to the plan and the affected KTD is rewritten before its unit starts.

### U2. Per-root frame outcomes and the shared window pass

- **Goal:** a slow app is never published as "all its windows closed" or "one window closed", and listing and signals share one pass.
- **Requirements:** R1, R8, R10.
- **Dependencies:** U1 (items 8, 10, 11); 3.4; 3.5 (`tree/absence.rs`, landed).
- **Files:** `crates/linux/src/system/registry_frames.rs`, `registry_frames_tests.rs`, `registry.rs`, `window_ops.rs`, `window_listing.rs` (`collect` zips apps with frames positionally and takes the per-root outcome), `unresponsive.rs`, `unresponsive_tests.rs`, `signal_inventory.rs`, `signal_inventory_tests.rs`, `listed_window.rs`.
- **Approach:** KTD3, KTD6, KTD10. `read_frames` returns a `FrameRead` per root (nodes plus `Read` or `TimedOut`); a `CHILDREN_SLICE` expiry, a `list_children` failure that is not "gone", a frame `read_node` that expires `FRAME_SLICE` or fails that way, each mark the root `TimedOut`; "gone" failures stay empty and `Read`. `windows_of` keeps its signature for listing by projecting the nodes. A new pass function takes the registry, the select closure and the deadline and returns the listed windows, the per-app outcomes and the focus evidence; `list_windows` calls it, keeping its concurrent X11 read and its `scoped_windows_of` stopped-target path (`list_windows` and `list_surfaces` return the same errors as before). `ROOT_SLICE`, `CHILDREN_SLICE` and `FRAME_SLICE` become `pub(crate)` so the pass clips them to the remaining deadline. `read_registry_for` (additive) skips the name and toolkit reads of non-matching pids for a process filter.
- **Patterns to follow:** `crates/windows/src/system/signal_inventory.rs` (one coherent pass), 3.4's `list_windows`.
- **Test scenarios:**
  - A root whose child list times out yields `TimedOut` and no windows, and a root whose child list fails with a gone-object error yields `Read` and no windows; the two never compare equal.
  - A root with two frames of which one frame read times out yields `TimedOut` and the one window it read; it is never `Read` with one window.
  - A root with two frames yields two windows with distinct ids and the same instance as the app's facts.
  - `list_windows` over the shared pass returns exactly what it returned before for a fixed node set (no behaviour change).
  - A scoped listing of a stopped target still returns `APP_UNRESPONSIVE` exactly as `unresponsive_tests.rs` pins, and `list_surfaces` is unchanged.
  - The select closure is applied before frames are read: a fake reader asserts it was never asked for an excluded root (invert: select after the read).
  - A process filter never asks the registry reader for the name of a non-matching pid.
- **Verification:** the scenarios pass; treating `TimedOut` as empty, or dropping the per-frame outcome, fails its row (invert-verified).

### U3. Window and sheet residual: the `SHOWING` clause and the menu-frame exclusion

- **Goal:** the dialogs `L18` found are windows and sheets, consistently, and a menu popup frame is not a window.
- **Requirements:** R4, R5.
- **Dependencies:** U1 (items 1, 3); 3.4 (the widened shape and sheet rule are landed).
- **Files:** `crates/linux/src/system/window_filter.rs`, `window_filter_tests.rs`, `crates/linux/src/tree/surfaces.rs`, `surfaces_tests.rs`. `window_listing.rs` changes only if the menu-frame exclusion needs the class (it already reads it for `is_bubble`).
- **Approach:** KTD8. Landed and pinned: a `MODAL` role-39 child with Component is a window; `is_sheet` is `MODAL` or a `SHOWING` dialog; an unreadable role or state is not a sheet. Left: the `SHOWING` clause on the role-39 branch of `is_window_shape` and on the `MODAL` branch of `is_sheet`, only if U1 item 1 shows a hidden `MODAL` top-level exists on a live toolkit (otherwise the rule stays as landed and the row says so); and a pure `is_menu_popup_frame` only if U1 item 3 shows a Chromium menu frame passes `is_bubble`.
- **Patterns to follow:** 3.4's `window_filter.rs`.
- **Test scenarios:** using the role, state and interface values `L18-11`, `L18-12`, `L18-13` record inline (the first four already exist and stay green):
  - GTK3 About (role 16, `SHOWING`, no `MODAL`) is a window and a sheet; GTK4 About (role 23, `MODAL`, `SHOWING`) is a window and a sheet; a plain GTK4 main window is a window and not a sheet; Keyboard Shortcuts (role 39, empty name, `MODAL`, `SHOWING`, Component) is a window and a sheet; the same node without Component is neither.
  - New, if built: the role-39 node `MODAL` without `SHOWING` is neither a window nor a sheet; a role-39 non-root child is untouched.
  - New, if built: the Chromium menu frame, with the values U1 records, is not a window.
- **Verification:** the scenarios pass; dropping the `SHOWING` clause or the menu-frame predicate fails its row (invert-verified); live proof is U9.

### U4. Population, focus honesty and completeness

- **Goal:** the pure rules that turn a pass into the three lists and the booleans.
- **Requirements:** R2, R3, R6, R8.
- **Dependencies:** U2's frozen pass interface (stubs until lane A merges); U3 residual only for the `SHOWING` clause.
- **Files:** `crates/linux/src/system/signal_population.rs`, `signal_population_tests.rs`, `signal_surfaces.rs`, `signal_surfaces_tests.rs`, `crates/linux/src/system/focus.rs`, `focus_tests.rs`.
- **Approach:** KTD5, KTD6, KTD12, KTD13. `signal_population` returns the `AppInfo` of apps with at least one window, each with the instance, merged by `app_ops::one_entry_per_process`. `signal_surfaces` builds a `SurfaceSignal` of kind `Sheet` per sheet window (id the window id, title the window title when non-empty, app, pid, instance) and owns `SIGNAL_SURFACE_KINDS`. `focus.rs` gains `focus_is_indeterminate(candidates, decided)` (at least one candidate and `decide_focus` returned none; it takes the result of the same call and re-implements none of its branching). A completeness constructor takes the scope, the per-root outcomes, whether surfaces were scanned, and focus indeterminacy.
- **Patterns to follow:** `crates/windows/src/system/signal_surfaces.rs` (pinned kinds), `signal_inventory.rs` (`focused_seen`).
- **Test scenarios:**
  - An app with no window is absent from the population; an app with one hidden window (not `SHOWING`) is present.
  - A handoff pass (the pid already present, a second frame) adds a window and no app.
  - The indeterminate rule: two `ACTIVE` candidates and no X11 answer, one candidate with each of X11 `Ambiguous`, `NoActive` and `Unmapped`, and a frame naming a non-candidate are indeterminate; two with an X11 answer naming one, one with no X11 evidence, and zero candidates are not.
  - Completeness: a scoped pass whose target is stopped is an error; a scoped pass whose target is `TimedOut` but not stopped is `Ok` with windows, apps and surfaces incomplete; an unscoped pass with a `TimedOut` app has windows and apps incomplete and surfaces incomplete; an unscoped pass never scans surfaces; a clean scoped pass is complete on all three.
  - A Qt process with two registry entries, one owning windows, yields one app (invert: skip the merge).
  - `SIGNAL_SURFACE_KINDS` equals `[Sheet]` and every emitted signal kind is in it.
  - A dialog that `list_surfaces` lists as both a `window` and a `sheet` under one id (the Calculator About of `L23-15`) yields exactly one `SurfaceSignal`, of kind `Sheet`, carrying that id, and one `window-opened` for the same id (invert: emit the window kind too).
- **Verification:** the scenarios pass; returning `complete()` unconditionally, counting window-less apps, treating a slow alive target as an error, skipping the Qt merge, or emitting a `Menu` kind each fails its row (invert-verified).

### U5. `capture_signal_baseline`, the boundary and the wiring

- **Goal:** core's `wait --event` works on Linux.
- **Requirements:** R1-R6, R8-R10.
- **Dependencies:** U2, U4; 3.9 (`process_facts` token shared, so app and window instances agree).
- **Files:** `crates/linux/src/system/signal_capture.rs`, `signal_capture_tests.rs`, `signal_boundary.rs`, `signal_boundary_tests.rs`, `crates/linux/src/system/adapter.rs` (the `SystemOps` wiring; `crates/linux/src/adapter.rs` holds only `ObservationOps` and does not change), `crates/linux/src/system/mod.rs` (module declarations), `crates/linux/src/system/registry.rs` (forget on connection failure), `crates/linux/src/system/bus.rs` (only to expose the fault classification the boundary narrows).
- **Approach:** KTD7, KTD10. `signal_capture` checks the budget, reads the registry, selects, runs the pass, derives the lists and completeness, checks the budget again, and returns the baseline. The boundary maps by cause: spent budget, slow read and a dropped transport are `TIMEOUT`; a scoped target confirmed stopped is `APP_UNRESPONSIVE` with its identity in `details`; missing bus stays `PLATFORM_NOT_SUPPORTED`; a fault narrowed from `INTERNAL` also forgets the cached connection. `system/adapter.rs` wires `capture_signal_baseline`.
- **Patterns to follow:** `crates/windows/src/system/signals.rs` (single pass, completeness from the pass, boundary at the end).
- **Test scenarios:**
  - Boundary rows: an `INTERNAL`-class fault from a read becomes `TIMEOUT`; a scoped target confirmed stopped becomes `APP_UNRESPONSIVE` carrying name, pid and instance, while a slice-expired scoped root of a process that is not stopped is not an error (the capture is `Ok` and incomplete); a spent deadline and a dropped transport become `TIMEOUT`; a missing-bus fault stays `PLATFORM_NOT_SUPPORTED`; a fault narrowed from `INTERNAL` forgets the cached connection (checked with a counting `forget` seam); no output code is outside `TIMEOUT`, `APP_UNRESPONSIVE` and that one fatal code (core's retry set is those two plus `ELEMENT_NOT_FOUND`, which the capture never returns).
  - A capture whose deadline is already spent returns `TIMEOUT` and no baseline; one that finishes after the deadline does too.
  - Two captures of an unchanged desktop produce baselines whose diff through core's `diff_signals` is empty (the core function is called from the test).
  - A fixture where a window appears between two captures yields exactly one `window-opened` (and one `app-launched` if its app is new), and the reverse yields `window-closed` and `app-terminated`; a handoff yields `window-opened` alone.
  - A scoped capture for app A never asks the reader for app B's roots.
- **Verification:** the scenarios pass; widening the boundary to pass `INTERNAL`, publishing late, or selecting after the read fails its row (invert-verified). Live proof is U9.

### U6. Menu detector adoption, header popover and process anchor

- **Goal:** one answer to "is a menu open in this process", honest on five toolkits plus the GTK3 header popover, from the one detector.
- **Requirements:** R11.
- **Dependencies:** step 0 (the detector import, KTD9); U1 (items 1, 5, 6).
- **Files:** `crates/linux/src/system/menu_popup.rs`, `menu_popup_tests.rs` (imported; this unit adds the `Gtk3Popover` family and its rows), `menu_reader.rs` (imported, unchanged unless U1 item 5 changes a budget), `menu_surface.rs` (imported detection half), `crates/linux/src/tree/observe_menu.rs` (`rooted`, `scan_walk`), `crates/linux/src/system/menu_anchor.rs`, `menu_anchor_tests.rs` (the process-to-popups function the wait loop calls).
- **Approach:** KTD9. `menu_anchor::popups_of_process(bus, process, deadline) -> Result<bool, AdapterError>` selects the process's windows through the shared pass and returns whether any `popups_of(.., settle: false, ..)` is non-empty, or reads `popups_at` on the app root if U1 item 5 shows that is equal and cheaper; it reads nothing for a process whose liveness is not `Alive`. Rule B is a pure addition to `menu_popup.rs`. `MenuPopup.family` and `.items` have no reader in this sub-phase; the menu-surface work reads them, so the imported struct keeps them and carries a single `expect(dead_code)` with that reason until it lands, or the fields are dropped from the import and restored by that branch (decided at step 0 by what clippy accepts).
- **Patterns to follow:** `crates/windows/src/system/menu_state.rs` (compound of measured sources), the detector's own tests.
- **Test scenarios:** one row per family using the role and state values the `L18` rows record, open and closed each (the first five exist in the imported tests and stay green):
  - GTK4 idle tree holds no menu nodes (closed); the popover tree is open; the toggle `PRESSED` with no menu node is closed.
  - GTK3 menu bar: items `VISIBLE` without `SHOWING` and a `SELECTED` menu are closed (`L18-3`); items `SHOWING` under the role-33 menu are open.
  - Qt: menu-bar items `SHOWING` at idle are closed; a role-41 popup with a `SHOWING` item is open.
  - Chromium: idle `HAS_POPUP` nodes are closed; a menu frame with `SHOWING` items is open; a role-33 menu with `SHOWING` items inside a `document web` subtree is ignored.
  - New: a `CHECKED` role-62 toggle plus `MODAL` `SHOWING` role-39 with `SHOWING` button descendants is open (`Gtk3Popover`); a `MODAL` role-39 top-level dialog with no `CHECKED` toggle is closed; the toggle alone is closed.
  - New: the anchor reads no window of a stopped or unrelated process; a menu popup frame that the window rule excludes (Chromium) still makes the process answer open (the read starts from the app root's children, not the listed windows).
  - A node set with an unreadable state is not open (absent evidence stays absent).
- **Verification:** the scenarios pass; each of "toggle alone is open", "tree presence is open", "bar items count", "web content counts" and "drop Rule B's conjunct" fails its row (invert-verified); the bounded reads find every signal within the budget U1 sets, live in U9.

### U7. `wait_for_menu`

- **Goal:** `wait --menu` and `--menu-closed` answer on Linux with Windows' guarantees.
- **Requirements:** R11, R12, R13.
- **Dependencies:** U6 (`menu_anchor` frozen signature, stub first); 3.9 (`liveness`).
- **Files:** `crates/linux/src/system/menu_wait.rs`, `menu_wait_tests.rs`, `crates/linux/src/system/adapter.rs` (wiring only, a step 0 edit).
- **Approach:** KTD11, KTD15. The loop is the Windows port: process check (`liveness`), detector read through `menu_anchor`, process check, absorb timeout, deadline only after a mismatch, 50 ms pause, direction-specific expiry text, `STALE_REF` for a gone target and `APP_UNRESPONSIVE` for a stopped one. Test seams force a predicate error and a process death.
- **Patterns to follow:** `crates/windows/src/system/wait.rs`.
- **Test scenarios:**
  - An already-open menu satisfies `--menu` after exactly one predicate read; an already-closed state satisfies `--menu-closed` after one read.
  - A predicate that flips on the third read returns on the third, with a 50 ms cadence (counted by the seam, no sleeps asserted).
  - A predicate `TIMEOUT` is absorbed and the next iteration runs; expiry with no successful read says the state was never read, with one read it does not.
  - A target that dies between the read and the return is `STALE_REF`, `not_delivered`, with pid and instance in `details`, and never `Ok`.
  - A stopped target is `APP_UNRESPONSIVE` without a 25 s wait.
  - A budget too small for the first read returns `TIMEOUT` rather than a false answer.
- **Verification:** the scenarios pass; removing the second liveness check, consulting the deadline before the first read, or returning `Ok` on a stale token each fails its row (invert-verified). Live in U9.

### U8. In-window dialogs (conditional on U1 item 4)

- **Goal:** `surface-appeared` and `surface-dismissed` cover a dialog drawn inside its window if, and only if, both transitions are observable.
- **Requirements:** R7.
- **Dependencies:** U1, U5; runs after lanes A and B merge, owned by the integration step. It edits `tree/observe.rs` (`root_element` and `settled_walk` only: the menu-surface branch also edits that file, with a hand-off at the top of `observe_tree`).
- **Files:** `crates/linux/src/system/signal_surfaces.rs`, `signal_surfaces_tests.rs`, `crates/linux/src/system/surfaces.rs` (`surfaces_of`, `list_surfaces`), `crates/linux/src/system/window_id.rs` (nested surface identity), `crates/linux/src/tree/observe.rs` (sheet rooting), `crates/linux/src/tree/surfaces.rs`.
- **Approach:** if U1 shows the dialog loses `SHOWING` (or leaves the tree) on dismissal and a scan to the measured depth finds it within the deadline share U1 sets, `signal_surfaces` adds a bounded scan of the scoped app's windows for a `SHOWING` role-16 descendant, keyed by (pid, instance, window id, node path); the surface is dismissed on a state check, not on node absence (`L18-14`: the node stayed). The scan runs only for a scoped capture and marks surfaces incomplete when it cannot finish. A surface the signal announces must be listable, selectable and rootable, and today none of the three can see a node below a top-level window: `list_surfaces` builds its entries from the top-level windows (`surfaces_of`), `snapshot --surface sheet` accepts a root only when the selected root itself is a sheet (`observe.rs`), and ids encode a frame address. So the positive branch owns the whole chain: (1) the same bounded scan feeds `surfaces_of`, which lists the nested dialog as a `sheet` entry whose title is the dialog's name and whose id is the nested identity, so `list_surfaces`, `wait --event surface-appeared` and the snapshot agree on one list; (2) `window_id.rs` encodes a nested identity as the owning window's address plus the dialog's node path (the codec's escape rules apply, and a decode that does not land on a `SHOWING` role-16 node is `STALE_REF`), and the resolver for a surface id accepts it; (3) `observe.rs` roots `snapshot --surface sheet` at the nested node when the id is nested, applying `is_sheet` to that root, with refs allocated under it resolving strictly through the same path. If U1 shows either transition unobservable, this unit builds nothing, writes the limit with the measurement into `docs/phases.md` and the core skill docs in U10, adds an owner item (or receiving sub-phase scope) in `docs/phases.md` naming what the adapter or core would need and citing the U1 item 4 row, and the unit closes as "not built, reason, evidence that would change it".
- **Test scenarios (if built):** the same nested dialog goes through every stage on a fake tree: it is announced by the signal, listed by `list_surfaces` as a `sheet` with the same id, selected by `snapshot --surface sheet` with that id, rooted at the nested node, and a ref allocated under it resolves strictly to the same node; a nested id whose node lost `SHOWING` is `STALE_REF`; a top-level sheet's ids and listing are unchanged; a `SHOWING` role-16 node inside a window is a sheet signal; the same node without `SHOWING` after dismissal is gone from the next capture though still in the tree; a node deeper than the scan budget is not found and marks surfaces incomplete; the signal id differs from the window id and is stable across polls.
- **Verification:** the scenarios pass and invert (state check replaced by absence fails the dismissal row; listing without the nested entry fails the announced-but-unlisted row; rooting at the window instead of the nested node fails the snapshot row); the live leg opens the nested dialog in the app U1 measured, and asserts signal, `list-surfaces`, `snapshot --surface sheet` and a click on a ref inside it agree; otherwise the limit sentence exists with its row id.

### U12. A stopped app at window resolution

- **Goal:** a stopped app answers `APP_UNRESPONSIVE` where it now answers `WINDOW_NOT_FOUND` "The window is no longer open" (follow-up routed from the capture work: `windows_of` skips stopped pids, so `resolve_window_strict` lists no window for them and `verify` calls it closed).
- **Requirements:** R8, R9 (same stopped evidence, one definition).
- **Dependencies:** none beyond 3.4 and 3.9.
- **Files:** `crates/linux/src/system/window_resolve.rs`, `window_resolve_tests.rs`.
- **Approach:** `verify` keeps its pure signature plus one `liveness` verdict for the expected window's pid and instance: an expected window that is not in the live list while that process is `Stopped` is `APP_UNRESPONSIVE` for the app (`AdapterError::app_unresponsive`, with the same stopped evidence `stopped_target` uses); a `Gone` process or a window missing from a live process stays `WINDOW_NOT_FOUND`. `resolve_window_strict` lists with a process-scoped filter through the shared pass, not every window of the desktop, so a stopped unrelated app costs nothing.
- **Test scenarios:**
  - An expected window of a stopped process, absent from the list, is `APP_UNRESPONSIVE`, never `WINDOW_NOT_FOUND` (invert: drop the verdict).
  - The same window of a gone process, and of a live process whose window closed, stay `WINDOW_NOT_FOUND` "no longer open".
  - A process id reused by another process (token differs) stays the "different process" refusal.
  - The listing asks only for the expected window's process.
- **Verification:** the scenarios pass and invert; live in U9 with a SIGSTOPped Calculator and a ref from before the stop.

### U9. Live verification and performance

- **Goal:** prove R1-R13 on real apps in a private session, and price the poll.
- **Requirements:** R1-R13, R16.
- **Dependencies:** U5, U7, U8, U12; 3.7 `click`, 3.8 `press` for openers and closers.
- **Files:** `crates/linux/src/system/signal_live_tests.rs`, `signal_live_surfaces_tests.rs`, `menu_wait_live_tests.rs`, `signal_live_apps.rs` (launch and opener helpers) (all `#[ignore]`, in the style of 3.4's `registry_live_tests.rs`, `sheet_live_tests.rs` and `stopped_live_tests.rs`, reusing their launch helpers).
- **Approach:** the tests launch their own apps inside `scripts/private-session.sh --wm`, drive core's `wait` through the real adapter, open and close with `click` and `press`, and stop everything they launched. Dialog openers are `L18-11` (Terminal Help > About, found by name), `L18-12` (Calculator `Main Menu` then the last new item, verified by the resulting dialog name; `L23-15` measured the same opener, popover items showing after 0.66 s and the About dialog 0.177 s after the last item's action, so the harness waits at least 1 s for the popover) and `L18-13` (Keyboard Shortcuts, third-from-last item); menu openers are the controls `L18-1` to `L18-7` name, each pinned to the node 3.7's U1 settled as accepting the action (outer button or inner toggle for GTK4). Menus that close only by key (GTK3 bar, Chromium) are closed with a global `press Escape` while the target window is focused, or `press --app`. Terminal starts after the activation environment is set, which the helper already does (`L18-17`).
- **Test scenarios:**
  - `window-opened` and `window-closed` on Text Editor `--new-window` (handoff: no `app-launched`, `L4-26`) and Calculator; `app-launched` on a fresh Calculator and `app-terminated` on its close; no `app-launched` for a helper that registers with no frames.
  - A second Calculator process (Calculator is not forwarded, `L23-2`) is `app-launched` and `window-opened` with a distinct id and pid but the same title (`L23-36`); Text Editor Ctrl+N is `window-opened` under the same pid without `app-launched`; a SIGKILLed Calculator leaves the registry within about 0.12 s and the next capture is `window-closed` plus `app-terminated` (`L23-33`).
  - `focus-changed` between two windows on X11 with the window manager; on a Wayland session with no input device (no `ACTIVE`, `L23-29`) it times out and the timeout carries `baseline_counts`, and after a first RemoteDesktop key the focused window carries `ACTIVE` and the event fires.
  - A window that opens and closes inside one wait is reported (seen-set), and `[click, wait window-closed]` as a batch does not abort on the seed capture.
  - `surface-appeared` and `-dismissed` for the three openers (named GTK3 About, `MODAL` GTK4 About frame, unnamed role-39 Shortcuts window), found by identity and not by name, and the in-window dialog per U8's decision.
  - `wait --menu` and `--menu-closed` in both directions on Calculator and Files (GTK4, closed by `click`), Terminal (GTK3 bar shown, closed by `press Escape`; header popover closed by `click`), the Qt window, Chrome (closed by `press Escape`); an already-open menu satisfies `--menu` at once; a hidden GTK3 menu bar times out with the direction message.
  - A SIGSTOPped target scoped wait returns `TIMEOUT` carrying `APP_UNRESPONSIVE` as `last_error` inside the deadline, and a batch `[action, wait --event]` with that target aborts as `pre_action_baseline_failed`; a scoped target that is alive but slow (U1 item 11) keeps the wait polling with the classes incomplete and no `last_error` of that code, and a batch seed is not aborted; an unscoped wait with an unrelated stopped app reports no events and returns inside the deadline; a target that exits mid-menu-wait is `STALE_REF`; a window ref taken before a SIGSTOP resolves to `APP_UNRESPONSIVE` (U12).
  - Chrome: opening a menu is not `window-opened` (U3); two real windows list and diff correctly.
- **Verification:** all pass in the private session; each scenario that guards a named rule has been seen to fail with the rule broken. Performance: min of seven with the warm-up discarded, with median and max beside it, for one unscoped capture pass against the 37 ms `L8-1` baseline, a scoped pass on one app, and one menu-detector poll (`menu_anchor`) on Calculator, Files, Terminal and Chrome; release size against 15 MiB. The numbers go in the PR description. The live-session re-run is outstanding until the owner restores the bus, with its scope listed.

### U10. Roadmap and ledger corrections

- **Goal:** `docs/phases.md` and the ledger say what is true after this PR.
- **Requirements:** R15.
- **Dependencies:** U9, U11 (it records their dispositions).
- **Files:** `docs/phases.md`, `probes/linux/FINDINGS.md`, `skills/agent-desktop/references/commands-system.md`.
- **Approach:** KD5, each correction in place with no "previously" text, each citing its row:
  - §3.11 scope: add `wait --menu` and `--menu-closed` and the per-toolkit predicate (`L18-9`, `L18-10`); replace "a post-action settle budgets at least 300 ms" with deadline-bounded polling at 50-100 ms and the 300 ms as a floor (`L18-2`); state the three named limits; restate the exit criterion as an in-crate live test driving a named, a `MODAL`-frame and an unnamed role-39 dialog to a baseline-diff discovery (the live harness is 3.12's, as Windows did); add the dependencies on 3.7 and 3.8 for the live legs; re-estimate the size (about 3.4k lines with tests, against the roadmap's ~1k, for the same reason Windows grew about fivefold) with the seam after U5.
  - §3.4: already corrected (window shape, sheet rule and `snapshot --surface sheet` text are in `docs/phases.md`); re-read at the cut and touch only what the residual U3 changes.
  - §3.12: "GTK4 popover contents were unreadable" and the openers sentence are disproved by `L18-1`, `L18-11` to `L18-13`; the popover and dialog targets and the AT-SPI opener route are written in.
  - `P3-O9` row: note that menu state is polled, not subscribed.
  - Owner items: the per-event-kind capability query, the partial-class completeness, core's `identity_unknown_error` code on churn (a vanished node reported as `APP_UNRESPONSIVE` while the app is not stopped), VS Code menus (`L18-8`). `snapshot --surface menu` is not an owner item: the menu-surface work builds it.
  - Core skill: `skills/agent-desktop/references/commands-system.md` (the `wait` documentation) states the Wayland no-`ACTIVE` limit for `focus-changed`, the window-owning app population rule, the in-window dialog outcome from U1 item 4, the `--menu`-before-`--menu-closed` caveat, and that a scoped batch seed fails on a stopped or slow target. The Linux platform skill is 3.13's.
  - Receiving scopes: `L18-15` is written into the `docs/phases.md` scope of 3.12 (`L18-16` is consumed by 3.7's capability table and verb list, U4, and needs no handoff from here), and the popover `DoAction` statements that `L18` disproves (`L4-20`, `L14-8`, `L14-14`) are corrected in 3.7's and 3.8's `docs/phases.md` sections.
  - Ledger numbering: `L18` (the pre-code area) and its 17 rows are already in the ledger; this sub-phase's new rows go in area `L33` (the next free area after `L32`, re-checked at the cut; if taken, the next free one and every citation in shipped docs and this plan moves together). Rows several sub-phases claim (`L4-25`, `L7-6`, `L7-7`, `L8-1`, `L8-3` to `L8-5`, `L8-7`, `L11-15`, `L13-4`, `L13-5`, `L14-14`) get a 3.11 clause appended, never a replaced disposition.
  - Ledger: add the rows of `plans/3.11-lane-rows-*.md` and U1's rows (area `L33`), amend `L4-20`, `L14-8` and `L14-14` as `L18`'s section 3 states (none is amended yet), close `L4-25`, `L4-26`, `L7-5` and every `L18` row naming 3.11 with a closure that `scripts/check-linux-findings.py` accepts.
- **Test expectation:** none -- documentation; `python3 scripts/check-linux-findings.py` and `scripts/check-no-phase-references.sh` run in the Verification Contract.
- **Verification:** the checks pass and a reader of §3.11 can plan 3.12 from it without opening this PR.

### U11. Stranger dogfood

- **Goal:** find what a new agent hits using only the shipped skill and the built binary.
- **Requirements:** R16.
- **Dependencies:** U9.
- **Files:** `docs/dogfood-reports/<date>-<serial>-feat-linux-3-11-signals-wait-parity-dogfood.md`.
- **Approach:** KD6. The operator is a subagent briefed that it holds the release binary and `skills/agent-desktop/` read through `skills get core` and nothing else, repository withheld, working inside `scripts/private-session.sh --wm`. It runs every example verbatim in the default shell, reads every step's `ok`, waits with the tool's own primitives and never a sleep, and treats any undocumented flag, variable or ordering (`AGENT_DESKTOP_HOME`, `XDG_CURRENT_DESKTOP`, `L4-25`) as a finding. A failure counts after a second independent observation; a behavioural claim is measured in both directions. Legs, positive and negative each:
  1. All seven tokens: Text Editor `--new-window`, Calculator, two-window focus, launch and close, and the three dialog openers.
  2. A no-transition pair yields nothing; the timeout carries `baseline_counts` and, when induced, `last_error`.
  3. The seed boundary: a window opens and closes inside one wait; `[launch, wait window-closed]` as a batch.
  4. `wait --menu` and `--menu-closed` both directions on GNOME Terminal and a GTK4 app, closing by a documented path; the closed-direction race of KTD15 measured both ways.
  5. The `--app` contract: not yet running, two instances, display name against process name.
  6. A stopped app: the wait degrades to `last_error` inside the deadline, not 25 s.
  7. Chromium and Gecko: lazy population, many frame-role children, bubbles, menu frames.
  8. A busy desktop with helpers, then an adversarial pass: zero timeout, tiny timeout against app-resolution cost, a target dying mid-wait.
  The report is judged: at least one finding, else the targets are made harder.
- **Test expectation:** none -- a run and a report; each finding is fixed (a named test that fails when the fix is broken), owned elsewhere (the receiving scope written into `docs/phases.md` in this PR) or accepted with a reason.
- **Verification:** the report is committed, every finding has one disposition, and every fixed finding's test was seen to fail with the fix removed.

---

## Verification Contract

| Check | Command or method | Proves |
|---|---|---|
| Lint | `cargo clippy --locked -p agent-desktop-core -p agent-desktop-linux -p agent-desktop -p agent-desktop-ffi --all-targets -- -D warnings` on the box | quality gate |
| Unit | `scripts/cargo-test-isolated-home.sh test --locked -p agent-desktop-linux` | the pure rules of U2-U8 on every host |
| Live, private session | through `scripts/remote-linux.sh`: `scripts/private-session.sh --wm -- cargo test --locked -p agent-desktop-linux -- --ignored signal_live menu_wait_live` | R1-R13 on real apps without the broken live bus |
| Live, live session | the same ignored tests on the live session once the owner restores its bus | the window-manager and Mutter legs the private session cannot show |
| Invert | break each rule named in U2-U8 and U12 (select after read, `TimedOut` as empty, per-frame outcome dropped, unconditional `complete()`, slow alive target treated as stopped, window-less apps counted, Qt merge skipped, `INTERNAL` passed through, late publish, `SHOWING` clause, toggle-alone or presence as open, Qt bar items, web content, Rule B conjunct, second liveness check, deadline before first read, stopped verdict at resolution), watch its test fail, restore | the tests guard what they name |
| MSRV | `cargo +1.89.0 check --locked -p agent-desktop-linux --all-targets` | toolchain floor |
| Isolation | `cargo tree -p agent-desktop-core` shows no platform crate and `git diff -- crates/core` is empty | KD1, R14 |
| Dependency | `git diff -- Cargo.lock Cargo.toml` is empty (no new crate) | the poll adds nothing |
| Size | release binary size against 15 MiB | R16 |
| Source rules | `scripts/check-rust-file-size.sh`, `scripts/check-no-phase-references.sh`, `scripts/check-stale-ref-constructor-misuse.sh`, `cargo fmt --all -- --check` | 400-line cap, no plan ids in source |
| Ledger | `python3 scripts/check-linux-findings.py` exits 0, then `--sub-phase 3.11` lists no `DEFERRED` row | citations valid, rows disposed |
| Perf | min-of-seven with warm-up discarded; min, median, max for the capture pass (unscoped and scoped) and one menu-predicate poll | no unexplained latency; Linux method (`scripts/perf-baseline-compare.sh` is macOS-bound) |
| Dogfood | the committed report with every finding disposed | R16 |

Requirement-to-check map: R1 → U2, U5 rows and U9 handoff and open-close legs; R2 → U4 population rows and U9 helper leg; R3 → U4 focus rows and U9 focus legs; R4-R5 → U3 rows, U4 kinds test and U9 openers; R6 → U4 and U5 scoped-scan rows; R7 → U8 or its limit sentence; R8 → U4 completeness rows, U12 and U9 stopped-app and slow-app legs; R9 → U5 boundary rows and the batch leg; R10 → U2 select-first row, U5 deadline rows and U9 stopped legs; R11 → U6 family rows (imported rows plus Rule B) and U9 menu legs; R12 → U7 rows and U9 target-death leg; R13 → U9 cadence leg and dogfood leg 4; R14 → Isolation, Dependency and Source rules; R15 → Ledger and U10; R16 → Perf, Size and Dogfood.

## Definition of Done

- Every Verification Contract row passes in the private session, the invert check has been run for each rule it names, and the live-session re-run is either done or listed in the PR description as outstanding with what it covers.
- Every ledger row naming 3.11 is disposed of in the PR description, implemented, owned elsewhere with the receiving scope in `docs/phases.md`, or accepted with the reason:
  - `L4-25`: owned elsewhere, 3.12's live harness (GNOME Settings needs `XDG_CURRENT_DESKTOP=GNOME`; no 3.11 scenario launches Settings); 3.11's clause is appended to the row and the receiving scope is written into `docs/phases.md` in U10.
  - `L4-26`: implemented as the handoff rule (U5, U9).
  - `L7-5`, `L23-19`: implemented as deadline-bounded polling, the floor replaced (KTD15, U7, U10).
  - `L7-6`, `L7-7`, `L8-1`, `L8-3`, `L8-4`, `L8-5`, `L8-7`, `L13-4`, `L23-2`, `L23-33`, `L23-36`: implemented in the pass, identity and focus rules (U2-U5).
  - `L13-5`, `L23-29`: accepted as a stated limit on a Wayland session with no seat keyboard; `ACTIVE` appears after the first key (KTD12, U10).
  - `L18-1` to `L18-7`, `L18-9`, `L18-10`: implemented as the imported detector, the header-popover family and the loop (U6, U7).
  - Follow-ups routed here: a stopped app at window resolution answers `APP_UNRESPONSIVE` (U12, fixed with a named test); core's `identity_unknown_error` on churn is owned by core (owner item written in U10); the 3.8 dogfood finding that `wait --event window-opened` and `wait --menu` answer `PLATFORM_NOT_SUPPORTED` is fixed by U5 and U7.
  - `L18-2`: implemented (KTD15).
  - `L18-11` to `L18-13`, `L23-15`: implemented as the widened window and sheet rules (U3), the one-signal-per-dialog rule (U4) and the live openers (U9).
  - `L18-14`: implemented or accepted per U1 item 4 (U8).
  - `L18-8`: owned elsewhere, 3.12, VS Code web-content menus.
  - `L18-15`: owned elsewhere, 3.12 (recorded in its scope). `L18-16`: consumed by 3.7 (U4 capability table and verb list).
  - `L18-17`: consumed (the private-session script), no code here.
  - U1's new rows: each with its verdict and a disposition.
- The dogfood report is committed with every finding disposed of and every fixed finding's test invert-verified.
- `crates/core` diff is empty, no shipped source names a plan, phase or decision id, no probe script, capture or box identity is committed, and no dependency was added.
- `docs/phases.md` states what was built and no longer states what the evidence disproved.
- No abandoned experiment code remains.

## Risks & Dependencies

| Risk | Mitigation |
|---|---|
| The cut base differs from 3.9 head (3.10 landed after it) | every fact in Assumptions is re-read at the cut; the Goal Capsule stops on a missing module |
| The detector import collides with the menu-surface branch's rebase | step 0 imports the detection half verbatim; that branch drops those files on rebase; the split is reported to the owner at the gate |
| A reused object path hides a fast close and reopen | U1 item 2 measures it; the stated limit or a distinguishing hash follows the measurement |
| The menu read is too slow for a 50 ms cadence on big trees | the detector's reads are bounded (2000 and 80 nodes); U1 item 5 prices them; the loop absorbs a slow read as a timeout, never a false answer; `Collection.GetMatches` is the pre-agreed alternative |
| A busy but alive scoped target reads as stopped, or a stopped one as slow | one definition (`stopped_target`: root unanswered and state `T`); U1 item 11 measures both; the slow case is incomplete, never an error |
| A GTK3 header popover rule rests on one app and is not in the detector | Rule B is added to the detector with its own rows, flagged one-family in the ledger and the skill; U1 item 1 can remove it |
| The closed-direction race returns true before the popover exists | measured both ways in U1 and the dogfood, stated in the skill; core's own settle covers the post-action case |
| An unscoped wait is blind while any app is hung | stated; the scoped form works; the partial-class completeness is an owner item |
| The Chromium menu frame reads as a window | U1 item 3 and U3's exclusion (only if measured) |
| The live AT-SPI bus is broken | all verification runs in a private session; the live-session re-run waits for the owner |
| PR size exceeds the ~2k guideline | measured at the merge of lanes; over 2,000 hand-written lines splits at the U5 and U12 seam into two stacked PRs (KD4) |

## Open Questions

### Deferred to implementation

- The detector's node budgets (if U1 item 5 moves them) and the deadline share for the in-window scan, set from U1's rows.
- The exact handling of the Chromium menu frame (excluded as a window or kept), from U1 item 3.
- Whether a distinguishing hash is needed for a reused object path, from U1 item 2.
- Whether `Collection.GetMatches` replaces the detector's bounded reads, by the condition in Scope Boundaries.

### For the owner

- A per-event-kind capability query in core would let an adapter refuse `focus-changed` by name on Wayland instead of timing out.
- A partial-class completeness in core would keep an unscoped wait useful when one app is hung.
- Whether menu snapshots (`snapshot --surface menu`) are wanted, which would let `Menu` become an announced surface.
- The live session's AT-SPI bus must be restarted before the live-session re-run.
- Confirm the detector hand-over: 3.11 imports `menu_popup.rs`, `menu_reader.rs`, the detection half of `menu_surface.rs` and two helpers of `observe_menu.rs` from the menu-surface branch, and that branch drops them when it rebases onto 3.11.

## Sources & Research

- `probes/linux/FINDINGS.md` rows `L4-25`, `L4-26`, `L7-1`, `L7-5`-`L7-9`, `L8-1`, `L8-3`-`L8-5`, `L8-7`, `L11-14`-`L11-17`, `L13-4`, `L13-5`, `L14-8`, `L14-13`, `L14-14`, `L3-25`, `L3-27`, `L3-30`; pre-code probe rows `L18-1` to `L18-17`, `L23-2`, `L23-15`, `L23-19`, `L23-29`, `L23-33`, `L23-36`; 3.5's `L17` error table.
- `crates/core/src/adapter/system.rs` (`capture_signal_baseline`, `wait_for_menu`, `process_state`), `signals.rs`, `signal_baseline.rs`, `signal_completeness.rs`, `signal_filter.rs`, `surface_signal.rs`, `event_kind.rs`, `commands/wait.rs`, `commands/wait_event.rs`, `src/batch/execution.rs` for the contract this sub-phase implements.
- `crates/windows/src/system/` (`signals.rs`, `signal_inventory.rs`, `signal_surfaces.rs`, `wait.rs`, `menu_state.rs`) and `docs/dogfood-reports/2026-08-15-001-feat-windows-2-11-signals-wait-parity-dogfood.md` for the Windows precedent.
- Menu-surface branch `feat/linux-3.14-shell-notifications` (`system/menu_popup.rs`, `menu_reader.rs`, `menu_surface.rs`, `tree/observe_menu.rs`) for the one detector, and `crates/linux/src/system/` (`liveness.rs`, `proc_stat.rs`, `window_resolve.rs`, `app_ops.rs`, `unresponsive.rs`) and `tree/absence.rs` for what 3.9 and 3.5 provide.
- `crates/linux/src/system/` (`registry.rs`, `registry_frames.rs`, `window_ops.rs`, `window_listing.rs`, `window_filter.rs`, `surfaces.rs`, `focus.rs`, `bus.rs`, `bus_deadline.rs`, `bus_slice.rs`) and `crates/linux/src/tree/surfaces.rs`, `state_bits.rs` for what 3.4 and 3.1 provide.
- Plans 3.4 (observation), 3.5 (resolution), 3.6 (actionability), 3.7 (semantic actions), 3.8 (input), 3.9 (lifecycle), 3.10 (capture and clipboard) and the cross-plan audit of 2026-10-18 for what lands before this sub-phase.
- `docs/solutions/logic-errors/a-deadline-cannot-interrupt-a-blocking-os-call.md`, `docs/solutions/best-practices/a-fixed-sleep-is-not-a-synchronization-primitive.md`, `docs/solutions/best-practices/real-app-tests-are-the-platform-adapter-gate.md`.
