# Hyprland verification legs

Planned inventory for proposal sections 4 and 6 (the proposal text is not part of this branch). These are acceptance
contracts, not recorded passes. Upstream sources:
[runner/legs.py](https://github.com/vercel-labs/agent-desktop/blob/main/tests/e2e-linux/runner/legs.py),
[E2E README](https://github.com/vercel-labs/agent-desktop/blob/main/tests/e2e-linux/README.md), and
[JSON contract](https://github.com/vercel-labs/agent-desktop/blob/main/docs/json-output.md).
Scope and milestone choices follow the [parity matrix](parity-matrix.md).

| Registry rule | Application |
|---|---|
| Tier | Add `hyprland` to `TIER_NAMES`. Every gate leg below registers on that tier. Keep existing tier registrations and behavior. |
| REUSE | Existing upstream ID and effect contract; register it for `hyprland`. Port X11 fixture/oracle calls to independent Hyprland reads where needed. Registration alone does not make an X11 implementation portable. |
| NEW | New ID, using upstream's `area.behavior` naming. Each ID is a separate `Leg`, with requirement, target, tiers, modes, oracles, disposition and refusal reason. |
| Modes | Only `headless` and `headed`. `headless, headed` means two executions, not a third mode. `headed` permits input inside the private nested session; it never permits input to the host. |
| Oracles | Keep registry names `atspi`, `pixels`, `applog`, `proc`; read raw AT-SPI, independent capture pixels, app logs and PID/start-time process records. Add a validated `hyprland` oracle name for IPC reads below. Toplevel protocol events supplement IPC reads. Never use the product's output as its effect oracle. |
| IPC notation | `j/clients`, `j/activewindow`, `j/cursorpos`, `j/layers` are independent reads on the bound compositor. Workspace checks also read `j/activeworkspace` and `j/monitors`, including each monitor's special workspace. Layout checks use `j/devices`. Host reads use the host endpoint, never the nested endpoint. |
| Dispositions | Runner outcomes are `pass` and `asserted-refusal`. They are distinct from JSON `disposition.delivery` and `disposition.retry`. A refusal needs the expected upstream error code, reason and independent absence of effect. No invented error codes. |
| Evidence | Record before, after and restored reads, argv, envelopes, dispatch times and attributed process generations. Restore changed app/window state. Stop attributed app generations; full teardown must leave no session members. |
| Controls | Run each row's controls as well as its positive case. A lying `ok:true` action must fail effect verification. A missing oracle or failed restoration fails the leg. Controls that inject fixture failure are expected harness failures, not product `asserted-refusal` results. |

| Harness prerequisite | Required check |
|---|---|
| Private session | Reuse upstream gate, SHA-pinned binary/helper images and `private-session.sh`; wrap with `scripts/private-hyprland-shell.sh`. Unique 0700 directories; private runtime/config/data/cache/product state; sanitized activation environment and endpoint guards. Start `at-spi2-registryd` explicitly on Arch. |
| Parent and rendering | Self-hosted Hyprland; outer window stays on a hidden host special workspace. Nested compositor creates a timed headless output and disables its parent-backed output. A hidden parent window alone does not render. See [parent probe](evidence/2026-10-31-headless-parent.md) and [protocol probes](evidence/2026-10-31-protocol-probes.md). |
| Ownership | Record PID and `/proc` start time for compositor, apps and renderer. Teardown only those generations. Cancellation follows the upstream readiness receipt and owner marker, not a guessed PID. |
| Isolation | Snapshot host focus, cursor and normal/special workspaces before startup, throughout actions, after cleanup and after compositor teardown. Only the harness-owned outer window may be created on the host. |
| Completion | Each milestone's cumulative legs and controls pass on the test host three consecutive times and once on a second Hyprland host. Keep other upstream tiers green. An unregistered, unavailable, blocked or unimplemented leg makes no live claim. |

## M1: binding, identity and focus

| Requirement | Leg ID | Modes | Independent oracle and expected effect | Disposition | Negative controls |
|---|---|---|---|---|---|
| R1 | NEW `session.hyprland_discovery_single_live_instance` | headless | Process + IPC + socket peer credentials: discover the single live IPC/Wayland pair without environment variables; stale instance directories do not select a dead compositor. | pass | SSH/systemd-like empty environment; dead sockets beside the live pair; lying discovery result. |
| R1 | NEW `session.hyprland_explicit_binding_with_other_instances` | headless | Process + IPC: explicit binding selects the validated compositor while a second live compositor exists; only selected `j/clients` is observed. | pass | Swap instance signatures or Wayland socket; compare both compositors for effects. |
| R1 | NEW `session.hyprland_ambiguous_or_mismatched_binding_refused` | headless, headed | Process + both IPC endpoints + app log: discovery with several live instances, no live instance, or IPC/Wayland peer mismatch sends no action. | asserted-refusal | Valid single pair succeeds; an explicit validated pair succeeds with several instances. |
| R1 | NEW `session.hyprland_silent_socket_deadline` | headless, headed | Process + monotonic timing + app log: silent IPC/Wayland endpoint refuses within upstream's silent-display budget without delivery. | asserted-refusal | Healthy endpoint succeeds; disconnect during discovery; socket that accepts but never replies. |
| R1 | NEW `session.other_wayland_route_unchanged` | headless | Process + raw AT-SPI: isolated non-Hyprland Wayland fixture retains its existing route and results. | pass | Hyprland-like environment with no matching compositor must not select Hyprland; rerun existing `wayland_actions.*` on `wayland`. |
| R2 | NEW `identity.hyprland_unique_evidence_chain` | headless | `j/clients` + process + raw AT-SPI + ext identifier/address and wlr handle mapping: exactly one frame matches PID/start time, instance, handle, exact title, size and workspace. | pass | Change one field at a time; two processes with same title; same-process windows with distinct size/title/workspace. |
| R2 | REUSE `duplicates.two_processes_distinct_window_ids` | headless | Raw AT-SPI + process + `j/clients`: both Calculator generations have distinct resolvable IDs. | pass | Identical titles and start ticks across different PIDs; check each tree belongs to its owner. |
| R2 | NEW `identity.hyprland_zero_or_multiple_candidates_refused` | headless, headed | `j/clients` + raw AT-SPI + app logs: no candidate or indistinguishable same-PID/title/size/workspace twins receive no input. | asserted-refusal | Give one twin a unique title/size and require success; never choose by enumeration order or guessed global AT-SPI position. |
| R2 | NEW `identity.hyprland_stale_handle_or_process_refused` | headless, headed | Process + `j/clients` + app log: destroyed handle, reused address/PID with changed start token, or compositor restart cannot retarget an old ID/ref. | asserted-refusal | Fresh ID resolves; unrelated live app receives no event. |
| R2 | REUSE `identity.restart_old_id_window_not_found` | headless | Raw AT-SPI + process + `j/clients`: old ID returns `WINDOW_NOT_FOUND` after restart. | pass | Fresh ID works; no old-generation alias. Preserve upstream's `pass` classification. |
| R2 | REUSE `identity.restart_new_id_resolves` | headless | Raw AT-SPI + process + `j/clients`: restarted app's new ID resolves to its new generation. | pass | Old ID does not resolve. |
| R2 | REUSE `identity.id_survives_unrelated_launch` | headless | Raw AT-SPI + process + `j/clients`: unrelated launch leaves the original ID bound to the same frame. | pass | Reorder inventory; launch an identically titled bystander. |
| R2 | REUSE `ref_contract.zoo_gtk3` | headless | Raw AT-SPI + app log: every GTK3 snapshot ref lands or is explicitly marked and refused. | pass | Unsupported, disabled, stale and ambiguous nodes; exactly-once effect. |
| R2 | REUSE `ref_contract.zoo_gtk3_windows` | headless | Raw AT-SPI + app log: GTK3 window surfaces resolve and operate correctly. | pass | Hidden/closed surface and wrong owner. |
| R2 | REUSE `ref_contract.zoo_gtk4` | headless | Raw AT-SPI + app log: same ref contract for GTK4. | pass | Unsupported, disabled, stale and ambiguous nodes; exactly-once effect. |
| R2 | REUSE `ref_contract.zoo_gtk4_windows` | headless | Raw AT-SPI + app log: GTK4 window surfaces resolve and operate correctly. | pass | Hidden/closed surface and wrong owner. |
| R2 | REUSE `ref_contract.zoo_qt` | headless | Raw AT-SPI + app log: same ref contract for Qt. | pass | Unsupported, disabled, stale and ambiguous nodes; exactly-once effect. |
| R2 | REUSE `ref_contract.zoo_qt_windows` | headless | Raw AT-SPI + app log: Qt window surfaces resolve and operate correctly. | pass | Hidden/closed surface and wrong owner. |
| R2b | NEW `identity.compositor_only_window_inventory` | headless | `j/clients` + process + toplevel events; raw AT-SPI confirms no tree: Ghostty and Alacritty IDs are listable, stable and explicitly lack AT-SPI proof. | pass | Duplicate titles, terminal restart, forged AT-SPI association; do not fabricate refs. |
| R3 | NEW `focus.hyprland_window_dual_proof` | headed | `j/activewindow` + wlr `activated` + raw AT-SPI frame focus all identify the target after `focus-window`. | pass | A stale compositor read or AT-SPI-only focus cannot establish success. |
| R3 | NEW `focus.hyprland_workspace_side_effects` | headed | `j/activewindow` + workspace/monitor reads + raw AT-SPI: focus across normal workspaces, reveal hidden special target, then leave special workspace. Envelope reports switch/reveal/close effects; restore prior state. | pass | Same-workspace target reports no invented switch; hidden target without `--headed` cannot activate. |
| R3, R2b | NEW `focus.compositor_only_window_proof` | headed | `j/activewindow` + wlr `activated` + process: terminal gains focus; result states AT-SPI proof is unavailable. | pass | Wrong handle or contradictory activation state refuses; no semantic proof claim. |
| R3 | NEW `focus.headless_existing_focus_or_refusal` | headless | `j/activewindow` + activated state + raw AT-SPI: already-focused scoped action verifies focus without activation; inactive/hidden target refuses. | pass | Record unchanged focus, cursor and workspace for inactive/hidden cases; fake focus agreement fails verification. |
| R3 | REUSE `ref_actions.focus` | headed | Raw AT-SPI + `j/activewindow`: Qt entry gains `FOCUSED`, then original button regains focus. | pass | Missing focus state; wrong frame; lying action. |
| R3 | REUSE `ref_actions.focus_headless_refusal` | headless | Raw AT-SPI + `j/activewindow` + `j/cursorpos`: headless ref focus refuses with no focus effect. | asserted-refusal | Headed counterpart works; hidden special target stays hidden. |
| R3 | NEW `focus.contradictory_or_missing_proof_refused` | headed | `j/activewindow` + raw AT-SPI + app log: focus proof disagreement or deadline prevents subsequent keys/clicks. | asserted-refusal | Agreeing proof permits delivery; report any activation that already occurred, never claim no side effects. |

The [identity probe](evidence/2026-10-31-protocol-probes.md) still lacks a host
refusal-rate sample for a notes app, a file manager and a multi-window browser. Before M1,
record matched/refused/total counts and reasons for a browser, an Electron app, a notes app,
a file manager and the test apps. This is measurement, not permission to weaken identity.
Existing generic Wayland focus-refusal legs stay on `wayland`; they are not
Hyprland success legs. `lifecycle.focus_window_active_reads` is headless and
X11-specific, so it is not reused for headed Hyprland activation.

## M2: capture

| Requirement | Leg ID | Modes | Independent oracle and expected effect | Disposition | Negative controls |
|---|---|---|---|---|---|
| R7, R2, R2b | NEW `capture.bound_toplevel_pixels` | headless | `j/clients` + process + app log + pixels: known pattern, bounds and nonce belong to the exact selected window, including tree-less Ghostty/Alacritty; another window/output has a different pattern. | pass | Bystander pattern; wrong handle; hidden, covered and non-active workspace targets; no activation. |
| R7 | NEW `capture.hidden_counter_frame_after_request` | headless | App log + ticking-counter pixels + monotonic request marker: captured value was rendered after the request, including on a hidden special workspace. | pass | Cached pre-request frame, wrong counter, full-buffer damage without fresh pixels must fail. |
| R7 | NEW `capture.static_frame_after_request` | headless | Independent protocol trace + static pattern pixels: a new frame completes after request although its pixels equal the previous frame. | pass | Replayed completion/frame; stopped nested rendering must time out, not return cached pixels. |
| R7 | NEW `capture.screen_exact_output` | headless | `j/monitors` + independent screencopy pixels + app pattern log: `--screen` captures the selected output with correct bounds/scale. | pass | Different output pattern; negative-origin and fractional-scale layout; no surface fallback. Add cursor exclusion at M4b. |
| R7 | NEW `capture.permission_denied_no_dialog` | headless | Raw AT-SPI + `j/clients`/`j/layers` + process: enforced permission denial returns an upstream error without a consent dialog or image. | asserted-refusal | Deny toplevel and screen separately; ensure no portal helper launch or fallback capture. |
| R7 | NEW `capture.deadline_disconnect_or_cancel` | headless | Process + protocol trace + pixels: no rendering, compositor loss or cancellation terminates within the deadline and yields no success frame. | asserted-refusal | Withheld `ready`, partial buffer, stale handle; healthy static target succeeds; cleanup releases resources. |

`capture.window_pixels_match_xwd` and `gaps.display_capture` require XWD and
are not reused. `gaps.wayland_window_capture_refused` stays on generic Wayland.
The [capture probe](evidence/2026-10-31-protocol-probes.md) shows full damage on
unchanged frames; neither damage nor frame arrival alone proves pixel freshness.

## M3: keys

| Requirement | Leg ID | Modes | Independent oracle and expected effect | Disposition | Negative controls |
|---|---|---|---|---|---|
| R4 | REUSE `headed_actions.type_text` | headed | Raw AT-SPI + app log + `j/activewindow`: exact text reaches the document and restores. | pass | Wrong focus; Unicode and punctuation; duplicate/missing characters. |
| R4 | REUSE `text_entry.press_combos_edit_document` | headed | Raw AT-SPI + key log: Return inserts one newline; Ctrl+A/Backspace clears; no held modifier remains. | pass | Minimal text-only keymap that emits Tab without toolkit navigation must fail. |
| R4 | REUSE `text_entry.held_key_released` | headed | App key log + key-state observer: held key is released after delivery. | pass | Abort with modifier held; next unmodified key must remain unmodified. |
| R4 | NEW `keys.owner_keymap_and_layout_restore` | headed | `j/devices` + independent `wl_keyboard.keymap` observer + raw AT-SPI/app log: preserve layout, variant and options; Tab navigates; Unicode arrives; physical map returns after virtual keyboard teardown. | pass | Non-US layout, variant/options, extra keysym; compare map before/during/after, including failures. |
| R4, R2b | NEW `keys.compositor_only_window_delivery` | headed | `j/activewindow` + activated state + terminal child log/process: targeted press and the defined terminal text route reach only Ghostty/Alacritty's child. | pass | No tree/ref; duplicate terminal title; focused bystander; echo alone is insufficient without child log. |
| R4 | NEW `keys.focus_theft_mid_batch` | headed | Timestamped app logs + `j/activewindow` + raw AT-SPI/key observer: theft stops later key-downs, releases held keys, reports `delivery_uncertain` and event counts. | pass | Steal before first key-down and between key-downs; check bystander log and final layout. |
| R4 | NEW `keys.cancel_or_disconnect_releases_keys` | headed | Process + app key log + keymap observer: cancellation or compositor loss stops delivery and cleans virtual resources; reconnect observes no held key/layout drift. | pass | Cancel with modifier held; dead compositor cannot receive release, so do not claim verified delivery to it. |
| R4, R3 | NEW `keys.headless_physical_delivery_refused` | headless | `j/activewindow`/`j/cursorpos` + app logs: physical press/type without headed permission is refused before delivery. | asserted-refusal | Semantic `type` still works without physical input; inactive and hidden targets never activate. |

The terminal text invocation must be defined in the R2b/R4 upstream design;
the current CLI requires an editable ref for `type`. The terminal leg cannot
pass by adding a fake ref. Document the focus check-then-send race; timestamped
observations cannot prove that race is closed.

## M4: pointer and close

| Requirement | Leg ID | Modes | Independent oracle and expected effect | Disposition | Negative controls |
|---|---|---|---|---|---|
| R6 | REUSE `pointer.mouse_click_delivery_report` | headed | `j/cursorpos` + raw AT-SPI/app log: click lands once; delivered and refused reports match observed effects. | pass | Missing target, undelivered event, lying success. |
| R6 | REUSE `pointer.mouse_wheel_delivery_report` | headed | `j/cursorpos` + raw AT-SPI/app log: signed vertical/horizontal wheel units and direction change the intended viewport. | pass | Zero/no-effect, opposite direction, missing receiver; preserve distinct MCP pages and CLI units. |
| R6 | REUSE `pointer.drag_delivery_report` | headed | `j/cursorpos` + raw AT-SPI/app log: complete drag report agrees with selection/drop and button release. | pass | Cancel after button-down, failed drop and partial delivery. |
| R6 | REUSE `ref_actions.drag` | headed | Raw AT-SPI + `j/cursorpos`: ref drag selects expected document text, then restores selection. | pass | Wrong endpoint, stale source/destination, occluded path endpoint. |
| R6 | REUSE `ref_actions.drag_headless_refusal` | headless | Raw AT-SPI + `j/cursorpos`: headless drag refuses with no selection or pointer change. | asserted-refusal | Headed counterpart succeeds. |
| R6 | REUSE `ref_actions.scroll` | headed | Raw AT-SPI + app log: visible Settings rows change, then restore. | pass | Wrong units/direction; lying delivery. |
| R6 | REUSE `ref_actions.scroll_headless_refusal` | headless | Raw AT-SPI + `j/cursorpos`: headless physical ref scroll refuses with unchanged rows. | asserted-refusal | Do not block independent semantic scroll-to routes. |
| R6 | NEW `pointer.global_layout_coordinates` | headed | `j/clients` + `j/monitors` + `j/cursorpos` + app event log: exact click/wheel/drag destinations across monitors, fractional scale and negative origins. | pass | Deliberate double scaling, rounding near boundaries, translated crop origin; compare app-local receiver coordinates. |
| R6 | NEW `pointer.ref_receiver_evidence` | headed | `j/clients` + `j/layers` + app logs: target point is inside mapped toplevel with no covering layer, floating or pinned window. | pass | Place each cover in turn; remove it and require delivery. Repeat with R8 suppression in M4b. |
| R6 | NEW `pointer.cover_or_missing_evidence_refused` | headed | `j/clients` + `j/layers` + logs from target and covers: covered/outside target or incomplete geometry/stack evidence prevents delivery. | asserted-refusal | Layer cover, floating cover, pinned cover, unreadable layers; both apps remain unchanged. |
| R6 | NEW `pointer.coordinates_have_no_receiver_proof` | headed | `j/cursorpos` + app event log: coordinate-only click/wheel/drag effects are observed while the envelope states no receiver proof. | pass | Coordinate happens to hit a known window; this must not upgrade the proof claim. |
| R6 | NEW `pointer.headless_or_cancelled_input` | headless | `j/cursorpos` + app logs: click/wheel without headed permission refuses before pointer movement or delivery. | asserted-refusal | Headed works; cancellation after dispatch is covered separately and cannot claim `not_delivered`. |
| R6 | NEW `pointer.cancel_or_disconnect_releases_buttons` | headed | App event log + pointer-state observer + process: cancellation/disconnect stops later events, releases buttons where connected and leaves no virtual pointer client. | pass | Cancel during drag; inspect partial counts and unsafe retry; dead compositor is not proof of delivered release. |
| R5 | NEW `lifecycle.close_app_process_exits` | headless | Process + raw AT-SPI + `j/clients`: graceful close exits the exact app generation; bystander process/window survives. | pass | Same-name bystander, multi-window app, lying close result; closing one window with process alive cannot satisfy process exit. |
| R5 | NEW `lifecycle.close_app_blocking_dialog_reported` | headless | Raw AT-SPI + `j/clients` + process: unsaved document leaves app alive and reports its actual blocking dialog. | pass | Suppress/misidentify dialog; command must not claim process exit or force-kill. Dismiss dialog during restoration. |
| R5 | REUSE `lifecycle.close_app_undelivered_report` | headless | Process + `j/clients`: stopped app generation stays alive and result reports `not_delivered`; resume before cleanup. | pass | Lying delivered report; unintended kill; account for any sent request under R10. |

Move and resize have no recorded parity-matrix use and stay at M6, along with
minimize/maximize/restore. Headed `focus-window` remains an explicit M1 gate.

## M4b: renderer and four agents

| Requirement | Leg ID | Modes | Independent oracle and expected effect | Disposition | Negative controls |
|---|---|---|---|---|---|
| R8 | NEW `cursor.session_renderer_lifetime` | headless | Process + `j/layers` + output pixels: `session start --cursor --multi-agent` starts one persistent renderer for all session cursors; command exit leaves it alive; enable returns `rendered:true`. | pass | Short-lived command-owned surfaces, second renderer for same session, wrong PID/start token. |
| R8 | NEW `cursor.agent_identity_and_profiles` | headless | `j/layers` + output pixels + process: one cursor per session/agent pair; flag and environment agent IDs work; distinct sessions remain separate; profile changes presentation only. | pass | Same pair repeated, same agent ID across sessions; profile cannot alter target, policy, lease or receiver evidence. |
| R8 | NEW `cursor.missing_agent_id_refused` | headless, headed | Process + `j/layers` + app log: multi-agent command with neither flag nor environment ID refuses, without cursor or app effect. | asserted-refusal | Provide each ID route separately and require success; include in four-agent run. |
| R8 | NEW `cursor.arrival_ack_before_dispatch` | headless, headed | Independent renderer protocol trace + app event timestamps + pixels: cursor arrival acknowledges before dispatch under bounded deadline. | pass | Delay/withhold acknowledgement; animation cannot be inferred from saved cursor state alone. |
| R8 | NEW `cursor.ack_timeout_or_renderer_loss_refused` | headless, headed | Protocol trace + process + app log: missing/dead renderer or withheld arrival acknowledgement prevents dispatch and times out within budget. | asserted-refusal | No late effect after timeout; healthy renderer permits next command. |
| R8 | NEW `cursor.layout_placement_and_hidden_targets` | headless | `j/clients` + `j/layers` + output pixels: cursors track global layout across fractional scales/negative origins; hidden workspace targets draw no visible cursor. | pass | Wrong output/scale, hidden target on visible output; no OS cursor movement. |
| R8 | NEW `cursor.no_focus_or_input_interference` | headed | `j/activewindow`/`j/cursorpos` + workspace reads + app log + `j/layers`: overlay layer, namespace `agent-desktop-cursor`, empty input region; pointer/key input reaches app. | pass | Place cursor over receiver; accidental input region or keyboard interactivity must fail; host stays unchanged. |
| R8, R7 | NEW `cursor.window_capture_excludes_overlay` | headless | `j/layers` + window pixels and independent output pixels: cursors visible on output are absent from bound window capture. | pass | Deliberately include cursor colours in a false capture; wrong-surface capture must fail. |
| R8, R6, R7 | NEW `cursor.hide_ack_suppression_until_show` | headed | Renderer trace + `j/layers` + pixels + app log: hide acknowledges before receiver check/screen capture/input; new semantic animations acknowledge arrival but draw nothing until hide holder shows. | pass | Another agent animates or sends show during suppression; delayed hide ack; screen/click must not proceed early. |
| R8 | NEW `cursor.disable_end_crash_and_compositor_loss` | headless | Process + `j/layers` + pixels: disable removes all session cursors; end exits renderer; crash/disconnect removes surfaces; no stale ownership record can kill a replacement generation. | pass | SIGTERM/SIGKILL renderer, compositor loss, reused PID; cancel a hide holder and require suppression cleanup before the next operation; unrelated session survives; restart does not duplicate cursors. |
| R9 | NEW `parallel.semantic_actions_different_targets_overlap` | headless | Timestamped app logs + raw AT-SPI + process: actions on different apps overlap in dispatch/effect intervals without the physical interaction lease; each final state is exact. | pass | Hold physical lease while semantic actions run; fake sequential scheduler must fail overlap test; no focus/cursor/workspace change. |
| R9 | NEW `parallel.same_target_conflict_safety` | headless | Raw AT-SPI + app logs: simultaneous mutations of the same frame cannot cause wrong-target, duplicate or falsely verified delivery. | pass | Barrier-synchronized conflicting set/type/click; stale ref mid-action; assert documented conflict/serialization result and exact final state before registration. |
| R9 | NEW `parallel.four_agents_three_interleavings` | headed | Process + all four app logs/raw AT-SPI + IPC + output pixels + renderer trace: four distinct agent processes drive different apps; semantic work overlaps, physical dispatch intervals serialize, queue reports match observed waits. | pass | Three schedules below; inherited lease disabled for children; missing ID, timeout and cancellation companions below. |
| R9 | NEW `parallel.physical_lock_timeout` | headed | Process + dispatch logs + raw AT-SPI: held lease makes contender return `TIMEOUT`, `details.kind=lock_timeout`; no contender events. Same-process companion requires `interaction_process_lock_timeout`. | asserted-refusal | Release lease permits queued delivery; no inherited harness lease; timeout process sends no late input. |
| R9 | NEW `parallel.cancel_waiter_and_holder` | headed | Process + dispatch logs + raw AT-SPI + key/button observer: cancelled waiter sends nothing; cancelled holder releases lease/held inputs; next agent proceeds and each app has the recorded final state. | pass | Cancel at queued, acquired and partial-delivery barriers; check partial delivery cannot report safe retry. |

| Four-agent schedule | Required interleaving and reads |
|---|---|
| Setup for all three passes | A/B/C/D have distinct agent IDs and apps in one private compositor. A dedicated runner phase releases the harness interaction lease, removes inherited lease tokens/descriptors from child environments, then starts the four children. Each child acquires the real per-user lease itself; the runner reacquires its lease after all children exit. Run only when no other agent-desktop use is active for that user. Log monotonic enqueue, acquire, dispatch start/end, effect and release times plus success-envelope queueing. Restore each app and renderer; verify the same owned generations. |
| Pass 1: simultaneous start | Release four barriers together. A/B semantic operations on different apps overlap. C/D headed operations queue and dispatch serially. Each app's counter/text/selection matches only its own commands; all four cursors have distinct ownership. |
| Pass 2: screen capture overlap | C acquires lease and gets hide acknowledgement for `--screen`. While suppressed, A/B semantic operations animate and acknowledge arrival; D queues. Capture pixels contain no agent cursor. A/B finish without waiting for C's physical lease. C shows after capture, then D dispatches. |
| Pass 3: headed click overlap | D holds lease, hides cursors and pauses before receiver check/click. A semantic action animates over D's click target while B mutates another app; C queues. A's arrival acknowledges with no surface drawn. D's receiver evidence and app log prove the click lands once; show occurs after dispatch. C then acquires. |
| Companion controls | Run the missing-agent-ID, timeout and cancellation legs with the same four-app setup. Verify every app, including bystanders and cancelled agents, not only successful envelopes. Use barriers/observed events, not sleeps, to select each timing. |

[Baseline](evidence/2026-10-31-baseline.md) semantic success does not prove R9:
upstream currently leases semantic ref actions too. The [renderer probe](evidence/2026-10-31-protocol-probes.md)
proved surface lifetime and capture behavior, but did not prove pointer
pass-through. The M4b leg must measure it.

## M6: remaining window and pointer operations

| Requirement | Leg ID | Modes | Independent oracle and expected effect | Disposition | Negative controls |
|---|---|---|---|---|---|
| R5 | NEW `lifecycle.hyprland_move_resize_restore` | headed | `j/clients` + app configure log: move/resize reaches requested logical geometry within stated tolerance, then restores. Test detected Lua and legacy config-language dispatch. | pass | Missing/stale ID; negative origins; fractional scale; wrong config dialect; no fallback to another window. |
| R5 | NEW `lifecycle.hyprland_minimize_restore` | headed | `j/clients` + toplevel state + raw AT-SPI/pixels: minimize owns and records its state; restore returns the same window to specified workspace, geometry and visibility. | pass | Already minimized, app restart, foreign minimizer, stale restore record. Specify ownership/postconditions before implementation; no borrowed X11 hidden-state proof. |
| R5 | NEW `lifecycle.hyprland_maximize_restore` | headed | `j/clients` + toplevel state + app log: maximize and restore return exact prior state/geometry/workspace. | pass | Already maximized, changed output, wrong target, incomplete state evidence. |
| R5 | NEW `lifecycle.hyprland_window_operation_refusals` | headless, headed | `j/clients` + process + app log: stale target, unsupported request or absent state evidence refuses; headless operation cannot activate or change active workspace/cursor. | asserted-refusal | Supported headed counterparts work; missing ID cannot affect real window. |
| R6 | REUSE `ref_actions.hover` | headed | Raw AT-SPI + app log + `j/cursorpos`: Qt native status text appears and restores. | pass | Covered receiver, missing geometry; overlay suppression applies. |
| R6 | REUSE `ref_actions.hover_headless_refusal` | headless | Raw AT-SPI + `j/cursorpos`: no hover/status change without headed permission. | asserted-refusal | Headed counterpart works. |
| R6 | REUSE `ref_actions.double_click` | headed | Raw AT-SPI + app log: exactly the expected word is selected, then restored. | pass | Single click cannot pass; occlusion; wrong count. |
| R6 | REUSE `ref_actions.double_click_headless_refusal` | headless | Raw AT-SPI + `j/cursorpos`: no selection/pointer effect. | asserted-refusal | Headed counterpart works. |
| R6 | REUSE `ref_actions.triple_click` | headed | Raw AT-SPI + app log: exactly the expected line is selected, then restored. | pass | Double click cannot pass; occlusion; wrong count. |
| R6 | REUSE `ref_actions.triple_click_headless_refusal` | headless | Raw AT-SPI + `j/cursorpos`: no selection/pointer effect. | asserted-refusal | Headed counterpart works. |

Existing `lifecycle.minimize_restore`, `lifecycle.maximize_restore` and
`lifecycle.window_operation_delivery_reports` are headless X11 contracts.
Keep them unchanged; the new Hyprland legs prove the approved Wayland state model.
Use foreign-toplevel requests where available; IPC dispatch is limited to move
and resize. Unsupported requests cannot be counted as successful operations.

## Every milestone: non-interference and contract parity

| Requirement | Leg ID | Modes | Independent oracle and expected effect | Disposition | Milestones and negative controls |
|---|---|---|---|---|---|
| R9 | REUSE `non_interference.active_window_unchanged_by_semantic_click` | headless | Raw AT-SPI verifies click; `j/activewindow`, `j/cursorpos` and workspace/monitor reads remain unchanged before/after/restoration. Port existing `scenarios/non_interference.py`. | pass | M1, M2, M3, M4, M4b, M6. Covered and hidden special-workspace app; intentionally activating fake must fail. |
| R9 | NEW `non_interference.host_focus_cursor_workspace_unchanged` | headless, headed | Independent host IPC sampling: same focus, cursor, active normal and special workspaces throughout startup, nested actions and teardown. | pass | M1, M2, M3, M4, M4b, M6. Deliberately drift a nested-only sentinel in oracle self-test; the comparison must detect it. Never inject host interference as a control. |
| R9 | NEW `non_interference.headless_route_preserves_desktop` | headless | Nested `j/activewindow`/`j/cursorpos` + workspace reads + raw AT-SPI/app logs: every available headless command preserves focus/cursor/workspace while its semantic/read-only effect is checked. | pass | M1 inventory/snapshot/semantic actions; M2 hidden capture; M3 semantic type and physical refusal; M4 close and pointer refusal; M4b cursor/parallel semantic work; M6 operation refusals and supported non-activating operations. Poll during work to catch transient changes. |
| R1, R8, R9 | NEW `session.cancel_cleanup_recorded_generations` | headless, headed | Process + runtime paths + host IPC: readiness-receipt cancellation removes private session members/directories, preserves host state and unrelated generations. Add renderer checks at M4b. | pass | M1, M2, M3, M4, M4b, M6. Stale owner marker/start token must refuse signalling; changed pinned image fails gate/run; observe no leaked descendants. |
| R10 | REUSE `error_contract.recoverable_error_envelopes` | headless | Raw AT-SPI + process: `INVALID_ARGS`, `SNAPSHOT_NOT_FOUND`, `APP_NOT_FOUND`, suggestions/recovery and exit 1 agree with no effect. | asserted-refusal | M1 onward, all six milestones. Bare ref, absent snapshot and unknown app; lying success must fail. |
| R10 | REUSE `observation.stale_ref_after_ui_change` | headless | Raw AT-SPI + process: closed-menu ref yields `STALE_REF`, refresh recovery and `not_delivered`; fresh snapshot gives a different ref. | asserted-refusal | M1 onward, all six milestones. Live fresh ref works; owner and display unchanged on refusal. |
| R10 | REUSE `batch_contract.entry_failure_and_stop_on_error` | headless | Raw AT-SPI: failed middle entry stays inside successful batch envelope; display is `78` normally and `7` with stop-on-error at index 1; Undo restores. | asserted-refusal | M1 onward, all six milestones. Do not infer effects from completed-entry counts alone. |
| R10 | NEW `error_contract.hyprland_delivery_boundaries` | headless, headed | App logs + raw AT-SPI + process + IPC: envelope schema, upstream codes, exit status and `physical_delivery_started` agree with independently logged dispatch boundaries. | pass | M1, M2, M3, M4, M4b, M6. Before dispatch: `not_delivered`/`safe`; after partial physical dispatch: `delivery_uncertain`/`unsafe`, accurate event counts, no blind retry. Include identity refusal, focus failure, permission denial, timeout, cancellation and disconnect as features arrive. Missing verification cannot yield `delivered_verified`. |

Every refusal row must pin its exact existing JSON error code and details in
its implementation and registry `reason`, following the upstream contract.
`asserted-refusal` describes a tested refusal, not an exemption from an unmet
positive requirement. R11's build, architecture, license, resource, size and
skill gates run alongside these live legs; this document does not replace them.

## R9b: research only

These NEW IDs name reproducible research cases, not `hyprland` gate registrations.
Their results do not block M1/M2/M3/M4/M4b/M6 or replace R8/R9. Recorded M0
results are in [independent-pointer evidence](evidence/2026-10-31-r9b-independent-pointers.md).

| Requirement | Research leg ID | Modes | Independent oracle | Expected disposition | Negative controls | Milestone |
|---|---|---|---|---|---|---|
| R9b | NEW `research.nested_compositors_independent_pointers` | headed | Both private `j/cursorpos` streams + app logs + process + host IPC: two nested compositors retain different pointer positions and affect only their own launched apps. | pass | Move A while B stays fixed, then reverse; owner's existing window cannot be driven through either nested session; host unchanged; teardown both. | M0 evidence only; informs M4b |
