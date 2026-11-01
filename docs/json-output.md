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

A verified window report names the exact readback in `details.verification_scope` (`window_size`, `window_position`, `window_minimized_state`, `window_maximized_state`, or `window_restored_state` on a bound Hyprland). Geometry scopes also state `details.tolerance` and `details.units`: 2 points on macOS, 8 px on Windows, 2 pixels on Linux. A window failure after the write keeps its code, details and disposition and gains `details.target` and the completed `details.steps`.

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

## Hyprland windows, focus and close

On a bound Hyprland, `list-windows` also lists toplevels that no accessibility
frame represents, such as foot or Alacritty, or a window its application
exposes no frame for:

```json
{ "id": "w-hypr-658-344-18000006", "title": "~", "app_name": "foot", "pid": 2210,
  "process_instance": "1f0c2a9e:51234", "bounds": { "x": 0, "y": 0, "width": 960, "height": 1080 },
  "is_focused": false, "visible": true, "accessible": false }
```

- The id is `w-hypr-<compositor pid>-<compositor start>-<toplevel identifier>`.
  Only the compositor process that issued it lists it, so after a compositor
  restart the old id is `WINDOW_NOT_FOUND`.
- The row has no `minimized` and no refs. `snapshot --window-id` and
  `find --window-id` refuse the id with `INVALID_ARGS`.
- An accessible window id resolves only to exactly one toplevel. With none,
  or with a changed process, it is `WINDOW_NOT_FOUND`; with several it is
  `AMBIGUOUS_TARGET`, and `error.details.candidates` lists each toplevel's
  `address` and `workspace`.

`focus-window` binds the window to exactly one compositor toplevel before it
sends anything, with the same codes as resolving an accessible window id
above. A `w-hypr-` id whose compositor or toplevel is gone is also
`WINDOW_NOT_FOUND`. On success it adds `data.activation` next to
`data.focused`:

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
  sent. Only a `--headed` command activates, through
  `zwlr_foreign_toplevel_handle_v1.activate`. A headless one proves existing
  focus or refuses.
- `proof` is `compositor_and_accessibility` for an AT-SPI window: Hyprland's
  active window, the toplevel's `activated` state and the frame's ACTIVE state
  agree, and a monitor shows the window's workspace with no special workspace
  in front of it. Hyprland keeps a window active after it moves to a hidden
  special workspace, so that window is not proven focused. `proof` is
  `compositor_only` for an `accessible: false` window, which has no frame.
- `side_effects` lists every desktop change the activation caused and is
  omitted when there is none. The kinds are `workspace_switched` (`monitor`,
  `from`, `to`), `special_workspace_revealed` and `special_workspace_closed`
  (`monitor`, `workspace`), and `cursor_moved` (`from`, `to`). Hyprland moves
  the cursor into the window it activates.

A refusal names its cause in `error.details.cause`:

| Cause | Code | Delivery | Meaning |
|-------|------|----------|---------|
| `window_not_focused` | `POLICY_DENIED` | `not_delivered` | A headless command found the window unfocused |
| `hidden_special_workspace` | `POLICY_DENIED` | `not_delivered` | A headless command found the window on a hidden special workspace; `details.workspace` names it |
| `special_workspace_open` | `ACTION_FAILED` | `not_delivered` | A special workspace is open over the window's monitor, and Hyprland keeps it in front; `details.workspace` and `details.monitor` name it |
| `activation_refused` | `ACTION_FAILED` | `delivered_unverified` | The activation was sent and Hyprland left another window active |
| `frame_not_active` | `ACTION_FAILED` | `delivered_unverified` when sent | Hyprland made the window active but its frame does not report ACTIVE |
| `window_hidden` | `ACTION_FAILED` | `delivered_unverified` when sent | Hyprland reports the window active, but no monitor shows its workspace |

