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

| Surface | Meaning | How to use it |
|---------|---------|---------------|
| `system-tray` | Top bar plus paired SNI items and their virtual menus | Snapshot headless; watcher required even when there are no items |
| `quick-settings` | System menu | Guarded headed open; an unproven receiver refuses. Snapshot reads existing menu content |
| `action-center` | Message list | Guarded headed open; an unproven receiver refuses. Snapshot reads existing message-list content |
| `taskbar` | Dock | Not advertised or openable by this build, even with a dock extension |

```bash
agent-desktop status
agent-desktop snapshot --surface quick-settings -i
agent-desktop snapshot --surface action-center -i
agent-desktop press escape
```

These snapshots need no `--app`. `list-surfaces` inventories the app selected
with `--app`. The shell is not an ordinary app window:
`list-windows --app gnome-shell` does not enumerate these containers.
`open-system-surface` returns the container's window identity only after the
popup is observed. Opening requires structural receiver proof; an unproven
receiver refuses before input, so open the popup manually when needed. These
snapshots inspect existing content and can refuse when the popup is closed.
Notification mutations close a popup they opened, and preserve one already open.

Opening a hidden popup headless returns `POLICY_DENIED`. An already open popup
can be adopted without another click. Shell ref clicks require `--headed`.
Close one popup with `press escape` before opening the other. Inspect any
`physical_delivery_started` / delivery disposition before retrying a failed
opener.

Overview is not a `--surface` value (`INVALID_ARGS`). On Mutter X11, while
Activities Overview is open, headed app focus returns `ACTION_FAILED`, cause
`overview_open`. Send global
`agent-desktop press escape`, then
`agent-desktop --headed focus-window --app gnome-calculator` before the app
action. Native Wayland app window control has its own refusal described in
the input reference; this X11 focus recovery is not a Wayland focus promise.
The shell's top bar uses a separate input route.

Shell clicks require the same structural receiver proof as ordinary app clicks,
including separation from same-window siblings, before any pointer event. Bounds
alone cannot authorize input. Overlap, unreadable ancestry or an unknown receiver
returns `ACTION_FAILED`, `not_delivered` and `physical_delivery_started: false`.
On X11, a proven target can use XTEST. On Wayland the shared hit test cannot
prove the receiver, so shell ref clicks and popup openers refuse before opening
an input session. Inspect the error before retrying; repeated snapshots cannot
supply missing receiver proof.

Wayland keyboard input is available only after detecting the session-bus
`org.gnome.Mutter.RemoteDesktop` capability. Other compositors return
`PLATFORM_NOT_SUPPORTED` before any event. Shell landmarks are English (`System`,
`Power Off Menu`, `Clear all notifications`); other languages and non-GNOME
shell observation remain outside this guide.

## Service requirements and exact refusals

The GNOME Shell accessibility tree, notification daemon and tray watcher are
separate requirements. A healthy app bus does not establish any of them.

| Missing requirement | Result |
|---------------------|--------|
| Shell surface absent from `status` (for example Openbox) | `snapshot` refuses `PLATFORM_NOT_SUPPORTED` at surface validation; headed open can reach `APP_NOT_FOUND`, “The desktop shell is not on the accessibility registry”; `details.shell_tree: absent` |
| No notification service | `PLATFORM_NOT_SUPPORTED`, “No notification service is running in this session”; `details.daemon: absent` |
| Notification server name other than `gnome-shell` | `PLATFORM_NOT_SUPPORTED`, “The notification service is '<name>'; only GNOME Shell is supported”; `details.daemon: unsupported`, `server: <name>` |
| No `org.kde.StatusNotifierWatcher` | Tray snapshot: `PLATFORM_NOT_SUPPORTED`, “No system tray watcher is running in this session”; `details.watcher: absent`; suggestion enables AppIndicator |
| Notification service or watcher cannot be read | `TIMEOUT`, “The notification service could not be read” or “The system tray watcher could not be read”; corresponding `daemon` / `watcher` detail is `unreadable`, `retryable: true` |
| `open-system-surface --surface taskbar` | `PLATFORM_NOT_SUPPORTED`, “The taskbar surface is not offered: no stock shell shows a dock” |

An unavailable daemon is not an empty notification list. An absent watcher is
not an empty tray. `status` can advertise `system-tray` from a healthy shell
while its snapshot refuses for the missing watcher. Do not restart the user's
shell or accessibility bus to repair these errors.

## Notifications

After policy checks, notification commands check the detected backend. A GNOME
service answers `ACTION_NOT_SUPPORTED`, `details.kind:
notification_identity_unavailable`, `retryable: false`, with `not_delivered`
and safe retry. The refusal happens before reading or opening the message list
and before any input. `wait --notification` returns the same refusal immediately.
Missing, unsupported or unreadable services retain the backend errors above.

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

Tray snapshots require `org.kde.StatusNotifierWatcher`. On GNOME, enable
AppIndicator to provide the watcher and panel integration; stock GNOME has no
native SNI watcher.
This implementation reads the watcher's registered items and their
`org.kde.StatusNotifierItem` properties. It does not make arbitrary XEmbed icons
into SNI items. KDE and other notification daemons are not GNOME notification
support merely because they implement the standard D-Bus interface.

`snapshot --surface system-tray -i` combines the top-bar nodes with watcher
items. Entries from `com.canonical.dbusmenu` are virtual children in this same snapshot; use their
refs rather than looking for an app `menu` surface. A `Passive` item remains in
the snapshot with `states: [hidden]`.

Read virtual item/menu names from the snapshot. Default `get` (text) on a
virtual tray ref fails `INVALID_ARGS`, “Native handle does not contain a Linux
accessibility element”, `details.kind: invalid_native_handle`. This is not an
ordinary AT-SPI node. On the paired visible item, `get --property bounds` and
`is --property visible` worked; that does not make the default text getter work.

A tray item supports headless `click` through advertised `Activate`, and
`right-click` through `ContextMenu`. `SecondaryActivate` is never substituted.
AppIndicator items can omit those primary methods: use a virtual menu entry.
Without a permitted semantic route, the headless CLI returns `POLICY_DENIED`.
A headed panel click on Mutter X11 requires the shared structural receiver
proof and refuses before input when that proof is unavailable. On Wayland this virtual item ref's
headed click instead hit the generic pointer gate: `ACTION_NOT_SUPPORTED`,
cause `wayland_session`, `physical_delivery_started: false`. The dedicated
shell-popup route does not imply that a virtual tray item's fallback works.
SNI methods and ordinary leaf picks report `delivered_unverified`, so observe
the receiving app before deciding whether to repeat them.

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
