---
title: Linux live E2E harness implementation contract
type: feat
date: 2026-10-18
origin: docs/phases.md
artifact_contract: ce-unified-plan/v1
product_contract_source: docs/phases.md §3.12
execution: code
---

# Linux live E2E harness

## Goal and delivery boundary

Implement a fail-closed CLI harness against private GNOME applications and a
permitted Qt toolkit probe. `runner/legs.py` is the executable inventory of
requirements, targets, tiers, modes and dispositions. A green selection proves
only those retained cells. Fakes are portable contract evidence, never live
passes. No owner-decision row remains.

The integration is local on `feat/linux-3.12-e2e-harness`. Rebase onto shipped core-delivery `7dc6500f`, preserve both intents in conflicts,
commit as Lahfir, and do not push.
`docs/phases.md` §3.12 and this plan describe the same delivered scope.
`3.12-integration-decisions.md` resolves all 76 review decision rows: 23 live
contracts, one asserted refusal, 50 transferred to explicit 3.15 scope, and
two removed invalid contracts (HTML setting and the Qt choice target). The separate unsupported HTML-import live leg is also retired; it is not an extra decision row. None is a deferred owner choice.

## Requirements

| ID | Shipped contract and proof |
| --- | --- |
| R1 | Build locked release CLI/helper; stage immutable hashes; hold the interaction lease; exit 0 only for an entirely passing executable selection, 1 for failures/outstanding/unavailable/owner results, 2 for prerequisites. |
| R2 | Own runtime, bus, accessibility bus, display, product home and GSettings store. Attribute window owners by PID generation and exact runtime membership, not launcher status. |
| R3 | TERM/HUP/INT unwind; unpublished startup runtimes are discovered and swept; generation-aware next-run cleanup handles abandoned sessions. |
| R4 | Compose private-session with the landed Openbox/Mutter/Wayland wrappers. A bounded WM/AT-SPI responsiveness gate runs before warm-up/leg baselines and after mapping; close Overview only on observed true. Expiry makes the tier unavailable, never passing. A socket/name or stable process set is insufficient. |
| R5 | A product response never proves its effect. Record independent before, after and final reads for each observable; reject missing/duplicate/unmatched/out-of-order evidence. |
| R6 | Every registered executable ID has one implementation and every implementation has a registry ID. Outstanding results remain nonzero. No OWNER row is an accepted result. |
| R7 | Final values equal their own before values. Check unchanged read-only channels too. Restore settings, document state, pointer and active window; compare owned process generations after cleanup. |
| R8 | Deadline polls decide readiness/effects; controlled trigger delays are inputs, never pass conditions. A leg does not retry itself. |
| R9 | Disabled Undo asserts default/explicit/short TIMEOUT and immediate ACTION_FAILED/enabled-check/not-delivered contracts. Controlled Qt enable checks exactly one activation. Terminal Copy selection state remains covered. Replace All version/target hardening is received by 3.15. |
| R10 | Files/Settings zero-axis containers are raw geometry/no-ref evidence; Terminal hidden items assert not visible and immediate refusal. Named zero-axis actions and version-specific Calculator history are 3.15 adversarial/version hardening, not claimed passes. |
| R11 | Distinct Calculator owner/window IDs, exact snapshot/focus and restart invalidation; Calculator Main Menu roles resolve separately. Engineered same-role/name resolver ambiguity belongs to 3.15, with exact AMBIGUOUS_TARGET and neither effect required. |
| R12 | Retained Settings viewport scroll and scroll-to reveal actual controls and restore the scrollbar. The retired Files 300-entry runtime specimen is not live scroll evidence. |
| R13 | Settings and Terminal state/value writes use raw checked/value and backing GSettings reads, with final restoration. Tier/mode coverage is exactly the registry. Workspace-count writes retain a deterministic safe STALE_REF finding after preparatory scrolling, owned by 3.15; no positive pass is claimed for a failing cell. |
| R14 | Retained Calculator disclosure legs read menu visibility/children in both directions. No Files tree-view success is inferred. |
| R15 | Settings no-route select asserts ACTION_NOT_SUPPORTED with its semantic-route reason. The unobservable Qt choice fixture is removed; native selection route/verification tests remain, without claiming positive live selection. |
| R16 | Settings Applications placeholder and Files absence use raw attributes and product snapshots. |
| R17 | Retained Calculator/Qt menu predicates and Terminal hidden-menu timeout; no unimplemented Terminal Preferences success is claimed. |
| R18 | Retained X11 ordinary/ABOVE cover controls compare editor caret/document, covering app, stacking and final state. Modal opener/surface acceptance remains outstanding on the landed product. |
| R19 | Retained seeded event transition/quiet cells, including the eleven restored Mutter owner/name/frame-timeout rows, assert token, scope and independent state; failed environment specimens are removed or received by 3.15. All surface-appeared/dismissed scoped/unscoped transition/quiet acceptance cells remain outstanding on the landed product. |
| R20 | Seed-before/after About surface acceptance remains outstanding on the landed opener; registry presence is not dedupe/baseline live proof. |
| R21 | Retained text-entry and held-modifier checks read actual document/key state. Product focus is never rescued by the harness. Search/replace version hardening is in 3.15. |
| R22 | Compare decoded supported capture pixels to XWD and assert exact unavailable-route refusals. Text/image clipboard content, setter lifetime and clear use foreign clients. HTML-only import is outside the supported target whitelist and is not claimed. Wayland keyboard-seat and consent runs belong to 3.15. |
| R23 | Private seatless Wayland observation/focus/capture/headed-ref refusal cells only. No first-key, seat/ACTIVE, direct hit-test or portal success is inferred. |
| R24 | Headless semantic click/value effects must succeed while pointer/active window remain unchanged, then all channels receive independent final reads. |
| R25 | Report each Openbox X11, Mutter X11 and headless Wayland mode independently. KDE and interactive runner provisioning/dispatch are 3.15 work. |
| R26 | Dispatch workflow wiring fails closed; hosted portable self-tests need no desktop. Wiring is not a measured hosted live run. |
| R27 | Python files below 400 lines; source-reference policy; recoverable trash cleanup; no dependency or core changes. |
| R28 | Implement only registered live families. FFI belongs to 3.13; shell/notification/tray to 3.14; cursor/hardware/integration hardening to 3.15. |
| R29 | Decide missing/retired target rows now; transferred requirements are in the receiving scope in docs/phases.md in the same commit. |
| R30 | Roadmap, this plan, README and findings ledger state measured behaviour; no removed check counts as a pass. |
| R31 | Stranger dogfood with a committed report and disposed findings; no performance baseline is required for this harness integration. |
| R32 | Retained GTK4/Qt gesture contracts use real app effects and pointer/focus restoration. The remaining toolkit × position × gesture stress matrix is explicitly required by 3.15. |
| R33 | Retained launch/close/focus/geometry/state and delivery-report cells use raw X11/process evidence. Mutter throttle/maximize/lift fixture hardening is explicit in 3.15. |
| R34 | All registered shell, tray and notification acceptance rows remain outstanding on the landed product, including bystander/no-signal and exact refusal twins. |
| R35 | Qt snapshot/action/menu/enable contracts on registered cells. VS Code and failed Mutter Qt launch cells require 3.15 generation-attributed private frames before dispatch. |
| R36 | Box access only through scripts/remote-linux.sh -c; bounded calls below 15 minutes; cargo starts with export CARGO_INCREMENTAL=0; owned PID kills only. |
| R37 | Compare delivery reports with actual effects and no-effect twins. A no-op reporting success or delivered flags cannot pass. |

