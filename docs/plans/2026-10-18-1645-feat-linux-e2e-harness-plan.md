---
title: Linux Live E2E Harness on Real GNOME Apps (Sub-phase 3.12) - Plan
type: feat
date: 2026-10-18
origin: docs/phases.md
artifact_contract: ce-unified-plan/v1
product_contract_source: docs/phases.md §Phase 3 sub-phase 3.12
execution: code
---

# Linux Live E2E Harness on Real GNOME Apps (Sub-phase 3.12) - Plan

## Owner validation amendment (2026-10-20)

This amendment governs the rebase validation and supersedes conflicting live
coverage, retry and shipping instructions below. Rebase onto the landed
core-delivery head, validate locally and on private Linux sessions, commit as
Lahfir, and do not push or open a PR.

Rerun a failed test or harness leg at most once. Delete timing, load, session
setup and environment-dependent checks, or replace them with a deterministic
fake contract or a real observed-state wait. Record the reason in a `test:`
commit. Fix product code only for a deterministic product defect, with priority
on wrong-target actions and false delivery claims. Invert every new test.

The current registry and README are the retained live inventory. Removed
stock-app, input-seat, role, modal, exact-rendering and service-membership
assumptions are not live passes. Keep 3.14 menu, popover, tray and notification
legs outstanding with that dependency. Measure every retained tier/mode and
report passed, failed and outstanding separately. An outstanding or unavailable
result exits nonzero; only a completely passing selection exits zero.

Independent X11 geometry excludes client shadows when observing visible-window
position, size and capture content. Read the margins through X11 properties,
never infer correctness from a product response. Cleanup carries process start
tokens and revalidates exact private runtime ownership before PID signals.
The README records concrete repaired contracts and deleted checks; the existing
L34 rows and §3.12 record the measured results.

## Goal Capsule

- **Objective:** a maintainer can run one command on a Linux GNOME box and learn, from independent observation of real desktop apps, whether every Linux command that exists on the landed stack works (shell surfaces, notifications and the tray are 3.14's commands and are driven here; FFI is 3.13's and the cursor overlay is an owner decision), in headless and headed mode, on each desktop backend; a green run means each named edge case was seen to behave and the box was left as it was found.
- **Means:** a Python runner under `tests/e2e-linux/` that drives the staged release binary against Calculator, Files, Terminal, Text Editor, Settings, the GNOME shell, one Qt window and VS Code inside a private session, with oracles that never go through the product (KTD1-KTD16).
- **Authority:** `docs/phases.md` §3.12 as corrected by 3.0-3.11 and further corrected here where the ledger or the code disproves it (R30). The pre-code rows `L23-1` to `L23-39` are the measurements this plan stands on; `tests/e2e/` is the contract to match, not the code to copy.
- **Stop conditions:** stop if a requirement needs a change under `crates/core`; if the entry gate (U1) finds a command this harness must drive still answering `PLATFORM_NOT_SUPPORTED` on the landed stack, because a harness cannot certify a command that does not exist; or if no private session starts on the box. A leg that cannot run is never converted into a skip.
- **Execution profile:** U1 first, because it reads the landed 3.5-3.11 and 3.14 code and re-measures the legs `L23` could only record as deferred; U2 and U3 in either order after it; U4 after U3; U5 after U4; U6, U7, U8, U14 and U15 in any order after U5; U9 after U2 and U7; U10 after U3; U11 after U5-U9, U14 and U15; U12 after those and U11; U13 last.
- **Finishes and ships:** the implementer runs the box work through `scripts/remote-linux.sh`, commits, and opens one PR from `feat/linux-3.12-e2e-harness` stacked on the head of the core-delivery PR with `gh stack` (stack order 3.0 to 3.11, then 3.14, then core-small, then core-delivery, then 3.12, then 3.13, then 3.15). The harness therefore tests the final core contract; nothing merges to `main`. The 3.14 worktree used for this refresh sits on a 3.9-era base, so every fact below that names 3.10, 3.11 or later ledger areas is re-read at the cut.

---

## Product Contract

### Summary

Give Linux the same verify-by-observation live gate that macOS and Windows have. After this sub-phase `bash tests/e2e-linux/run.sh` stages the release binary, starts private sessions for each backend tier, launches real GNOME apps, drives every in-scope Linux command through the product CLI, and judges each effect by reading it back from a second channel: raw AT-SPI, the backing GSettings keyfile, X11 properties, `/proc`, pixels and a foreign clipboard client. Every edge case in the roadmap has a named real-app target or a recorded owner decision. Core is unchanged and no fixture app is built.

### Problem Frame

The macOS harness leans on a purpose-built SwiftUI fixture and Windows on a native one; the Linux roadmap forbids a fixture, so the targets are whatever GNOME ships, and a stock app only presents an edge case if the harness reaches it by a route the product itself does not use. The `L14` probes found no target for most cases and that was read as "none exists"; `L23` found one for all but three by waiting long enough for the GTK4 popover to settle, giving a window the focus it needs, enabling the Files tree view in a private settings store, and writing the scrollbar and waiting 1.3 s. The harness therefore needs three things the old harnesses did not: openers and oracles of its own, a session it fully owns, and a leg registry that refuses to let an unreached case pass quietly.

The desktop also fragments. GNOME on X11 under Openbox, under private mutter, and headless Wayland disagree about focus, `ACTIVE`, stacking and input delivery (`L23-14`, `L23-25`, `L23-29`), so results are reported per backend tier and never merged into one verdict. The roadmap's "same contract as `tests/e2e/run.sh`" is also wider than this sub-phase's dependencies: the macOS script carries FFI and cursor scenarios that belong to 3.13 and to platforms with a cursor overlay, so the contract is enumerated rather than inherited (R28). Its notification scenarios are reachable on Linux now that 3.14 lands first, so they are driven here (R34).

### Requirements

**Runner, isolation, cancellation**

- R1. `bash tests/e2e-linux/run.sh` builds the locked release binary, stages an immutable hashed copy, takes the shared interaction lease, and exits `0` (every selected registered leg passed), `1` (behavioural failure, contamination, failed restoration, outstanding or unavailable work) or `2` (prerequisite gate), as `tests/e2e/README.md` defines for macOS.
- R2. Every run uses its own runtime directory, session bus, accessibility bus, display, state root and GSettings keyfile; the live bus, display, shell and clipboard are never contacted (`L23-3`). Launched apps are attributed by registry diff plus runtime-directory membership and tracked as (pid, start time), never by the spawned pid (`L23-1`, `L23-4`).
- R3. Cancelling `scripts/private-session.sh` or the runner with `SIGTERM`, `SIGHUP` or `SIGINT` leaves no member process and no runtime directory, and a runner sweep after `SIGKILL` removes the same (`L23-34`).
- R4. Each shell tier starts through the landed wrapper for it (`scripts/private-mutter-shell.sh` for `gnome-shell --x11`, `scripts/private-wayland-shell.sh` for the headless Wayland shell) under its bounded retry and health gate, closes the Activities Overview before the first leg, kills a failed attempt by process group, and fails the tier with a named reason when the attempts are spent (`L23-28`, `L28-2`, `L28-10`, `L13-3`). Applications that the session bus activates (Terminal, Files) map on the tier's own display, read back by the runner before the first leg (`L18-17`).

**Verdict discipline**

- R5. Every effect is asserted by a read from a channel the product does not use; an `ok: true` is never the evidence, and a leg that finished without a second-channel read fails.
- R6. Every leg is registered with a requirement, target, backend tier, mode and disposition (`pass`, `asserted-refusal`, `owner-decision`, `outstanding`); a `pass` or `asserted-refusal` leg that did not execute, and an executed leg that is not registered, fail the run, while `owner-decision` and `outstanding` legs are reported with their reason and keep the selection nonzero. A seeded failing leg reaches a nonzero exit.
- R7. A vanished target, an unexpected owned process, a teardown failure or an unrestored value fails the run; mutated settings, folders and views are read back to their start values through the channel that read the start.
- R8. Waits are deadline polls on a state; a fixed sleep never decides a verdict, and a leg never retries inside a run, because a retry would hide a dispatch-twice defect.

**Edge-case targets (each asserted on its named real app)**

- R9. Delayed enable: Calculator `Undo` stays disabled through a controlled delay, becomes enabled when a separate helper presses `7`, and a command waiting on it succeeds under the default and explicit timeouts, fails the short one, and dispatches exactly once (display `7`, not empty; `L23-10`, `L23-11`). A permanently disabled `Undo` with `--timeout-ms 2000` returns `TIMEOUT` with `details.kind` `actionability_timeout` near 2 s, the same action with no timeout flag times out near the untouched 5 s default, and a zero timeout makes one immediate check with no late effect (the macOS `AE3`, `AE7` and `AE2` zero-timeout rows). Text Editor `Replace All` enables on typed search text and disables when the text is removed (`L23-13`); Terminal `Copy` enables and disables with a text selection (`L14-2`).
- R10. Zero bounds: Files and Settings zero-axis containers receive no ref and are covered as raw geometry only; the Calculator history `listbox` and the Terminal hidden-menu-bar items are driven through the CLI and read as `visible: false` or the 3.5 outcome the landed resolver defines (`L23-8`, `L23-9`).
- R11. Duplicates: two Calculator processes and a Text Editor `Ctrl+N` window have distinct window ids that each snapshot and focus exactly; a restart turns the old id into `WINDOW_NOT_FOUND` and the new one resolves (`L23-36`). Calculator `Main Menu` resolves its `menubutton` and `switch` to separate nodes with no `AMBIGUOUS_TARGET` (`L23-6`).
- R12. Scroll: Files in a 300-file folder reaches the last item through the product's scroll route, the harness polling the realised set for at least 2 s and re-finding the target, then restores the view (`L23-24`).
- R13. State writes commit: Settings `Hot Corner` flips and restores, `Pointer Speed` and `Number of Workspaces` take values that the backing GSettings keys confirm after the panel is reopened (`L23-27`); a Terminal `Read-Only` check item flips and restores.
- R14. Disclosure: a Files list-view folder row with the tree view enabled in the private settings store expands and collapses, with `expanded` and its children read both ways (`L23-20`).
- R15. Select by display value: Settings `Power` `Screen Blank` takes a new value through the route the landed `select` defines, confirmed by the `idle-delay` key; the leg asserts a refusal where no semantic route exists (`L23-23`).
- R16. Placeholder: Settings `Applications` search reports `placeholder` "Search apps" and no `description`; Files search reports none (`L23-22`).
- R17. Menus and navigation: Terminal hidden-bar items report not visible, the shown bar opens `Edit`, `Preferences` opens modal; Calculator `Main Menu` opens and closes; `gnome-control-center <panel>` lands on the named panel content (`L23-17`, `L23-15`, `L14-15`).
- R18. Occlusion and modality (X11): an ordinary cover is removed by focus preparation and the action lands in the target; an above-layer cover survives it and the action is refused with no effect anywhere; a same-app modal (Files `Properties`, Terminal `Preferences`, Calculator `About`) blocks its parent and a second app is unaffected (`L23-25`, `L23-26`).

