# Menus, Shell, Tray and Notifications on Linux

Run inside the target desktop session. `agent-desktop status` lists snapshot
capabilities, not proof that a popup is open or a tray watcher is running.
Openbox is an X11 window manager without GNOME Shell; Mutter here means a
GNOME Shell session on X11. GNOME Shell also supplies the Wayland surfaces.

## App menus and GTK4 popovers

`window`, `sheet` and `menu` are available with a healthy accessibility bus on
all three backends. After an opener, read its `data.surfaces`: a `type: menu`
entry means snapshot the menu next. If it is absent, wait for the menu, then
run `list-surfaces --app gnome-calculator`; GTK4 popovers appear asynchronously.
An incomplete scan marks `menu` in the window entry's `unclassified` list;
an absent menu in that result does not prove it is closed. A definite action
refusal or a successful action already satisfied without delivery does not
start the popup-settle wait.

This example needs a running Calculator and `jq`. It discovers a fresh opener
ref. On Mutter X11, first close Overview and focus Calculator using the recovery
below: a click can report success without opening its menu while Overview is
covering the app. Inspect each command's JSON result, including the waits.

```bash
menu_ref=$(agent-desktop snapshot --app gnome-calculator -i | jq -r '.. | objects | select(.role? == "switch" and .name? == "Main Menu") | .ref_id // empty')
test -n "$menu_ref" && agent-desktop click "$menu_ref"
agent-desktop wait --menu --app gnome-calculator --timeout 5000
agent-desktop list-surfaces --app gnome-calculator
agent-desktop snapshot --surface menu --app gnome-calculator -i
agent-desktop press escape
agent-desktop wait --menu-closed --app gnome-calculator --timeout 5000
```

`list-surfaces` lists the app's windows, sheets and open menu, with
`item_count` on the menu. `snapshot --surface menu` roots at the popup, and
`find --surface menu` uses the same scope. Menu refs identify the actual AT-SPI
object within a freshly verified, complete popup context, including its labels
and tree shape. A named GTK4 popover item such as Calculator's Preferences can
be found and clicked through that identity even without usable screen bounds.
An unreadable context or multiple matching targets is refused before delivery.
After closure the same ref returns `STALE_REF`, `details.kind: menu_closed`. A
snapshot of a closed menu returns `ELEMENT_NOT_FOUND`, “The window does not present an
open menu”. Open again and take new refs.

An open popup whose content changed returns `menu_context_changed`; an incomplete
context read returns retryable `menu_context_unknown`. Keep it open and take a
fresh menu snapshot before using a new ref.

The toolkit comparison below was measured on private Openbox X11; Calculator's
GTK4 popover was also read on Mutter X11 and Wayland.

| Toolkit | Exposed menu shape and boundary |
|---------|--------------------------------|
| GTK3 | Menu-bar child or separate context-menu window |
| GTK4 / libadwaita | Popover within the frame; item names can be recovered from their labels |
| Qt | `QMenu` popup; a `QLineEdit` context menu may expose nothing through AT-SPI |
| Chromium | Native context/toolbar popup in a separate frame; web content is not a general menu-surface API |

A visible menu bar alone is not an open popup. `wait --menu` and
`wait --menu-closed` use the same detector as menu snapshots; scope them with
`--app`. `press escape` sends a global key without first activating the parent
window. Read `wait --menu-closed` before assuming closure: in the headless
Wayland session, one run reported `delivered_unverified` but the menu remained
open and that wait returned `TIMEOUT`, `kind: wait_timeout`, `predicate: menu`,
“Menu did not close before the deadline”. The isolated recheck closed it; do
not treat the key's success as a closure guarantee.

In the measured headless Wayland setup, global `press` can leave the seat without
a keyboard after its temporary RemoteDesktop session ends. Calculator's GTK4
opener can then become expanded without presenting a native menu; a direct AT-SPI
action has the same outcome. The tool reports `ACTION_FAILED`, `popup_not_observed`,
with unverified delivery and unsafe retry. A keyboard present from app launch and
retained across the commands avoids this measured limitation; adding one after
the failure did not recover the popup. An expanded opener alone is never proof
of an open menu; inspect the native menu before continuing.

