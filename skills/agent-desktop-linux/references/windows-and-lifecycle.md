# Windows and Lifecycle on Linux

This file covers app and window identity, `launch`, `close-app`, window
operations, focus, workspaces and stopped apps. Window control uses EWMH client
messages on the X11 display (the display of an X11 session, or the Xwayland
display of a Wayland session). `wmctrl` and `xdotool` are not used.

## Naming an app

`--app` accepts three kinds of name:

- the process name `list-apps` prints (`gnome-calculator`, `nautilus`,
  `gnome-text-editor`);
- the registry name (`org.gnome.Nautilus`);
- the `Name` of a desktop entry (`Files`, `Text Editor`) as an alias, when the
  entry's `Exec` program matches the process. The listed name does not change.

Scoped app matching is exact, ignoring case, and the process must belong to this
session. It accepts the listed name, registry name, executable basename, `comm`,
desktop-entry name or id, and known launcher/server pairs (`gnome-terminal`
matches `gnome-terminal-server`). `list-windows --app` uses that exact rule.
`list-apps --app` combines a substring match on the listed name with those exact
adapter aliases; an alias need not be part of the displayed process name.

Window ids are opaque addresses of 34 to 70 characters, valid for the life of the
window's process. `list-apps` rows carry `name`, `pid`, `bundle_id` (desktop id
when known) and `process_instance`. `list-windows` rows carry `id`, `title`, `pid`,
`is_focused`, `minimized`, `visible`, `accessible` and `process_instance`, plus `bounds` when
the display maps one. Several instances of one app share a name: use `--window-id`
to pick one. Two windows of one process with the same title and bounds make
`focus-window --window-id` answer `ambiguous_window`.
`close-app` and `launch` cannot choose between instances and answer
`AMBIGUOUS_TARGET`.

## `launch`

```bash
agent-desktop launch gnome-calculator
agent-desktop launch gnome-text-editor --arg=/tmp/note.txt
```

- An identifier is looked up in this order: an absolute path, a desktop id, a bare
  command name, the `Name` of a desktop entry (`launch "Text Editor"` works). A relative
  path or a name with a separator or a NUL is `INVALID_ARGS`; an unknown name is
  `APP_NOT_FOUND`; an entry that is `Terminal=true` or has no `Exec` is
  `ACTION_NOT_SUPPORTED`.
- The `Exec` line is parsed in process and spawned with null stdio in its own
  process group, so a caller that reads the output through a pipe sees end of
  file at exit.
- `launch` attaches to a running app by default: one match with no arguments
  returns the owner. Arguments do not force a new process: a running Text Editor
  took `--arg=--new-window` and opened a second window in the same pid. `--no-attach`
  on a running app fails with `ACTION_FAILED` naming the pid. A second
  `launch zenity` starts a new process, because each dialog is its own process.
- `--arg` passes one argument and repeats; `--arg --info` works as written, and
  `--arg=-x` is the safe form for a value that starts with a hyphen. A handoff to
  a running instance applies the arguments but not `--env` or `--cwd`, and the
  result says so.
- The pid reported is the owner of the new window, found from `_NET_WM_PID` and
  the registry, never the launcher's pid. The command waits for a window. A
  registry owner gets a 2 s first-window grace, or 5 s for a new window when
  windows were already open. Both are capped by `--timeout` (default 30000).
  If the spawned process never registers on the accessibility bus, owner lookup
  can consume the full timeout; the result then has no `window`.
  When no window appears the result can be `ok` with no `window`;
  `--activate` uses the full remaining timeout for the window wait.
  A process that exits first is `ACTION_FAILED` with
  `details.kind: exited` and `delivered_unverified`. The result carries `app`,
  `pid`, `process_instance` and, when one appeared, `window`.
- Negative numbers need the `=` form: `move-window --x=-100 --y=-20`.
- A no-argument attach returns an existing window of the running app. A new
  launch or argument handoff reports only a newly opened window; if the handoff
  opens a tab in an existing window, the result can have no `window`.
- Chromium, Electron and Qt apps need flags to appear on the bus; pass them with
  `--arg`. See `references/troubleshooting.md`.

## `close-app`

`close-app` closes the app, including all its windows, and succeeds only when the
process is gone. There is no `close-window` command. To close just one of several
windows, click that window's own Close button ref. For the last window, use
`close-app` when you intend to exit the app.

For example, with Text Editor windows titled `a.txt` and `b.txt` already open, this
Bash recipe uses `jq` to select the unique `b.txt` window and its Close button.
It stops if either selection is ambiguous; it does not pick the first match.

