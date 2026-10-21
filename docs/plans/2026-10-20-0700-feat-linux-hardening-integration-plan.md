---
title: Linux Hardening and Integration Review (Sub-phase 3.15) - Plan
type: feat
date: 2026-10-20
origin: docs/phases.md
artifact_contract: ce-unified-plan/v1
product_contract_source: docs/phases.md §Phase 3 sub-phase 3.15
execution: code
---

# Linux Hardening and Integration Review (Sub-phase 3.15) - Plan

## Goal Capsule

- **Objective:** prove that `feat/linux-adapter` is production-grade as one whole, close every item that 3.4 to 3.14 routed here, and prepare the single release-noted `feat!` promotion to `main`. The plan builds nothing new for the product surface. It fixes the defects that the final verification finds, and it runs the live legs that no earlier private session could run.
- **Means:** one measured closure matrix (every owned item, one row, one disposition), a small set of Linux-crate hardening fixes, platform-neutral `fix(core)` commits only where a core owner item has a measured reason, a final live gate on the box (private tiers) plus an owner-run window for the legs that need the owner's live session, a final whole-adapter stranger dogfood, a full-branch review, and the docs, skill and release sync.
- **Authority:** `docs/phases.md` §3.15 as merged from every branch that edited it (union, see the Planning Contract), the Cross-cutting sub-phase DoD, `CLAUDE.md`, `plans/dogfood-brief.md`, and the ledger rows that name 3.15. Pre-code rows of this sub-phase are area `L24` (`rows/L24.md`, 27 rows, plus the six new rows of this plan, `L24-28` to `L24-33`). Rows measured while building use the next free area at the cut (re-check `probes/linux/FINDINGS.md`; 3.12 uses `L34`).
- **Stop conditions:** stop if any of 3.0 to 3.14, core-small, core-delivery, 3.12 or 3.13 is not on the base. Stop if a requirement needs a Linux name, `cfg` or dependency under `crates/core`. Stop if a deferral has no measured block. Stop if the box rules would be broken (no `ssh`, no `sudo`, no name-based kill, no touch of the live bus, live clipboard, Xvfb `:99`, `vda-*`, docker, postgres or Chrome pid 12669).
- **Execution profile:** U0 (cut and closure matrix) first. U1 (probes at the cut) before any fix. U2 to U7 are the fixes, in disjoint lanes. U8 to U11 are verification. U12 is the promotion receipt. Merge to `main` is the owner's act.
- **Finishes and ships:** one PR from `feat/linux-3.15-hardening`, stacked on the 3.13 branch with `gh stack`, into `feat/linux-adapter`. Nothing merges to `main` here. The `feat/linux-adapter` to `main` promotion is prepared (commit body, release notes, receipts) and handed to the owner.

---

## Product Contract

### Summary

After 3.15 an agent on Ubuntu 24.04+ GNOME gets the documented Linux command surface, and every claim that the README, the Linux skill and `docs/phases.md` make about Linux is backed by a receipt from the real box. The archive loader floor remains glibc 2.35; other distributions need the at-spi2-core bridge fix dc0dc33. Where Linux cannot do a thing, the product says so with the right code, and the document says why with a ledger row.

### Problem Frame

Fourteen sub-phases each ran in a private session. A private session cannot show the owner's live apps, the shell's own windows, a real Wayland session, a real consent dialog, a privileged input route, scale 2, a second monitor, a non-English shell, or Gecko. Each sub-phase wrote those legs into 3.15. Five more things are only visible on the whole branch: the batch process tests that need a bus, a detector that routes subsystems differently, a reader that leaks a thread per timeout, a cursor overlay that answers `ok: true` and draws nothing, and a core owner list that nobody has closed.

### Requirements

- R1. Every item in the Closure Matrix (Appendix A) has exactly one disposition: fixed here with a named test that is invert-verified, measured and closed with a ledger row, owned by the owner with a measured block, or accepted with a cited ledger row proving a measured block or a reason confined to owner scope (product policy). A reason alone cannot accept an unresolved technical item. "Recorded" is not a disposition.
- R2. The bounded X11 reader holds a flat thread count and a flat descriptor count over at least 50 consecutive timed-out calls in an FFI host. The test runs against a server that accepts and never answers, and it fails if the reader is changed back (L24-29, `phases.md` §3.15 bullet 7).
- R3. One display detector routes every subsystem. For each environment case in the L24-28 matrix, displays, screenshot, clipboard, focus, pointer and key input choose the same backend, and the same cause gives the same error code. The matrix is a committed live test, not a one-off probe.
- R4. The batch process tests (`cli_process`) pass outside a private session and with the owner's bus down, with no test skipped and no assertion weakened (L20-7).
- R5. The live gate (3.12 harness) runs on the final candidate artifacts in `x11-openbox`, `x11-mutter` and `wayland` tiers, headless and `--headed`, with complete leg accounting. An `outstanding` leg is a failed gate.
- R6. The legs that need the owner's live session (3.5 and 3.6 ignored live tests, the Chrome password field, Gecko offscreen, live RemoteDesktop refusal shape, attended portal consent, `screenshot --screen` under logical layout, the Wayland Ctrl+V paste) run in one owner window with a written runbook, or each has a measured block recorded in `docs/phases.md`. The live AT-SPI bus socket was still missing on the box at planning time (L24-31), so the window starts with the owner's restore step.
- R7. The root-gated Wayland input question (`ydotool`, L5-2, L5-12, L13-15, L1-17) gets a measurement. `/dev/uinput` is `root:root 0600` and the box user has no `input` group (L24-31), and the box rules forbid `sudo`, so the measurement is a lab script that the owner runs once with privilege. The result decides build or documented opt-in.
- R8. No performance baseline is taken. The perf item of §3.15 becomes budget and correctness checks (Appendix B): the 15 MiB size limit, every command bounded by its deadline against a silent display, a silent bus and a stopped app, snapshot `complete` on the large apps, and flat threads, descriptors and memory in a long-lived FFI host.
- R9. `cursor-overlay` and `session start --cursor` on Linux tell the truth. Measured now: `session start --cursor` answers `ok: true` with `cursor_overlay.enabled: true` while the lifecycle update is skipped with `PLATFORM_NOT_SUPPORTED` in a warning on stderr (L24-33). Implement an override-redirect ARGB window on X11 driven by the agent pointer position; report the overlay unsupported on Wayland. Both `cursor-overlay` and `session start --cursor` report the actual overlay state, including `rendered`, instead of a bare success. Lane B owns the Linux overlay and trait wiring; lane C owns the platform-neutral core session-response field in its own `fix(core)` commit with relevant tests executing on macOS, Windows and Linux.
- R10. The adapter fixes owed here are done: `AppInfo.bundle_id` carries the desktop id (owner decision, adapter side), a Files node that vanishes during a search no longer answers `APP_UNRESPONSIVE resolution_identity_unknown` when the app is not stopped, a lost accessibility bus after connection maps by an explicit decision (decision 42), and the cause of the shell window outage (L32-45) is found or its measured limit is documented.
- R11. Every core owner item (Appendix A, group G6) is either fixed as a separate platform-neutral `fix(core)` commit with a test that runs on macOS, Windows and Linux targets, or left with a measured block cited by ledger row and the owner's name on it, or an owner-scope product-policy decision under R1. Core gets no Linux word, `cfg` or dependency.
- R12. The docs match the code. `docs/phases.md` is corrected in place (no annotation) for every statement that the work disproves (research §4.3, the perf vehicle, the Gecko text, the cross-platform snapshot claim, `watch` acceptance, approve-once capture, notification claims). README, the Linux skill, release notes and the Phase 3 text agree.
- R13. The whole-branch review (multi-agent plus codex) finds nothing open. Rounds repeat until one round has zero confirmed defects. Only findings accepted under R1 remain; a general reason alone is insufficient.
- R14. The final whole-adapter stranger dogfood (nine areas in parallel, plus cross-app agent scenarios) repeats until one full round finds zero confirmed defects. Every finding has one of the three dispositions.
- R15. The promotion receipt ties review, tests, live evidence, budget checks and the release dry-run to one candidate commit. A change after the receipt voids it.