A refusal after an activation carries `details.activation_sent: true`, the last
`compositor_active`, `toplevel_activated`, `in_view` and `frame_active` reads,
the `side_effects` already observed and, when the last read failed,
`last_read_error`. A compositor that does not advertise
`zwlr_foreign_toplevel_manager_v1`, `hyprland_toplevel_mapping_manager_v1` or
`wl_seat` is `PLATFORM_NOT_SUPPORTED`, and `platform_detail` names the missing
global.

On a bound Hyprland, `move-window`, `resize-window`, `minimize`, `maximize`
and `restore` bind the window the same way and prove each request by a fresh
read of Hyprland's client list. A success is `delivered_verified`, `restore`
included, with `details.verification_scope`, `details.observed` (`x`, `y`,
`width`, `height`, `floating`, `fullscreen` and `workspace`) and, when the
operation changed the desktop, `details.side_effects`:

```json
{
  "action": "minimize",
  "disposition": { "delivery": "delivered_verified", "retry": "unsafe" },
  "details": {
    "verification_scope": "window_minimized_state",
    "observed": { "x": 40, "y": 60, "width": 630, "height": 755, "floating": true,
                  "fullscreen": 0, "workspace": "special:agent-desktop-minimized-1" },
    "side_effects": [ { "kind": "active_window_changed", "from": "0x55d0a1", "to": "0x55d2f0" } ]
  }
}
```

`side_effects` holds `active_window_changed` (`from` and `to` are Hyprland
addresses, `null` for no focused window) and the `focus-window` kinds above.
A window already as asked is `not_delivered` with `details.observed`. Every
refusal below is `ACTION_NOT_SUPPORTED`, `not_delivered`, with
`details.physical_delivery_started: false`:

| Cause | Meaning |
|-------|---------|
| `tiled_window` | Move or resize of a tiled window |
| `window_maximized` | Move or resize of a maximized or fullscreen window |
| `window_minimized` | Move, resize or maximize of a minimized window |
| `window_fullscreen` | Maximize of a fullscreen window |
| `maximize_takes_focus` | Maximize of a window while another window has focus |
| `legacy_config_focused_only` | Maximize, or restore of a maximized window, under a legacy `hyprland.conf` |
| `workspace_name_unsupported` | Minimize of a window whose workspace name is not letters, digits, `-` and `_` |
| `config_dialect_unknown` | The compositor answers as neither a Lua nor a legacy config |

A request the compositor answers with an error is `ACTION_FAILED`, cause
`compositor_rejected`, with `details.reply`; it is `not_delivered` unless an
earlier request of the same operation was applied. A request whose read-back
does not hold within 1 s is `ACTION_FAILED`, `delivered_unverified`, cause
`readback_mismatch` with `details.requested` and `details.observed`, or
`readback_unavailable`.

`close-app` on a bound Hyprland sends `zwlr_foreign_toplevel_handle_v1.close`
to each toplevel of the process, after it checks that the process instance
still runs and still owns that toplevel. The `OperationReport` is
`delivered_verified` once the process has exited; a process that is still
running at the deadline is `TIMEOUT` with `delivered_unverified`. A toplevel
with a parent counts as a dialog, and so does a showing top-level frame of the
process with a dialog role in the accessibility tree, parent or not. A pending
save prompt is then `ACTION_FAILED`
with cause `dialog_pending` and its title in `details.dialog_titles`, and
nothing answers it. On any other Wayland compositor, graceful close stays
`ACTION_NOT_SUPPORTED` with cause `wayland_session` unless the app runs under
XWayland.

## Hyprland binding

On Hyprland, `list-displays` reads the Wayland socket of the one Hyprland
instance it binds, and `list-windows` reads that instance's clients. Both
commands, and every command that resolves a window id, report a refused
binding with `PLATFORM_NOT_SUPPORTED` and a cause in `error.details.display_unavailable`.
`hyprland_ambiguous` means several instances answer and none is named;
`details.signatures` lists them. `hyprland_mismatch` means the named instance
is not live, or it and the Wayland socket belong to different compositor
processes. A socket that accepts the connection and never answers is `TIMEOUT`
with cause `silent`, and `details.detail` names it.

