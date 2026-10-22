# Windows and Lifecycle on Linux

Window control uses EWMH on the X11 display: an X11 session's display, or the
Xwayland display of a Wayland session.

## Naming apps and windows

- `--app` takes the name `list-apps` prints, or a desktop entry's `Name` or id. A
  desktop alias resolves only when the entry's `Exec` matches the running
  process; a `Name` shared by several entries gets none. Matching is exact and
  case-insensitive, within this session. `list-apps --app` also matches a
  substring.
- Window ids are opaque and live as long as the window's process.
- Several instances of one app share a name: pick one with `--window-id`.
  `close-app` and `launch` cannot choose and answer `AMBIGUOUS_TARGET`.
- Two windows of one process with the same title: `focus-window --window-id` and
  the window operations act on the toolkit's active one and, when exactly two
  share the title, on its twin. Any other tie is refused with cause
  `ambiguous_window`.
- `list-windows` lists the focused window first, then the X11 stacking order from
  the top. A window with no single X11 client follows in registry order.

## `launch`

```bash
agent-desktop launch gnome-calculator
agent-desktop launch gnome-text-editor --arg=/tmp/note.txt
```

- The identifier is an absolute path, desktop id, bare command or desktop entry
  `Name` (`launch "Text Editor"`), tried in that order. A relative path, or a
  name with a separator or NUL, is `INVALID_ARGS`; an unknown name is
  `APP_NOT_FOUND`; an entry with `Terminal=true` or no `Exec` is
  `ACTION_NOT_SUPPORTED`. An absolute `.desktop` path must lie in a trusted XDG
  data directory.
- The child gets null stdio, its own process group and `AGENT_DESKTOP_LAUNCHED=1`,
  so a caller that reads through a pipe sees end of file at exit.
- `launch` attaches to a running app only when its executable and full arguments
  match the process; names never match. One match with no arguments returns the
  owner. One match with arguments spawns the command to forward them, and `--env`
  and `--cwd` do not apply (the result says so). Several matches are
  `AMBIGUOUS_TARGET`. `--no-attach` refuses any match: `ACTION_FAILED` naming
  the pids.
- `--arg` passes one argument and repeats; write `--arg=-x` for a value that
  starts with a hyphen. Chromium and Qt need a flag or variable (`--arg`,
  `--env`): `references/troubleshooting.md`.
- The result carries `app`, `pid`, `process_instance` and, when one appeared,
  `window`. Attach with no arguments returns an existing window; a new launch or
  a handoff reports only a newly opened window, so a tab opened in an existing
  window gives none.
- Without `--activate` it waits 2 s for a started app's window and 5 s for a new
  window of a running app; with `--activate` it waits up to `--timeout`.
  `--timeout` (default 30000) caps every wait. With no window the result is still
  `ok`. A process that never registers on the accessibility bus can hold the
  command for the whole timeout, and its pid may belong to a wrapper.
- A launcher that exits with no verified owner is `ACTION_FAILED`,
  `details.kind: exited` (`crashed` after a signal), `delivered_unverified`. An
  owner that exits before its window appears, or a forwarding launcher whose
  existing owner has different arguments, is `ACTION_FAILED`,
  `delivered_unverified`, without that kind. Re-list windows before retrying.

## `close-app`

`close-app` closes the whole app and succeeds only when the process is gone.
There is no `close-window`: click a window's own Close button ref to close one
window, and use `close-app` for the last one.

Example: with Text Editor windows `a.txt` and `b.txt` open, close only `b.txt`.
The recipe stops if a selection is ambiguous.

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
close a document or tab and leave its window open.

- Graceful close sends `_NET_CLOSE_WINDOW` to each top-level window of the pid and
  no signal. An app that outlives its windows (Files) ends in `TIMEOUT`,
  `delivered_unverified`; `--force` ends it. An unconfirmed send is uncertain and
  unsafe to retry: observe the app first.
- A save prompt keeps the app open as a new top-level window, counted in
  `details.dialogs`: read it with `snapshot --app <name> --surface sheet`, answer
  it, close again. A dialog already standing: `ACTION_FAILED`, cause
  `dialog_pending`, `not_delivered`, `retry: safe`, nothing sent. A dialog that
  opens in answer to the request ends the wait with the same cause, the title in
  `details.dialog`, `delivered_unverified`, `retry: unsafe`.
- `--force` sends `SIGTERM` through a pidfd, then `SIGKILL` after 2 s; a stopped
  process gets `SIGKILL` at once.
