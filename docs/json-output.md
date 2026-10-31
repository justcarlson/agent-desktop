# JSON output contract

Every command returns structured JSON:

```json
{
  "version": "2.5",
  "ok": true,
  "command": "click",
  "data": {
    "action": "click",
    "disposition": { "delivery": "delivered_unverified", "retry": "unsafe" }
  }
}
```

Errors include machine-readable codes and recovery hints:

```json
{
  "version": "2.5",
  "ok": false,
  "command": "click",
  "error": {
    "code": "STALE_REF",
    "message": "@s8f3k2p9:e7 not found in current RefMap",
    "suggestion": "Re-run a snapshot to obtain fresh refs, then retry with the new ref (CLI: snapshot [--skeleton]; FFI: ad_snapshot then pass the printed qualified ref to ad_execute_by_ref)",
    "recovery": {
      "strategy": "refresh_snapshot_then_retry_original",
      "retryable": true,
      "requires_fresh_snapshot": true
    },
    "disposition": {
      "delivery": "not_delivered",
      "retry": "safe"
    }
  }
}
```

`error.code`, `error.message` and `error.disposition` are always present. `suggestion`, `recovery`, `platform_detail` and `details` are omitted when empty. `recovery` appears only when `disposition.retry` is `safe`, and its `retry_after_ms` is set only for `TIMEOUT` and `APP_UNRESPONSIVE`.

Version 2.2 replaces the 2.1 `TIMEOUT` error for a budget-exhausted snapshot
with a successful, explicitly partial response, and adds `data.complete`.
Snapshot data always carries `complete`. A snapshot that runs out of budget now
returns `ok: true` with `"complete": false`, the tree it did observe,
`"truncated": true`, and `nodes_observed`, instead of a `TIMEOUT` error and no
tree. Every node whose descendants were cut short carries
`"subtree_truncated": true`, so a reader can walk from the root to each
boundary. Consumers must read `data.complete` to decide whether a tree is whole;
a `TIMEOUT` error no longer signals an oversized tree. A `--root` drill-down is
unaffected: it replaces refs inside an existing snapshot, so an incomplete
observation still returns `TIMEOUT` rather than a partial tree.

Version 2.1 replaces the 2.0 `error.retry_command` string with the structured
`error.recovery` object and adds `error.disposition`. Consumers must select a
recovery strategy from `recovery.strategy` only when `disposition.retry` is
`safe`; command strings from older envelopes must not be executed blindly.
The removed `retry_command` field has no compatibility alias.

## Disposition values

Every error carries `disposition`, and commands that act on a target may also
carry it on their result. `delivery` is one of `unknown`, `not_delivered`,
`delivery_uncertain`, `delivered_unverified` or `delivered_verified`. `retry`
is one of `unknown`, `safe` or `unsafe`, and it follows from `delivery`:
`not_delivered` is `safe`, and every other non-`unknown` value is `unsafe`.

`unknown` means the command established nothing about delivery. A failure of a
mutating command that carries no delivery evidence, such as a `TIMEOUT` on an
operation that may already have landed, keeps it. A read-only command sends
nothing and invalid arguments never reach the application, so their errors, and
a lookup failure such as `APP_NOT_FOUND`, carry `not_delivered`. Treat `unknown`
as not safe to retry automatically: read `error.code` and the message, and
choose a recovery strategy only when `disposition.retry` is `safe`.

## Version 2.4 migration

Stateful ref actions (`set-value`, `type`, `clear`, `check`, `uncheck`, `toggle`, `expand`, and `collapse`) now reject observed postcondition contradictions after one delivery attempt. Readback failures preserve the adapter's error code and carry `details.kind: "post_action_verification"`. Missing evidence alone remains a successful `delivered_unverified` result with `details.verification_scope: "unavailable"`; neither that result nor verified element state proves application-level persistence.

macOS mutation errors that do not prove non-delivery stop fallback chains with an unsafe retry disposition. This affects activation, selection, disclosure, scrolling, and window operations; an unchanged or unreadable state is not proof that a mutation was never delivered. Value-writing chains also stop after unverified delivery instead of applying a second mechanism. Failures after preparatory scrolling carry `details.preparatory_scroll`, which records that the view may have changed. Their retry disposition follows the failure: a refusal that sent nothing is `not_delivered` and safe to retry, and any other failure keeps its own disposition.

Inspect `disposition.retry` before retrying or choosing an alternative action, regardless of `error.code`. An error may include `post_state` and `postcondition_satisfied` without erasing the original execution failure. The same verification rules apply to FFI ref actions; the C ABI is unchanged. The Rust `ActionResult::from_execution` constructor now takes only action and steps and returns an infallible result; live verification belongs to `execute_verified_action`.

## Version 2.5 migration