## Implementation units

| Unit | Owner boundary | Implementation |
| --- | --- | --- |
| U1 | Inventory and prerequisite gate | runner/legs.py, gate.py; registration/selection tests |
| U2 | Session health and cleanup | scripts/private-session.sh, private-mutter-shell.sh, private-wayland-shell.sh; runner/session*.py |
| U3 | Staging, product boundary, verdict, reports | runner/stage.py, product.py, verdict.py, main.py, report.py |
| U4 | Independent oracles and app ownership | oracle/, runner/apps.py, helper.py |
| U5 | Observation and semantic state | scenarios/observation.py, state.py, values.py, zero_bounds.py, duplicates.py, placeholder.py, select.py |
| U6 | Input/ref/occlusion/non-interference | scenarios/ref_*.py, pointer*.py, headed_actions.py, text_entry.py, occlusion.py, non_interference.py |
| U7 | Events, identity, launch and lifecycle | scenarios/events.py, gaps_events.py, identity.py, launch.py, lifecycle.py, gaps_mutter.py |
| U8 | Capture and clipboard | scenarios/capture.py, gaps_capture.py, clipboard.py |
| U9 | Wayland refusal/observation | scenarios/wayland_actions.py and controlled_targets.py; KDE protocol in README, owned by 3.15 |
| U10 | Hosted portable/dispatch wiring | .github/workflows/ci.yml, linux-e2e.yml |
| U11 | Source-of-truth synchronization | docs/phases.md, this plan, integration-decisions.md, README, probes/linux/FINDINGS.md |
| U12 | Bounded live validation | Every retained leg once per registered tier/mode; sole diagnostic rerun on failure |
| U13 | Stranger dogfood | Operator uses only the shipped skill, binary and README; source withheld; root owns box calls |
| U14 | Outstanding surface contracts | scenarios/shell_surfaces.py, notifications.py, tray.py, surfaces.py; pending live acceptance on landed surfaces |
| U15 | Qt toolkit probe | apps/qt_window.py, toolkit_qt.py, controlled_targets.py; own log is an independent effect channel |