### Scope Boundaries

- No new adapter API, no new command, no new `Action` variant. `watch`, text-range commands and `Action` variants stay owner items (decision 45); the plan only records that they are absent.
- No perf baseline compare of any kind (owner rule).
- No web pages. Desktop apps only. Chrome and Chromium run only as desktop apps for the password-field and menu legs.
- No change to `crates/core` except separate `fix(core)` commits under R11.
- The `feat/linux-adapter` to `main` merge is not done by this plan.

### Success Criteria

The Verification Contract (below) passes on the candidate commit, and the Closure Matrix has no row without a disposition.

---

## Planning Contract

### Context and Research

**Sources read (union of every branch that edited §3.15):**

| Source | What it adds |
|---|---|
| `docs/phases.md` §3.15 on the 3.11 base | 12 scope bullets, key APIs, depends-on, exit criteria |
| same section on the 3.10 branch | 1 bullet with four Wayland capture legs |
| same section on the 3.14 branch | 4 bullets (menu click, shell window outage, second monitor and scale 2, the unmeasured set) |
| same section on the 3.9, 3.12, 3.13, core-small and core-delivery branches | the 3.6 bullet words Firefox as its inner snap binary (L27-44) instead of "Gecko cannot run in a private session". The two wordings disagree; U2 settles it by measurement (L16-15 against L27-44). |
| `docs/phases.md` Release, Skill and Docs checklist ("folds into 3.13 / 3.15") | 12 checklist lines |
| `docs/phases.md` Testing section | 4 cross-platform validation bullets, 6 extension bullets |
| `docs/phases.md` Phase 3 introduction | core and owner lists (union of branches: about 40 bullets) |
| `probes/linux/FINDINGS.md` | 14 rows that name 3.15 literally; DEFERRED rows whose closure is elsewhere (verify closed) |
| `rows/L24.md` | 27 pre-code rows of this sub-phase |
| `OWNER-ITEMS.md`, `STATE.md` | items 3, 13, 42, 43, 44, 50, 51, 53, 54 and the 3.6 follow-up name 3.15 |
| `research-3.15.md` | 12 probe proposals, 9 unit proposals, 10 inherited contradictions, the verification list |
| 3.12, 3.13, 3.14 plans | residue routed here (runner deferrals, arm64 and GNOME 42, KDE, X11 live desktop, scale 2) |

**Phases.md corrections this plan makes (R12, unit U2):**

1. The perf vehicle. `scripts/perf-baseline-compare.sh` builds a Swift fixture, needs git, and its base is `main` with a stub Linux adapter (L24-26). The owner bans baselines. The bullet becomes the budget and correctness checks of Appendix B.
2. The 3.6 bullet wording (Gecko in a private session). Settle by measurement at U1.
3. `phases.md` Testing: "structurally identical JSON on all 3 platforms" is false for the same GTK4 app on Wayland and Xwayland (L13-4) and for Electron across launches (L3-24). Change it to the contract that holds: same envelope, same codes, same field names.
4. `watch --event value-changed` acceptance: the `watch` command did not ship in core. Remove the acceptance line and point to the owner item.
5. The PipeWire "approve once" acceptance: unproven (L13-20, L29-25). Replace with the measured behaviour from the attended run, or with an owner item.
6. Notification wording: `Notify` is a method, `ActionInvoked` is a server signal, and `CloseNotification` closes only ids that the probe sent (L10-1, L10-4). Match the 3.14 outcome.
7. The generic DoD names unscoped workspace commands. Host package sets apply (`CLAUDE.md`).
8. The raw-evidence rule: Phase 3 keeps raw dumps outside the repository.
9. Linux skill registration "in core": the owner-approved seam from 3.13 is the text that stays.
10. `cursor-overlay`: add Linux to the honesty text and to the scope with the U7 decision.

### Key Technical Decisions