Version 2.5 is a breaking payload change for the commands that dispatch pointer, drag, clipboard, app-close and window operations: `mouse-click`, `mouse-move`, `mouse-wheel`, `hover`, `drag`, `clipboard-set`, `clipboard-clear`, `close-app`, `move-window`, `resize-window`, `minimize`, `maximize` and `restore`.

The unconditional success flags are removed with no compatibility alias: `clicked`, `moved`, `scrolled`, `hovered`, `dragged`, `resized`, `minimized`, `maximized`, `restored`, `closed`, `cleared`, the `requested` field of `close-app`, and the inner `ok` of `clipboard-set`. A flag that every successful call carried proved nothing about the operation.

A verified window report names the exact readback in `details.verification_scope` (`window_size`, `window_position`, `window_minimized_state` or `window_maximized_state`). Geometry scopes also state `details.tolerance` and `details.units`: 2 points on macOS, 8 px on Windows, 2 pixels on Linux. A window failure after the write keeps its code, details and disposition and gains `details.target` and the completed `details.steps`.

Each of these commands now returns the adapter's own report inside `data`:

```json
{
  "action": "drag",
  "disposition": { "delivery": "delivered_unverified", "retry": "unsafe" },
  "steps": [ { "label": "pointer_drag", "outcome": "succeeded", "mechanism": "physical_synthetic" } ],
  "target": { "kind": "drag", "from": { "x": 10, "y": 20 }, "to": { "x": 80, "y": 20 } },
  "from": { "x": 10, "y": 20 },
  "to": { "x": 80, "y": 20 }
}
```

- `disposition.delivery` is `delivered_verified` only after the adapter read back the postcondition it names, `delivered_unverified` after a completed dispatch without that proof, and `not_delivered` only when the adapter observed that the operation was already satisfied (for example `close-app` on an application that had already exited). `ok: true` means the command returned a delivered or already-satisfied result, not that the application effect was verified.
- `target.kind` is `window` (`id`, `title`, `process`), `app` (`name`, `process`), `pointer` (`point`), `drag` (`from`, `to`) or `clipboard`. The adapter supplies the identity it validated and used. A pointer target is the dispatch coordinate and does not prove which application received the input.
- Request context (`x`, `y`, `count`, `dx`, `dy`, `width`, `height`, drag endpoints, `drop_delay_ms`, `type`, `app`, `method`, and a confirmed `focused`) keeps its earlier meaning. These fields echo the request and are not observed post-state.
- Errors keep `error.disposition`. A failed `--wait-for` after a delivered operation keeps the operation's own delivery (verified, unverified, or not delivered) in `error.disposition` and carries the report under `details.after_action`.
- The Rust `ActionResult` and `OperationReport` readers reject a payload without an explicit `disposition`; a missing disposition no longer reads back as delivered-unverified.
- The C ABI is unchanged. Status-only functions such as `ad_close_app` and `ad_drag` still cannot expose the disposition, steps or target of a successful call.

## Headed action verification

- Headed ref interactions activate the exact source app/window before final pointer hit testing. Read-only readiness polling must not prevent a background target from reaching that focus phase; final hit testing remains mandatory before delivery.
- Focus and preparatory scrolling can change coordinates. Stability is verified against fresh live observations, not by requiring coordinates to remain equal to the original snapshot.
- On macOS, post-activation focus confirmation uses fresh system-wide `AXFocusedApplication` and exact-window evidence with process-generation checks. A cached `NSWorkspace` foreground snapshot is not sufficient in a process that just activated another app.
- Focusing an inactive text field can change its selection. A pre-focus range cannot establish the expected insertion into a nonempty field; such a result is explicitly unverified rather than falsely contradicted.
- A settle-loop pacing budget does not replace the caller's deadline for native reads. Tests assert deadline identity rather than arbitrary remaining-time thresholds.

## Find result shape

`find` match objects report the element's actual accessibility `name`. Elements
without an accessible name omit `name` entirely; their content stays in `value`.
Earlier output synthesized a display name, falling back to `value` or
`description`, then a `(unnamed <role>)` placeholder, so consumers that treated
`name` as always present must tolerate its absence and apply their own fallback
to `role` or `value`.

## Hyprland windows and focus

On Hyprland, `list-windows` also lists toplevels that no accessibility frame
represents, such as foot or Alacritty, or a window its application exposes no
frame for. Such a window carries
`"accessible": false` and an id of the form
`w-hypr-<compositor pid>-<compositor start>-<toplevel identifier>`, and it has
`bounds`, `visible` and `is_focused` but no `minimized`. It has no refs:
`snapshot --window-id` and `find --window-id` refuse it with `INVALID_ARGS`. Only the compositor instance
that issued the id accepts it. `list-windows --app` matches it by its class, command
name or executable.

