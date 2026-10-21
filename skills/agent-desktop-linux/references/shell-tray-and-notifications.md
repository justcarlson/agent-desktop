# Menus, Shell, Tray and Notifications on Linux

Popup menus work on every desktop that exposes them through AT-SPI. The shell
surfaces, the system tray and the notification commands are built for the GNOME
shell. `agent-desktop status` lists `supported_surfaces`: if a surface is not in
that list, the session does not offer it.

## Menus

`menu` is in `supported_surfaces`. A popup is found by a state change or a new
top-level window, never by toolkit name:

| Toolkit | How a menu shows up |
|---------|---------------------|
| GTK4 | A popover subtree inside the frame, created on open and destroyed on close |
| GTK3 | A menu bar item flips `SHOWING`; a context menu is a new nameless top-level window |
| Qt | A menu-bar popup is a child of its item that keeps its nodes, items cleared on close |
| Chromium | A page or toolbar menu is a new nameless top-level frame |

```bash
agent-desktop click @s8f3k2p9:e51
agent-desktop list-surfaces --app gnome-calculator
agent-desktop snapshot --surface menu --app gnome-calculator -i
```

- A `menu` appears in `list-surfaces` after the action that opened it, and
  `snapshot --surface menu` roots at it. Its refs resolve and act.
- The `menu` entry of `list-surfaces` waits up to about 500 ms for a popup that is
  not open yet, but only when this process delivered an action in the last 2 s and
  the window holds focus. A plain `list-surfaces` from a new process reads once.
- A menu is open when it shows and one of its items shows. Menu-bar items show
  while idle, so they do not count as an open menu by themselves.
- GTK4 menu items have an empty name; the label is the text of the node the item is
  labelled by. A ref whose popup closed is `STALE_REF` with `details.kind`
  `menu_closed`: open the menu again and take a new snapshot.
- GTK4 popup items take the semantic `click`: the pointer is not aimed at them
  because their extents have origin 0,0. Use `click`, not `mouse-click --xy`.
- `wait --menu` and `wait --menu-closed` use the same detector and need `--app` (with
  no `--app` they look for the focused app and can answer `APP_NOT_FOUND`). A menu bar that is
  hidden opens nothing, so the wait times out. A closed-wait that follows an
  opener holds until the menu was seen open or 2 s passed, because a GTK4 popover
  shows 350 to 1000 ms after the action.
- A Qt `QLineEdit` context menu does not appear in the accessibility tree and is
  not observable. A `--headed right-click` on it returns ok with nothing to read.
- A menu walk reads 48 children of a list, table or tree, so a popover item among
  later rows is not seen.

## Shell surfaces (GNOME)

The GNOME shell is its own root and never appears in window listings. The CLI
checks three independent facts, each with its own error and suggestion: the
notification daemon (it must be the shell), the shell tree on the accessibility
bus, and the tray watcher. `supported_surfaces` adds `system-tray`,
`action-center` (the message list) and `quick-settings` (the system menu) when the
shell tree is present. `taskbar` is never offered: no stock shell shows a dock.

```bash
agent-desktop status
agent-desktop snapshot --surface system-tray -i
agent-desktop snapshot --surface action-center -i
agent-desktop --headed open-system-surface --surface action-center
```

- `snapshot --surface <shell surface>` needs no `--app`.
- The message list and the system menu open on a headed pointer click on the date
  menu and the System button. `snapshot --surface` and the notification commands
  close what they opened and read the close back. `open-system-surface` returns the
  window identity and leaves the popup open. Only one shell popup is open at a time:
  in a measured run, opening `action-center` while the system menu was open ended in
  `TIMEOUT` after 5 s, and the reverse. A second click on the System button closes
  the menu.
- `open-system-surface` and shell-surface clicks need `--headed`; headless they answer
  `POLICY_DENIED`.
- Shell clicks are headed only, land on the centre of valid on-screen extents,
  and are refused when the extents meet no display. A shell target's `hit_test`
  answers reaches or unknown, never intercepted.
- The landmarks the shell is found by are English strings. A non-English shell is
  unmeasured.
- On Wayland a shell click is aimed with absolute motion on a screen-cast stream of
  the monitor under the point. The session needs PipeWire, which a GNOME session
  starts. Without it a click on the top bar still works and a click inside a popup
  answers `ACTION_FAILED` with cause `no_screencast_stream` before any event.
