# Menus, Shell, Tray and Notifications on Linux

Run inside the target desktop session. `status` shows snapshot capabilities, not
whether a popup is open or a tray watcher runs.

## App menus and GTK4 popovers

After a click that may open a popup, read `data.surfaces`. A `type: menu` entry
means `snapshot --surface menu --app <name> -i`. Without one, run `wait --menu`,
then `list-surfaces`; GTK4 popovers appear asynchronously. A scan that could not
classify every node lists `menu` under `unclassified`, so an absent menu does not
prove it is closed.

Needs a running Calculator and `jq`. On Mutter X11, close Overview and focus
Calculator first (see Overview below). Read every result, waits included.

```bash
menu_ref=$(agent-desktop snapshot --app gnome-calculator -i | jq -r '.. | objects | select(.role? == "switch" and .name? == "Main Menu") | .ref_id // empty')
test -n "$menu_ref" && agent-desktop click "$menu_ref"
agent-desktop wait --menu --app gnome-calculator --timeout 5000
agent-desktop list-surfaces --app gnome-calculator
agent-desktop snapshot --surface menu --app gnome-calculator -i
agent-desktop press escape
agent-desktop wait --menu-closed --app gnome-calculator --timeout 5000
```

- `list-surfaces` shows the open menu with `item_count`. `snapshot` and `find`
  with `--surface menu` root at the popup.
- A menu ref lives only while its popup stays open and unchanged. Afterwards:
  `STALE_REF` with `details.kind` `menu_closed` or `menu_context_changed`, or
  retryable `menu_context_unknown`; a menu snapshot returns `ELEMENT_NOT_FOUND`.
  Open the menu again and take new refs.