`focus-window` binds the window to exactly one compositor toplevel before it
sends anything, and adds `data.activation` next to `data.focused`:

```json
{
  "focused": { "id": "w-hypr-658-344-18000006", "accessible": false, "is_focused": true },
  "activation": {
    "activated": true,
    "proof": "compositor_only",
    "side_effects": [
      { "kind": "workspace_switched", "monitor": "Virtual-1", "from": "1", "to": "3" },
      { "kind": "cursor_moved", "from": { "x": 322.0, "y": 413.0 }, "to": { "x": 957.0, "y": 604.0 } }
    ]
  }
}
```

- `activated` is `false` when the window already had focus and nothing was
  sent. Only a `--headed` command activates. A headless one proves existing
  focus or refuses.
- `proof` is `compositor_and_accessibility` for an AT-SPI window. That needs
  Hyprland's active window, the toplevel's `activated` state and the frame's
  ACTIVE state to agree, and a monitor to show the window's workspace with no
  special workspace in front of it. Hyprland keeps a window active after it
  moves to a hidden special workspace, so that window is not proven focused. It is `compositor_only` for an `accessible: false`
  window, which has no AT-SPI frame to read.
- `side_effects` lists every desktop change the activation caused, and is
  omitted when there is none. The kinds are `workspace_switched` (`monitor`,
  `from`, `to`), `special_workspace_revealed` and `special_workspace_closed`
  (`monitor`, `workspace`), and `cursor_moved` (`from`, `to`). Hyprland warps
  the cursor into the window it activates.

A refusal names its cause in `error.details.cause`:

| Cause | Code | Delivery | Meaning |
|-------|------|----------|---------|
| `window_not_focused` | `POLICY_DENIED` | `not_delivered` | A headless command found the window unfocused |
| `hidden_special_workspace` | `POLICY_DENIED` | `not_delivered` | A headless command found the window on a hidden special workspace; `details.workspace` names it |
| `special_workspace_open` | `ACTION_FAILED` | `not_delivered` | A special workspace is open over the window's monitor, and Hyprland keeps it in front; `details.workspace` and `details.monitor` name it |
| `ambiguous_window` | `ACTION_FAILED` | `not_delivered` | Several toplevels match the window; `details.candidates` lists each `address` and `workspace` |
| `window_not_mapped` | `ACTION_FAILED` | `not_delivered` | No toplevel matches the window |
| `activation_refused` | `ACTION_FAILED` | `delivered_unverified` | The activation was sent and Hyprland left another window active |
| `frame_not_active` | `ACTION_FAILED` | `delivered_unverified` when sent | Hyprland made the window active but its AT-SPI frame does not report ACTIVE |
| `window_hidden` | `ACTION_FAILED` | `delivered_unverified` when sent | Hyprland reports the window active, but no monitor shows its workspace |

A refusal after an activation carries `details.activation_sent: true`, the last
`compositor_active`, `toplevel_activated`, `in_view` and `frame_active` reads, the
`side_effects` already observed and, when the last read failed,
`last_read_error`. An id the fresh `list-windows` no longer reports is
`WINDOW_NOT_FOUND`: a closed toplevel, a restarted process (its windows get new
ids), or a compositor id from another compositor instance. A process that
changes between that listing and the bind is `STALE_REF`.

## Hyprland capture

On a bound Hyprland, `screenshot` uses the compositor's own capture protocols.
It never asks the desktop portal, so no dialog appears.

- A window target (`--window-id` or `--app`) binds to exactly one toplevel, the
  way `focus-window` does, and is captured through `ext_image_copy_capture_v1`.
  A window on a hidden special workspace, on a workspace out of view, or with
  no accessibility tree is captured where it is. Nothing is activated, shown
  or moved. The image is the window's size times its monitor's scale.
- A window in view is captured from one frame. In view means it is on the
  workspace its monitor shows, or on the special workspace open over that
  monitor, and no other window on its workspace is fullscreen or maximized.
  Hyprland asks such a window for every frame, so the first frame holds the
  newest drawing the window committed before the request, at most one drawing
  old. A window in view takes about 45 ms to capture.
- Any other window waits for a redraw. Hyprland stops asking a window out of
  view, or one behind a fullscreen or maximized window, for frames and draws
  it from the buffer it last drew, which can be seconds old. A window on a
  workspace under an open special workspace still draws, but it is not in
  view, so it waits too: its frame is correct, only slower. The capture takes
  one frame to make the window draw, then opens a new capture session every
  25 ms. It returns the first frame whose pixels differ from that first one;
  those pixels were drawn after the request. A window whose pixels stay the
  same for 10 sessions, at least 250 ms, returns that unchanged frame, which
  can be older than the request. A drawing that lands later is not waited
  for. An unchanged window out of view takes about 430 ms to capture, so a
  budget too short for the wait answers `TIMEOUT`.
