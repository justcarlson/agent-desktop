---
title: A matching window rectangle can be mid-animation
date: 2026-10-20
category: runtime-errors
module: crates/linux/src/system/window/window_op.rs, crates/linux/src/system/window/window_op_hold.rs, crates/linux/src/system/window/window_op_shadow.rs
problem_type: runtime_error
component: tooling
symptoms:
  - "A move or resize reports success but the next listing shows a different rectangle."
  - "A GTK window lands one shadow-width away from the requested position."
  - "A clamped resize error reports geometry from before its position was restored."
root_cause: async_timing
resolution_type: code_fix
severity: high
tags: [linux, x11, mutter, geometry, read-back, animation, gtk, delivery]
---

# A matching window rectangle can be mid-animation

## Problem

A window manager can pass through the requested rectangle while it is still moving or resizing the window. Returning success on that read records a transient frame as the final result. Mutter slides windows; GTK can briefly accept a requested size before clamping it (ledger L31-60).

## Symptoms

- A move reads at the requested position once, then continues past it.
- A GTK4 resize to 5000x5000 briefly matches, then settles at 1280x800 on a 1280x800 display; GTK3 can actually retain the larger size.
- A shadowed window's move and listing disagree because one uses the raw X11 client and the other uses visible bounds (L31-67).

## What Didn't Work

One matching read proves only that the rectangle matched at that instant. Two immediate reads do not establish that it held, and shortening the confirmation sleep to the remaining deadline recreates that mistake. Comparing raw client geometry against a shadow-free request also produces a false verdict even after motion stops.

## Solution

1. Use the `list-windows` rectangle throughout the public window surface: the X11 client minus `_GTK_FRAME_EXTENTS` shadow. For shadow `(left, top, right, bottom)`, visible bounds are `(client.x + left, client.y + top, client.width - left - right, client.height - top - bottom)`. `window_op_shadow.rs` converts move/resize requests back to client coordinates and converts error `requested` / `observed` rectangles back to visible coordinates. Do not substitute potentially stale AT-SPI sizes for the X11 client size.
2. In `window_op.rs::settle`, poll within the operation deadline capped by the 250 ms settle budget. A satisfied move/resize is provisional: require enough remaining budget for the full 120 ms confirmation pause, sleep that full interval, then read and judge again. If it no longer satisfies the request, keep polling while budget remains. Without time for the full pause, return an unverified mismatch even if the first read matched.
3. Preserve a resize's previous position when the resized client fits a display. `window_op_hold.rs::move_client` accepts a client already at the place only after `confirm_rectangle` has slept the full pause and found the client rectangle unchanged; merely being at the old position during a slide is insufficient. `holds_place` skips the restore for a client larger than every display, which cannot be judged by a position the manager cannot hold. Maximize remembers the ordinary rectangle (`remember_place`) so restore can give its position back.
4. If a clamped resize also shifted the client, restore its position and re-read before stamping the error's `observed` rectangle. An unsatisfied request carries `readback_mismatch`, the requested and latest observed visible bounds, display bounds, and `delivered_unverified`. An unavailable read-back carries `readback_unavailable`; it cannot prove the request was ignored.

## Why This Works

The verdict is about a requested state that survives a measured interval, not a successful request or a convenient animation frame. A single coordinate convention makes the write, independent observation and error evidence comparable. The pause is a bounded confirmation after a matching observation, not a blind delay used instead of checking the result.

## Prevention

- Preserve both controls in `crates/linux/src/system/window/window_op_settle_tests.rs`: a move seen only in passing must fail, and a matching move without time for the full pause must remain unverified. A late confirmation must report its latest rectangle.
- Keep shadow conversion and clamped-error coverage in `crates/linux/src/system/window/window_op_shadow_world_tests.rs`.
- Verify under openbox and private mutter, with GTK3 and GTK4, using independent X11 geometry and the shadow conversion. A manager without client-side shadows cannot exercise the conversion defect.

## Related Issues

- [Make permissioned fixture and real-app checks the adapter gate](../best-practices/real-app-tests-are-the-platform-adapter-gate.md)
- [A fixed sleep is not a synchronization primitive](../best-practices/a-fixed-sleep-is-not-a-synchronization-primitive.md)