```bash
(
  set -euo pipefail
  window_id=$(agent-desktop list-windows --app gnome-text-editor |
    jq -er '[.data[] | select(.title == "b.txt")] |
      if length == 1 then .[0].id else error("choose one window") end')
  close_ref=$(agent-desktop snapshot --app gnome-text-editor --window-id "$window_id" -i --compact |
    jq -er '[.. | objects | select(.role? == "button" and .name? == "Close" and .ref_id?) | .ref_id] |
      if length == 1 then .[0] else error("choose the window Close button") end')
  agent-desktop click "$close_ref"
  agent-desktop list-windows --app gnome-text-editor
)
```

Check that only the intended window disappeared. If a save prompt appears,
snapshot `--surface sheet`, answer it, then list the windows again. Ctrl+W may
close only a document or tab and leave its window open.

- Graceful close sends `_NET_CLOSE_WINDOW` to each top-level window of the pid and no
  signal. A file manager that outlives its windows ends in `TIMEOUT` with
  `delivered_unverified`; `--force` ends it. A terminal server closes like any
  other app: `close-app gnome-terminal` ended `gnome-terminal-server`.
- A save prompt keeps the app open. The dialog is a new top-level window of the
  same pid, counted in `details.dialogs`. Read it with a plain
  `snapshot --app <name>` or `--surface sheet`, answer it, then close again. When
  a dialog already stands, `close-app` sends nothing and answers `ACTION_FAILED`
  with cause `dialog_pending`, `not_delivered` and `retry: safe`. A dialog that
  opens in answer to the request ends the wait at once with the same cause and
  the dialog title in `details.dialog`, but `delivered_unverified` and
  `retry: unsafe`.
- `--force` sends `SIGTERM` through a pidfd, then `SIGKILL` after 2 s; a stopped
  process gets `SIGKILL` at once.
- A process of a Wayland session with no X11 window is `ACTION_NOT_SUPPORTED`,
  cause `wayland_session`.
- Protected processes (the shell, display server, session and system buses, the
  AT-SPI launcher, audio and keyring daemons, the window manager, portal and
  settings daemons) are refused with `INVALID_ARGS` and `not_delivered`.

## Window operations

`move-window`, `resize-window`, `minimize`, `maximize` and `restore` use EWMH.
Move and resize read back geometry inside 250 ms with a 2 px tolerance; minimize
and maximize read back their state. Their successful reports name that
`details.verification_scope`. Restore has no uniform final read-back across
its branches and reports `delivered_unverified`; observe the window afterwards.
Reports contain `action`, `target`, `steps` and `disposition`, without a
`moved`, `resized` or other unconditional success flag.

- A clamp or an ignored request is `ACTION_FAILED` with `delivered_unverified` and
  the requested, observed and display bounds. The window may still have changed
  in part: read the bounds.
- Without `--app` or `--window-id`, `move-window`, `resize-window` and `minimize`
  act on the focused window. `data.target` names the window id, title and process
  identity, even when no selector was supplied. Pass `--app` or `--window-id`
  unless you mean the focused window.
- A Wayland-native window, and an un-minimize on a Wayland session, are refused
  with cause `wayland_session` before anything is sent.
- A window the process cannot be mapped to (none or two clients) is refused.
- A window manager may ignore a restore sent 1 to 2 s after a minimize, or a
  minimize sent within about 250 ms of a restore. A retry after a pause succeeds.
- `move-window` requires both `--x` and `--y`; a lone one is `INVALID_ARGS` at
  parse time (exit 2). Negative coordinates need the `=` form:
  `move-window --x=-100 --y=20`.

## Focus

```bash
agent-desktop focus-window --app gnome-calculator
```

`focus-window` succeeds only when `_NET_ACTIVE_WINDOW` and the frame's `ACTIVE`
state agree.

- A minimized or hidden target is lifted first. A window on another workspace
  switches the user's current desktop to its workspace before the request: this
  changes what the user sees.
- When a modal dialog holds the focus, `focus-window --app <name>` succeeds with
  the dialog as `data.focused`. Read `data.focused.title` and act on the dialog,
  not the window you named. A stopped target is `APP_UNRESPONSIVE` before
  anything is sent.
- A window the window manager never activated can end `ACTION_FAILED` with
  `activation_not_granted`: under mutter, until a window has held focus once.
  While the GNOME Activities Overview is open the cause is `overview_open`; close
  the overview first. A fresh GNOME shell opens it and it blocks activation for
  every window. Send a global `--headed press escape` (no `--app`), then call
  `focus-window --app <name>`; measured `overview_open` from the first call, the
  Escape closed the overview, and the second call returned a focused window with
  `is_focused: true`. A headed ref action runs `focus-window` for you and ends
  the same way.