- KTD1. **Closure first, fixes second.** U0 builds the matrix from the sources above. U1 measures. A fix unit exists only for a row that U1 or the earlier probes proved to be a defect.
- KTD2. **Real box over mocks.** Each fix has one live test on the box and one unit test. Each fix is invert-verified (break, watch the test fail, restore).
- KTD3. **Reader fix by shutdown, not by a bigger pool.** The preferred fix keeps a handle to the connection socket and shuts it down when the deadline passes, so the blocked worker returns and the thread ends. The alternative is one worker per display. U3 measures both against R2 and keeps the smaller one that passes. A read timeout on the socket is the third option and is tried first if the connection type allows it.
- KTD4. **One routing table.** The display detector (`crates/linux/src/system/display_server.rs`) is the only reader of the environment. Clipboard, input, focus and capture ask it. The error mapping for "display not reachable" lives in one function that returns one code per cause (refused, missing, silent, auth, not configured).
- KTD5. **The test for L20-7 uses a real bus.** The two tests start a private session bus and AT-SPI bus through the existing helper, as the live tests do. They do not skip and they do not weaken the assertion that a `wait` after an ended session is skipped. A separate test pins the other contract: with no bus, a batch with a `wait` entry hard-stops at preflight with `PERM_DENIED` (measured both ways, L24-30).
- KTD6. **Live-session legs are owner-run.** The dedicated owner-live flag is set only by `scripts/linux-live-gate.sh --owner`; private-session guards accept it for the ignored live tests. Ordinary test invocations retain the private-session requirement. The agents never touch the live bus, the live clipboard or the live display. The owner alone invokes `scripts/linux-live-gate.sh --owner` (U8) through `scripts/remote-linux.sh`. The script prints rows. Agents read the rows and write the ledger.
- KTD7. **The ydotool lab is a script, not a feature.** U9 ships `scripts/uinput-lab.sh` (documented opt-in lab). The product does not call `ydotool`. A private compositor does not isolate uinput: the device is kernel-global. Before any event, the owner-run script measures the created device's libinput/udev seat property and checks live-session seat membership; it stops if isolation is unproven or the device would reach a seat with a live session. If the lab shows that a Wayland pointer route exists, building it becomes an owner decision with the result attached.
- KTD8. **Budget checks are ceilings.** A ceiling comes from a deadline the product already states (default timeout, lease deadline, 15 MiB). No check compares one revision to another.
- Drag's `--timeout-ms` covers preparation and the physical gesture on every platform. Core rejects explicit `--duration` plus `--drop-delay` at or above the budget as `INVALID_ARGS` before endpoint work. Platform pickup/default delays and completion also consume that budget; optional post-action waits use their separate wait budget.
- KTD9. **Core items need a measured reason.** A core item is fixed only when it is platform-neutral, small, and its relevant tests execute on macOS, Windows and Linux. Cross-target `cargo check` is supplemental compile coverage, not test execution. Otherwise it stays with the owner with its measured block.
- KTD10. **Dogfood fixes precede final review.** Run the final whole-adapter dogfood under `plans/dogfood-brief.md`, fix its findings and repeat to a clean round, then run the final full-branch review covering those fixes. Every candidate change reruns box clippy, lib and live tests, and the E2E rows it touches, and refreshes every artifact-bound evidence item before the receipt is issued.

### Open Questions

**Resolve before building:**

