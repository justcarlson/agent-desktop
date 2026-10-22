---
name: agent-desktop-linux
version: 0.10.1 # x-release-please-version
tags: linux-automation, accessibility, at-spi, x11, wayland, ai-agent, gui-automation, cli
requirements:
  - agent-desktop
description: >
  Linux platform guide for the agent-desktop CLI. Lists per command group
  what works on X11 and on Wayland, how to check the AT-SPI accessibility bus,
  the first-contact gotchas (app names, Chromium and Qt launch flags, headed
  focus, blocked key combos), and where to find lifecycle, screenshot,
  clipboard, tray, notification and troubleshooting details.
  Triggers on: "linux desktop automation", "AT-SPI tree", "automate GNOME app",
  "agent-desktop on Linux", "accessibility bus not available", "empty
  supported_surfaces", "wayland screenshot consent", or any Linux GUI
  interaction task.
---

# agent-desktop-linux

The observe-act loop, refs, JSON envelope, sessions and tracing match every
platform. This file covers only what differs on Linux.

Needs Ubuntu 24.04+ (x86_64 or aarch64), a graphical session of the signed-in
user and a running AT-SPI bus. Other distributions need the at-spi2-core bridge
fix `dc0dc33`; Ubuntu 22.04 aborts in `Action.GetActions`. Run the CLI inside the
target's desktop session: a TTY, an SSH shell without session variables, a
container without a bus or another user's session sees no apps. GNOME is the
measured desktop. Observation and semantic actions need no helper tool.

## Capability Table

| Group | Commands | On Linux |
|-------|----------|----------|
| Observation | `snapshot`, `find`, `get`, `is`, `list-surfaces` | AT-SPI, X11 and Wayland. Surfaces `window`, `sheet`, `menu`; GNOME adds shell surfaces |
| Ref actions | `click`, `right-click`, `type`, `set-value`, `clear`, `select`, `toggle`, `check`, `uncheck`, `expand`, `collapse`, `scroll-to` | Semantic AT-SPI calls on both servers. Need a readable live identity and the AT-SPI capability |
| Pointer | `double-click`, `triple-click`, `hover`, `drag`, `scroll`, `mouse-click`, `mouse-move`, `mouse-wheel` | `--headed`, X11 only. Wayland refuses: `ACTION_NOT_SUPPORTED`, cause `wayland_session` |
| Focus | `focus` | `--headed`; uses the node's own focus action |
| Keys | `press` | X11, and Mutter Wayland. Details in `references/display-servers-and-input.md` |
| Held input | `key-down`, `key-up`, `mouse-down`, `mouse-up` | Always refused: `ACTION_NOT_SUPPORTED`; `mouse-down` and `mouse-up` are `POLICY_DENIED` headless |
| Apps, windows | `launch`, `close-app`, `list-apps`, `list-windows`, `focus-window`, `move-window`, `resize-window`, `minimize`, `maximize`, `restore` | Launch and listing work on both servers. Window control, `focus-window` and graceful close need X11. See `references/windows-and-lifecycle.md` |
| Displays | `list-displays` | Works |
| Capture | `screenshot` | X11: a window, no prompt; a display only under a non-compositing manager. Wayland: portal consent, primary display by default. See `references/capture-and-clipboard.md` |
| Clipboard | `clipboard-get`, `clipboard-set`, `clipboard-clear` | Text, images, file lists. Wayland needs `wl-clipboard` |
| Wait | `wait` | `ms`, `--element`, `--window`, `--text`, `--menu`, `--menu-closed`, `--event`, and `--notification` on GNOME |
| Notifications | `list-notifications`, `dismiss-notification`, `dismiss-all-notifications`, `notification-action` | GNOME Shell only (verified on 46.0). List is headless; mutations need `--headed` and X11, and the single-item ones also `--expected-app` or `--expected-title`. Another notification owner: `PLATFORM_NOT_SUPPORTED` |
| Shell surfaces | `open-system-surface` | `system-tray` (needs a StatusNotifierWatcher, else `watcher: absent`) and `action-center` work. `quick-settings` and `taskbar` refuse; `start-menu` and `system-tray-overflow` are unsupported, though `--help` lists them |
| Cursor overlay | `cursor-overlay` | Needs a session (below) |
| System | `status`, `permissions`, `version`, `batch`, `skills`, `session`, `trace` | Works |

## First Contact

- **Run `agent-desktop status` first.** An empty `supported_surfaces` means the
  accessibility bus did not answer, not that the desktop is empty. Do not
  restart a bus you did not start; see `references/bus-and-permissions.md`.
- **`--app` takes the name `list-apps` prints** (`gnome-calculator`, `nautilus`)
  or a desktop-entry name or id (`Files`), never a window title. `list-windows
  --app` matches exactly; `list-apps --app` also matches a substring.
- **Window ids are opaque** and live as long as the window's process. List
  windows again after a restart.
- **GTK4 output nests deeper than `--max-depth` or `--skeleton` suggest**, because
  they count semantic levels. `-i --compact` still shows nameless `group` nodes.
- **A window appears before its content.** Run `wait --window "Text Editor" --app
  gnome-text-editor`, then `wait --text`. `wait --text` matches a node's own
  name, value or description. A web app's snapshot waits for its document and
  stays incomplete at the deadline without one. A `--timeout` shorter than the
  first read returns `TIMEOUT`, never a partial tree.
