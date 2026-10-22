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
ref, and use `wait --text` for a choice before taking a window snapshot. A
combobox with no popup state, no selection and no action, such as a libadwaita
combo row, takes `--headed select @ref "Choice"` on X11: it clicks the
combobox, finds the choice in the list that appears, selects it through that
list and the popup's default action, and checks the value the combobox shows.
A choice that is not there closes the popup with Escape and answers
`ELEMENT_NOT_FOUND`. Without `--headed` that select is `POLICY_DENIED`, kind
`pointer_required`, unless the combobox already shows the choice. For a dropdown where `select` still
refuses, adding `--headed` does not supply a missing semantic action.
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
| `action-center` | A unique `LIST` ancestor of `NOTIFICATION` rows | Existing content only when this role chain proves the container |
| `quick-settings` | System menu | Refuses: the exposed generic panel chain cannot prove its container |
| `taskbar` | Dock | Not advertised or openable by this build |

Shell snapshots need no `--app`; `list-surfaces` inventories an app selected
with `--app`. Shell containers do not appear in `list-windows`. Empty history,
English labels, wrapper depth and screen position do not prove a message-list
container. Unproven containers and date-menu/system-button openers return
`ACTION_NOT_SUPPORTED`, `shell_container_unproven`, before input. Opening a
hidden popup headless still requires headed permission; that permission does
not establish a missing opener identity.

Overview is not a `--surface` value (`INVALID_ARGS`). On Mutter X11, while
Activities Overview is open, headed app focus returns `ACTION_FAILED`, cause
`overview_open`. Send global `agent-desktop press escape`, then
`agent-desktop --headed focus-window --app gnome-calculator` before the app
action. This X11 recovery does not establish Wayland window-control support.

Native shell clicks require the same structural receiver and sibling-separation
proof as app clicks before any pointer event. Bounds alone cannot authorize
input. Overlap, unreadable ancestry or an unknown receiver refuses without
delivery. On Wayland the shared hit test cannot prove the receiver, so shell
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
| Notification service owner differs from the detected shell owner | `PLATFORM_NOT_SUPPORTED`, `daemon: unsupported`; server branding is not identity |
| No `org.kde.StatusNotifierWatcher` | `PLATFORM_NOT_SUPPORTED`, `watcher: absent`; guidance names the required protocol backend |
| Unreadable notification service or watcher | `TIMEOUT`, corresponding `daemon` or `watcher` detail `unreadable` |

An unavailable daemon is not an empty notification list; an absent watcher is
not an empty tray. Do not restart a shell or accessibility bus you do not own.

## Notifications

After policy checks, notification commands check the detected backend. A service
with the same process owner as the detected shell answers `ACTION_NOT_SUPPORTED`, `details.kind:
notification_identity_unavailable`, `retryable: false`, with `not_delivered`
and safe retry. The refusal happens before reading or opening the message list
and before any input. `wait --notification` returns the same refusal immediately.
Missing, unsupported or unreadable services retain the backend errors above.
A separately owned GNOME notification process also retains the ownership
refusal; its branding cannot establish delegation to the detected shell.

The standard `org.freedesktop.Notifications` protocol supplies an ID in the
`Notify` reply and app name, summary, body and action keys in the call, but no
history query. GNOME's AT-SPI notification nodes expose neither that ID nor
relations identifying app, summary and body. Label position and English time
phrases cannot establish those fields. Anonymous buttons and footer position
cannot establish expand, close or clear-all semantics. No notification list,
empty history, target identity or successful delivery is inferred from them.
A monitor started now cannot recover notifications sent earlier.

Single-item mutations still require `--headed` and at least one of
`--expected-app` / `--expected-title`. Missing identity is `INVALID_ARGS`;
headless mutation is `POLICY_DENIED`. Supplying these flags does not bypass the
identity refusal. Ordinary application controls with supported AT-SPI actions
remain available through the ref-based observe-act loop.

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