- Q1. The decision for `bundle_id` is made (desktop id, adapter side). U6 verifies that `close-app org.gnome.Calculator` and `launch org.gnome.Calculator` accept the same identifier.
- Q2. Decision 42: a bus lost after connection remains `INTERNAL` with a retry suggestion; `PLATFORM_NOT_SUPPORTED` means the bus was unavailable at detection. Delivery stays unknown, so the mapper cannot declare retry safe. A permission probe drops the failed cached connection and a later call reconnects; the long-lived [C consumer's lost-bus leg](../../tests/linux-dist/consumer.c) verifies loss and recovery on the same adapter.

**Owner decisions that the plan cannot make:**

- Whether the `ydotool` lab is wanted at all (decision 44). U9 gives the owner the script and the question.
- Which phase builds `watch`, text ranges and `Action` variants (decision 45).
- The release notes text for the promotion commit (decision 43).
- Merge of `feat/linux-adapter` to `main`.

### Implementation Strategy

U0 and U1 prepare the cut and probes; U2 to U7 are six documentation and fix units; U8 to U11 are four verification units; U12 is the receipt. The fix lanes touch disjoint files (see `plans/3.15-split.md`). Verification runs after the fix lanes merge, on one frozen candidate binary, in execution order U8, U9, U11 (dogfood and fixes), U10 (final review including those fixes), then U12. Any candidate change reruns the runtime gates and refreshes all artifact-bound evidence under U12.

---

## Implementation Units

### U0. Cut, freeze and closure matrix

- **Goal:** one list of everything 3.15 owns (Appendix A), with the source line for each item and an empty disposition column.
- **Files:** `docs/phases.md` (read), `probes/linux/FINDINGS.md` (rows, at the cut), `.claude/linux-phase3/plans/3.15-closure-matrix.md` (working copy; not committed).
- **Approach:** create the worktree `agent-desktop-wt/linux-3.15`, branch `feat/linux-3.15-hardening`, from the final head of the 3.13 branch. Re-run `python3 scripts/check-linux-findings.py --sub-phase 3.15`. Recount the Phase 3 introduction core lists by script. Copy the 27 `L24` rows that the ledger lacks (rule: the owning sub-phase adds the rest). Add `L24-28` to `L24-33`.
- **Test scenarios:** the ledger checker passes. Every row of Appendix A appears once in the working matrix. A row without a source line fails a review check.
- **Verification:** matrix row count equals the Appendix A count plus any new row found at the cut.

### U1. Probes at the cut

- **Goal:** measure the unknowns that decide the fixes. Rows go to `plans/3.15-probe-rows.md`, then the ledger.
- **Items (run on the box through `scripts/remote-linux.sh`, private sessions, frozen candidate binary):**
  1. Detector matrix across every consumer, on the final head (the 3.11 run is `L24-28`). Add the cases that the 3.11 base could not hold: `focus-window`, headed `press`, `type`, `drag`, `screenshot --window`, clipboard with `wl-clipboard` installed in the private Wayland tier.
  2. Reader leak before the fix: confirm L24-29 on the cut and identify the connection type and socket access that U3 needs. The post-fix 50-call flat-thread-and-descriptor check belongs to U3 verification (R2).
  3. Stopped identity with a real ref action: Calculator, Files, a desktop Chrome (own profile directory) on the final head, where the semantic click exists. Measure `click` on an answering app first. The 3.11 run showed Files `click` ending `TIMEOUT` at 5.0 s on a freshly launched app before it stopped (not confirmed twice).
  4. Cursor overlay implementation probe: an override-redirect ARGB window on X11 (openbox and mutter), driven by the agent pointer position. Measure rendering with pixels read by an independent decoder and teardown with resource counts. Wayland reports unsupported; this probe informs the X11 implementation rather than deciding whether to build it.
  5. Shell window outage (L32-45): sample the registry every 200 ms across notification posts, with the shell under `gnome-shell --x11` and the Wayland tier. Find what removes the content window (shell JS, AT-SPI cache, focus).
  6. Logical layout mode and scale on Wayland (L29-30): the headless shell refuses the mode, so the leg stays an owner-run live leg unless a virtual monitor scale is possible. Measure what the headless shell offers (scale list, layout modes).
  7. Lost bus after connection, in a long-lived FFI host: what code and text an agent sees (L22-8), and whether the same adapter recovers.
  8. Firefox in a private session through the inner snap binary (L27-44) against the older claim (L16-15): which wording is true.
  9. Non-English shell: can a translation catalogue be extracted to a user-space directory without root (`apt-get download`, `dpkg -x`) and be read by the shell through a user-space locale path? If not, the leg is an owner-run live leg and the block is the measured result.
  10. Hosted runner capability (L24-4, 3.13 U1): a workflow dispatch that boots a private session. Decides where the aarch64 and GNOME 42 legs run.
- **Probes already run at planning time (frozen 3.11 binary `c818f8b5…`, release CLI 5,312,496 bytes, 3.13-era FFI `.so`):** see Appendix C. They stand as rows `L24-28` to `L24-33`.
- **Test scenarios:** each item produces a row with an oracle that does not go through the product, and a result read twice.

### U2. Documentation corrections

- **Goal:** `docs/phases.md` states what was built and no longer states what the evidence disproved (R12).
- **Files:** `docs/phases.md`.
- **Approach:** rewrite §3.15 in place (no annotation): the scope names the budget checks, the closure matrix gate, the owner window, the lab script, and the cursor overlay. Apply the ten corrections of the Planning Contract. Write each deferral that U1 proves into the receiving text with its row. Remove the 3.15 bullets that earlier units finished.
- **Test scenarios:** `scripts/check-no-phase-references.sh` passes on `crates/**` and `src/**`. `check-linux-findings.py` passes. A grep for each disproved sentence finds nothing.
- **Verification:** reviewer reads §3.15 against Appendix A and finds one bullet per open row.

### U3. X11 reader hardening (lane A)

- **Goal:** R2.
- **Files:** `crates/linux/src/system/x11/bounded.rs`, `bounded_tests.rs`, `reader.rs`, the connection constructor (`live_display.rs` or the file U1 shows), `tests/ffi-python/` leak leg.
- **Approach:** try a socket read timeout. If the connection type hides the socket, keep a duplicate descriptor and call `shutdown` when the deadline passes. Keep one thread per call only while the call is inside its deadline.
- **Test scenarios:** (a) a loopback server that accepts and never answers: 50 timed-out `ad_list_displays` calls in a Python FFI host end with the start thread count and descriptor count; (b) a server that answers late (after the deadline) does not crash the worker; (c) a healthy server still answers in under 20 ms; (d) the deadline is unchanged (5.0 s default). Invert: restore the detached worker and see (a) fail with 50 extra threads.
- **Live check:** `L24-29` repeated on the box with the final `.so`.

### U4. One routing table and one error mapping (lane A)

- **Goal:** R3.
- **Files:** `crates/linux/src/system/display_server.rs` and its tests, the clipboard route file, the input route and error files, the focus route, the capture route. (Exact files re-read at the cut.)
- **Approach:** every subsystem asks the detector. A unit-testable function maps the cause to the code: not configured and refused and missing map to `PLATFORM_NOT_SUPPORTED` (permanent, with the named cause), silent maps to `TIMEOUT`, and an authorization refusal maps to `PLATFORM_NOT_SUPPORTED` with the server's reason. Pointer, focus and key input stop using `ACTION_FAILED` for "display not reachable" (measured: `mouse-move` and `focus-window` answer `ACTION_FAILED` while `list-displays` answers `PLATFORM_NOT_SUPPORTED` for the same cause, L24-28). Clipboard under a Wayland socket that accepts and never answers follows the same detector (L24-28: clipboard went to X11 with zero connection attempts).
- **Test scenarios:** the L24-28 matrix becomes a live test: nine environment cases times nine commands, asserting one backend and one code per cause. Unit tests per case. Invert: route clipboard around the detector and see the matrix fail.
- **Live check:** the matrix on `x11-openbox`, `x11-mutter` and `wayland` tiers.

### U5. Test hygiene (lane C)

- **Goal:** R4 and the audit items.
- **Files:** `src/tests/cli_process.rs` and its helper, `.githooks/pre-commit`, `CLAUDE.md` (the Darwin host package list: the Linux crate is Linux-only since `libc::pipe2`; owed from 3.9), `.github/workflows/ci.yml` only if a lane list needs the change.
- **Approach:** the two batch tests start a private session bus and an AT-SPI bus through the shared helper. Add the no-bus preflight test (L24-30). Remove `-p agent-desktop-linux` from the macOS host package lists in the hook and in `CLAUDE.md` if the cut still shows it.
- **Test scenarios:** `cargo test -p agent-desktop` passes on the box outside a private session with the live bus down, and inside one. The pre-commit hook runs on macOS without the Linux crate.
- **Live check:** run both ways on the box through the remote runner with the bus-down condition of L24-30.

### U6. Adapter fixes owed here (lane B)

- **Goal:** R10.
- **Files:** `crates/linux/src/system/app_ops.rs` and `app_name.rs`, the resolver (`crates/linux/src/tree/resolve*.rs`), the shell reader (`crates/linux/src/system/shell*`), the bus-error mapping (`crates/linux/src/system/bus*`).
- **Approach:** (1) `bundle_id` carries the desktop id; `close-app` and `launch` accept the same identifier. (2) When a node vanishes during a search and the app answers, the resolver answers `ELEMENT_NOT_FOUND` or `STALE_REF`, never `APP_UNRESPONSIVE`; U1 item 3 gives the repro. (3) Apply decision 42. (4) Apply the U1 item 5 result to the shell window outage: fix the cause if it is in the adapter, else document the limit with the row.
- **Test scenarios:** per item one live test with an independent oracle, and its negative twin (a really stopped app still answers `APP_UNRESPONSIVE`). Invert each.

### U7. X11 cursor overlay and honest session response (lanes B and C)

- **Goal:** R9.
- **Files by owner:** lane B: `crates/linux/src/system/cursor_overlay.rs`, `crates/linux/src/adapter.rs` for the `PlatformAdapter::update_cursor_overlay` implementation; integration reserves the module registration in `crates/linux/src/system/mod.rs` at step 0. Lane C: the platform-neutral core session-response field and session dispatch plumbing under `crates/core/**` and `src/dispatch/**`, in its own `fix(core)` commit. Integration through U2 owns `docs/phases.md`; lane D through U10 owns `skills/agent-desktop-linux/`.
- **Approach:** implement the X11 overlay as an override-redirect ARGB window driven by the agent pointer position. Wire it through the existing trait method and follow the macOS and Windows teardown rules (no residual window, timer or thread). Wayland returns an explicit unsupported overlay state. Lane C makes `session start --cursor` return the actual adapter overlay state, including `rendered`, rather than a bare success; the existing cursor-overlay command response alone does not supply the session field.
- **Test scenarios:** on `x11-openbox` and `x11-mutter`, `session start --cursor` and `cursor-overlay enable` report `rendered: true`, independently decoded pixels track agent pointer updates, and `disable` leaves no window, timer or thread. Wayland reports unsupported with `rendered: false`. Lane C's relevant session-response tests execute on macOS, Windows and Linux and are invert-verified independently of the Linux overlay fix.

### U8. Live gate and owner window (integration)

- **Goal:** R5 and R6.
- **Files:** `tests/e2e-linux/` (run, not edited, except a finding), `scripts/linux-live-gate.sh` (new), the private-session test guards used by the 3.5 and 3.6 ignored live tests (integration owns the dedicated owner-live path), `docs/runbooks/linux-live-gate.md` (new, lane D).
- **Private library gate:** `scripts/linux-live-gate.sh` runs the complete Linux library test inventory, including ignored cases, across three isolated private sessions at a time per tier. Ordinary X11 live cases run on openbox and mutter; native Wayland cases use their own private compositor fixtures, and tests with explicit manager requirements keep those requirements. Exact test-name receipts must match the frozen inventory without omissions, duplicates or failed cases. The three tiers can run as separate calls against one frozen manifest and merge into one gate result. The attended owner-window registry remains a separate mode and is not certified by a passing private library suite. The cost inventory and runbook name every retired or moved test and its retained contract owner.
- **Approach:** run the 3.12 harness on the frozen candidate in the three tiers, headless and headed. Then the owner window, invoked by the owner alone through `scripts/remote-linux.sh`; agents only read the resulting rows. Only `scripts/linux-live-gate.sh --owner` sets the dedicated owner-live flag for its test subprocesses; the private-session guards accept that path so ignored tests can run on the owner's live session. Direct invocations without that flag still require a private session. The script `linux-live-gate.sh` runs, in order and with the live session only: restore check of the AT-SPI bus; the 3.5 and 3.6 ignored tests (`cargo test -p agent-desktop-linux -- --include-ignored --test-threads=1`, the legs named `L25-1` to `L25-10`, `L20-3`, the Chrome password field, Firefox offscreen); the Wayland `press` and `type` refusal shape; the attended portal consent (approve, reject, close, timeout, repeat, new process); the portal ScreenCast route (`CreateSession`, `SelectSources`, `Start`, and an independently decoded PipeWire frame), recording whether `Start` returns a `restore_token` in its own leg verdict (Appendix A row 14); `screenshot --screen` under logical layout with one scaled and mixed scales; the Ctrl+V paste of a Wayland clipboard; the menu click through the CLI on the merged branch (L32-21, L32-39); the second monitor and scale 2 shell click (L32-46); the non-English shell and the tray corner cases (L32-16, L32-40), Gecko and VS Code menus, the GTK 3 header-bar popover. The script prints one row per leg and changes nothing outside the apps it launches. The owner restores the AT-SPI bus first. The script never restarts it.
- **Test scenarios:** the harness exits `0` with no `outstanding` leg. The script run on the box in a private session (as a dry run with a flag) prints every leg id and refuses a live display unless the owner passes `--owner`. Verify that ordinary ignored-test invocations reject a live session and the owner-live path passes the guards and executes the named tests rather than skipping them.
- **Verification:** leg accounting equals the registry; every owner-window row has a verdict.

### U9. Root-gated Wayland input lab (integration, owner)

- **Goal:** R7.
- **Files:** `scripts/uinput-lab.sh` (new), `probes/linux/FINDINGS.md`.
- **Approach:** the script checks `/dev/uinput`, starts a privileged `ydotoold` only when run by the owner with root, and identifies its created uinput device before sending any event. uinput is kernel-global; a private headless `gnome-shell --headless --wayland` does not isolate it. Record the device's libinput/udev seat property and compare the effective seat with every live session. Stop before any event if the seat cannot be established or the device would reach a seat with a live session; record the measured block with its ledger row. Only after proving an isolated seat may the script send keys and pointer events into a Wayland-native GTK4 window, read the effect independently from the application and print rows. It records whether it gives the Wayland pointer route that RemoteDesktop lacks (L13-17). If yes, the root requirement is a documented opt-in and the build decision goes to the owner. If no, the rows close as measured.
- **Measured block now:** `/dev/uinput` is `crw------- root root`; the box user is in `adm`, `sudo`, `docker`, `plugdev` and not `input` (L24-31). The box rules forbid `sudo` for agents.

### U10. Full-branch review, audits and docs sync (integration, lane D)

- **Goal:** R1, R12, R13.
- **Approach:** after U11 dogfood and all its fixes, (a) multi-agent final review of the whole diff against `main`, including the dogfood fixes: one reviewer per sub-phase 3.0 to 3.14 plus core-small, core-delivery, 3.12, 3.13, plus a codex read-only pass over the branch; findings fixed, not recorded; repeat until clean. (b) Audits: LOC cap (`scripts/check-rust-file-size.sh`), no phase words in shipped source, `cargo tree -p agent-desktop-core` free of platform names (including dev and build edges), core cross-compile to the Windows and Linux targets, empty `git diff main -- crates/core` except the `fix(core)` commits and the 3.13 seam, supply-chain (`cargo deny`, exact pins, lockfile), 15 MiB. (c) Docs sync: README Linux column and install text, the Linux skill (including the GTK4 role-61 secret limit, the stopped-app query note `L24-17`, the stamp-free focus statement, the Wayland refusals), `workflows.md`, release notes, and the whole checklist of Appendix A group G4. Every example in the shipped docs runs verbatim in bash.
- **Test scenarios:** the review receipt lists zero open findings. A seeded defect in a scratch branch is found by the review (sanity).

### U11. Final whole-adapter dogfood (integration)

- **Goal:** R14.
- **Approach:** give each stranger operator the shipped skill, the frozen binary and `plans/dogfood-brief.md` only. Nine areas in parallel: observation, ref actions, input, lifecycle and windows, capture and clipboard, waits and signals, shell and notifications, sessions batch and trace, FFI and npm package. Add the cross-app agent scenarios of the brief (open a GTK4 app, find a field, type unicode, verify by AT-SPI, copy, paste into a Qt app, screenshot, close both; on mutter and Wayland). Fix each confirmed defect, rerun the runtime gates and refresh artifact-bound evidence under U12, and repeat dogfood until one full round has zero confirmed defects. Then run U10 final review, which includes all dogfood fixes. Reports are not committed (rule); the dispositions are.
- **Test scenarios:** every fixed finding names an invert-verified test.

### U12. Promotion receipt

- **Goal:** R15.
- **Approach:** freeze the candidate commit and its artifacts (hash). Tie to it: review receipt, test results, harness result, owner-window rows, budget checks, release dry-run (13 assets: 4 CLI and 4 FFI tarballs, 2 zips, checksums, plus two Linux CLI archives as 3.13 set), FFI header drift. Prepare the single `feat!` commit message body for release-please with the owner's release notes text. Write the report of box side effects (OWNER-ITEMS items 55 to 61 and the later entries). Hand the receipt to the owner. Any candidate change, including a dogfood or review fix, invalidates the receipt. Rerun runtime gates on the new candidate: box clippy with `-D warnings`, lib tests, live tests and every E2E row the change touches. Refresh every artifact-bound evidence item, including harness results, owner-window and lab rows, budget checks, cross-platform core-test receipts, release dry-run artifacts and FFI header drift, against the new commit and hashes. Repeat U11 dogfood and its fixes before U10 final review (including those fixes), then issue U12 again. Evidence from older artifacts cannot certify the changed candidate.

---

## Verification Contract

| Check | Pass condition | Unit |
|---|---|---|
| Closure matrix | every row has one disposition; no "recorded" | U0, U10 |
| Reader leak | 50 timed-out calls, threads and descriptors flat; inverted fix fails | U3 |
| Routing matrix | nine cases times nine commands: one backend, one code per cause | U4 |
| `cli_process` | passes outside a session with the live bus down, and inside | U5 |
| Live gate | harness exit `0`, all tiers, both modes, no `outstanding` leg | U8 |
| Owner window | every leg row has a verdict; blocks recorded in `docs/phases.md` | U8 |
| Lab | owner-run device/seat measurement before any event; isolated-seat results or a cited measured block | U9 |
| Budget checks | Appendix B all within ceilings | U10 |
| Review | after dogfood and its fixes, zero open findings; reviews all dogfood fixes | U10 |
| Dogfood | one full round with zero confirmed defects | U11 |
| Host gates | `cargo fmt`, clippy `-D warnings`, `cargo test` over each host's package set; for every `fix(core)`, relevant test execution on macOS, Windows and Linux plus supplemental core cross-checks; file size; phase-word scan; ledger checker | all |
| Release | dry-run artifacts, glibc floor 2.35 (`readelf` gate), npm install, FFI header drift | U12 |

## Definition of Done

- Every row of Appendix A has a disposition and, where fixed, a named invert-verified test.
- Every requirement R1 to R15 maps to a unit and to a check above.
- `docs/phases.md` and the code agree, and the Phase 3 text matches README and the Linux skill.
- The receipt names one candidate commit.
- The Cross-cutting sub-phase DoD of `docs/phases.md` holds for the whole branch.

## Risks

| Risk | Mitigation |
|---|---|
| The owner window does not happen | Each live-session leg has a recorded block; the receipt states which rows are owner-run and open. The promotion is the owner's call. |
| A fix needs a core change | One `fix(core)` commit, platform-neutral, tested on three targets, or an owner item. |
| The reader fix changes timeouts | Test (d) pins the deadline. |
| Mutter timing flakes in the live gate | The 3.12 rule: assert measured outcome sets, never retry inside a run. |
| A hosted runner cannot boot a private session | U1 item 10 decides; the box runs x86_64 and the arm64 residue stays an owner infrastructure item with the evidence. |
| Review rounds do not converge | Findings are fixed, not recorded; the round count is reported. |

---

## Appendix A. Closure Matrix (everything 3.15 owns)

Count: 74 rows in 7 groups. "Src" is the source line: `P` = `docs/phases.md` (section and bullet), `L` = ledger row, `O` = `OWNER-ITEMS.md` item, `S` = `STATE.md`, `R` = research-3.15, `PL` = another plan.

**G1. §3.15 scope on the 3.11 base (12)**

| # | Item | Src |
|---|---|---|
| 1 | Full-branch multi-agent review | P §3.15 b1 |
| 2 | Live e2e headless and headed on the GNOME runner | P §3.15 b2 |
| 3 | Performance baseline vs `main` (becomes Appendix B) | P §3.15 b3, L24-26, L24-27 |
| 4 | LOC, size and isolation audits | P §3.15 b4 |
| 5 | Docs and skills sync | P §3.15 b5 |
| 6 | `cli_process` batch tests without a live bus | P §3.15 b6, L20-7 |
| 7 | Bounded X11 reader thread accumulation | P §3.15 b7, L24-29 |
| 8 | Root-gated Wayland input (`ydotool`) | P §3.15 b8, L5-2, L5-12, L13-15, L1-17 |
| 9 | RemoteDesktop access control on the live session | P §3.15 b9, L5-3 |
| 10 | Merge to `main` as one release-noted `feat!` | P §3.15 b10 |
| 11 | Re-run 3.5 ignored live tests (L25-1 to L25-10, L20-3, Chrome password) | P §3.15 b11, S 3.5 |
| 12 | Re-run 3.6 ignored live tests with Gecko (L16-15, L27-44) | P §3.15 b12 |

**G2. Added to §3.15 by other branches (5 bullets, 9 legs)**

| # | Item | Src |
|---|---|---|
| 13 | Wayland capture: display-to-pixel under logical layout (L29-30) | P §3.15 (3.10) leg 1 |
| 14 | Portal ScreenCast route and `restore_token` (L9-17, L13-21, L29-28) | P §3.15 (3.10) leg 2 |
| 15 | Consent flow on the live Wayland session (L29-25, L29-29) | P §3.15 (3.10) leg 3, O 54 |
| 16 | Ctrl+V paste of a Wayland clipboard into an app (L13-25) | P §3.15 (3.10) leg 4 |
| 17 | Menu item `click` through the CLI on the merged branch (L32-21, L32-39) | P §3.15 (3.14) b1 |
| 18 | Cause of the shell application dropping its window (L32-45, L32-42) | P §3.15 (3.14) b2 |
| 19 | Wayland shell click on a second monitor and at scale 2 (L32-46) | P §3.15 (3.14) b3 |
| 20 | Non-English shell landmarks (L32-40) | P §3.15 (3.14) b4 |
| 21 | Wayland RemoteDesktop refusal shape, scale-2 pointer mode, tray corner cases, Gecko and VS Code menus, GTK 3 header-bar popover (L32-16) | P §3.15 (3.14) b4 |

**G3. Exit criteria and DoD (4)**

| # | Item | Src |
|---|---|---|
| 22 | Cross-cutting DoD for the whole branch (fmt, clippy, tests, conformance) | P §3.15 exit, §Cross-cutting DoD |
| 23 | Every ledger row naming 3.15 disposed (14 literal rows plus the L24 owner rows 5, 14, 15, 17, 25) | P DoD, L rows |
| 24 | DEFERRED rows with closure elsewhere verified closed (L1-18, L4-23, L8-9, L11-27, L11-28, L12-8, L12-9, L13-24, L14-3, L14-8, L14-14, L18-8, L29-18, L29-19) | P DoD (reviewer duty) |
| 25 | Dogfood gate: findings with three dispositions, stranger operator, final whole-adapter rounds | P DoD, `dogfood-brief.md` |

**G4. Release, Skill and Docs checklist "folds into 3.13 / 3.15" (12, verify the 3.13 result and finish)**

| # | Item | Src |
|---|---|---|
| 26 | Linux CLI in the release matrix (x86_64, aarch64) | P Release b1 |
| 27 | npm `PLATFORMS` flip and `check-npm-package.js` agreement | P Release b2 |
| 28 | Linux `ad_*` covered by the FFI drift check and attestation | P Release b3 |
| 29 | Release notes: Linux support, glibc 2.35, display servers, Flatpak and Snap | P Release b4, O 43 |
| 30 | `skills/agent-desktop-linux/SKILL.md` content | P Skill b1 |
| 31 | Skill registration and postinstall advice (owner seam) | P Skill b2, O 1 |
| 32 | `workflows.md` Linux patterns | P Skill b3 |
| 33 | README platform table Linux **Yes** | P README b1 |
| 34 | README Linux install (npm, binary, source) | P README b2, b5 |
| 35 | README Linux permissions section | P README b3 |
| 36 | README live-desktop floor (Ubuntu 24.04+) and archive loader floor (glibc 2.35); other distributions unverified | P README b4 |
| 37 | Skill notes owed by rows: GTK4 role-61 secret limit (O security item), stopped-app registry-name query (L24-17), stamp-free focus (decision), Wayland refusals | O, L24-17 |

**G5. Testing section and inherited contradictions (16)**

| # | Item | Src |
|---|---|---|
| 38 | Cross-platform validation: same-app structure (correct the claim) | P Testing b1, R 4.3 |
| 39 | Same error envelope on three platforms | P Testing b2 |
| 40 | Same notification envelope on three platforms | P Testing b3 |
| 41 | Same tray envelope on three platforms | P Testing b4 |
| 42 | `native_id` from measured coverage | P Testing ext b1 |
| 43 | `watch` acceptance (command not shipped: correct) | P Testing ext b2, R 4.3 |
| 44 | `get_text_selection` UTF-16 offsets, non-BMP and combining mark | P Testing ext b3 |
| 45 | Portal screenshot approve-once (correct or measure) | P Testing ext b4, R 4.3, O 54 |
| 46 | Toolbar surface in Firefox and Files | P Testing ext b5 |
| 47 | Flatpak and Snap compatibility, both directions | P Testing ext b6, L12-8, L12-9 |
| 48 | Unscoped workspace gate wording | R 4.3 |
| 49 | Raw-evidence placement rule | R 4.3 |
| 50 | Notification dismissal generality (L10-4) | R 4.3 |
| 51 | Notification action by signal (L10-1) | R 4.3 |
| 52 | Other-DE notification history | R 4.3 |
| 53 | Linux skill registration in core versus the seam | R 4.3, O 1 |

**G6. Core and owner lists (4 rows, many bullets)**

| # | Item | Src |
|---|---|---|
| 54 | Phase 3 introduction core and owner bullets (3.1, 3.4, 3.5, 3.6, 3.8, 3.14 lists; about 40 in the union): each fixed as `fix(core)` or left with evidence | P Phase 3 intro |
| 55 | `OWNER-ITEMS.md` core-change items 1 and 6 to 28 and the later core collection (success flags, headed click order, `super+` combos, CodeQL uid taint, exited-process recovery, `list-apps --app` alias, `launch --wait-for`, same-name window pick, `focus_window` stamp-free policy, lease lock sharing): same rule | O |
| 56 | Pending owner decisions with no phase owner: `watch`, text ranges, `Action` variants (decision 45) | O 45, S |
| 57 | Pre-existing flaky tests on `main` (macOS app inventory, Windows `menu_state`): watch during the promotion CI | O 46 |

**G7. Orphans and residue found by this plan (17)**

| # | Item | Src |
|---|---|---|
| 58 | Cursor overlay on Linux answers `ok: true`, draws nothing, no sub-phase owns it | L24-33 (measured), P §2.15, §2.16 |
| 59 | Detector routes subsystems differently (clipboard, input, focus) | L24-28, L24-14 (measured) |
| 60 | `bundle_id` equals the desktop id | O 13, S |
| 61 | Files node vanishing mid-search answers `APP_UNRESPONSIVE resolution_identity_unknown` | S 3.6 |
| 62 | Bus lost after connection answers `INTERNAL` (decision 42) | L22-8, O 42 |
| 63 | Live AT-SPI bus socket restore and the live `gnome-shell` XI2 grab | O 53, O 3, L24-31 |
| 64 | Infrastructure: Ubuntu 22.04 GNOME 42 desktop, native arm64 desktop (L22-3, L22-15) | O 50, PL 3.13 |
| 65 | Infrastructure: KDE Plasma and other notification daemons, real X11 desktop, scale 2, interactive runner (L21-22, L21-36, L23-35, L23-39) | O 51, PL 3.14 |
| 66 | 3.13 runner deferral (C consumer legs on a runner that cannot host a private session) | PL 3.13 R4 |
| 67 | Windows §2.15 mirror: ratify out-of-reach items (multi-monitor, mixed scale, arm64 desktop, KDE) as measured or ratified, in writing | P §2.15 |
| 68 | Live e2e CI lane decision: the harness is owner-run on the box (no runner registered), state it | P §2.15, PL 3.12 |
| 69 | Darwin host package list drops the Linux crate (hook and `CLAUDE.md`) | S 3.9 OWED |
| 70 | GTK4 secret fields as documented limit | O security item |
| 71 | Connect-and-drop claim unverified on other servers (L19-2, L24-15) | L24-15 |
| 72 | Box side effects report for the owner | O 55 to 61, later entries |
| 73 | Final gate delivery review after 3.15 ships (separate run) | S |
| 74 | Core-delivery and core-small residue check (every fix landed, none reverted) | S |

## Appendix B. Budget and correctness checks (replace the perf baseline)

| Check | Ceiling or condition |
|---|---|
| Release CLI size | at most 15 MiB (measured 3.11: 5,312,496 bytes) |
| Every command under a silent display, a silent bus and a stopped app | returns within its timeout plus 1 s with the right code (measured now: 5.0 to 6.3 s for the default 5 s) |
| Snapshot of Files, Terminal, Calculator, Qt, VS Code | `complete: true`, or `complete: false` with `truncated` and no read error |
| FFI host over 50 timed-out calls | threads and descriptors flat |
| FFI host over 200 healthy calls | resident memory flat within 5 percent after warm-up |
| Cold bus start | answers or fails inside the lease deadline; value recorded, not compared |
| glibc floor | `readelf` max `GLIBC_2.34` or lower on the 22.04 build |

## Appendix C. Probe rows measured at planning time

All rows ran in private sessions through `scripts/remote-linux.sh` on the box (Ubuntu 24.04.4, glibc 2.39), twice where stated. Binary: frozen 3.11 CLI `c818f8b5333d3d39ce8dcab2397fd73658c22767610eab1f6cf986157385cad0`. FFI `.so`: `f9995fc1a9c8e7dac82d59712c6694b30bdeebf5e027921b9d6e0edafdeb326b` (`bounded.rs` is byte-identical on the 3.11, 3.13 and 3.14 branches).

| id | Result |
|---|---|
| L24-28 | Detector matrix, 9 cases times 9 commands, two identical runs. Wayland socket that accepts and never answers, plus X11: `list-displays`, screenshot, focus, `mouse-move` and headed `press` follow Wayland, but `clipboard-set` and `clipboard-get` returned `ok` through X11 with zero connections to the socket (a read back through the X11 route returned the value). `DISPLAY` refused: `list-displays`, screenshot and clipboard answer `PLATFORM_NOT_SUPPORTED`; `mouse-move`, `focus-window` and headed `press` answer `ACTION_FAILED`. No display: `focus-window` and `press` answer `ACTION_FAILED`, the rest `PLATFORM_NOT_SUPPORTED`. Silent display: every command ends `TIMEOUT` at 5.0 to 6.3 s, but `list-windows` answers `ok` after 5.0 s. Stale, missing and empty Wayland values and `unix:N` fall through to X11 correctly. `xclip` is not installed, so an X11 clipboard oracle needs another tool. |
| L24-29 | Reader leak, FFI host, silent loopback server: 10 of 10 timed-out `ad_list_displays` calls added one thread and one descriptor each (4 to 14 threads, 9 to 19 descriptors). `ad_adapter_destroy` did not release them. Closing the server released all. A healthy display held 4 threads and 9 descriptors flat. |
| L24-30 | `cli_process` batch failure reproduced with the frozen CLI: with no session bus, a batch with a `wait` entry hard-stops at preflight with `PERM_DENIED` (`Accessibility permission not granted`), results 0; a batch with only `version` passes. Inside a private session all three batches pass. |
| L24-31 | Box facts: `/dev/uinput` is `crw------- root root`; user groups are `adm cdrom sudo dip plugdev users lpadmin docker`, no `input`; no `ydotool`, `xclip`, `wl-copy` or `xdotool` on `PATH`; the live session is `tty2` on seat0 with `DISPLAY=:0`, empty `WAYLAND_DISPLAY` and `XDG_SESSION_TYPE=tty`, so there is no live Wayland session; the live AT-SPI socket under the runtime directory is still missing (every remote run warns). |
| L24-32 | Stopped identity with a ref action, Calculator and Files, 2 reps each: `click` on a stopped app answers `APP_UNRESPONSIVE` after 4.0 s, `not_delivered`, `retry: safe`; `snapshot` answers it in 6 ms; `get` in 5.0 s. After `SIGCONT` the ref resolves again. The 3.11 base has no semantic dispatcher (`click` answers `PLATFORM_NOT_SUPPORTED` on an answering Calculator), so the answering-app half runs at the cut. Files `click` on a freshly launched app ended `TIMEOUT` at 5.0 s before the stop (one run; confirm at the cut). |
| L24-33 | `session start --cursor` on Linux answers `ok: true` with `cursor_overlay.enabled: true` while stderr carries `cursor overlay lifecycle update was skipped code=PLATFORM_NOT_SUPPORTED`. `status` lists `supported_surfaces` `window`, `sheet`. Nothing in the Linux sections of `docs/phases.md` owns overlay rendering. |

Items that need a feature beyond the 3.11 base (shell, notifications, menus, FFI install, harness) and every live-session leg are marked "probe at cut" in U1 and U8.

## Appendix B: live test flakes and tool gaps

- Deleted `actions::live_widget_tests::a_chromium_page_takes_a_click_and_a_check`: Chromium accessibility refresh and widget-click ordering made the live result timing-dependent. Toolkit button and check/uncheck live tests and deterministic semantic-action tests retain action-contract coverage. Browser-specific timing is no longer a gate.
- Deleted `signal_live_gtk3_about_dialog_is_a_sheet_by_identity` and its unused About launcher: the terminal's initial frame could arrive after the baseline and be counted as a second window. Deterministic `signal_surfaces_tests` retain dialog identity and deduplication coverage; GTK4 About and shortcuts live tests retain real dialog coverage.
- Deleted the sleep-based `key_dispatch_live_a_popup_grab_in_another_app_gets_no_keys_and_the_target_none` and its private fixture helpers, removing the `wmctrl` dependency. `physical_live_tests::live_keys_under_a_menu_grab_are_reported_unverified` already uses X11 activation and bounded state waits without `wmctrl` and remains the live grab contract owner.
- Deleted `trace_artifacts`' `failing_action_still_captures_post_screenshot` and its unused adapter. Its exact screenshot-call count depended on the best-effort capture deadline surviving host load. The primary action error and successful pre/post PNG capture remain covered by the action-error and artifact tests; post-capture under a failing action is no longer a timing gate.
- Replaced `trace_read::segment_tests::trace_meta_sets_schema` and `truncated_final_line_is_skipped` with `metadata_and_unfinished_tail_preserve_complete_events`. It writes and closes one fixture before reading, checks the metadata schema, and excludes a syntactically valid final event without a newline. Temporary paths use a process id and atomic sequence so concurrent tests cannot share a segment through a coarse clock tick.