## Hyprland capture

On a bound Hyprland, `screenshot` uses the compositor's own capture protocols.
The portal is never asked and no dialog appears; a refusal is the answer, never
a portal fallback.

- A window target (`--window-id` or `--app`) binds to exactly one toplevel, the
  way `focus-window` does, and is captured from that toplevel through
  `ext_image_copy_capture_v1`. A window on a hidden special workspace, on a
  workspace out of view, or with no accessibility tree is captured where it is.
  Nothing is activated, shown or moved. `data.scale_factor` is the frame's
  pixel width over the window's logical width.
- A window in view is answered with the first frame. In view means it is on the
  workspace its monitor shows, or on the special workspace open over that
  monitor, and no other window on its workspace is fullscreen or maximized.
  Hyprland asks such a window for every frame, so the first frame holds the
  newest drawing it committed before the request.
- Any other window waits for a redraw, because Hyprland draws it from the
  buffer it last drew, which can be seconds old. The capture takes one frame to
  make the window draw, then opens a new capture session every 25 ms and
  returns the first frame whose pixels differ from that first one. A window
  whose pixels stay the same for 10 sessions, at least 250 ms, returns that
  unchanged frame. A drawing that lands later is not waited for, and a budget
  too short for the wait answers `TIMEOUT`.
- `--screen N`, or no selector for the primary display, captures that output
  through `zwlr_screencopy_v1` at its full pixel size. `data.scale_factor` is
  the output's scale. Only a frame presented after the request counts, and an
  unchanged frame is valid.
- On any other Wayland compositor, a window target is `PLATFORM_NOT_SUPPORTED`
  and reaches neither the portal nor X11.

Every capture error is `not_delivered` with `retry: safe`, and no file is
written.

| Code | When | `details` |
|------|------|-----------|
| `PERM_DENIED` | `ecosystem:enforce_permissions` is on; nothing was requested | `option`, `enforced: true` |
| `TIMEOUT` | No frame arrived before the deadline | `kind: "capture_frame"` |
| `ACTION_FAILED` | The compositor refused every frame; a refused window buffer is retried on new sessions for up to 2 s while a moved window's size settles | `kind: "capture_refused"`, `reason: "buffer_constraints"` or `"unknown"` |
| `ACTION_FAILED` | The frame came from before the request, its size did not match the display, or no buffer layout was readable | |
| `WINDOW_NOT_FOUND` | The window matches no toplevel, its process was replaced, its toplevel closed, changed or had its address passed to another window before or during the capture, or its id came from another compositor | |
| `AMBIGUOUS_TARGET` | The window matches more than one toplevel | `candidates`, each with `address` and `workspace` |
| `ACTION_NOT_SUPPORTED` | The compositor lacks a global capture needs; `platform_detail` names it | |
| `INVALID_ARGS` | The display is gone or changed during the capture | `display` when the compositor no longer offers it |
| `PLATFORM_NOT_SUPPORTED` | The compositor closed the connection | `display_unavailable: "missing"` |

## Hyprland cursor

On a bound Hyprland, `cursor-overlay enable` and `session start --cursor`
return `data.rendered: true`; a renderer process draws the cursor on the
compositor's overlay layer. On any other Wayland compositor they return
`data.rendered: false` and nothing is drawn.

`cursor-overlay disable` and `session end` report how the session's renderer
stopped in `data.details.cursor_renderer`. `session gc` reports
`data.details.cursor_renderer` as an object from each removed session id to
its value.

| Value | Meaning |
|-------|---------|
| `stopped` | The renderer exited when asked |
| `terminated` | It ignored the request; a signal stopped it after its process identity was checked, and its exit is confirmed |
| `not_running` | No renderer was running for the session |
| `stale` | The record named a process that is gone or replaced; nothing was signalled and the record was removed |
| `incompatible` | A renderer from another agent-desktop build ignored the request; it was not signalled and keeps its files |