**Waits, identity, surfaces**

- R19. The seven event tokens and `--menu`/`--menu-closed` are each driven to success and to a no-transition timeout, scoped to a process and unscoped, and a target that dies mid-wait is reported as such.
- R20. An unnamed dialog is discovered purely by baseline diff in one seeded batch `[open Calculator About, wait --event surface-appeared]` with no title filter, and the diff dedupes a dialog listed as both `window` and `sheet` under one id (`L23-15`).

**Input, capture, backends**

- R21. Headed text entry reaches a GTK4 window: typing, `Ctrl+F` and `Ctrl+H` in Text Editor, with the focus the product itself established; a window the product could not focus is reported as that refusal and not rescued by the harness (`L23-12`, `L23-14`).
- R22. Capture and clipboard are checked on content: a product capture decodes to pixels that match the application's window content read by `xwd`, and each clipboard format round-trips against a foreign producer and reader with the owner's lifetime and clear verified (`L23-38`).
- R23. Wayland tier: semantic actions work with no input device, a seat keyboard makes the focused window `ACTIVE`, the product's first key lands by read-back, and headed ref actions are asserted as the refusal the landed focus decision returns (`L23-29`, `L23-30`, `L23-31`).
- R24. Headless semantic actions leave the pointer position and the active window as they were, while the independently observed effect succeeds (non-interference).
- R25. Results are reported per backend tier (`x11-openbox`, `x11-mutter`, `wayland`) and per mode (headless, headed); a tier the box cannot start is reported as unavailable, and KDE is a documented manual matrix that stays outstanding until a Plasma session runs it (`L23-39`). The mutter tier follows the discipline of KTD15 (Overview closed and read back before every leg, waits on observed state, an idle box, a read-back desktop switch).

**Process and proof**

- R26. A `linux-e2e` `workflow_dispatch` job exists, refuses to run without a proved interactive session, and fails visibly rather than going green; the hosted Linux lane runs the harness self-tests that need no desktop.
- R27. The harness scripts and `docs/phases.md` follow the repo rules: no inline comments, no delivery-plan vocabulary, no probe script or capture committed, recoverable cleanup through `trash`, `crates/core` unchanged.
- R28. The `run.sh` scenario families (observation, interaction, `AE1` to `AE7`, reliability, surfaces, notifications, silent text, trace and electron-live) each have a Linux disposition: implemented here, owned by 3.13, or an owner decision, each written into `docs/phases.md`. Notifications and shell surfaces are implemented here (R34); FFI is 3.13's; multi-agent cursors are an owner decision; `AE5` (permission mapping) gets a deterministic Linux analogue against an unreachable accessibility bus.
- R29. An edge case with no real-app target is stated as an owner decision in `docs/phases.md` with its `L23` evidence, never dropped: a named ref-able zero-size control, a role-and-name duplicate element pair, a semantic-only select, headed Wayland ref actions, mutter focus of a never-focused window, the interactive runner, KDE, live-session RemoteDesktop consent.
- R30. `docs/phases.md` §3.12 and the ledger state what was built and no longer state what `L23` disproved (the eleven replacement sentences and the corrections in KTD13); every ledger row naming 3.12 is disposed of.
- R31. A stranger dogfood is run and its report committed with at least one finding, each disposed of. No performance baseline is taken for this sub-phase (owner decision); a finding fix in `crates/linux` is judged on correctness and the 15 MiB size gate only.

**Added by the refresh gate**

- R32. Headed pointer gestures (`click`, `double-click`, `triple-click`, `right-click`, `hover`, `drag`, `mouse-move`, `mouse-click`, `mouse-wheel`, `scroll`) land on GTK3, GTK4 and Qt targets under **both** `x11-openbox` and `x11-mutter`, GTK4 at two window positions on each. Openbox has no client-side shadow and mutter has, so a GTK4 geometry error hides under one and shows under the other. Each gesture is read by the pointer position (`xdotool getmouselocation`), the application's own effect, and raw AT-SPI extents plus the X11 client origin (`L23-37`, `L27-22`, `L28-12`, `L28-14`, `L28-15`).
- R33. Lifecycle and window operations (`launch`, `close-app`, `focus-window`, `resize-window`, `move-window`, `minimize`, `maximize`, `restore`) are driven on both X11 tiers with an independent oracle each (`xprop`, `xwininfo`, `/proc`, the registry). The mutter cases carry the measured tolerances (`L31-53`, `L31-54`, `L31-56`): the lift case pins `restored_and_active` on openbox and any valid outcome on mutter, the activation refusal is accepted as `activation_refused` or `activation_not_granted`, and the desktop switch is read back before anything is asserted.
- R34. The shell surfaces, notifications and tray that 3.14 ships are driven through the CLI on the mutter tier: `list-notifications` equals the independent sender's record, `dismiss-notification`, `dismiss-all-notifications` and `notification-action` are judged by a separate `dbus-monitor` and a re-read list, `wait --notification` passes and times out, headless mutations are `POLICY_DENIED`, the `system-tray`, `action-center` and `quick-settings` surfaces snapshot and open and close by ref, tray items and entries deliver `Activate` and `Event(id,"clicked")` as the sender's own log shows, and the `menu` surface lists and snapshots on GTK4, GTK3, Qt and Chromium (`scripts/notification-live.sh`, `scripts/notification-oracle.sh`, `L32-*`). The Wayland shell tier is not claimed for these legs unless U1 finds it answers the Notifications interface (3.14 measured 1 usable start in 4).
- R35. One Qt window (`~/.local/ad-tools/qtvenv/bin/python` with `QT_ACCESSIBILITY=1`, `QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1`, `QT_QPA_PLATFORM=xcb`) and VS Code (`~/.local/ad-tools/vscode/app/code`, started with `--user-data-dir` in the runtime directory, `--password-store=basic` and `--ozone-platform=x11`, `L28-16`) are driven as desktop apps: refs and completeness on first snapshot, headless and headed actions where the toolkit supports them, the Qt `no_scroll_container` refusal, `wait --menu` in both directions, and the VS Code frame staying registered long enough to read its menu (`L18-8`). Web pages are never driven.
- R36. Remote runs obey `scripts/remote-linux.sh`: a kill names a pid only (the guard refuses `pkill`, `killall` and `kill` by name, and so does every harness command), a full run sets `AGENT_DESKTOP_REMOTE_TIMEOUT` above the default 3600 s or runs one tier per invocation, and `AGENT_DESKTOP_REMOTE_LOCK=0` is used only for self-tests inside a private session.

- R37. Where a command has a delivery report (`drag`, `mouse-wheel`, `mouse-click`, `close-app`, `move-window`, `resize-window`, `minimize`), the leg asserts the report fields against the independent oracle in both directions: a delivered effect reports delivered, an undelivered one reports not delivered, and core never shows a success flag the oracle disproves. The exact field names, and the core-small changes (combo super and win keys, `list-apps` alias, `launch` suggestion, `recovery.strategy`, dispositions), are read from the core-delivery head at the cut and cited in the leg records. Lying-wrapper legs also fail on a false delivery claim.

### Acceptance Examples

- AE1. Covers R9. Given Calculator with `Undo` disabled, when the helper presses `7` 2.0 s after the first disabled read and the leg waits on `Undo` enabled with a 5 s timeout, then the command succeeds, the display reads `7` after one `Undo` click and the same wait with a 1 s timeout times out.
- AE2. Covers R5, R6. Given a CLI wrapper that answers `ok: true` to every action and performs none, when the suite runs against it, then every effect leg fails and the exit code is `1`.
- AE3. Covers R3. Given a private session running Calculator, when the wrapper receives `SIGTERM` during a foreground payload, then within the grace period no process references its runtime directory and the directory is gone.
- AE4. Covers R20. Given Calculator with a baseline captured before the opener runs, when the batch opens `About`, then `surface-appeared` is satisfied with no title argument and one surface id.
- AE5. Covers R18. Given Calculator moved over Text Editor with the `_NET_WM_STATE_ABOVE` flag, when a headed click is issued at the editor, then the product refuses and the editor document is unchanged; without the flag the same click lands in the editor.

### Key Decisions

