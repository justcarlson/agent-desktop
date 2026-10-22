# Linux Troubleshooting

Start with `agent-desktop status` and `agent-desktop list-apps`.

## Symptom to action

| Symptom | Cause and action |
|---------|------------------|
| `PLATFORM_NOT_SUPPORTED` "AT-SPI accessibility bus is not available", or empty `supported_surfaces` | The bus did not answer. Run inside the desktop session; see `references/bus-and-permissions.md` |
| `list-apps` empty, `status` healthy | No app registered. Start an app, or restart one that started before the bus |
| App runs, `--app` finds nothing | Not on the bus, or wrong name. Use the `list-apps` name; see "Apps not on the bus" |
| Chromium listed, no windows | Add `--force-renderer-accessibility` |
| Thin `snapshot` | Content not ready. `wait --window`, then `wait --text`; raise `--timeout` or `--timeout-ms` |
| `TIMEOUT` on `snapshot` | No usable observation before the deadline, or an incomplete `--root` drill-down. Raise `--timeout-ms` |
| `ACTION_FAILED` `activation_not_granted` | The window manager never activated the window: `references/windows-and-lifecycle.md` |
| `ACTION_FAILED` `overview_open` | GNOME Overview is open. Close it and retry |
| `ACTION_FAILED` `frame_not_active`, or `check: window_active` | The app does not report its window active. Run `snapshot` on the app once and retry (intermittent on Electron) |
| `POLICY_DENIED` on `click` or `type` | Read the failed check. Headed policy gives physical input only where the receiver can be proven |
| `POLICY_DENIED` on `press` | Blocked combo. `--force` sends it |
| `ACTION_NOT_SUPPORTED` `wayland_session` | Wayland has no focus, headed scoped `press` or native-window control. Focus the target yourself; use global `press` or the app's own UI |
| `ACTION_NOT_SUPPORTED`, `physical_delivery_started: false` | Pointer input on Wayland. Use a semantic action, `set-value` or global `press` |
| `ACTION_NOT_SUPPORTED` `selection_index_unproven` | A headless `select` cannot map the row, and fresh refs do not help. On X11 run `--headed select` with the same ref: it clicks the row and checks it reads selected. Check the pane too |
| `ACTION_NOT_SUPPORTED` `shell_container_unproven`, or `PLATFORM_NOT_SUPPORTED` with `daemon:` or `watcher:` | Shell, notification or tray service: `references/shell-tray-and-notifications.md` |
| `STALE_REF` | The UI changed or the window closed. Read `details.kind` below |
| `AMBIGUOUS_TARGET` `window_position_ambiguous` | Two windows of one app share a title. Close one or use a window with another title, then snapshot again |
| `APP_UNRESPONSIVE` | The app is stopped (`SIGSTOP`) or its main loop is blocked. Resume it; `close-app --force` ends it |
| `INTERNAL` "private file parent is accessible by group or other users" | The state root is group- or world-accessible. `chmod 700` it |

| `STALE_REF` `details.kind` | Action |
|----------------------------|--------|
| `resolve_no_candidate` | Nothing matches now. Retryable: an action waits its budget before it reports this |
| `resolution_identity_unknown` | A candidate could not be read. Retry after the app finishes updating its tree; the app is not hung |
| `resolve_target_destroyed`, `menu_closed` | The target is gone. Take a fresh snapshot (reopen a closed menu first); the old ref cannot work again |

Any other kind carries `details.retryable`.

## Apps not on the bus

An app is in `list-apps` only when its toolkit registered with the accessibility
bus. The CLI says so when a process runs but is not on the bus.

| Toolkit | Needs |
|---------|-------|
| GTK3, GTK4 | Nothing, if the app started after the bus. Restart one that started before |
| Qt | `QT_ACCESSIBILITY=1` in the process environment |
| Chromium browsers | `--force-renderer-accessibility`. `ACCESSIBILITY_ENABLED=1` alone gives a bus entry with no windows |
| Electron | No accessibility flag. `--no-sandbox` where `chrome-sandbox` is not setuid |
| Flatpak, snap | The sandbox must share the bus: `references/shell-tray-and-notifications.md` |

```bash
agent-desktop launch google-chrome --arg=--force-renderer-accessibility
```

Chrome lists as `chrome`: use `--app chrome` afterwards. The `launch` result
carries `renderer: chromium`. Chromium and Electron read thin until they settle:
pass `--timeout-ms` to `snapshot`. A Chromium menu frame is a nameless top-level
frame, not a window, so it is not a window event.

## Observation

- Check `data.complete` before treating a missing control as absent. A snapshot
  cut short after usable observation succeeds with `complete: false`,
  `truncated: true` and `nodes_observed`: retry with a larger `--timeout-ms`. An
  incomplete `--root` drill-down returns `TIMEOUT` and keeps the old refs.
- `--max-depth` and `--skeleton` count semantic levels, not layout wrappers, so
  nesting can exceed your number. GTK4 `-i --compact` still shows nameless
  `group` nodes: read past them.