A refusal is `ACTION_FAILED` with `details.kind`, except where the table says
otherwise:

| `details.kind` | When |
|----------------|------|
| `cursor_renderer_incompatible` | Another build's renderer serves the session; `details.pid` names it |
| `cursor_socket_path_too_long` | The renderer socket path exceeds the Unix limit; `details.length` and `details.limit` (107) |
| `cursor_renderer_exit_unconfirmed` | `disable` signalled the renderer and its exit was not confirmed |
| `cursor_renderer_gone` | The renderer exited before it answered |
| `cursor_renderer_draw_failed` | The compositor connection failed while drawing |
| `cursor_renderer_session_mismatch` | The control names another session than the renderer serves |
| `cursor_holders_not_supported` | A request to hide the cursor for a capture; the renderer does not hide it |
| `cursor_renderer_bad_request` | The renderer could not read the request |
| `cursor_renderer_start_timeout` | `TIMEOUT`: the renderer did not start listening in time |

An `XDG_RUNTIME_DIR` that is not absolute is `PLATFORM_NOT_SUPPORTED`.

## Hyprland pointer

On a bound Hyprland, `--headed` pointer commands send through a
`zwlr_virtual_pointer_v1` bound to the output that holds the point.
Coordinates are global logical layout points.

A coordinate command (`mouse-move`, `mouse-click`, `mouse-wheel`, `drag`,
`hover --xy`) proves nothing about the receiver. Its `OperationReport` adds two
fields to `data.details`:

| Field | Meaning |
|-------|---------|
| `receiver_proof` | Always `false` |
| `delivered_events` | Events sent: motion, button and wheel |

A ref command (`click`, `double-click`, `triple-click`, `scroll`) reads the
compositor's window stack and layers before each burst. Every layer counts as a
cover, whatever its namespace. A refusal before any event is `ACTION_FAILED`,
`not_delivered`, with `details.kind: "receiver_evidence"`,
`details.check: "hyprland_stack"` and `physical_delivery_started: false`:

| Cause | Meaning | Extra `details` |
|-------|---------|-----------------|
| `evidence_missing` | A compositor read failed or was incomplete | `evidence` names the read |
| `target_not_shown` | The window is not on a workspace an output shows | |
| `point_outside_target` | The aim point lies outside the window | |
| `layer_cover` | A `top` or `overlay` layer holds the point | `layer_namespace`, `layer_level` |
| `window_cover` | A floating, pinned or fullscreen window holds the point | `cover_address` |

Other refusals before any event carry `details.kind: "hyprland_pointer"` and
`not_delivered`:

| Cause | Code | Meaning |
|-------|------|---------|
| `point_off_output` | `INVALID_ARGS` | The point lies on no output |
| `hover_unsupported` | `ACTION_NOT_SUPPORTED` | `hover` by ref has no receiver evidence |
| `modifiers_unsupported` | `ACTION_NOT_SUPPORTED` | Held modifiers were asked for |
| `unsupported_step` | `ACTION_NOT_SUPPORTED` | The plan holds a button or key the virtual pointer does not send |

`drag` by ref refuses before any event, because Wayland gives a ref no global
bounds.

The active window is read before every button press and wheel step. A change
stops delivery with `ACTION_FAILED`, `details.kind: "focus_gate"`,
`cause: "focus_changed"`, `from` and `to`. SIGINT, SIGTERM or SIGHUP stop
delivery with `details.kind: "hyprland_pointer"`, `cause: "cancelled"`. A stop
before the first event is `not_delivered`. A stop after it is
`delivery_uncertain` with `retry: unsafe` and these fields:

| Field | Meaning |
|-------|---------|
| `physical_delivery_started` | `true` |
| `delivered_events` | Events sent before the stop |
| `buttons_released` | `true` when every held button was released; `false` when the compositor went away first |

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

## Exit codes

`0` success, `1` structured error (JSON on stdout), `2` command-line parse error (the envelope is still printed, with `INVALID_ARGS`).