- KD1. **Real stock GNOME apps; no fixture app, no harness-served page.** (session-settled: user-directed — chosen over a purpose-built Qt or GTK fixture: the roadmap and the owner require real software.) Governs R5, R9-R18.
- KD2. **Every effect is asserted by independent observation in both headless and `--headed` mode.** (session-settled: user-directed — chosen over trusting the command's own `ok: true`.) Governs R5, R6, R24. Headed-in-both applies to every backend where focus and input are implemented; the Wayland exception is KD6.
- KD3. **Core is untouched; needs of core are owner items in `docs/phases.md`.** (session-settled: user-directed — chosen over a Linux arm or a workaround in core.) Governs R27, R29.
- KD4. **Real runs happen on the box through `scripts/remote-linux.sh` in private sessions; the cancellation defect in `scripts/private-session.sh` is fixed here.** (session-settled: user-directed — chosen over the live session and over a runner-only workaround.) Governs R2, R3.
- KD5. **Probes and fixtures never enter the repository; tests exist only where they can fail for a real reason and each is invert-verified.** (session-settled: user-directed — chosen over broad mocked suites.) Governs R27, R31.
- KD6. **A leg the stack cannot honour is an asserted refusal or a recorded owner decision, never a silent drop.** (session-settled: user-approved — chosen over silently narrowing the deliverable: the dossier found headed Wayland ref actions blocked because core focuses before dispatch and 3.6 and 3.9 refuse Wayland focus.) Governs R23, R29.
- KD7. **Stacked PR on the core-delivery branch into `feat/linux-adapter`.** (session-settled: user-directed, stack order decided as 3.11, 3.14, core-small, core-delivery, 3.12, 3.13, 3.15 — chosen over a PR against `main` and over a stack on 3.11.)
- KD8. **The stranger dogfood is a gate.** (session-settled: user-directed — chosen over an author-run check.) Governs R31.
- KD9. **No performance baselines in this sub-phase.** (session-settled: owner decision — chosen over the Linux min-of-seven method.) Governs R31.
- KD10. **Headed pointer rows run under both openbox and private mutter.** (session-settled: user-directed — chosen over openbox alone, because openbox has no client-side shadow and GTK4 geometry errors hide there.) Governs R32.
- KD11. **A Qt window is the one app the harness supplies.** (OPEN, owner confirmation requested: the roadmap and KD1 forbid a fixture, but the box has no stock Qt app and the 3.6, 3.8 and 3.11 exit criteria name Qt. The window is a short committed Python script under `tests/e2e-linux/apps/`, its own log is the effect oracle, and it is a toolkit probe, not a product fixture. If the owner refuses, the Qt legs become `owner-decision` records and Qt stays covered by the in-crate live tests.) Governs R35.

### Scope Boundaries

- Not built: a product fixture app (the Qt window of KD11 is a toolkit probe and awaits confirmation), a harness-served page or any web-page driving, FFI live tests (3.13), a multi-agent cursor probe (no Linux cursor overlay exists; R28 records the owner), a performance baseline (KD9), and any change under `crates/core`. Electron is covered only as the desktop app VS Code (R35); Chromium web content is not driven.
- Considered and not built:
  - **Retrying a failing leg.** It would hide a dispatch-twice or timing defect. Evidence that would change the call: a measured harness-only flake class.
  - **A harness-side focus or key warm-up.** The first key lost on Wayland and the unfocused mutter window are product behaviours (`L23-30`, `L23-14`); rescuing them in the harness would pass a defect.
  - **Ingesting AT-SPI through the product adapter.** An oracle that shares the product's reader shares its bugs.
- Deferred, with the receiving scope written into `docs/phases.md` in U11: the interactive runner and Plasma session (owner), live-session RemoteDesktop consent (owner), FFI live acceptance (3.13), non-English shell strings and tray corner cases (3.15, `L32-40`, `L32-16`), the Wayland shell notification legs if U1 finds the Notifications interface still unanswered there, app-level `--app` ambiguity (3.4 and 3.5 owner decision, `L23-36`), Chromium web-content menus.

---

## Planning Contract

### Assumptions

- 3.5-3.11 land with the commands this harness drives answering on real apps; U1 proves it and stops otherwise. Draft unit and function names in the sibling plans are intent, not fact; U1 and each scenario unit read the landed code and cite what exists.
- The harness reads the same release binary the user would: staged, hashed, version-checked, never `cargo run`.
- `python3-gi` (Gio D-Bus and Atspi), `xdotool` (the 2016 release line, `L1-3`; newer flags are not available), `xprop`, `xwininfo`, `wmctrl`, `xwd`, `xclip`, `wl-clipboard`, `gsettings`, `gdbus`, `dbus-monitor`, `gnome-shell` and Openbox are on the box, with the helper tools under `~/.local/ad-tools` (`root/usr/bin`, `qtvenv`, `vscode`). Missing tools exit `2` at the gate.
- Settings that GNOME apps write go to a private keyfile through `GSETTINGS_BACKEND=keyfile` in the private runtime directory (`L23-27`); that is why restoration is cheap and why it is still asserted.
- `scripts/private-session.sh` is 3.5's script (Xvfb from `:100`, private bus, optional Openbox, member sweep at exit). It stays the outer wrapper. The shell tiers are composed inside it by the landed wrappers `scripts/private-mutter-shell.sh` (own Xvfb from `:150`, `gnome-shell --x11`, Overview closed through `OverviewActive` and Escape), `scripts/private-wayland-shell.sh` (`--headless --wayland --no-x11 --virtual-monitor`, five consecutive RemoteDesktop create/start/stop cycles as the gate, eight attempts, killed by process group) and, from 3.14, `scripts/notification-live.sh` (`gnome-shell --x11 --replace`, three consecutive `GetServerInformation` answers inside 25 s, four attempts). This sub-phase fixes the cancellation defect in the outer wrapper and composes the three; it writes no fourth launcher.
- `tests/e2e/test_harness_contract.py` forbids permanent `rm` in shell under `scripts/` and `tests/e2e/`, and already carries named exemptions (with reasons) for `private-session.sh` and `test-private-session.sh` in `SCRATCH_CLEANUP_EXEMPTIONS`. A new script that needs `rm` gets its own named entry; the scan list gains `tests/e2e-linux/*.sh`.
- `scripts/check-bash3-compat.sh` and `shellcheck` scan `scripts/*.sh` and `tests/e2e/*.sh`; U3 extends both to `tests/e2e-linux/*.sh` and keeps the entry script to bash 3.2 syntax. The Python modules are not shell-scanned.

### Key Technical Decisions

- KTD1. **A Python runner with shell only at the entry.** `tests/e2e-linux/run.sh` (build, prerequisite gate, lease handoff) execs `python3 -I tests/e2e-linux/runner/main.py`; everything else is Python 3 standard library plus PyGObject imported lazily. Rationale: structured oracles, D-Bus reads and a typed leg registry are painful in bash 3 and the Windows harness is already a native, separate suite rather than a port of `lib.sh`. The macOS harness is not edited. Governs R1, R6.
- KTD2. **Oracles are independent of the product by construction.** `oracle/atspi_raw.py` talks to the accessibility bus over Gio D-Bus (registry, `GetChildren`, `GetState`, `Action.DoAction`, `Text`, `Value`, `GetExtents`) with no import of or call into the product; `oracle/gsettings.py` runs `gsettings get` against the private keyfile; `oracle/x11.py` reads `_NET_ACTIVE_WINDOW`, `_NET_CLIENT_LIST_STACKING`, window geometry and pointer position with `xprop`/`xdotool`/`wmctrl`; `oracle/proc.py` reads `/proc`; `oracle/pixels.py` decodes `xwd`; `oracle/clipboard.py` uses `xclip` and `wl-copy`/`wl-paste`. Raw role names and states are asserted where the claim is raw (`SENSITIVE`) and the canonical vocabulary where the claim is the CLI's (`enabled`, `disabled`) (dossier 4.2). Governs R5.
- KTD3. **The leg registry is the acceptance matrix.** `legs.py` holds one record per leg: id, requirement, target app, backend tier, mode, oracle kinds, disposition. Scenario files register implementations by id. A contract test fails when a registered leg has no implementation, an implementation has no record, a record has no requirement, or an `owner-decision` id is absent from `docs/phases.md`. A leg's verdict helper refuses to pass unless it recorded a before-read and an after-read from an oracle; this enforces R5 mechanically. Governs R5, R6, R29.
- KTD4. **Launch ownership by registry diff, generation by (pid, start time).** The runner snapshots the AT-SPI registry owners before a launch, spawns the app in its own process group inside the private session, and takes ownership of the new registry owners whose `/proc/<pid>/environ` carries the private runtime directory. Terminal's launcher exits at once and its owner is `gnome-terminal-server` (`L23-1`); Files and Terminal gain a window on a second launch while Settings and Text Editor raise the existing one and Calculator forks a second process (`L23-2`), so counts of owners per name are asserted, never assumed. Every command that takes a window uses `--window-id`, because `--app` with two same-name processes silently picks one (`L23-36`). Governs R2, R11.
- KTD5. **One runtime directory per tier, supplied by `private-session.sh`, with the shell composed by the landed wrappers.** The runner calls `private-session.sh --wm -- <payload>` for `x11-openbox`, `private-session.sh -- private-mutter-shell.sh -- <payload>` for `x11-mutter`, and `private-session.sh -- private-wayland-shell.sh -- <payload>` for `wayland`; the shell-surface and notification legs use the mutter tier with the `notification-live.sh` recipe (AppIndicator and dock extensions enabled through the keyfile settings backend) unless U1 shows `private-mutter-shell.sh` answers the Notifications interface with the same extensions, in which case the three launchers collapse into the mutter wrapper and `notification-live.sh` becomes a thin caller. The health gates are the landed ones (RemoteDesktop cycles for Wayland, `OverviewActive` for mutter, `GetServerInformation` for the notification recipe); this plan does not replace them, and U1 re-measures start-up reliability before the numbers go into the roadmap. 3.14 saw the headless Wayland shell never answer the Notifications interface in 6 gated starts, so the `GetServerInformation` gate is not used for the Wayland tier. The runner never hardcodes a display: it reads `DISPLAY` from the tier. Bus-activated apps inherit the activation environment, which the outer wrapper sets to its own Xvfb (`:100` and up), not the mutter shell's `:150` and up, so the mutter tier pushes its `DISPLAY` and `XAUTHORITY` into the activation environment before any app starts and the runner reads each launched window's display back (`L18-17`). Governs R4.
- KTD6. **Cancellation: payload in the background, trap forwards to the group, sweep by membership.** Bash defers a trap until the foreground child exits, which is why `SIGTERM` left four members and the directory behind (`L23-34`). `private-session.sh` runs the payload in its own process group in the background and `wait`s; the `TERM`/`HUP`/`INT` trap signals that group, escalates after a grace period, then runs the existing member sweep and removes the directory. A `SIGKILL` of the wrapper cannot be trapped, so the runner starts the wrapper in its own process group, records the runtime directory, and on any exit path signals the group and sweeps by `XDG_RUNTIME_DIR` membership before checking that nothing references the directory. The inner wrappers start the shell with `setsid`, which leaves the payload's group, so the sweep by membership (not the group signal) is what reaches the shell. All signals go to pids or groups, never to names (R36). Governs R3.
- KTD7. **Shared lease: the canonical lock for real runs, a private root for self-tests.** The runner takes `interaction_lock.py`'s canonical lease (`/tmp/agent-desktop-<euid>/interaction.lock`) the way `tests/e2e/run.sh` does and passes the inherited descriptor through `bounded_process.py`; the Linux crate reads the same descriptor through `system/interaction_lease.rs`. Self-tests use a private lock root (`L23-32`) so they never block other agents on the shared box. Both modules are imported from `tests/e2e/` by path, not copied. `scripts/remote-linux.sh` adds an outer box lock (`~/ad-wt/.box.lock`) and a 3600 s command timeout, so the lease and the box lock nest; a full three-tier run sets `AGENT_DESKTOP_REMOTE_TIMEOUT` or runs one tier per call (R36). Governs R1.
- KTD8. **Bounded children everywhere.** Every external command, product call and oracle tool runs through `bounded_process.py` with an absolute timeout and an output cap (hung child plus grandchild reaped in 2.25 s, flood capped at 2000 bytes in 255 ms, `L23-32`). Governs R1, R8.
- KTD9. **Headed scope is X11 in both modes; Wayland headed is an asserted refusal.** Core calls `focus_window` before every headed ref action, and 3.6 and 3.9 refuse Wayland focus, so a Wayland headed ref action cannot reach dispatch whatever the assertion says (dossier 4.4). ACTIVE does appear once a seat keyboard exists (`L23-29`), so the refusal leg asserts the landed focus decision's exact error and a second leg reads `ACTIVE` through the raw oracle. Under private mutter a never-focused window ignores `_NET_ACTIVE_WINDOW` (`L23-14`); the mutter headed legs assert whichever behaviour 3.6's `focus_window` landed with, and a refusal there is reported as the finding it is, not rescued by a click in the harness. Both are owner items 1 and 2. 3.14 adds a Wayland pointer route for shell controls only (absolute motion on a screen-cast stream, `L32-46`); it does not change core's pre-dispatch focus for application refs, so the refusal legs stand and U1 re-reads the landed decision before they are written. Conflict call-out: this narrows "every ref action in both modes" for Wayland only, because the alternative is a harness that bypasses the product's focus step. Governs R21, R23.
- KTD10. **The delayed-enable helper is a thread in the runner using raw AT-SPI.** It reads `Undo` disabled, waits a controlled delay, then presses `7` through the oracle's `DoAction`, never through the product. Propagation of about 260 ms is expected (`L23-10`), so assertions bracket the delay, not an instant. Exactly-once is read from the Calculator display node (the first `text box` with the Text interface): one dispatch leaves `7`, two leave an empty display (`L23-11`). Governs R9.
- KTD11. **Opening and focus routes are measured, not invented.** Calculator `About`: `Main Menu` toggle, then the last popover item; the new app-root child is both a `window` and a `sheet` under one id (`L23-15`). Terminal `Preferences`: show the menu bar through `View`, open `Edit`, then activate `Preferences` while the menu is open (`L23-17`). Files `Properties`: select all in the view, `Alt+Return` (`L23-18`). Files list view: the private keyfile sets `org.gnome.nautilus.list-view use-tree-view` and `default-folder-viewer`, reset afterwards. GTK4 popovers settle for 0.66 s, so every wait on a popover has a deadline of at least 1 s (`L23-19`). Pointer-opened libadwaita combos need window-coordinate extents plus the X11 window origin, because GTK4 screen extents are `[0,0,w,h]` (`L23-37`). Governs R14-R18.
- KTD12. **Waiting and failing are the product's own primitives.** Legs use the CLI's `wait` and the oracle's deadline polls, record both reads, and on failure store before/after evidence in the suite directory for the operator to reproduce with `--leg <id>`. The runner never confirms a failure by rerunning; the dogfood's two-observation rule is the operator's protocol, so `--leg` exists and runs one leg in a fresh tier. Governs R8.
- KTD13. **Roadmap corrections the evidence forces** (carried as U11): the zero-bounds target is not reachable "by index path" and the Files and Settings containers are raw-geometry coverage only (dossier 4.1, `L23-8`); the enabled recipe pins raw `SENSITIVE` and canonical `enabled` separately (4.2); duplicate-title splits into duplicate elements and duplicate windows (4.3, `L23-6`, `L23-36`); headed Wayland focus is an asserted refusal (4.4); "same contract as `run.sh`" becomes an enumerated Linux acceptance map (4.5); the "no stock app presents" heading becomes "not yet reached in the sampled stock apps" (4.6); `L8-7` is owned by 3.4 and 3.12 owns only the restart scenario (4.7); the Qt window "the harness constructs" is a fixture and is removed. Governs R28-R30.
- KTD14. **Size is stated, not shrunk to the estimate.** The roadmap's ~2k lines is not validated (dossier section 8). The plan estimates about 7k lines now that pointer, lifecycle, shell, tray, Qt and VS Code legs are in: runner and session ~900, oracles ~1k, scenarios ~3.6k, self-tests ~1.2k, shell and CI ~400; U13 replaces the estimate with the measured count. The seams for two PRs, if the owner asks, are after U8 (infrastructure and X11 legs) and after U14 and U15 (shell, Wayland, toolkits and CI). KD7 keeps one PR. Governs R6.

- KTD15. **Mutter discipline.** Before every leg on `x11-mutter` the runner reads `OverviewActive`, sends Escape if it is true, and waits on the read-back (at least 2.5 s allowed, never a fixed sleep as the verdict); the desktop-switch setup reads `_NET_CURRENT_DESKTOP` (or the shell's workspace interface) until it shows the switch before `focus_window` is asserted; mutter lifecycle legs run with no other agent on the box (the lease and box lock guarantee it) and assert the measured outcome sets of `L31-53`, `L31-54` and `L31-56`, not a single fixed result. Governs R25, R33.
- KTD16. **Shell oracles are the landed 3.14 helpers.** The independent notification sender, closer and `dbus-monitor` recorder are `scripts/notification-oracle.sh` called by path; the shell tree is read by raw Atspi in `oracle/atspi_raw.py`; the tray emitter is a harness-side StatusNotifierItem sender (no stock GNOME app registers one; the 3.14 sender lives as a Rust string in `crates/linux/src/tray/live_scripts.rs` and is not importable, so the harness carries its own short Python sender under `tests/e2e-linux/apps/` and logs every call it receives as the effect oracle); the shell is closed after each open-surface leg by the same ref that opened it, with the close read back. Governs R34.

### High-Level Technical Design

*This illustrates the intended approach and is directional guidance for review, not implementation specification.*

One run:

```
run.sh: build --locked --release -> gate (tools, session, lease) -> stage + hash
   |-> per tier in {x11-openbox, x11-mutter, wayland}:
   |     private-session.sh --wm | -- private-mutter-shell.sh | -- private-wayland-shell.sh   (own group; the wrapper's gate)
   |       |-> runner: launch app -> attribute (registry diff + runtime-dir membership)
   |       |-> leg: oracle before -> product command (--window-id, ref) -> oracle after
   |       |-> verdict: both reads present, effect matches, restored, owned set unchanged
   |     teardown: signal group -> sweep by membership -> assert nothing references the dir
   `-> report per tier x mode; exit 0 | 1 | 2
```

Backend tiers and what each can show:

| Tier | Window manager | Input | Focus and `ACTIVE` | Legs it owns |
|---|---|---|---|---|
| `x11-openbox` | Openbox (focus on map) | XTEST | `_NET_ACTIVE_WINDOW` and frame `active` | headed and headless, occlusion (ordinary and above-layer), text entry |
| `x11-mutter` | private `gnome-shell --x11` | XTEST | never-focused window ignores activation (`L23-14`) | the same legs plus shell surfaces, notifications, tray (R34); focus behaviour reported separately |
| `wayland` | private headless `gnome-shell --no-x11` | RemoteDesktop keys only | no `ACTIVE` until a seat keyboard exists (`L23-29`) | semantic actions, keys, clipboard, asserted headed refusal, 3.6's Wayland arms (`hit_test` `Unknown`, `focus_window` `wayland_session`, no `list-windows` position) |

Leg lifecycle:

```
registered -> setup (launch/open, settle by deadline) -> before-read -> act -> after-read
           -> restore -> restoration read -> owned-set check -> verdict
```

### Sequencing

U1 settles the landed surface and the deferred measurements before any scenario code. U2 and U3 are independent after it; U4's oracles are tested against real apps only once U3's session runs. U5-U8 add scenario families in any order; U9 needs U2's shell tiers. The seam for splitting the PR is after U8.

---

## Implementation Units

| Unit | Title | Files touched | Depends on |
|---|---|---|---|
| U1 | Entry gate, acceptance map and deferred re-measurements | `tests/e2e-linux/runner/legs.py` | landed 3.5-3.11 and 3.14 |
| U2 | `private-session.sh`: cancellation; shell tiers composed from the landed wrappers | `scripts/private-session.sh`, `scripts/private-mutter-shell.sh`, `scripts/notification-live.sh`, `scripts/test-private-session.sh` | U1 |
| U3 | Runner envelope | `tests/e2e-linux/run.sh`, `runner/` | U1 |
| U4 | Oracles and app drivers | `tests/e2e-linux/oracle/`, `runner/apps.py` | U3 |
| U5 | Headless semantic scenarios | `tests/e2e-linux/scenarios/` | U4 |
| U6 | Headed X11, pointer gestures, input, occlusion, non-interference | `scenarios/headed_actions.py`, `pointer.py`, `occlusion.py`, `modal.py`, `text_entry.py`, `non_interference.py` | U5 |
| U7 | Waits, identities, menus, surfaces, batch, lifecycle and window operations | `scenarios/events.py`, `menus.py`, `identity.py`, `surfaces.py`, `launch.py`, `lifecycle.py` | U5 |
| U8 | Capture and clipboard | `scenarios/capture.py`, `clipboard.py` | U5 |
| U9 | Wayland tier and KDE manual protocol | `scenarios/wayland_*.py`, `README.md` | U2, U7 |
| U10 | CI job and self-test lane | `.github/workflows/ci.yml`, `.github/workflows/linux-e2e.yml` | U3 |
| U11 | Roadmap and ledger corrections | `docs/phases.md`, `probes/linux/FINDINGS.md` | U5-U9, U14, U15 |
| U12 | Live verification and invert | none (runs) | U5-U9, U11, U14, U15 |
| U13 | Stranger dogfood | `docs/dogfood-reports/` | U12 |
| U14 | Shell surfaces, notifications, tray | `scenarios/shell_surfaces.py`, `notifications.py`, `tray.py`, `apps/sni_item.py` | U5, U2 |
| U15 | Qt and VS Code toolkit legs | `scenarios/toolkit_qt.py`, `toolkit_electron.py`, `apps/qt_window.py` | U5 |

### U1. Entry gate, acceptance map and deferred re-measurements

- **Goal:** know exactly what the landed stack offers, and settle the legs `L23` could only defer, before writing a scenario.
- **Requirements:** R6, R28, R29.
- **Dependencies:** 3.5 U6-U8 (`find`, `is`, `get`, strict resolution), 3.6 U5-U7, 3.7 U6-U11, 3.8 U5-U7, 3.9 U5-U9, 3.10 U5-U12, 3.11 U5-U7 landed; `scripts/check-linux-findings.py --sub-phase 3.12`.
- **Files:** `tests/e2e-linux/runner/legs.py` (the registry skeleton, one record per leg, all `outstanding` until their unit lands); new ledger rows under the next free area number at the cut (probe script outside the repository).
- **Approach:**
  1. Run each CLI command the harness drives against a real app in a private session and record any `PLATFORM_NOT_SUPPORTED`: `find`, `is`, `get`, every ref action, the pointer commands (`hover`, `drag`, `mouse-*`), `press`, `focus-window` and the window operations, `launch`, `close-app`, `wait --event`, `wait --menu`, `wait --notification`, `screenshot`, `clipboard-get/set/clear`, `list-surfaces`, `open-system-surface`, `list-notifications`, `dismiss-notification`, `dismiss-all-notifications`, `notification-action`. Any hit is a stop condition (the answer to `L23-7`, `L23-9`, `L23-16`, `L23-31`, `L23-38`).
  2. Read the landed behaviour the legs assert and record it as the leg's expected outcome, with the file read: Wayland focus (refusal text), mutter focus of a never-focused window, modal-blocked ref action, `select` on a libadwaita combo, the Calculator `listbox` ref outcome, the `list-surfaces` window-and-sheet dedupe (owner item 3), `snapshot --app` with two same-name processes.
  3. Read `tests/e2e/run.sh` and every scenario file and assign each scenario family a Linux disposition (R28): observation, interaction, the macOS acceptance scenarios AE1-AE7, reliability, surfaces and notifications here; FFI to 3.13; trace and session isolation as one leg each here; multi-agent cursors recorded as an owner decision; `electron-live.sh` becomes the VS Code leg of U15.
  4. Re-measure with the CLI the rows `L23` deferred and the cells still open: the `Replace All` typed path through the product, the Terminal `Copy` selection preparation, Undo exactly-once under product dispatch, the 300-file scroll through the product route, combo select, and headed text entry. Each gets a row with a verdict; a contradicted KTD is rewritten before its unit starts.
  5. Confirm the entry assumptions of the sibling plans against the landed code (detector, stable app names under `SIGSTOP`, `process_state`) and record any that fail as findings, not workarounds.
  6. Probe, before any code, the cells the refresh gate found open (rows to `plans/3.12-probe-rows.md`, then the repository ledger area **L34**): the display a bus-activated Terminal and Files take under `private-mutter-shell.sh`; whether that wrapper's shell owns the Notifications interface with the extensions enabled through the keyfile; headed pointer gestures on GTK4 at two positions under openbox and under mutter; the VS Code launch that keeps its frame registered (`L18-8`); the Qt window recipe and its log oracle; mutter lifecycle flake rate over 20 cycles; Terminal `Preferences` and Files `Properties` openers through the CLI; and whether the Wayland shell answers the Notifications interface at all.
- **Test expectation:** none -- a measurement and mapping run; the registry skeleton is exercised by U3's contract test.
- **Verification:** every deferred row has a verdict, every `run.sh` family has a disposition, and the stop conditions were checked.

### U2. `private-session.sh`: cancellation; shell tiers composed from the landed wrappers

- **Goal:** the outer wrapper cleans up when cancelled, and the three shell launchers compose into the tiers without a fourth launcher.
- **Requirements:** R3, R4, R2.
- **Dependencies:** U1; 3.5's `scripts/private-session.sh` and `scripts/test-private-session.sh`; 3.8's `scripts/private-mutter-shell.sh` and `scripts/private-wayland-shell.sh`; 3.14's `scripts/notification-live.sh`.
- **Files:** `scripts/private-session.sh`, `scripts/private-mutter-shell.sh`, `scripts/notification-live.sh` (only the changes U1 shows are needed), `scripts/test-private-session.sh`; `tests/e2e/test_harness_contract.py` only if a script gains an `rm`.
- **Approach:** KTD5 and KTD6. In `private-session.sh` the payload `"$@" || status=$?` runs in the foreground, which is why `SIGTERM` waits for the child (`L23-34`); the payload moves to its own process group in the background and the script `wait`s, the `TERM`, `HUP` and `INT` trap signals that group, waits a grace period, escalates, then runs the existing member sweep and removes the directory. The mutter wrapper gains the activation-environment push for its own `DISPLAY` and `XAUTHORITY` (U1 item 6 decides whether it is needed). The wrappers keep their own gates; nothing here adds `--shell`. The existing `rm` exemptions cover `private-session.sh` and `test-private-session.sh`; any other script with `rm` gets its own named entry.
- **Test scenarios:** `SIGTERM` to the wrapper during a foreground payload leaves zero members and no directory within the grace period (fails on the old foreground wait); `SIGHUP` and `SIGINT` the same; a payload that ignores `TERM` is `KILL`ed after the grace period; the payload's exit status is the wrapper's; a shell started with `setsid` by an inner wrapper is gone after cancellation (membership sweep); `private-mutter-shell.sh` with a stub shell that never becomes healthy fails with its message and leaves no member; a Terminal or Files started by bus activation under the mutter wrapper maps on the mutter display; the existing two checks (missing openbox, inherited accessibility overrides) still pass. Stubs are shell functions in the test, not committed fixtures.
- **Verification:** the checks pass on the box through `scripts/remote-linux.sh bash scripts/test-private-session.sh` and invert (restore the foreground wait, drop the group signal, drop the membership sweep, drop the activation push) -- each inversion fails the test that names it.

### U3. Runner envelope

- **Goal:** one entry point that gates, stages, owns the session and lease, runs registered legs and exits with the right code.
- **Requirements:** R1, R2, R6, R7, R8, R25, R27.
- **Dependencies:** U1; 3.9 U9 only for the product `launch` leg (U7).
- **Files:** `tests/e2e-linux/run.sh`, `runner/main.py`, `runner/gate.py`, `runner/stage.py`, `runner/session.py`, `runner/verdict.py`, `runner/report.py`, `runner/legs.py`; `tests/e2e-linux/test_runner_contract.py`; `.githooks/pre-commit` and `scripts/check-bash3-compat.sh` file lists; `.github/workflows/ci.yml` shellcheck list; `scripts/check-no-phase-references.sh` (scan root extended to `tests/e2e-linux/`, with a planted-token case in its own self-test).
- **Approach:** `run.sh` builds `--locked --release -p agent-desktop` into the canonical target directory, takes the lease as the macOS script does (KTD7), and execs the runner. The gate checks the tool list, the interaction lease, a free box lock and the tier prerequisites and exits `2` with the first missing item. `stage.py` copies the binary to a read-only path under a suite directory made by `mktemp -d`, records the hash, and re-verifies the hash and `--version` identity before reporting success; the suite directory is removed with `trash` through a Python port of `trash_recoverably`, and a failed removal retains the directory and warns. `session.py` starts the tier through `private-session.sh` in its own process group, records the runtime directory, and on every exit path (finally, signal, interpreter exit) signals the group and sweeps by membership before asserting nothing references the directory (KTD6). `verdict.py` implements KTD3's rule: a leg passes only with a recorded oracle before-read and after-read, restoration read and an unchanged owned-process set. `report.py` prints per tier and mode, lists every leg with its disposition, and exits `0`, `1` or `2`. `--tier`, `--leg <id>` and `--lying-actions` (the AE2 wrapper, generated at runtime in the suite directory, not committed) are flags. Environment: the suite sets `AGENT_DESKTOP_HOME` to a private state root and `XDG_CURRENT_DESKTOP=GNOME` (needed by Settings, `L4-25`) inside the session, not outside it.
- **Test scenarios:** a leg that records no oracle read fails; a leg that passes while the owned process set changed fails; an unregistered implementation, an unimplemented record, a record without a requirement and an `owner-decision` id missing from `docs/phases.md` each fail the contract test; the seeded failing leg gives exit `1`; a missing tool gives exit `2` before any session starts; a staged binary altered mid-run fails the run; a malformed product envelope is a leg failure with the raw output stored, not a crash; a hung product call and a flooding one are bounded and reported; a teardown that cannot remove the session directory fails the run; the `rm` rule in the shell contract test also scans `tests/e2e-linux/*.sh`. Pure logic runs on the hosted lane; session tests run on the box.
- **Verification:** the self-tests pass and invert (accept a leg with no oracle read, skip the owned-set check, swallow a teardown failure, return `0` after a failed leg, skip the hash re-check) -- each inversion fails the test that names it.

### U4. Oracles and app drivers

- **Goal:** read the desktop without using the product, and launch and attribute real apps.
- **Requirements:** R2, R5, R7, R9, R22.
- **Dependencies:** U3.
- **Files:** `tests/e2e-linux/oracle/atspi_raw.py`, `gsettings.py`, `x11.py`, `proc.py`, `pixels.py`, `clipboard.py`; `runner/apps.py`, `runner/helper.py`; `tests/e2e-linux/test_oracle_contract.py`.
- **Approach:** KTD2, KTD4, KTD10. `atspi_raw.py` finds an app by registry owner, walks `GetChildren`, matches by raw role and name with an index-path fallback for the zero-axis raw coverage only, and exposes `states`, `extents` (both coordinate types, `L23-37`), `text`, `value`, `do_action(name)` and a `poll(predicate, deadline)`. `apps.py` launches, attributes, counts owners per name, waits for a window by deadline, stops an app by its group, and re-launches with a new generation. `helper.py` runs the delayed trigger on a thread and records when it fired. `pixels.py` decodes `xwd` and `PNG` with the standard library. The oracle library imports `gi` lazily so the pure parts run on a host without it.
- **Test scenarios:** the XWD and PNG decoders on bytes built in the test (a decoder that ignores stride fails the row test; a PNG with the wrong CRC is rejected); `poll` returns at the deadline and not before the predicate; the state-bit table maps `SENSITIVE`, `SHOWING`, `EXPANDED`, `CHECKED`, `ACTIVE` from raw bitsets; attribution picks the owner whose environment carries the runtime directory and refuses a foreign pid; the owner count for two Calculator processes is two; the helper fires after its delay and records the time. Real-app reads are exercised by U5-U9.
- **Verification:** pure tests pass and invert (stride ignored, CRC unchecked, first owner taken regardless of runtime directory); the oracle against a live Calculator in a private session returns the `Undo` state in both directions.

### U5. Headless semantic scenarios

- **Goal:** every ref action and read works headless on its named real-app target, verified by the oracle.
- **Requirements:** R5, R9, R10, R11, R12, R13, R14, R15, R16, R17, R24.
- **Dependencies:** U4; 3.5 U6-U8 (strict resolution, `find`, live reads), 3.6 U6 (live `visible`), 3.7 U4-U10 (click, toggle, expand and collapse, set-value, clear, type, select), 3.4 window ids.
- **Files:** `tests/e2e-linux/scenarios/observation.py`, `state.py`, `values.py`, `disclosure.py`, `select.py`, `placeholder.py`, `scroll.py`, `zero_bounds.py`, `duplicates.py`.
- **Approach:** each family is the R-ID named, on the target KTD11 names, with the oracle in the channel the claim lives in:
  - Observation and reliability: qualified refs survive unrelated snapshots; stale, drill-down, selector waits; session isolation; one trace-segment leg; `is enabled` pinned canonical and raw `SENSITIVE` pinned raw (R9, 4.2).
  - State: Calculator `Undo` with the delayed helper under default, explicit and short timeouts, exactly-once by display, plus the permanently disabled `TIMEOUT` with `actionability_timeout` near 2 s, the untouched default near 5 s and the zero-timeout single check (`AE3`, `AE7`, `AE2`); Terminal `Copy` with a raw-prepared selection; the Text Editor semantic `type` path (the `Replace All` gating needs keys and is U6's leg) (R9).
  - Values: `Hot Corner` flip and restore against `enable-hot-corners`; `Pointer Speed` and integer `Number of Workspaces` against `speed` and `num-workspaces` with panel reopen; the fractional case is not a leg (`L23-27` shows readback without commit) and is written as a limit in the roadmap (R13).
  - Disclosure: Files tree view, expanded and children both ways, view setting reset and read back (R14). Placeholder, select, scroll, zero bounds and duplicates per R10-R12, R15, R16 and the observed `L23` shapes. The zero-bounds legs assert the outcome U1 recorded for the Calculator `listbox`.
  - Non-interference: each headless action leg also reads pointer position and `_NET_ACTIVE_WINDOW` before and after on the X11 tiers (R24).
- **Test scenarios:** per leg, the positive and its twin in the same session: the action changes the oracle value and restoring returns it; the negative twin (the same command on a disabled, absent or stale target) leaves the oracle unchanged and returns the matching error code; a stale ref after the app is killed fails the leg with `STALE_REF` or `ELEMENT_NOT_FOUND`; zero-timeout reads in `false` immediately.
- **Verification:** all legs pass in the private session on `x11-openbox`; invert by running with the lying wrapper (every effect leg fails, AE2) and by breaking one oracle assertion per family (display compared to empty instead of `7`, key compared to the pre-state, children presence ignored) -- each fails its named leg.

### U6. Headed X11, input, occlusion, non-interference

- **Goal:** every ref action also works `--headed` on X11, the product's own focus step is observed, and a covering window is refused only when it survives preparation.
- **Requirements:** R5, R18, R21, R24, R25.
- **Dependencies:** U5; 3.6 U3, U5, U7 (stacking, `hit_test`, X11 `focus_window`), 3.7 U11, 3.8 U2, U4-U6 (keyboard vocabulary, X11 sink, physical legs).
- **Files:** `scenarios/headed_actions.py`, `occlusion.py`, `modal.py`, `text_entry.py`, `non_interference.py`.
- **Approach:** the pointer family (R32, `pointer.py`) runs `click`, `double-click`, `triple-click`, `right-click`, `hover`, `drag`, `mouse-move`, `mouse-click`, `mouse-wheel` and `scroll` on GTK3 (Terminal), GTK4 (Calculator, Files, Settings combos) and Qt (U15 supplies the window) at two window positions on **both** X11 tiers, reading the pointer position, the application's own effect and raw extents plus the X11 client origin (`L23-37`), so a client-side-shadow offset that openbox hides fails under mutter. The headed repetition of U5's action legs on `x11-openbox` and `x11-mutter`, each reading `_NET_ACTIVE_WINDOW`, the frame `active` state and the effect. Text entry: `type`, `press ctrl+f`, `press ctrl+h` in Text Editor through the product after the product has focused the window (`L23-12`); the typed text and the search bar controls are read raw; `Replace All` gates on typed search text and releases on removal (`L23-13`). Occlusion: Calculator over Text Editor, ordinary and `_NET_WM_STATE_ABOVE` (set with `wmctrl`), both WMs (`L23-25`): ordinary cover, headed click lands in the editor after preparation; above-layer cover, the product refuses and neither document changes (AE5). Modal: Files `Properties`, Terminal `Preferences`, Calculator `About` each block a ref action on their parent per the outcome U1 recorded, and a Text Editor action in another app is unaffected (`L23-26`). Under `x11-mutter` the focus leg asserts the landed behaviour for a never-focused window; a refusal is reported with `L23-14`, never worked around (KTD9). Non-interference pairs a successful headless effect with unchanged pointer and active window; a product failure that leaves the desktop untouched cannot pass, because the effect read is required.
- **Test scenarios:** per tier, headed twin of every U5 action leg; modal parent action blocked with no oracle change; cover removed then the same click lands; above-layer cover refused with no effect in either window and the cover still above after the call; typed text reaches the document and a key held without release is not left pressed (`xdotool` key state read); a window that never became active gives the landed refusal and no input arrives in any window.
- **Verification:** pass on both X11 tiers; invert by dropping the `ABOVE` flag (AE5 must then land the click, proving the leg distinguishes), pointing the oracle at the wrong window, and removing the focus read -- each fails its named leg.

### U7. Waits, identities, menus, surfaces, batch

- **Goal:** the event waits, window identity and the seeded surface batch behave on real apps through the CLI.
- **Requirements:** R11, R17, R19, R20.
- **Dependencies:** U5; 3.9 U5-U9 (`close_app`, `window_op`, `focus_window`, `launch_app`), 3.11 U5, U7 (and U8 if built), 3.4 identity.
- **Files:** `scenarios/events.py`, `menus.py`, `identity.py`, `surfaces.py`, `launch.py`.
- **Approach:** events: Text Editor `--new-window` (`window-opened`, not `app-launched`, `L4-26`), Calculator launch and close (`app-launched`, `app-terminated`), two-window `focus-changed` on the X11 tiers, `surface-appeared` and `surface-dismissed` for Calculator `About`, Terminal `Preferences` and Files `Properties`, each with a no-transition timeout and a scoped and an unscoped form; a target killed mid-wait. Menus: `--menu` and `--menu-closed` both directions on Calculator and Terminal, the already-open case, and the hidden-bar timeout message. Identity: two Calculator processes and Text Editor `Ctrl+N` with `--window-id` (R11), restart then stale id. AE4 as one batch with the baseline seeded before the opener (the analog of the macOS acceptance scenario of that name), deduping the window-and-sheet pair by id. Product `launch` of each app is a leg here with attribution checked by the harness's own registry diff. `lifecycle.py` (R33) drives `close-app`, `focus-window`, `resize-window`, `move-window`, `minimize`, `maximize` and `restore` on both X11 tiers with `xprop`, `xwininfo` and `/proc` as oracles and the KTD15 mutter discipline; a leg that only passes with a fixed sleep is a finding.
- **Test scenarios:** each wait twin has a transition and a quiet run; a window that opens and closes inside one wait is reported; the batch seed does not abort on a healthy desktop; a menu predicate reads the already-open state at once; restart makes the old id `WINDOW_NOT_FOUND`; the two Calculator processes keep distinct ids and each snapshots.
- **Verification:** pass on both X11 tiers (`focus-changed` cannot occur on Wayland without a seat keyboard; U9 owns that leg); invert by seeding the baseline after the opener (AE4 must fail), by comparing against the title instead of the id (the dedupe row fails), and by dropping the process-instance check from the restart row.

### U8. Capture and clipboard

- **Goal:** screenshots and clipboard are judged on content.
- **Requirements:** R22.
- **Dependencies:** U5; 3.10 U5-U6 (window and display capture), U9-U12 and U16 (clipboard read, selection owner, setter, clear, Wayland), U14 and U17 live legs.
- **Files:** `scenarios/capture.py`, `clipboard.py`.
- **Approach:** capture a window with typed text and compare the decoded product pixels with `xwd` of the same window by region similarity and a stable-at-idle check, and again after the text changes (the PNG magic bytes or a portal node are not evidence, `L13-21`); clipboard formats round-trip against `xclip` and `wl-copy`/`wl-paste` as foreign producer and reader, including ownership lifetime after the setter exits and `clipboard-clear`; the foreign `xclip` servers are stopped by pid at the end (`L23-38`). Routes 3.10 left gated stay gated and become `asserted-refusal`.
- **Test scenarios:** content change moves the compared region; an identical window compares equal across two captures; a blank PNG fails; a clipboard text, html and image round trip; the owner outliving the setter; clear leaves an empty clipboard; a foreign read of a product-set selection after the CLI exited.
- **Verification:** pass on the X11 tiers and on `wayland` for the shipped routes; invert (compare to a different window, skip the second capture, read the clipboard before the setter returns) -- each fails.

### U9. Wayland tier and KDE manual protocol

- **Goal:** the headless Wayland tier is exercised with what it can do and the refusals it must give; KDE has a written protocol.
- **Requirements:** R4, R23, R25.
- **Dependencies:** U2, U7; 3.8 U7 (RemoteDesktop keyboard), 3.6 U7 (Wayland focus refusal), 3.10 U11-U12, 3.11 (no `ACTIVE`).
- **Files:** `scenarios/wayland_actions.py`, `wayland_input.py`, `wayland_focus.py`; `tests/e2e-linux/README.md` (KDE section).
- **Approach:** semantic actions with no input device (display `7`, `L23-29`); the seat check separate from the shell check (`wl-paste --list-types` "no keyboard" is a distinct outcome from a hung shell); after the first RemoteDesktop key the focused window carries raw `ACTIVE` and the later-mapped window takes focus; the product's first key is asserted to land by read-back, and a miss is a finding (`L23-30`); `Ctrl+F` via two keycodes and clipboard paste work. Headed ref action on Wayland returns the landed focus refusal, asserted by code and message (KTD9); `focus-changed` on a session with no `ACTIVE` times out and the timeout carries `baseline_counts`. The 3.6 Wayland arms run here (`hit_test` answers `Unknown`, `focus_window` refuses `wayland_session` before anything is sent, `list-windows` reports no position). The README's KDE section lists the manual matrix (apps, steps, expected shape, record sheet) and states it is outstanding until a Plasma 5.24+ session runs it.
- **Test scenarios:** the tier fails with the named reason when the shell never becomes healthy (U2's stub); the seat-without-keyboard case is reported as the seat and not the shell; a key sent before the throwaway period is not rescued by the harness; modifiers work; each refusal leg fails if the product instead dispatches.
- **Verification:** pass on `wayland`; invert (assert success for the headed refusal, replace the read-back with `ok: true`, treat a missing seat as a shell failure) -- each fails.

### U10. CI job and self-test lane

- **Goal:** a dispatchable `linux-e2e` job that cannot go green without a real session, and the desktop-free self-tests on the hosted lane.
- **Requirements:** R26, R27.
- **Dependencies:** U3.
- **Files:** `.github/workflows/linux-e2e.yml`, `.github/workflows/ci.yml` (the self-test and shellcheck steps).
- **Approach:** the workflow is `workflow_dispatch`, runs on a runner label supplied by repository variable `LINUX_E2E_RUNNER`, and its first step is the harness gate, which exits `2` unless the lease, session, tools and a GNOME shell are present; there is no `continue-on-error` and no skip step. The hosted lane adds `python3 -I -m unittest discover -s tests/e2e-linux -p 'test_*.py'` for the pure tests (the session tests mark themselves skipped only through an explicit environment gate that the hosted lane sets and the box does not) and the shellcheck and bash-3 lists. The interactive runner is an owner item (R29); until it exists the job is written and unproven, which U12 and the PR description say plainly.
- **Test scenarios:** the workflow refuses to start on a runner without a session (gate exit `2`); the hosted self-test step fails when a pure test fails; the shellcheck list includes the new entry script.
- **Verification:** `actionlint` or the repo's workflow check, a dispatch dry run where the owner has a runner, and a deliberately failing self-test seen red on the hosted lane.

### U11. Roadmap and ledger corrections

- **Goal:** `docs/phases.md` and the ledger say what is true after this PR.
- **Requirements:** R28, R29, R30.
- **Dependencies:** U5-U9, U14, U15.
- **Files:** `docs/phases.md`, `probes/linux/FINDINGS.md`, `skills/agent-desktop/references/commands-system.md` only if a finding changes a shared command's documented behaviour.
- **Approach:** each correction in place, no "previously" text, each citing its row:
  - §3.12 scope: the eleven replacement sentences from `L23` section 3 (duplicate elements and windows, zero-bounds receiving no ref, delayed enable by helper with the display oracle, the input path and `Replace All` gating, Terminal `Preferences`, the Files tree view, Settings `Applications` placeholder, the combo select recipe, scroll by scrollbar write with polling, occlusion ordinary and above-layer, Wayland `ACTIVE` with a seat keyboard); the heading becomes "Edge cases not yet reached in the sampled stock apps" and lists only the unreached cases; the Qt-window sentence becomes the toolkit probe of KD11 (or an owner decision if refused); the 3.6 Wayland arms bullet and the mutter-tier bullet become what was built; the Chromium launch bullet becomes the VS Code leg; `Depends on` adds 3.4, 3.5, 3.6 and 3.14; the exit criterion names the backend tiers, the asserted refusals and the owner decisions; the size estimate is replaced by the measured count and the reason.
  - §3.8 exit: headed gestures pass on X11 under both openbox and mutter, and Wayland carries the keyboard route plus the asserted pointer refusal. §3.14 `Depends on` loses 3.12 (3.14 now lands first) and its text about the harness recipe is written as what 3.12 consumes.
  - The enumerated Linux acceptance map (R28) with 3.13 (FFI) and 3.14 (notifications, shell surfaces) receiving their scopes in detail sufficient to act without this PR, and the cursor-overlay owner decision.
  - Owner items under the Phase 3 introduction: Wayland focus (`L23-31`), mutter focus fallback (`L23-14`), window-and-sheet dedupe (`L23-15`, 3.11's), app-level same-name ambiguity (`L23-36`), cancellation (fixed here), interactive runner and KDE (`L23-35`, `L23-39`), live-session RemoteDesktop consent, the named ref-able zero-size control, the role-and-name duplicate pair, semantic-only select, and the core items from dossier section 6.
  - Ledger: `L23` is a shared area that earlier sub-phases already fill by copying the rows they cite (`L23-37` from 3.6; `L23-1`, `L23-2`, `L23-4`, `L23-29`, `L23-33`, `L23-36` from 3.9 and 3.11). It is not renumbered. 3.12 copies the remaining cited `L23` rows verbatim into that area without duplicating, and puts its own new rows (U1 and the lane rows) in area **L34**, the next free area after 3.11's `L33` (re-check at the cut). Dispose of every row the checker lists for 3.12 (37 on the 3.11 head, more once 3.14 is under it) and every `L23` row marked `DEFERRED`; amend `L8-7` to "3.4 implemented the identity; 3.12 owns the restart scenario"; amend `L18-15` where `L23-17` measured a Terminal `Preferences` route; append 3.12 clauses to rows several sub-phases claim, never replacing their dispositions.
- **Test expectation:** none -- documentation; `python3 scripts/check-linux-findings.py` and `scripts/check-no-phase-references.sh` run in the Verification Contract, and U3's contract test links `owner-decision` ids to the roadmap text.
- **Verification:** the checks pass and a reader of §3.12 can plan 3.13 and 3.14 without opening this PR.

### U12. Live verification and invert

- **Goal:** prove R1-R27 and R32-R36 on the box.
- **Requirements:** R1-R27, R32-R36.
- **Dependencies:** U5-U9, U11, U14, U15.
- **Files:** none.
- **Approach:** through `scripts/remote-linux.sh`, the full run on each tier, then each tier alone; the lying-wrapper run (AE2); the seeded failing leg; a cancellation run (`SIGTERM` to the runner mid-leg, then `SIGKILL`, then the membership scan); a hostile run with an unrelated process in the private runtime directory to prove the owned-set check fails it; the Wayland health-gate retry measured over repeated starts. No performance baseline is taken (KD9). The whole suite's wall time per tier is recorded as information, not a gate; the release binary is checked against 15 MiB when `crates/linux` changed. The live-session run and the interactive runner remain outstanding with their scope listed.
- **Test expectation:** none -- verification runs; every harness rule that has a test was seen to fail with the rule broken.
- **Verification:** the Verification Contract rows pass; the numbers go in the PR description.

### U13. Stranger dogfood

- **Goal:** find what a new agent hits using only the shipped skill and the built binary against the real apps.
- **Requirements:** R31.
- **Dependencies:** U12.
- **Files:** `docs/dogfood-reports/<date>-<serial>-feat-linux-3-12-e2e-harness-dogfood.md`.
- **Approach:** KD8. The operator is a subagent briefed that it holds the release binary, `skills/agent-desktop/` through `skills get core`, and `tests/e2e-linux/README.md`, with the repository source withheld, working in a private session. It runs the README's commands and every skill example verbatim in the default shell, reads each step's `ok`, waits with the tool's primitives, and treats an undocumented flag, variable, environment or ordering as a finding. Legs: the documented one-command run on each tier; one leg alone with `--leg`; a prerequisite miss (rename a tool) and what it prints; cancelling mid-run; the lying wrapper; reading a failure report well enough to reproduce it; the KDE protocol as written. A failure counts after a second independent observation and a behavioural claim is measured in both directions. The report is judged: at least one finding, else the targets are made harder.
- **Test expectation:** none -- a run and a report; each finding is fixed with a named test that fails when the fix is broken, owned elsewhere (receiving scope written into `docs/phases.md` in this PR) or accepted with a reason.
- **Verification:** the report is committed, every finding has one disposition, every fixed finding's test was seen to fail with the fix removed.

### U14. Shell surfaces, notifications, tray

- **Goal:** the 3.14 commands are judged on the real shell through independent senders and recorders.
- **Requirements:** R34, R5, R7.
- **Dependencies:** U5; U2 (the mutter tier with the notification recipe); 3.14 landed.
- **Files:** `scenarios/shell_surfaces.py`, `notifications.py`, `tray.py`, `apps/sni_item.py`.
- **Approach:** KTD16. Notifications: post through `notification-oracle.sh notify`, `list-notifications` equals the sender's record in order with the app filter case-insensitive; `wait --notification` passes near the send and times out with none; headless mutations are `POLICY_DENIED` with the list unchanged; headed `dismiss-notification`, `dismiss-all-notifications` and `notification-action` are judged by `dbus-monitor` (`NotificationClosed` reason 2, `ActionInvoked`) and a re-read list, with a bystander left alone; wrong expected app and an empty list are `NOTIFICATION_NOT_FOUND` with no signal. Shell surfaces: `snapshot --surface system-tray|action-center|quick-settings`, the System button opens and closes by ref with a pixel difference and the raw tree as oracles, the message list closes after each leg. Tray: the harness sender registers an item with `Activate` and a menu and one with neither; headless `click` delivers `Activate` and `Event(id,"clicked")` per the sender's log, the disabled entry and the item without `Activate` are refused with no call logged, headed click on the item without `Activate` opens the shell popup, and a killed owner turns the old ref into `STALE_REF`. Menu surface: `list-surfaces` and `snapshot --surface menu` item counts equal a raw `SHOWING` count on Calculator, a GTK3 context menu, Qt and Chrome (count by role, since the Qt popup states differ).
- **Test scenarios:** each mutation has its positive and its wrong-target twin; a delivered mutation that answers `INTERNAL` fails the leg (3.14 D1); the shell content window dropping out of the registry for about 2 s after a list toggle is waited out by deadline poll, not retried; the dock leg asserts the landed `supported_surfaces` against `open-system-surface` agreement.
- **Verification:** pass on `x11-mutter`; invert (judge by the command's `ok`, skip the bystander, read the list before the click returns) -- each fails.

### U15. Qt and VS Code toolkit legs

- **Goal:** the toolkits the 3.6, 3.8 and 3.11 exit criteria name are driven by the same harness.
- **Requirements:** R35, R32, R19.
- **Dependencies:** U5, U6; KD11 confirmed for the Qt window.
- **Files:** `scenarios/toolkit_qt.py`, `toolkit_electron.py`, `apps/qt_window.py`.
- **Approach:** Qt: the script window (menu bar `File`, a button, a text field) logs every activation to a file in the runtime directory, which is the effect oracle; refs and completeness, headless and headed click, `scroll` refused with `no_scroll_container`, `wait --menu` and `--menu-closed` both directions, the already-open case. VS Code: launched with the R35 flags into a runtime-directory profile; the leg first proves the frame stays registered for the whole leg (`L18-8`), then reads refs and completeness on the first snapshot, opens the application menu bar with the keyboard and reads it, and `wait --menu` both directions; it opens no web page and drives no editor content beyond a typed character read back from the window title or document. A leg whose app cannot be kept alive is `outstanding` with the probe row, never skipped.
- **Test scenarios:** per leg the positive and its negative twin; the Qt log shows exactly one activation per headed click; a VS Code frame that vanishes mid-leg fails the leg.
- **Verification:** pass on the X11 tiers; invert (read the Qt log before the click, accept a stale VS Code frame) -- each fails.

---

## Verification Contract

| Check | Command or method | Proves |
|---|---|---|
| Self-tests (pure) | `python3 -I -m unittest discover -s tests/e2e-linux -p 'test_*.py'` on any host | R5, R6, R8, R27 rules |
| Self-tests (box) | `scripts/remote-linux.sh bash scripts/test-private-session.sh` and the session tests | R3, R4 |
| Full run | `AGENT_DESKTOP_REMOTE_TIMEOUT=<long> scripts/remote-linux.sh bash tests/e2e-linux/run.sh` per tier and `--tier` alone | R1-R25 and R32-R35 on real apps |
| Lying wrapper | `run.sh --lying-actions` exits `1` with every effect leg failing | AE2, R5 |
| Seeded failure | the seeded failing leg gives a nonzero exit | R6 |
| Cancellation | `SIGTERM` then `SIGKILL` to the runner mid-leg, then a membership scan of `/proc` | R3 |
| Invert | break each rule named in U2-U9 (foreground wait, group signal, per-attempt kill, single health answer, oracle-less pass, owned-set check, hash re-check, stride, CRC, owner by pid, `ABOVE` flag, baseline after opener, title instead of id, headed refusal as success, `ok: true` as the read-back), watch its test fail, restore | the tests guard what they name |
| Lint | `cargo clippy --locked -p agent-desktop-core -p agent-desktop-linux -p agent-desktop -p agent-desktop-ffi --all-targets -- -D warnings` on the box; `shellcheck -x` over the shell lists; `bash scripts/check-bash3-compat.sh` | quality gate |
| Source rules | `scripts/check-rust-file-size.sh`, `scripts/check-no-phase-references.sh` (extended to scan `tests/e2e-linux/`), `scripts/check-stale-ref-constructor-misuse.sh`, `cargo fmt --all -- --check`; Python files stay under 400 lines | R27 |
| Isolation | `cargo tree -p agent-desktop-core` shows no platform crate and `git diff -- crates/core` is empty | KD3 |
| Dependency | `git diff -- Cargo.lock Cargo.toml` is empty | no new crate |
| Size | release binary against 15 MiB when `crates/linux` changed | R31 |
| Ledger | `python3 scripts/check-linux-findings.py` exits 0, then `--sub-phase 3.12` lists no undisposed row | R30 |
| Dogfood | the committed report with every finding disposed | R31 |

Requirement-to-check map: R1 → U3 gate and stage tests, Full run; R2 → U3 session tests, U4 attribution rows and the hostile run; R3 → U2 tests and Cancellation; R4 → U2 stub rows and U12 retry measurement; R5-R6 → U3 verdict tests, Lying wrapper, Seeded failure; R7 → U3 teardown rows and every restoration read; R8 → U3 timeout rows and U13; R9-R17 → U5 legs with U1 outcomes; R18 → U6 occlusion and modal legs; R19-R20 → U7 legs; R21 → U6 text entry; R22 → U8; R23 → U9; R24 → U5 and U6 non-interference reads; R25 → U3 report tests and U9; R26 → U10; R27 → Source rules and Isolation; R28-R30 → U11 and Ledger; R31 → Dogfood and Size; R32 → U6 pointer legs; R33 → U7 lifecycle legs; R34 → U14; R35 → U15; R36 → U12 runs and the guard test.

## Definition of Done

- Every Verification Contract row passes, the invert check has been run for each rule it names, and the live-session run, the interactive runner and the Plasma matrix are listed in the PR description as outstanding with what each covers.
- Every registered leg is accounted for: passing, asserted refusal with the landed error, owner decision linked to `docs/phases.md`, or outstanding with the reason. No leg is skipped without a registry entry.
- Every ledger row naming 3.12 is disposed of in the PR description, implemented here, owned elsewhere with the receiving scope in `docs/phases.md`, or accepted with the reason:
  - `L6-7`, `L6-8`, `L14-4`: implemented as raw geometry coverage plus the Calculator `listbox` and Terminal hidden-item CLI legs; the named ref-able zero-size control is an owner decision.
  - `L8-7`: implemented as the restart scenario (identity itself is 3.4's, amended).
  - `L13-3`: implemented as the health-gated retry (U2).
  - `L14-1`, `L14-2`, `L14-16`, `L14-3`: implemented (U5, U6).
  - `L14-5`: implemented as the duplicate-window and `Main Menu` legs; the role-and-name pair is an owner decision.
  - `L14-6`, `L14-8`, `L14-9`, `L14-10`, `L14-11`, `L14-12`, `L14-13`, `L14-14`, `L14-15`: implemented (U5-U7); `L14-10` semantic-only select is an owner decision.
  - `L14-7`: implemented (U5 scroll). 
  - `L23-7`, `L23-9`, `L23-16`, `L23-31`, `L23-38`: closed by U1's re-measurement and the matching legs; `L23-35` and `L23-39`: owned elsewhere, owner items written into `docs/phases.md`.
  - `L18-8` (VS Code frame), `L18-15`, `L18-17`, `L28-2`, `L28-10`, `L31-56`, `L4-25`, `L1-3`: implemented in U2, U7, U15 or recorded as the harness rule they state.
  - U1's new rows in area `L34`: each with a verdict and a disposition.
- The dogfood report is committed with every finding disposed of and every fixed finding's test invert-verified.
- `crates/core` diff is empty, no shipped source names a plan, phase or decision id, no probe script, capture or box identity is committed, no dependency was added, no performance baseline was taken, and the measured size replaces the estimate in `docs/phases.md`.
- `docs/phases.md` states what was built and no longer states what the evidence disproved.
- No abandoned experiment code remains.

## Risks & Dependencies

| Risk | Mitigation |
|---|---|
| A prerequisite sub-phase lands without a command this harness drives | U1 stops; the stack is not certified by skipping its legs |
| Landed Wayland or mutter focus differs from the drafts | KTD9 asserts the landed behaviour, reports it, and writes the owner item; no harness rescue |
| Private mutter or Wayland starts hang (3 in 8 single starts) | the health gate and six bounded attempts (KTD5); a spent budget fails the tier with a reason |
| A popover or scroll read happens before the app settles (`L14` artefacts) | deadline polls of at least 1 s and 2 s (KTD11, R12); no fixed sleeps |
| The oracle shares a bug with the product | independent reader, raw bus (KTD2); a second channel for every commit claim |
| The shared box is used by other agents | private roots for self-tests, the canonical lease for real runs (KTD7), bounded children (KTD8) |
| An unproven CI job is mistaken for coverage | the job fails closed and the PR states it is unproven until an interactive runner exists (U10) |
| Size exceeds the roadmap estimate | stated and justified (KTD14); the split points are named |
| Mutter timing flake (`L31-53`, `L31-54`) turns a leg flaky | KTD15 discipline, measured outcome sets, a probe row sizing the flake rate; a leg never retries (R8) |
| A bus-activated app maps on the wrong display under the mutter tier | the activation-environment push and the per-window display read-back (KTD5) |
| The Wayland shell never answers the Notifications interface (3.14 D8) | shell legs run on mutter only; the Wayland gap is written into `docs/phases.md` as an owner item |
| The Qt window is refused as a fixture (KD11) | the Qt legs become `owner-decision` records; in-crate live tests keep the Qt coverage |
