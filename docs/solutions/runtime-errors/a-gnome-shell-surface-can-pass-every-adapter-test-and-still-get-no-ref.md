---
title: A GNOME Shell surface can pass every adapter test and still get no ref
date: 2026-10-19
category: runtime-errors
module: crates/linux/src/system/tray, crates/linux/src/system/shell, crates/linux/src/notifications, scripts/notification-oracle.sh
problem_type: runtime_error
component: gnome-shell-surfaces
symptoms:
  - "snapshot --surface system-tray reports a ref_count equal to the number of shell buttons, and no tray item or menu entry has a ref."
  - "A shell node looked up through its panel's direct children is not found, so it reads as offscreen and a click fails with 'visible (live offscreen state is true)'."
  - "gdbus monitor --dest org.freedesktop.Notifications prints no signal while a notification is clicked."
  - "snapshot --surface menu answers WINDOW_NOT_FOUND after a headed click opened a tray item's popup."
root_cause: wrong_assumption
resolution_type: code_fix
severity: high
tags: [linux, gnome-shell, status-notifier-item, dbusmenu, notifications, at-spi, refs, d-bus]
---

# A GNOME Shell surface can pass every adapter test and still get no ref

## Problem

The shell surfaces (tray, notifications, quick settings) are built from virtual nodes and shell widgets, and four of their properties differ from an application's. Each one broke the end-to-end path while the adapter's own tests stayed green (`probes/linux/FINDINGS.md` L32-34, L32-35, L32-28, L32-37).

## What was true

1. **Core drops an entry whose identifier kind is `Unknown`.** `validate_ref_entry` rejects it and allocation returns `SkippedInvalidEntry`, with no error. A platform node that has no native id must carry a typed kind. The tray identifiers use `RuntimeId` (`crates/linux/src/system/tray/node_evidence.rs:20-28`), which passes the gate at `crates/core/src/refs_validate.rs:38-45`. The adapter's live tests built their `RefEntry` by hand and never met the gate; only the CLI's own `ref_count` showed it.
2. **Shell landmarks sit several levels below the container you expect.** The tray panel is two levels above the `System` menu button, the message list container two above `Clear all notifications`, and the system menu popup six above `Power Off Menu` (L32-22). The direct children of the tray panel are nameless groups and the item buttons are `menu` nodes under them, so a lookup by direct child found nothing and every item read as offscreen and unbound (L32-35). The tray no longer binds items to panel nodes: a tray node carries no bounds of its own and an unknown offscreen state (`crates/linux/src/system/tray/live.rs:23-61`, pinned by `a_tray_node_projects_no_bounds_of_its_own_and_none_are_invented` in `crates/linux/src/system/tray/fit_tests.rs`).
3. **The shell emits notification signals from its own unique name.** A monitor filtered on the destination `org.freedesktop.Notifications` sees none of them. Match on the interface instead, as `scripts/notification-oracle.sh` does with `dbus-monitor`. The daemon also keeps no replay: a rule added after the click receives nothing (L32-15).
4. **The tray popup is not an application menu surface.** A headed click on an item opens the shell's popup, but the shell tree has no popup node and `list-surfaces` lists none. Its entries are virtual children of the item, read through dbusmenu and picked with `Event(id, "clicked")` (L32-36, L32-37). Do not wait for a `menu` surface after a tray click.

## Prevention

- Check a new virtual node against core's gates by reading `ref_count` and `ref_id` from the CLI's JSON, not by building the entry in a test.
- Measure a lookup's depth against the real shell tree before you rely on direct children.
- Pick the D-Bus oracle by the sender's unique name, and subscribe before the action.
- Read a shell popup through its data source, never through a surface the shell does not expose.