- A process of a Wayland session with no X11 window is `ACTION_NOT_SUPPORTED`,
  cause `wayland_session`.
- Protection follows the process instance, never its name. `INVALID_ARGS`,
  `not_delivered`, for:
  - the owner of the display socket, or of the X root's supporting-WM window;
  - an app on the accessibility bus with no managed window that the session
    started (it owns a session-bus name, runs as a user service unit, or is a
    child of the display owner: portal backends, shell helpers, decoration
    hosts), unless this tool launched it;
  - any process that owns a session-bus name and does not answer on the
    accessibility bus, launched or not.

  Missing ownership evidence (an unreadable `/proc` entry, no session bus) is
  `ACTION_NOT_SUPPORTED` before anything is sent; an unreachable display is
  `PLATFORM_NOT_SUPPORTED`.

## Window operations

`move-window`, `resize-window`, `minimize`, `maximize` and `restore` use EWMH.
Move and resize read the geometry back (2 px tolerance); minimize and maximize
read back their state (`details.verification_scope`). `restore` has no uniform
read-back and reports `delivered_unverified`: look at the window afterwards.
Reports carry `action`, `target`, `steps` and `disposition`, with no success
flag.

- A clamp or an ignored request is `ACTION_FAILED`, `delivered_unverified`, with
  the requested, observed and display bounds; the window may have changed in
  part.
- Without `--app` or `--window-id` they act on the focused window, and
  `data.target` names it. Pass one unless you mean that.
- A Wayland-native window, and an un-minimize on a Wayland session, are refused
  with cause `wayland_session` before anything is sent.
- A window the process cannot be mapped to is refused before any write: no client
  is `ACTION_NOT_SUPPORTED`, cause `window_not_mapped`; two clients are
  `AMBIGUOUS_TARGET`, cause `ambiguous_window`.
- A window manager can ignore a restore sent soon after a minimize, or the
  reverse: read the state before retrying. An unconfirmed restore send is
  uncertain and unsafe to retry.
- `move-window` needs both `--x` and `--y` (a lone one is `INVALID_ARGS`, exit 2).
  Write negative values with `=`: `move-window --x=-100 --y=20`.

## Focus

```bash
agent-desktop focus-window --app gnome-calculator
```

`focus-window` succeeds only when `_NET_ACTIVE_WINDOW` names the window and its
application reports it active: its frame or another frame of the app carries
`ACTIVE`, one of its menus is open, or it is a modal or dialog frame that holds
the input focus without `ACTIVE`. After the request is sent, a failed read-back
or a modal refusal is `delivered_unverified` and unsafe to retry, and a later
timeout does not make the earlier request safe to repeat.

- A minimized or hidden target is lifted first. A stopped target is
  `APP_UNRESPONSIVE` before anything is sent.
- When a modal dialog holds the focus, `focus-window --app <name>` succeeds with
  the dialog as `data.focused`: act on the dialog, not the window you named.
- A window the manager never activated ends `ACTION_FAILED` with cause
  `activation_not_granted` (under mutter, until a window has held focus once),
  `activation_refused` (another window stayed active) or `frame_not_active` (the
  manager named the window, but its frame never reported `ACTIVE`).
- While the GNOME Activities Overview is open (a fresh GNOME shell opens it) the
  cause is `overview_open`, and every activation is blocked. Send a global
  `press escape` (no `--app`), then `focus-window --app <name>`. A headed ref
  action runs `focus-window` for you and ends the same way.
- A Wayland session is `ACTION_NOT_SUPPORTED`, cause `wayland_session`.
- A headless ref `click` on a minimized window refuses with live `offscreen`
  evidence and leaves it minimized. On X11, `--headed click` first focuses the
  ref's window, restoring it if minimized, then clicks; it refuses if the window
  manager cannot show and activate it, or the target stays hidden, offscreen or
  occluded. A physical-route refusal falls back to the semantic click only when
  nothing was sent. `restore` restores a window without clicking it.
- A window on another workspace lists `visible: false` and cannot take a headless
  `press`. `focus-window` brings it forward by switching the user's workspace:
  decide first whether that is acceptable.

## `press --app` and background targets

Headless `press --app <name>` delivers only while the target process owns the
active window. On X11 a background or minimized app is refused: `ACTION_FAILED`,
`check: window_active`, cause `display_active_window`, `not_delivered`.
`--headed` brings the window forward, un-minimizes it and sends the keys (X11).
On Wayland, `focus-window` and headed app- or window-scoped `press` refuse with
`ACTION_NOT_SUPPORTED`, cause `wayland_session`, Xwayland clients included.