- A Wayland session is `ACTION_NOT_SUPPORTED`, cause `wayland_session`.
- A headless ref `click` on a minimized window refuses with live `offscreen`
  evidence and leaves it minimized. On X11, `--headed click` first focuses the
  ref's exact source window, restoring it if minimized, then delivers the click.
  It refuses if the window manager cannot show and activate that window, or if
  the target remains hidden, offscreen or occluded after focus. An unsupported
  physical route or geometry refusal can fall back to semantic click only when
  it proves nothing was sent. Ordinary-app headed pointer actions on Wayland
  refuse with cause `wayland_session` before delivery. Use `restore` when you want
  to restore a window without clicking it.

## Workspaces

A window on another workspace lists `visible: false`. It cannot take a headless
`press`. `focus-window` brings it forward by switching the user's workspace;
decide whether that is acceptable before you call it.

## `press --app` and background targets

`press --app <name>` delivers only while the target process owns the active
window. A headless press to a background or minimized app is refused with
`ACTION_FAILED` ("The target application is in the background and focus changes are
not allowed, so no key was sent"), cause `display_active_window`,
`not_delivered`. `--headed` brings
the window forward, un-minimizes it and sends the keys. A headless
`press --window-id` that names another window of the process than the active one
is refused with cause `other_window_of_target` and a suggestion to use `--headed`.

## Stopped apps

An app stopped with `SIGSTOP` is told from `/proc` without waiting for a probe.
A ref used against it, a scoped `snapshot`, `list-windows --app`, `focus-window`
and the window operations answer `APP_UNRESPONSIVE` in milliseconds, not
`STALE_REF`. `close-app` answers `APP_UNRESPONSIVE`, `not_delivered` and
`retry: safe`; `--force` ends it. An unscoped `list-windows` still lists its
windows, marked `accessible: false`.

## Waiting for lifecycle events

```bash
agent-desktop wait --window "Calculator" --app gnome-calculator --timeout 10000
agent-desktop wait --event window-opened --app gnome-calculator --timeout 10000
```

`wait --event` takes `window-opened`, `window-closed`, `app-launched`,
`app-terminated`, `focus-changed`, `surface-appeared` and `surface-dismissed`. It
diffs a baseline taken at wait start against fresh reads. On Linux:

- An app counts as launched when it owns a window. A helper process with no
  window never produces `app-launched`, and a second window of a running app
  (`--new-window`) is `window-opened`, never `app-launched`. A Qt process
  registers twice and counts once.
- A dialog that is its own top-level window is one `window-opened` and one
  `surface-appeared`. An unscoped `window-opened` sees it (measured on a Nautilus
  "New Folder" dialog); `surface-appeared` and `surface-dismissed` require
  `--app` and an unscoped one is `INVALID_ARGS` ("requires --app"). A dialog
  drawn inside its window is found only when `--app` scopes the wait, and it
  appears 0.7 to 1.5 s after the action.
- `focus-changed` needs a window that reports `ACTIVE`. On a Wayland session no
  window does until a seat keyboard exists, so the wait times out there and the
  `TIMEOUT` carries `baseline_counts`.
- A wait scoped to a stopped app fails at once with top-level
  `APP_UNRESPONSIVE`, `not_delivered` and `retry: safe`; it is not a `TIMEOUT`.
  This holds for every mode, `--event` included: a scoped event wait answers in
  milliseconds instead of running its timeout. A `batch` of an action followed by
  such a wait does not start the action (`pre_action_baseline_failed`). An app
  that is alive but slow keeps the wait polling; classes that could not be read
  are left out of the diff and no event is invented from them.
- `batch` itself succeeds when an entry fails: the top-level envelope is
  `ok: true`, the exit status is 0, and each entry carries its own `ok: false`
  and `error`. Read every entry; a failed entry does not fail the command.
- `--menu-closed` on a menu you have not opened is true at once: wait for
  `--menu` first, then close the menu, then wait for `--menu-closed`.
- For an already-focused menu or Save As dialog, `agent-desktop press escape`
  sends Escape to the current keyboard focus. Omit `--app` and `--window-id`
  for this global form: scoped delivery checks the application's active frame,
  while a popup can own the keyboard grab. Confirm the intended app is focused
  first, then verify closure with `wait --menu-closed --app <name>` or a fresh
  window/sheet observation. Sending Escape alone does not prove closure.
- A window that neither shows nor is iconified is the husk of a closed window and
  is left out of every list, so `window-closed` fires when the dialog closes.
- `wait` polls with a 50 ms pause and re-resolves a ref on every poll. A wait for
  a `--window` appears before its content: follow it with `--text`.

## Reading the disposition

Read `data.disposition` or `error.disposition` before you retry. A refusal made
before anything was sent (a protected process, a relative path, a background
`press`) carries `not_delivered` and `retry: safe`. An unknown app or window
in the pre-delivery lookup for `focus-window`, `close-app` or window operations
also carries `not_delivered` and `retry: safe`. A timeout after delivery
or a read-back mismatch carries `delivered_unverified` and `retry: unsafe`: look
at the state with `list-windows` or `snapshot` before you repeat it.