## Evidence and failure policy

Rerun a failing test/leg at most once. For timing/load/session/environment
failures delete the specimen or replace it with a deterministic fake contract
or real observed-state wait, in a `test:` commit naming the reason. Do not tune
a flaky test until it passes. Fix product code only for a deterministic defect,
especially wrong-element dispatch or false delivery. Retain deterministic
product repros as README findings. Transferred/removed checks never count as
passes. Keep unmeasured surface acceptance outstanding; the rebase includes the product surfaces.

Use separate named observables for separate state. Qt lifecycle log evidence
has the same members/log shape before launch, after effect, and after teardown.
The final read checks actual log removal and process cleanup. X11 geometry and
capture exclude independently read client shadows. Cleanup validates PID start
time and exact runtime membership immediately before signaling.

## Verification contract

- Mac: `python3 -I -m unittest discover -s tests/e2e-linux -p 'test_*.py'`
  and the corresponding `tests/e2e` portable suite.
- New unit tests fail under a deliberate broken effect/refusal/restoration
  check, then pass after restoration. Do not add source-structure/lane-ownership
  tests that reject valid integration; runtime verdict tests own evidence rules.
- `scripts/check-no-phase-references.sh`, Python line cap,
  `python3 scripts/check-linux-findings.py`, `git diff --check`.
- Gitleaks on `7dc6500f..HEAD`; author and committer Lahfir only; no attribution
  footers. Use explicit git paths; no push; finish with clean status.
- Build and run via `bash scripts/remote-linux.sh -c '...'`, cargo prefixed
  `export CARGO_INCREMENTAL=0 &&`. Batch retained IDs per tier, keeping each
  call under 15 minutes. Headed means input inside the private session.
- Never touch vda-ci/vda-parity, docker, postgres, Chrome PID 12669, Xvfb :99,
  the live accessibility bus/clipboard, or restart shared desktop services.
  Write only under ~/ad-wt, ~/ad-target, ~/.local/ad-tools. Signal own PIDs only.
- Read the separate Mutter diagnosis when available and apply a named harness
  repair. Never delete the whole Mutter tier to conceal a deterministic cause.
- README records current per-tier/mode pass/fail/outstanding and findings;
  historical lane receipts cannot certify the integrated head.

## Definition of done

All named gates complete, every retained executable cell has a current result,
no owner-decision row remains, every transfer is in the receiving roadmap
scope, and all claimed passes carry independent effect and final-state reads.
The stranger report contains dispositions. Missing box access or unavailable
hardware is reported plainly; it is not converted into a green receipt.

## Final validation receipt

The retained registry has 185 IDs, 244 executable tier/mode cells and 52
outstanding live surface acceptance cells on landed product surfaces. Live
results are 242 pass and two safe deterministic workspace STALE_REF failures
(Openbox headless/headed), retained as a product finding owned by 3.15.
Mutter's second-position fixture and legacy app-launched teardown-race variants
are deleted after their sole diagnostic reruns; Openbox and deterministic
contract coverage remain. No deleted cell counts as a pass. The 11 restored
Mutter owner event cells, four ref actions and canonical Qt headed click pass.

Release build, box Clippy and Linux library gate (2,673 pass, 251 ignored),
Mac portable harness (311 tests, one skip), shared E2E (58), line/source/findings,
secret and author gates are the targeted contract. README has per-mode counts,
release hashes, repros and artifact locations. The source-withheld operator
ran the documented clipboard selection and blanket-response negative control;
normal and failure cleanup were observed, while live cancellation was not.