- An unsupported desktop, or a missing daemon, shell tree or watcher, answers
  `PLATFORM_NOT_SUPPORTED` or the detector's own error with a suggestion. It is
  not an empty result.

## System tray

`snapshot --surface system-tray` shows the panel nodes with the D-Bus identity of
the watcher item they pair with. Pairing uses a title that is unique among nodes
and items. Items with no node are appended unbound, and nodes with no item are
marked unbound. The tray needs the AppIndicator extension on GNOME; without it
the error suggests enabling the extension.

- A tray item lists `Click` when its introspection lists `Activate`, and
  `RightClick` when it lists `ContextMenu`. A headless `click` is delivered as
  `Activate`. A secondary method is never substituted for a click.
- An item that lists no `Activate` needs `--headed`: the click is a pointer click on
  its panel node, which opens the shell's popup. Headless it is `POLICY_DENIED`.
- The shell's tray popup is not a `menu` surface: `snapshot --surface menu` answers
  `WINDOW_NOT_FOUND` for it. A menu entry of a tray item is a virtual child of the
  item in `snapshot --surface system-tray`. Pick it with `click` on its ref. The
  CLI re-reads the layout first and confirms the entry exists, is enabled, is not
  a separator and belongs to the revision the ref came from.
- A disabled entry is refused the same way headless and headed; passing `--headed`
  does not help.
- `check`, `set-value`, `scroll` and `expand` on a tray item or entry return
  `ACTION_NOT_SUPPORTED`: tray items take `click` and `right-click` only.
- `get`, `is` and `get --property bounds` work on tray refs.

## Notifications

```bash
agent-desktop list-notifications
agent-desktop list-notifications --app Slack --text mail
agent-desktop --headed dismiss-notification 1 --expected-app Slack
agent-desktop --headed dismiss-all-notifications
agent-desktop --headed notification-action 1 Reply --expected-app Slack
agent-desktop wait --notification --timeout 10000
```

- `list-notifications` reads the message list container only, in tree order, so
  banner copies do not count. It works headless with the list closed. The `--app`
  filter is a case-insensitive contains, `--text` looks at title, body and app, and
  an index is a position that keeps its meaning under a filter (1-based).
- A mutation reads the closed list first. A missing or changed entry answers
  `NOTIFICATION_NOT_FOUND`; an empty list returns an empty result without opening
  anything. Otherwise it opens the list, re-reads, checks the expected app and
  title at the index, clicks, and closes the list. Single-item mutations need
  `--headed` and one of `--expected-app` or `--expected-title`. Without an expected
  field the call is `INVALID_ARGS`; without `--headed` it is `POLICY_DENIED`.
- `dismiss-notification` clicks the entry's close button and judges the result by
  the list: the target left and nothing else did.
- `dismiss-all-notifications` returns `{dismissed_count}`; empty `failures` is
  omitted. Without a filter it clicks the footer clear-all button once. With
  `--app` it closes the matching entries one by one.
- `notification-action` expands the entry, clicks the named button, and reports
  `delivered_unverified` with `details.evidence` `action_invoked` when the daemon's
  report was seen, otherwise the entry's disappearance. It never claims the app's
  effect. `list-notifications` rows carry an `actions` list; an action the entry lacks
  returns `ACTION_NOT_SUPPORTED` with `available_actions`. The entry usually leaves the
  list once its action ran, so a second call on the same index answers
  `NOTIFICATION_NOT_FOUND`.
- The shell does not fill in the time label of an entry. An entry whose labels have
  not filled in is read again for up to 1.2 s.
- A headed open of a list that has just emptied can time out; the closed read
  avoids that for every command that needs no click.

## Sandboxed apps

An app in a sandbox is on the accessibility bus only when the sandbox shares it.

- **Flatpak:** the sandbox needs `--talk-name=org.a11y.Bus`.
- **snap:** there is no accessibility interface or plug to grant (`snap interface a11y`
  does not exist). Connect `desktop-legacy` or `unity7`.
- The adapter's own error for a missing or hidden bus names both fixes. Neither has
  been measured against a sandboxed app yet (the Flatpak override and the snap
  disconnect tests are open), so an empty `list-apps` for a sandboxed app is a
  reason to check these first.