- A GTK4 popover item (Calculator's Preferences) clicks through its ref when its
  popup has a provable screen position. On Wayland it has none: `ACTION_FAILED`,
  `details.check` `geometry`.
- `click`, `expand` and `collapse` on a popup opener (push button, menu button)
  are checked against the popup itself. If it does not appear or leave, the
  result is `ACTION_FAILED`, `popup_not_observed`, `delivered_unverified`: take a
  fresh snapshot before acting. `expand` on an open popup succeeds without
  delivery.
- `press escape` is a global key and its success does not prove closure; use
  `wait --menu-closed`.
- Headless Wayland: a global `press` can leave the seat without a keyboard, and a
  GTK4 opener then fails `popup_not_observed`. A keyboard present from launch
  avoids it.
- Physical clicks on menu items work on X11 only, after a popup position is
  observed, and stay unverified. On Wayland they refuse with
  `ACTION_NOT_SUPPORTED`, cause `wayland_session`.

| Toolkit | Menu shape |
|---------|------------|
| GTK3 | Menu-bar child or separate context-menu window |
| GTK4, libadwaita | Popover inside the frame |
| Qt | `QMenu` popup; a `QLineEdit` context menu may expose nothing |
| Chromium | Native popup in a separate frame; web content is not a menu surface |

GTK4 list-backed dropdowns (Settings' Night Light Schedule) are not menu popups:
their choices stay in the window tree. Click the combobox ref, `wait --text` for
a choice, then snapshot the window. `select` rules are in `SKILL.md`. Where
`select` refuses on X11, click the combobox with `--headed`, then use headed
`press down`, `press up` and `press return`. Read the value back with
`snapshot --root` on the combobox ref, because `get` may return only its label.

## GNOME Shell surfaces

The same values work for `snapshot --surface` and `open-system-surface`.
`--help` lists the values of every platform.

| Value | Result |
|-------|--------|
| `system-tray` | Needs `org.kde.StatusNotifierWatcher`, else `PLATFORM_NOT_SUPPORTED`, `watcher: absent`. `open-system-surface` returns no window |
| `action-center` | The message list. `open-system-surface` opens it through the date menu when its container and opener are proven |
| `quick-settings` | `ACTION_NOT_SUPPORTED`, `shell_container_unproven`, before any input |
| `taskbar` | `PLATFORM_NOT_SUPPORTED`: no stock shell shows a dock |
| `start-menu`, `system-tray-overflow`, others | `PLATFORM_NOT_SUPPORTED` |

- Shell snapshots need no `--app`. Shell containers are not in `list-windows`.
- Containers are found by role shape and uniqueness, never by English labels or
  screen position. A container or opener that cannot be proven fails
  `ACTION_NOT_SUPPORTED`, `shell_container_unproven`, before input.
- Overview is not a `--surface` value (`INVALID_ARGS`). On Mutter X11, with
  Overview open, headed app focus fails with cause `overview_open`. Run
  `agent-desktop press escape`, then
  `agent-desktop --headed focus-window --app gnome-calculator`. Until then a
  click can report success without opening the menu.
- Shell clicks need `--headed`. On X11 the adapter first reads the shell tree:
  a covered, moving, hidden or missing target fails `ACTION_FAILED`,
  `pointer_hit_unproven`, with nothing sent. Retry after the shell settles. Each
  effect is read back from the shell.
- On Wayland a shell ref click fails `ACTION_NOT_SUPPORTED`, cause
  `wayland_session`, and a popup opener fails `pointer_hit_unproven`.

## Service requirements

A healthy accessibility bus proves none of these.

| Missing | Result |
|---------|--------|
| Shell session owner or matching accessibility tree | Cause-specific missing-tree refusal |
| Notification service | `PLATFORM_NOT_SUPPORTED`, `daemon: absent` |
| Notification owner other than GNOME Shell's | `PLATFORM_NOT_SUPPORTED`, `daemon: unsupported` |
| `org.kde.StatusNotifierWatcher` | `PLATFORM_NOT_SUPPORTED`, `watcher: absent` |
| Unreadable notification service | `TIMEOUT`, `daemon: unreadable` |
| Unreadable tray watcher | `TIMEOUT` on a call timeout, else `INTERNAL` |

An unavailable service is not an empty list. Do not restart a shell or
accessibility bus you do not own.

## Notifications

GNOME Shell only. Every field comes from the shell's widget layout, which no
protocol names. It is verified on GNOME Shell 46.0 (Ubuntu 24.04), X11 and
Wayland, and on no other version. A different layout fails with `INTERNAL`
("no readable title") or reads wrong; nothing is guessed.

- `list-notifications` is headless, works with the list closed and skips banner
  copies. `index` is the 1-based position in the unfiltered list, newest first;
  filters never renumber it. `--app` is a case-insensitive contains, `--text`
  searches app, title and body, `--limit N` keeps the first N after filters. An
  empty list is a successful empty result.
- `wait --notification` waits for an entry that is new since its baseline.
- GNOME exposes no notification ID. An index is a position: re-list after every
  mutation and never reuse an old index.

This example assumes one disposable notification titled `Beta` from
`Skill Example`. `jq` picks the index, and the mutation guards both identity
fields:

```bash
agent-desktop list-notifications
notice_index=$(agent-desktop list-notifications --app "Skill Example" --text Beta | jq -r '.data.notifications[] | select(.app_name == "Skill Example" and .title == "Beta") | .index')
test -n "$notice_index" && agent-desktop --headed dismiss-notification "$notice_index" --expected-app "Skill Example" --expected-title Beta
agent-desktop list-notifications --app "Skill Example"
```

| Command | Needs | Notes |
|---------|-------|-------|
| `dismiss-notification INDEX` | `--headed`, X11, `--expected-app` or `--expected-title` | Returns the removed entry as `data.dismissed` |
| `notification-action INDEX ACTION` | Same | `ACTION` is a name from the row's `actions`, case-insensitive |
| `dismiss-all-notifications` | `--headed`, X11; `--app` limits scope | Returns `data.dismissed_count`, `data.failures`, `data.failed_count` |

- No identity flag: `INVALID_ARGS`. Headless: `POLICY_DENIED`, an empty
  `dismiss-all-notifications` included. Missing index, changed identity or an
  entry from another `--app`: `NOTIFICATION_NOT_FOUND`. Unknown action:
  `ACTION_NOT_SUPPORTED` with `available_actions`. Wayland: `ACTION_FAILED`,
  `pointer_hit_unproven`, before any input.
- A mutation opens a closed message list and closes it again; a list already
  showing stays showing. If it cannot open the list, the error suggestion says
  to open it by hand.
- Dismissal counts only when the entry's own object leaves the list. An entry
  still standing, or an unreadable list after the click, is `ACTION_FAILED`,
  `delivered_unverified`, `retry: unsafe`: re-list before acting.
- `notification-action` expands a collapsed entry first. It counts when the entry
  leaves the list, which a resident notification does not do. The result is
  always `delivered_unverified`: the receiving app's effect is never claimed.
  `details.evidence` is `action_invoked` when exactly one `ActionInvoked` signal
  was also seen, else `entry_removed`.
- `dismiss-all-notifications` closes one entry at a time and never presses the
  footer button. A headed run on an empty list returns zero.

## StatusNotifierItem and AppIndicator tray

Needs `org.kde.StatusNotifierWatcher`. XEmbed icons are not SNI items. GNOME
usually gets a watcher through AppIndicator; install or enable one only in a
session you own.

`snapshot --surface system-tray -i` shows watcher items under a logical root,
with `com.canonical.dbusmenu` entries as virtual children. Titles never bind a
native panel node. A `Passive` item shows `states: [hidden]`, and a click on it
fails `ACTION_FAILED` before any call.

- Refs are virtual: bounds, placement and offscreen state are unknown, and native
  text getters fail `INVALID_ARGS`, `invalid_native_handle`.
- Headless `click` calls `Activate`; `right-click` calls `ContextMenu`. Neither
  falls back to `SecondaryActivate` or a physical click, and `--headed` adds no
  fallback (on Wayland it can refuse first, `wayland_session`). When an item
  lacks the method, use a virtual menu entry.
- Item methods report `delivered_unverified`. A menu pick is `delivered_verified`
  only when a toggle flips or submenu children appear. Otherwise observe the app
  before repeating.
- A menu-entry click re-resolves its label path and entry ID in a fresh layout.
  A gone or replaced entry fails `STALE_REF`; a label repeated at one level
  fails `AMBIGUOUS_TARGET`. Both come before any event.
- Disabled entries are never clicked: `TIMEOUT`, `kind: actionability_timeout`.
- After the owner exits, old refs fail `STALE_REF`. Items take `click` and
  `right-click`, entries take `click`; other actions are refused.

## Sandboxed apps

A sandboxed app is on the accessibility bus only when its sandbox shares it.
Flatpak: `--talk-name=org.a11y.Bus`. snap: no accessibility plug exists; connect
`desktop-legacy` or `unity7`. Neither fix is measured. Check them first when a
sandboxed app is missing from `list-apps`.