GTK4 raw menu items can be unnamed while the snapshot shows their labels.
A readable label does not establish a safe physical click position. Ordinary app pointer actions on Wayland
refuse with `ACTION_NOT_SUPPORTED`, cause `wayland_session`. On X11, only use
physical input after observing a usable popup position and verify its effect.
A successful input report alone does not prove an item was picked.

GTK4 list-backed dropdowns, such as Settings' Night Light Schedule, are not
menu-role popups. Their choices stay in the window tree: `wait --menu` and
`snapshot --surface menu` do not cover them. Find the combobox, click its fresh
ref, and use `wait --text` for a choice before taking a window snapshot. If
`select` refuses, adding `--headed` does not supply a missing semantic action.
On X11, after a headed click focuses the open dropdown, explicit headed
`press down` / `press up` and `press return` can choose an item. Inspect the
current choice first and verify the resulting value before another mutation.
If `get` returns only the combobox label, use `snapshot --root` on its fresh
ref to read the displayed value. A successful key report does not prove the
choice was committed.

## GNOME Shell: Overview, top bar and popups

Shell identity comes from the session-bus owner of `org.gnome.Shell`, correlated
with its accessibility-bus process. Process names and translated labels cannot
prove it. Advertised surfaces also require a resolvable structural container.

| Surface | Meaning | Availability |
|---------|---------|--------------|
| `system-tray` | Logical watcher items and their virtual menus | Headless snapshot with a StatusNotifierWatcher; no native panel binding |
| `action-center` | The container above the footer row of a label, a toggle and a push button | Content while the list is showing; `open-system-surface` opens it with a click on the date menu |
| `quick-settings` | System menu | Refuses: the exposed generic panel chain cannot prove its container |
| `taskbar` | Dock | Not advertised or openable by this build |

Shell snapshots need no `--app`; `list-surfaces` inventories an app selected
with `--app`. Shell containers do not appear in `list-windows`. The message
list and the date menu that opens it are found by role shape and uniqueness;
English labels, wrapper depth and screen position prove nothing. Other
containers and the system-button opener return `ACTION_NOT_SUPPORTED`,
`shell_container_unproven`, before input. Opening a hidden popup headless still
requires headed permission.

Overview is not a `--surface` value (`INVALID_ARGS`). On Mutter X11, while
Activities Overview is open, headed app focus returns `ACTION_FAILED`, cause
`overview_open`. Send global `agent-desktop press escape`, then
`agent-desktop --headed focus-window --app gnome-calculator` before the app
action. This X11 recovery does not establish Wayland window-control support.

Native shell clicks need headed permission and an aim point inside the target's
own extents on a display; the shell answers no point query finer than a
container, so no sibling proof exists for a shell node, and each effect is read
back from the shell. On Wayland the shared hit test cannot prove the receiver, so shell
ref clicks refuse before opening an input session. Wayland keyboard input
requires the session-bus `org.gnome.Mutter.RemoteDesktop` capability; an
unsupported backend returns `PLATFORM_NOT_SUPPORTED` before any event.

## Service requirements and exact refusals

The shell accessibility tree, notification daemon and tray watcher are separate
requirements. A healthy app bus does not establish any of them.

| Missing requirement | Result |
|---------------------|--------|
| Shell session owner or matching accessibility tree | Cause-specific missing-tree refusal; no guessed shell identity |
| Unproven shell container or opener | `ACTION_NOT_SUPPORTED`, `shell_container_unproven` |
| No notification service | `PLATFORM_NOT_SUPPORTED`, `daemon: absent` |
| Notification service owner is neither the shell nor a process of the same user whose argv names the shell's own notification module | `PLATFORM_NOT_SUPPORTED`, `daemon: unsupported`; server branding is not identity |
| No `org.kde.StatusNotifierWatcher` | `PLATFORM_NOT_SUPPORTED`, `watcher: absent`; guidance names the required protocol backend |
| Unreadable notification service or watcher | `TIMEOUT`, corresponding `daemon` or `watcher` detail `unreadable` |

An unavailable daemon is not an empty notification list; an absent watcher is
not an empty tray. Do not restart a shell or accessibility bus you do not own.