- `--root @ref` takes the window and options from the ref's snapshot.
- `find` writes a snapshot; `status` reports it as the latest.
- `find --first` returns `data.match`, not `data.matches`.
- `find --text` matches a subtree's text, so ancestors match too. `wait --text`
  matches a node's own name, value or description.
- A cell inside a cell is two refs.
- A node without a usable position has no `bounds` and is not hit-tested.
- Only a password role is withheld. A GTK4 `Gtk.Entry` with visibility off, an
  entry with `InputPurpose.PASSWORD` and `Adw.PasswordEntryRow` all show their
  text.
- Snapshots carry no live `offscreen` or `hidden`, and `offscreen` is not a
  `get` or `is` property. An actionability refusal reports the live evidence.

## GTK editor tabs

GNOME Text Editor can show one buffer's text under two tab groups, and a group
label can name a file whose content sits in another tab. Labels and focus do not
prove which buffer a ref edits. Switch to the tab, take a fresh snapshot and
check the content before writing. For example, with `beta.txt` active and
`alpha.txt` to its left, holding `BETA FILE` and `ALPHA FILE`:

```bash
agent-desktop press ctrl+pageup --app gnome-text-editor
agent-desktop snapshot --app gnome-text-editor -i --compact
agent-desktop press ctrl+a --app gnome-text-editor
agent-desktop press ctrl+c --app gnome-text-editor
agent-desktop clipboard-get
```

The copied text must be `ALPHA FILE`; the copy replaces the clipboard.
`ctrl+pagedown` switches right. If the identity stays unclear, stop before
writing. For separate targets, open each document in its own window and use
`--window-id` with fresh snapshots:

```bash
agent-desktop launch gnome-text-editor --arg=--new-window --arg=/tmp/note.txt
```

## Toolkit limits

| Toolkit | Limit |
|---------|-------|
| GTK4 | Bounds are window-relative. The adapter places them on screen through the window's X11 client, matched by process and title; for two same-titled windows it pairs the active window with the active client. With no match there is no position: headless semantic actions still work, and a request that needs a pointer position fails `AMBIGUOUS_TARGET`, `details.kind: window_position_ambiguous` |
| GTK3 | `scroll-to` works through the scroll bar. A held handle can answer stale values after its object is destroyed, so waits re-resolve on every poll |
| Qt | `scroll-to` does not work: use `scroll --headed`. A `QLineEdit` context menu is not observable. The combo popup takes no semantic selection. A Qt process registers twice; `list-apps` shows one entry |
| Chromium, Electron | `scroll-to` works through `ScrollTo`. Hit tests answer unknown for an Electron host |

A search control may expose no `SetValue`, and headed `type @ref` can refuse
`target_focus`. Open the search through an observed control or the app's own
shortcut. Find the editor by role, editable state and focus, never by a
translated label, and check the query after typing or pasting. If focus cannot be
proven, stop before typing.

Hit tests answer `Unknown` on Wayland, without a window manager, for a
minimized or hidden window, and where position or landing cannot be proven
(GTK4 with an ambiguous X11 mapping, Electron hosts). Pointer actions refuse an
unproven result with `ACTION_FAILED`, `details.kind: pointer_hit_unproven`.
Overlapping siblings in one window refuse with `hit_test_unproven`,
`reason: sibling_overlap` and `physical_delivery_started: false`, as do missing
child bounds or an unreadable branch. Remove the overlap and snapshot again. A
covered button stays `visible` and headless-actionable: occlusion is checked
only before pointer delivery.

## Errors and `platform_detail`

Linux fills `platform_detail` with the raw D-Bus or X11 error.

| `error.code` | `platform_detail` | Meaning |
|--------------|-------------------|---------|
| `PLATFORM_NOT_SUPPORTED` | `org.freedesktop.DBus.Error.ServiceUnknown` | AT-SPI not installed or not visible |
| `PLATFORM_NOT_SUPPORTED` | `org.freedesktop.DBus.Error.Spawn.*` | The bus launcher cannot start |
| `PLATFORM_NOT_SUPPORTED` | `Failed to connect to address ... No such file or directory` | The accessibility bus socket is missing: restart the bus launcher. With the session bus address: no session bus |
| `TIMEOUT` | "The accessibility bus did not answer in time" | A hung or slow bus; nothing learned |
| `INTERNAL` | "The accessibility bus returned an unexpected error" | Unclassified reply: retry once, then report it |
| `INVALID_ARGS` | "Native handle does not contain a Linux accessibility element" | The ref is the wrong kind for this command |

## Missing tools

X11 needs no outside tool. The Wayland clipboard needs `wl-clipboard`. The X11
clipboard needs `agent-desktop-linux-clipboard-owner` beside the binary. The bus
needs `at-spi2-core`. A missing tool is reported by name with its package, never
as an empty result.

## Shared hosts

The interaction lock (`/tmp/agent-desktop-<uid>/interaction.lock`) serializes
input per user. `TIMEOUT` with `lock_timeout` and a high `contention_count` means
another agent of the same user holds it. Do not restart the user's accessibility
bus; run a second bus only in a private runtime directory.