Headless scoped `press` never activates a window. If focus cannot be confirmed,
bring the target forward yourself and use global `press` (no `--app`,
`--window-id` or `--headed`). A headless `press --window-id` with unconfirmed
exact focus returns `STALE_REF`, `check: exact_window_focus`, cause
`exact_window_focus_unconfirmed`, `not_delivered`: bring the window forward,
refresh `list-windows`, retry with its current id.

## Stopped apps

An app stopped with `SIGSTOP` is detected from `/proc` without a probe. A ref used
against it, a scoped `snapshot`, `list-windows --app`, `focus-window`, the window
operations and a scoped `wait` answer `APP_UNRESPONSIVE` at once, not
`STALE_REF`. `close-app` answers `APP_UNRESPONSIVE`, `not_delivered`,
`retry: safe`; `--force` ends it. An unscoped `list-windows` still lists its
windows with `accessible: false`. A scoped listing of a busy app that left every
frame unanswered is `TIMEOUT`, not an empty list.

## Waiting for lifecycle events

```bash
agent-desktop wait --window "Calculator" --app gnome-calculator --timeout 10000
agent-desktop wait --event window-opened --app gnome-calculator --timeout 10000
```

`wait --event` takes `window-opened`, `window-closed`, `app-launched`,
`app-terminated`, `focus-changed`, `surface-appeared` and `surface-dismissed`. It
diffs a baseline taken at the start against fresh reads. On Linux:

- An app counts as launched when it owns a window. A windowless helper never
  produces `app-launched`; a second window of a running app (`--new-window`) is
  `window-opened`. A Qt process registers twice and counts once.
- A dialog that is its own top-level window is one `window-opened` and one
  `surface-appeared`. `surface-appeared` and `surface-dismissed` need `--app`
  (`INVALID_ARGS`, "requires --app"); a dialog drawn inside its window is found
  only when `--app` scopes the wait.
- `focus-changed` needs a window that reports `ACTIVE`. On Wayland none does
  until a seat keyboard exists, so it times out with `baseline_counts`.
- A window that neither shows nor is iconified is a closed window's husk and is
  left out of every list (unless the window manager lists an unclaimed client of
  it), so `window-closed` fires when a dialog closes.
- `--menu-closed` on a menu you have not opened is true at once: wait for `--menu`
  first.
- `--window` is its own wait mode, not a `--text` filter. With `--event` it
  narrows the event by title, and `--window-id` narrows event waits only. Both
  work only with `window-opened`, `window-closed` and `focus-changed`; any other
  `--event` with them is `INVALID_ARGS`.
- `wait --text --app` can return `AMBIGUOUS_TARGET` before it checks the text,
  when visibility, focus and accessibility do not select one window (a document
  and its dialog). For a save prompt, wait on the title, then snapshot the sheet:

  ```bash
  agent-desktop wait --window 'Save Changes?' --app gnome-text-editor --timeout 5000
  agent-desktop snapshot --app gnome-text-editor --surface sheet -i --compact
  ```

- To dismiss an open menu or a Save As dialog, send a global
  `agent-desktop press escape` (no `--app` or `--window-id`): a popup can own the
  grab that scoped delivery checks against. Escape alone does not prove closure:
  check with `wait --menu-closed --app <name>` or a fresh observation.
- A `batch` is `ok: true`, exit 0, even when an entry fails. Read every entry.

## Reading the disposition

Read `data.disposition` or `error.disposition` before you retry. A refusal before
anything was sent (a protected process, a relative path, a background `press`, an
unknown app or window) is `not_delivered`, `retry: safe`. A timeout after delivery
or a read-back mismatch is `delivered_unverified`, `retry: unsafe`: look at the
state with `list-windows` or `snapshot` before repeating.

## Window classification

Windows come from AT-SPI roles, interfaces and state. On X11, managed-client
ownership, native window type and override-redirect evidence tell user windows
from decoration, dock and popup frames. Class names, titles and process names
never classify a window. Mixed classes or incomplete groups are
`ACTION_NOT_SUPPORTED`.

Where the window manager manages no window of the process, two kinds of frame are
left out: one whose process owns no visible native window, and one of a process
the session itself started (the display owner or its child, or a user service
unit; never one this tool launched), such as the shell's own window or a
decoration frame. A frame that is neither is never hidden: the call fails with the
refusal above. Minimized frames stay listed. On Wayland, frames of the compositor
are left out; other apps keep their AT-SPI classification.
