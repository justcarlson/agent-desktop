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

A match is exact, never a substring, and the process must belong to this
session. Two commands read the rule differently: `list-windows --app` is exact
and `list-apps --app` is a substring of the listed name that does not use the
desktop-entry alias. A server process can own the window of a launcher name
(`gnome-terminal` is the process `gnome-terminal-server`); launch and `close-app`
know that pair, but `--app gnome-terminal` on an observation command does not
match. Read `list-apps` and use what it shows.

Window ids are opaque addresses of 34 to 70 characters, valid for the life of the
window's process. `list-apps` rows carry `name`, `pid` and `process_instance`
and no `bundle_id`. `list-windows` rows carry `id`, `title`, `pid`, `is_focused`,
`minimized`, `visible`, `accessible` and `process_instance`, plus `bounds` when
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
  the registry, never the launcher's pid. The command waits for a window up to
  `--timeout` (default 30000). An app that shows no window returns `ok` with no
  `window` at the timeout; with `--activate` it also waits up to that timeout. A
  process that exits first is `ACTION_FAILED` with `details.kind: exited` and
  `delivered_unverified`. The result carries `app`, `pid`, `process_instance`
  and, when one appeared, `window`.
- Negative numbers need the `=` form: `move-window --x=-100`.
- A launch reports a window the launch produced, never one that was open.
- Chromium, Electron and Qt apps need flags to appear on the bus; pass them with
  `--arg`. See `references/troubleshooting.md`.

## `close-app`

`close-app` succeeds only when the process is gone.

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

`move-window`, `resize-window`, `minimize`, `maximize` and `restore` send EWMH
messages and judge the effect by a read-back inside 250 ms with a 2 px tolerance.

- A clamp or an ignored request is `ACTION_FAILED` with `delivered_unverified` and
  the requested, observed and display bounds. The window may still have changed
  in part: read the bounds.
- Without `--app` or `--window-id`, `move-window`, `resize-window` and `minimize`
  act on the focused window, and the result does not name it. Pass `--app` or
  `--window-id` unless you mean the focused window.
- A Wayland-native window, and an un-minimize on a Wayland session, are refused
  with cause `wayland_session` before anything is sent.
- A window the process cannot be mapped to (none or two clients) is refused.
- A window manager may ignore a restore sent 1 to 2 s after a minimize, or a
  minimize sent within about 250 ms of a restore. A retry after a pause succeeds.
- Negative coordinates need `--x=-100`.

## Focus

```bash
agent-desktop focus-window --app gnome-calculator
```

`focus-window` succeeds only when `_NET_ACTIVE_WINDOW` and the frame's `ACTIVE`
state agree.

- A minimized or hidden target is lifted first. A window on another workspace
  switches the user's current desktop to its workspace before the request: this
  changes what the user sees.
- A modal dialog that took the focus is named in `details` and is never a
  success. A stopped target is `APP_UNRESPONSIVE` before anything is sent.
- A window the window manager never activated can end `ACTION_FAILED` with
  `activation_not_granted`: under mutter, until a window has held focus once.
  While the GNOME Activities Overview is open the cause is `overview_open`; close
  the overview first. A headed ref action runs `focus-window` for you and ends
  the same way.
- A Wayland session is `ACTION_NOT_SUPPORTED`, cause `wayland_session`.
- A hidden window refuses a pointer action with the same reason as `click`
  ("the window manager reports the window hidden"): run `restore --app <name>`.

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
- A dialog is one `window-opened` and one `surface-appeared`. A dialog drawn
  inside its window is found only when `--app` scopes the wait, and it appears
  0.7 to 1.5 s after the action.
- `focus-changed` needs a window that reports `ACTIVE`. On a Wayland session no
  window does until a seat keyboard exists, so the wait times out there and the
  `TIMEOUT` carries `baseline_counts`.
- A wait scoped to a stopped app fails at once with top-level
  `APP_UNRESPONSIVE`, `not_delivered` and `retry: safe`; it is not a `TIMEOUT`.
  A `batch` of an action followed by such a wait does not start the action
  (`pre_action_baseline_failed`). An app that is alive but slow keeps the wait
  polling; classes that could not be read are left out of the diff and no event
  is invented from them.
- `--menu-closed` on a menu you have not opened is true at once: wait for
  `--menu` first, then close the menu, then wait for `--menu-closed`.
- A window that neither shows nor is iconified is the husk of a closed window and
  is left out of every list, so `window-closed` fires when the dialog closes.
- `wait` polls with a 50 ms pause and re-resolves a ref on every poll. A wait for
  a `--window` appears before its content: follow it with `--text`.

## Reading the disposition

Read `data.disposition` or `error.disposition` before you retry. A refusal made
before anything was sent (a protected process, a relative path, a background
`press`) carries `not_delivered` and `retry: safe`. `focus-window` on an unknown
app is `not_delivered` and `safe`. `close-app` and the window operations answer
`unknown` and `unknown` for `APP_NOT_FOUND`, although nothing was sent. A timeout
or a read-back mismatch carries `delivered_unverified` and `retry: unsafe`: look
at the state with `list-windows` or `snapshot` before you repeat it.