## Notifications

**Every notification field comes from the shell's widget layout.** Neither the
notification protocol nor GNOME's accessibility tree names a notification's
app, title, body, age or controls, so the adapter reads them from where the
shell puts them. This was verified on GNOME Shell 46.0 (Ubuntu 24.04) in
private Mutter X11 and native Wayland sessions, and on no other version. A
shell that lays an entry out differently is refused (`INTERNAL`, "no readable
title") or misread; it is never repaired by guessing. No text, no English word
and no position of a button in the footer is used to find anything.

`list-notifications` reads GNOME Shell's message list headless, including when
closed. Banner copies are excluded. Indices are 1-based positions in the
unfiltered list, newest first; filtering does not renumber them. `--app` is
case-insensitive contains; `--text` searches app, title and body. Empty is a
successful empty list. `wait --notification` establishes a baseline and waits
for a matching entry new to that baseline.

How an entry is read:

- The list is the container two levels above a footer row of a label, a toggle
  and a push button. An empty list is that container with no entry in it.
- The header is the container that holds an entry's unnamed push buttons. Its
  first label is the app; later labels are the age and are dropped.
- Labels outside the header are the title, then the body.
- Named push buttons outside the header are the actions.
- An entry with no such header cannot be read, and the command fails.
- The notification owner must be the shell or its own helper process; any other
  owner is `PLATFORM_NOT_SUPPORTED`.

The protocol has no history query and GNOME exposes no notification ID, so an
index is a position and not an identity. Within one command an entry is its
accessibility object plus its words: every mutation reads the list twice, checks
the expected app and title at the index, and checks that both reads found the
same object. It then proves the effect by the object leaving the list.

This example assumes exactly one disposable notification from `Skill Example`,
titled `Beta`, already exists. `jq` selects the actual index and the mutation guards
both identity fields:

```bash
agent-desktop list-notifications
notice_index=$(agent-desktop list-notifications --app "Skill Example" --text Beta | jq -r '.data.notifications[] | select(.app_name == "Skill Example" and .title == "Beta") | .index')
test -n "$notice_index" && agent-desktop --headed dismiss-notification "$notice_index" --expected-app "Skill Example" --expected-title Beta
agent-desktop list-notifications --app "Skill Example"
```

Single-item `dismiss-notification` and `notification-action` require
`--headed` and at least one of `--expected-app` / `--expected-title`.
Missing identity is `INVALID_ARGS`; headless mutation is `POLICY_DENIED`.
Missing index or changed identity is `NOTIFICATION_NOT_FOUND`. Re-list after
each mutation; an index can now name another notification.

Mutations press the shell's controls with the pointer, which needs X11: on
Wayland a mutation fails with `ACTION_FAILED`, `pointer_hit_unproven`, before
any input. The command opens the message list by a click on the clock menu and
closes it again. If it cannot open the list, the error suggestion says so: open
the list by hand and retry, because a list that is already showing is used as it
is.

Dismissal presses the entry's header buttons, the last one first. The two
buttons look alike, so which one closes the entry is told by its effect: after a
press the entry's own object must be gone from the list and no other entry's
may be. A button that only expands the entry leaves it standing, so the other
button is pressed next. An entry that still stands after both is
`ACTION_FAILED` with `delivery: delivered_unverified`, never success. A partial
post-click read gives the same code with “The click was delivered but the
message list could not be read afterwards” and `retry: unsafe`; the target may
already be gone. Re-list before deciding what remains; do not repeat the old
index blindly.

`dismiss-all-notifications` closes each entry in scope through its own header
buttons, one at a time, and counts an entry as dismissed only when its own
object has left the list. It never presses the footer button, which has no
property but its place. `--app` limits the scope. A headed empty dismiss-all
returns zero without opening the message list. Headless dismiss-all is
`POLICY_DENIED`, even with an empty list. Entries that could not be closed are
listed in the result's failures.

For `notification-action`, pass the exact name in the row's `actions` list,
for example `Open`. A missing action is `ACTION_NOT_SUPPORTED` with
`available_actions`. The command expands the entry first when its buttons are
not laid out, using the header button before the last one; if pressing it
removes the entry instead, the result is `ACTION_FAILED`. An action counts only
when the entry's object leaves the list, which GNOME does after an action unless
the notification is resident. `details.evidence` is `action_invoked` when
the daemon's own `ActionInvoked` signal was also seen once, and `entry_removed`
otherwise; the result is always `delivered_unverified`, and the receiving app's
effect is never claimed. A button that was pressed while the entry stayed in the
list is `ACTION_FAILED` with `delivered_unverified`.

## Surfaces that `open-system-surface` accepts

`open-system-surface --help` shows the values of every platform. On Linux only
three can work:

| Value | Result |
|-------|--------|
| `system-tray` | Needs `org.kde.StatusNotifierWatcher`. Without it: `PLATFORM_NOT_SUPPORTED`, `watcher: absent`, the same refusal as `snapshot --surface system-tray`. No window is returned |
| `action-center` | Opens the GNOME message list when its container and opener are proven; otherwise `ACTION_NOT_SUPPORTED`, `shell_container_unproven` |
| `quick-settings` | Opens the GNOME system menu with the same proof rule |
| `taskbar` | `PLATFORM_NOT_SUPPORTED`: no stock shell shows a dock |
| `start-menu`, `system-tray-overflow`, other values | `PLATFORM_NOT_SUPPORTED`, not supported on this platform |

Retrying an unsupported value gives the same refusal.

## StatusNotifierItem and AppIndicator tray

Tray snapshots require `org.kde.StatusNotifierWatcher`. The adapter reads its
registered items and `org.kde.StatusNotifierItem` properties; arbitrary XEmbed
icons are not SNI items. Install or enable a backend only for the session you
own; GNOME commonly obtains one through AppIndicator.

`snapshot --surface system-tray -i` exposes watcher items beneath a logical
root, with `com.canonical.dbusmenu` entries as virtual children. Titles,
including `System` and duplicate titles, never bind a native panel node. A
`Passive` item remains in the snapshot with `states: [hidden]`.

These refs are virtual protocol identities, not AT-SPI elements. Native text
getters can return `INVALID_ARGS`, `invalid_native_handle`. Bounds, placement
and offscreen state are unknown; no paired native item supplies geometry.

Headless `click` uses advertised `Activate`; `right-click` uses `ContextMenu`.
`SecondaryActivate` and physical panel clicks never substitute for a missing
primary method. Use a virtual menu entry when the item omits those methods.
Headed permission supplies neither native geometry nor a pointer fallback;
Wayland headed pointer policy can refuse earlier with `wayland_session`.
SNI methods and leaf picks report `delivered_unverified`; independently observe
the receiving application before deciding whether to repeat them.

A menu-entry `click` re-resolves the label path and entry ID in a fresh layout;
a changed revision alone does not invalidate an otherwise identical entry.
An enabled leaf sends `Event(..., clicked, ...)`; a submenu uses `AboutToShow`
and rereads for children. Disabled entries are not clicked.
Disabled entries refuse before delivery; the actionability gate may report
`TIMEOUT`, `kind: actionability_timeout`, with a failed `enabled` check and
reason “live enabled state is false” in `last_report.report.checks`.
`--headed` does not make them enabled; on Wayland its earlier pointer gate can
return `ACTION_NOT_SUPPORTED`, cause `wayland_session`, instead of waiting.
Neither route sent the disabled entry's event. Owner exit makes old refs `STALE_REF`.
Tray items accept click/right-click; menu entries accept click. Other actions
can be refused by policy first or `ACTION_NOT_SUPPORTED` in the adapter.

## Sandboxed apps

An app in a sandbox is on the accessibility bus only when the sandbox shares it.

- **Flatpak:** the sandbox needs `--talk-name=org.a11y.Bus`.
- **snap:** there is no accessibility interface or plug to grant (`snap interface a11y`
  does not exist). Connect `desktop-legacy` or `unity7`.
- The adapter's own error for a missing or hidden bus names both fixes. Neither has
  been measured against a sandboxed app yet (the Flatpak override and the snap
  disconnect tests are open), so an empty `list-apps` for a sandboxed app is a
  reason to check these first.