- **A ref that carries only geometry needs the same size and position** in its
  recorded frame: take a fresh snapshot after a window move or resize.
  Identical siblings are `AMBIGUOUS_TARGET`; the text views of three tabs in one
  Text Editor window share one rectangle (`candidate_count` 3). A ref belongs to
  the window it was taken in, so same-title windows resolve their own refs.
- **Chromium and Qt apps need a launch flag or variable** to appear on the bus.
  Electron needs none, only `--no-sandbox` where `chrome-sandbox` is not setuid.
  See `references/troubleshooting.md`.
- **Headed ref actions need a window that can take focus.** One the window
  manager never activated ends `activation_not_granted`.
- **A GTK4 entry with hidden characters is readable.** Only a password role is
  withheld.
- **Blocked key combos refuse without `--force`.**
- **A click that opens a popup lists it in `data.surfaces`.** A `menu` entry means
  `snapshot --surface menu --app <name> -i`. Without one, try `wait --menu`, then
  `list-surfaces`. A list-backed dropdown is part of the window, not a menu
  surface.
- **Shell clicks need receiver proof.** On X11 a headed shell click is proven
  from the shell's own tree and refuses if the target is covered or moving;
  Wayland shell clicks refuse. Notifications need the GNOME notification
  service; the tray needs a StatusNotifierWatcher.
- **The state root must be private.** If `AGENT_DESKTOP_HOME` or
  `~/.agent-desktop` is group- or world-accessible, `snapshot` fails with
  `INTERNAL`; run `chmod 700` on it.

## Wait for a window

```bash
agent-desktop wait --window "Text Editor" --app gnome-text-editor --timeout 10000
```

To wait for a close that you cause, batch it so the baseline is taken first
(Calculator has no save prompt):

```bash
agent-desktop batch '[{"command":"close-app","args":{"app":"gnome-calculator"}},{"command":"wait","args":{"event":"window-closed","app":"gnome-calculator","timeout":10000}}]' --stop-on-error
```

To watch someone else close a window, start
`agent-desktop wait --event window-closed --app gnome-text-editor --timeout 10000`
first. There is no `--window-closed` flag.

## Cursor overlay

Pass a session ID to every overlay command; without one they return
`INVALID_ARGS`. X11 draws the overlay (`data.rendered: true`); Wayland only
saves the setting (`false`).

```bash
session_id=$(agent-desktop session start --name docs-example | jq -r '.data.session_id')
agent-desktop cursor-overlay enable --session "$session_id"
agent-desktop cursor-overlay disable --session "$session_id"
agent-desktop session end "$session_id"
```

## What differs on Linux

- Ref actions are semantic and never move the pointer or the active window.
  `--headed` makes `click`, `right-click` and `type` try physical input first.
- A headed action falls back to the semantic one only after
  `ACTION_NOT_SUPPORTED` or a geometry refusal (`check: geometry`), both
  `not_delivered`; `details.physical_fallback` records it. Physical success is
  `physical_synthetic` and unverified. Windows works the other way round.
- On Wayland, headed pointer commands (`click`, `hover`, `double-click`,
  `mouse-*`, wheel, `select`) are refused at once with `ACTION_NOT_SUPPORTED`,
  cause `wayland_session`. A headed `type` sends keys through Mutter
  RemoteDesktop; other compositors return `PLATFORM_NOT_SUPPORTED`.
- `select` tries the semantic route first. Headed on X11, a GTK4 list row is
  clicked, and a libadwaita combo row is opened and the choice picked from its
  popup; both check the result. Headless, a list refuses with
  `selection_index_unproven`, and a combo with no semantic opener with
  `POLICY_DENIED`, kind `pointer_required`, unless it already shows the choice.
  A GTK4 drop-down that opens through its own toggle takes `select` without
  `--headed`.
- Headless `type` writes through `EditableText`. A terminal's text node
  (`textfield` named `Terminal`, only `RightClick`) takes keys only under
  `--headed`; headless `type` there is `POLICY_DENIED`, check `supported_action`.
- Firefox/Gecko `select` fires no `change` event: the result stays
  `delivered_unverified`, so check the page's own effect.
- In key combos `cmd` and `meta` are Super. Key names are the macOS and Windows
  names plus X keysym names. `press` takes mapped keys only; `type` handles
  characters the layout lacks.
- Window ops act on X11 clients; `focus-window` and headed scoped `press` need
  X11. Headless scoped `press` needs live target focus on either server. Focusing
  a window on another workspace switches the user's desktop.
- Linux FFI refs that carry bounds need window-relative geometry, even with a
  name or identifier; refs without geometry stay supported.

## References

- `references/bus-and-permissions.md`: bus bootstrap, `status`, `permissions`, state root.
- `references/display-servers-and-input.md`: X11 versus Wayland, `press`, `type`, pointer, wheel, drag.
- `references/windows-and-lifecycle.md`: `launch`, `close-app`, window ops, focus, stopped apps.
- `references/capture-and-clipboard.md`: `screenshot`, the Wayland consent dialog, clipboard.
- `references/shell-tray-and-notifications.md`: menus, popovers, GNOME shell, notifications, tray, sandboxes.
- `references/troubleshooting.md`: symptom-to-cause map, launch flags, toolkit limits, error codes.

Read any file from the binary:
`agent-desktop skills get agent-desktop-linux references/troubleshooting.md`.