- `--screen N`, or no selector for the primary display, captures the output
  `list-displays` names, through `zwlr_screencopy_v1`, at its full pixel size.
  `data.scale_factor` is the output's scale, fractional scales included.
  Outputs at negative origins work too.
- Each capture makes Hyprland send `screencast` events on its event socket.
- `permissions` reports `screen_recording` as `unknown`.

| Code | `details` | Meaning |
|------|-----------|---------|
| `PERM_DENIED` | `option: "ecosystem:enforce_permissions"`, `enforced: true` | Hyprland enforces permissions and would withhold the frame without asking. Nothing was requested |
| `TIMEOUT` | `kind: "capture_frame"` | The compositor sent no frame before the deadline, or the deadline ended the wait for a window's new drawing or for a refused buffer's retry. A compositor that does not answer at all gives `display_unavailable: "silent"` |
| `ACTION_FAILED` | `kind: "capture_refused"`, `reason: "buffer_constraints"` or `"unknown"` | The compositor refused every frame. A refused buffer is retried on new sessions for up to 2 s, while a moved window's size settles |
| `ACTION_FAILED` | none | The frame size does not match the display, or the compositor offered no buffer layout this tool reads |
| `WINDOW_NOT_FOUND` | none | The window closed before or during the capture, or the id belongs to another compositor instance |
| `ACTION_NOT_SUPPORTED` | none | The compositor does not advertise a protocol the capture needs; `platform_detail` names it |
| `INVALID_ARGS` | `display` when known | The display is gone, or changed during the capture |
| `PLATFORM_NOT_SUPPORTED` | `display_unavailable: "missing"` | The compositor closed its socket during the capture |

A window bind refusal has the same `details.cause` as for `focus-window`. Every
`screenshot` error before the image is written is `not_delivered` with
`retry: safe`, and no file is created.

## Error codes

| Code | Meaning |
|------|---------|
| `PERM_DENIED` | Accessibility permission not granted |
| `ELEMENT_NOT_FOUND` | No element matched the ref or query |
| `APP_NOT_FOUND` | Application not running or no windows; the lookup failed before anything was sent, so the disposition is `not_delivered` |
| `STALE_REF` | Ref could not be re-identified in the live UI |
| `AMBIGUOUS_TARGET` | Ref recovery matched multiple plausible targets |
| `SNAPSHOT_NOT_FOUND` | Snapshot ID is missing or expired |
| `POLICY_DENIED` | Physical/headed path blocked by policy |
| `ACTION_FAILED` | Action rejected, or its observed result contradicted the request |
| `ACTION_NOT_SUPPORTED` | The target does not expose the requested action |
| `APP_UNRESPONSIVE` | The matching application stopped responding |
| `PLATFORM_NOT_SUPPORTED` | Adapter method not implemented on this platform |
| `TIMEOUT` | Wait condition expired, or a live identity read stayed incomplete until the deadline |
| `INVALID_ARGS` | Invalid argument values |
| `WINDOW_NOT_FOUND` | The requested window does not exist, including a `screenshot --window-id` that matches no window; the disposition is `not_delivered` |
| `NOTIFICATION_NOT_FOUND` | The notification ID was not found |
| `INTERNAL` | Unexpected error or caught panic |

### Unreachable display on Linux

A Linux command that needs the display server and cannot reach it puts the
cause in `details.display_unavailable` and its retryability in
`details.retryable`:

| Cause | Code | Meaning |
|-------|------|---------|
| `not_configured` | `PLATFORM_NOT_SUPPORTED` | No `WAYLAND_DISPLAY` or `DISPLAY`, and no Hyprland instance was found under the runtime directory |
| `missing` | `PLATFORM_NOT_SUPPORTED` | The configured server or the bound Hyprland instance is not running |
| `refused` | `PLATFORM_NOT_SUPPORTED` | The server refused the connection |
| `auth_refused` | `PLATFORM_NOT_SUPPORTED` | The X server refused this client's authorization |
| `silent` | `TIMEOUT` (`retryable: true`) | The server, or a Hyprland IPC or Wayland socket, did not answer in time; `details.detail` names the socket when one is known |
| `hyprland_ambiguous` | `PLATFORM_NOT_SUPPORTED` | Several Hyprland instances answer and none is selected; `details.signatures` lists them |
| `hyprland_mismatch` | `PLATFORM_NOT_SUPPORTED` | The Hyprland instance and the Wayland socket belong to different compositors, or the bound compositor was replaced |

## Exit codes

`0` success, `1` structured error (JSON on stdout), `2` command-line parse error (the envelope is still printed, with `INVALID_ARGS`).
