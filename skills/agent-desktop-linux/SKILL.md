---
name: agent-desktop-linux
version: 0.10.1 # x-release-please-version
tags: linux-automation, accessibility, at-spi, x11, wayland, ai-agent, gui-automation, cli
requirements:
  - agent-desktop
description: >
  Linux platform guide for the agent-desktop CLI. Lists per command group
  what works on X11 and on Wayland, how to check the AT-SPI accessibility bus,
  the first-contact gotchas (app names, Chromium and Electron flags, headed
  focus, blocked key combos), and where to find lifecycle, screenshot,
  clipboard, tray, notification and troubleshooting details.
  Triggers on: "linux desktop automation", "AT-SPI tree", "automate GNOME app",
  "agent-desktop on Linux", "accessibility bus not available", "empty
  supported_surfaces", "wayland screenshot consent", or any Linux GUI
  interaction task.
---

# agent-desktop-linux

The observe-act loop, ref system, JSON envelope, sessions and tracing are the
same on every platform. This file covers only what differs on Linux.

Requires a graphical session of the signed-in user with a running AT-SPI
accessibility bus, glibc 2.35 or newer (Ubuntu 22.04 baseline), on x86_64 or
aarch64. Run the CLI inside the same desktop session as the target apps: the
session bus, the display and the runtime directory come from its environment.
A TTY login, an SSH shell with no session variables, a container with no bus
and another user's session see no apps. GNOME is the measured desktop; X11 and
Wayland sessions both work with the differences below. The CLI needs no helper
tool for observation or semantic actions.

## Capability Table

| Group | Commands | Status on Linux |
|-------|----------|-----------------|
| Observation | `snapshot`, `find`, `get`, `is` | Works on X11 and Wayland through AT-SPI. Supports `window`, `sheet` and `menu`; GNOME adds shell surfaces. Virtual tray getters have limits; see the shell reference and check `status` |
| Surfaces | `list-surfaces` | Lists an app’s windows, sheets and open menus; shell reads use `snapshot --surface` |
| Ref interaction | `click`, `right-click`, `type`, `set-value`, `clear`, `select`, `toggle`, `check`, `uncheck`, `expand`, `collapse`, `scroll-to` | Needs readable live identity and the required AT-SPI capability. See the shell reference for menu-ref resolution limits; X11 headed fallback also needs usable geometry and does not bypass identity checks |
| Wheel scrolling | `scroll`, `mouse-wheel` | Physical wheel input; X11 and `--headed` only |
| Pointer gestures | `double-click`, `triple-click`, `hover`, `drag` | `--headed`, X11 only. On Wayland the pointer cannot be aimed and these refuse with `ACTION_NOT_SUPPORTED` |
| Focus | `focus` | Needs `--headed` for the focus permission; delivered through the node's own focus action, not the pointer |
| Keyboard and mouse | `press`, `mouse-click`, `mouse-move`, `mouse-wheel` | `press` and text keys work on X11 and Wayland; pointer commands are X11 and `--headed`. See `references/display-servers-and-input.md` |
| Held input | `key-down`, `key-up`, `mouse-down`, `mouse-up` | Fail closed: `key-down` and `key-up` with `ACTION_NOT_SUPPORTED`; `mouse-down` and `mouse-up` with `POLICY_DENIED` headless and `ACTION_NOT_SUPPORTED` with `--headed` |
| App and window | `launch`, `close-app`, `list-windows`, `list-apps`, `focus-window`, `move-window`, `resize-window`, `minimize`, `maximize`, `restore` | Launch and listing work on X11 and Wayland; native Wayland window control and graceful close refuse with cause `wayland_session`. See `references/windows-and-lifecycle.md` |
| Displays | `list-displays` | Works |
| Capture | `screenshot` | X11: a window without permission; a display only under a non-compositing manager. Wayland: portal consent, with display cropping. With no selector the primary display is returned. See `references/capture-and-clipboard.md` |
| Clipboard | `clipboard-get`, `clipboard-set`, `clipboard-clear` | Works; text, images and file lists. Wayland needs `wl-clipboard` |
| Wait | `wait` | Works for ms, `--element`, `--window`, `--text`, `--menu`, `--menu-closed`, `--event`. Menu waits share the menu-surface detector; `--notification` waits for a new notification on GNOME |
| Notifications | `list-notifications`, `dismiss-notification`, `dismiss-all-notifications`, `notification-action` | GNOME Shell only; list headless, mutations `--headed` with identity guards. See the shell reference |
| Shell surfaces | `open-system-surface` | GNOME: `system-tray`, `action-center`, `quick-settings`; opening popups requires `--headed`. See the shell reference |
| Cursor overlay | `cursor-overlay` | Renders on X11: `enable` returns `data.rendered: true`; Wayland saves the setting with `data.rendered: false` |
| System | `status`, `permissions`, `version`, `batch`, `skills`, `session`, `trace` | Works |

## First Contact

- **Run `agent-desktop status` first.** An empty `supported_surfaces` means the
  accessibility bus did not answer, not that the desktop is empty. Read
  `references/bus-and-permissions.md` and do not restart a bus you did not start.
- **`--app` takes the name `list-apps` prints** (`gnome-calculator`,
  `nautilus`) or the registry name (`org.gnome.Nautilus`), never a window title.
  A desktop-entry name such as `Files` is accepted once the entry exists. The
  rule differs per command: `list-windows --app` is exact (aliases included),
  while `list-apps --app` combines a substring of the listed name with exact
  adapter aliases, ignoring case.
- **Window ids are opaque addresses** of 34 to 70 characters, valid for the
  life of the window's process. Run `list-windows` again after a restart.
  `list-apps` rows carry `bundle_id` (desktop id when known) and `process_instance`.
- **GTK4 apps nest deeply.** `--max-depth` and `--skeleton` count semantic
  levels, so empty layout wrappers add output levels. `-i --compact` still
  shows nested nameless `group` nodes: read past them.
- **A window appears before its content.** `wait --window` first, then
  `wait --text`. `--text` matches a subtree, so ancestors of the text match.
  A `--timeout` shorter than the first read returns `TIMEOUT`, never a
  partial tree.
- **Chromium, Electron and Qt apps need a launch flag or variable** to appear
  on the bus. See `references/troubleshooting.md`.
- **Headed ref actions need a window that can take focus.** A window the
  window manager never activated ends `activation_not_granted`. See
  `references/windows-and-lifecycle.md`.
- **A GTK4 secret field can show its text.** Only a password role is withheld.
  Treat a GTK4 entry with hidden characters as readable by every observation
  command.
- **Blocked key combos refuse without `--force`.** See
  `references/display-servers-and-input.md`.
- **After opening a menu, inspect `data.surfaces`.** A `menu` entry leads to
  `snapshot --surface menu --app <name> -i`. If absent, use `wait --menu`
  and `list-surfaces` before assuming the opener failed. See the shell reference.
- **GNOME shell input has its own route on Wayland.** Headed shell clicks can
  work while ordinary app pointer commands refuse. Notifications need the
  GNOME notification service; tray snapshots also need a StatusNotifierWatcher.
- **The state root must be private.** A group-writable or world-accessible
  `AGENT_DESKTOP_HOME` or `~/.agent-desktop` makes `snapshot` fail with
  `INTERNAL`; run `chmod 700` on it or let the tool create it.

## What differs on Linux

- Ref actions are semantic AT-SPI calls that never move the pointer or change
  the active window. `--headed` adds physical input and focus.
- Under `--headed`, `click` and `type` use physical input on X11 (a real click
  or keys, step `physical_synthetic`, `verified: false`), falling back to the
  semantic call only after a safe unsupported-input or geometry refusal on that
  X11 path. On Wayland the CLI rejects headed pointer actions before that
  adapter fallback: ordinary-app headed
  pointer commands (`click`, `hover`, `double-click`, `mouse-*`, wheel) are refused
  at once with `ACTION_NOT_SUPPORTED`, cause `wayland_session`,
  `physical_delivery_started: false`. A headed `type` sends Wayland keys through
  `RemoteDesktop.type_text`. Windows works the other way round. Read the step in
  `data` to see which ran.
- Headless `type` writes through `EditableText`. A terminal's text node is a
  `textfield` named `Terminal` whose only action is `RightClick`; headless `type`
  on it is `POLICY_DENIED` with check `supported_action`, and its value is null.
  It receives keys only under `--headed`.
- In key combos `cmd` and `meta` are the Super key. Key names are the macOS and
  Windows names plus X keysym names.
- X11 and Wayland differ in input, capture and window control; the Wayland
  limits are stated where they apply.
- Window ops, `press --app` and `focus-window` act on X11 clients. A window on
  another workspace can switch the user's current desktop when focused.
- Held-input commands fail closed on every platform.

## References

- `references/bus-and-permissions.md`: bus bootstrap, the three bus states,
  `status` and `permissions` output, state-root rules.
- `references/display-servers-and-input.md`: X11 versus Wayland table,
  `press`, `type`, pointer, wheel, drag, blocked combos, key names.
- `references/windows-and-lifecycle.md`: `launch`, `close-app`, window ops,
  focus, workspaces, stopped apps, background `press`.
- `references/capture-and-clipboard.md`: `screenshot` on X11 and Wayland,
  the Wayland consent dialog, clipboard owner and helper.
- `references/shell-tray-and-notifications.md`: menus and GTK4 popovers, GNOME
  Overview/top bar, shell reads and input, notification identity guards, SNI
  and AppIndicator requirements and refusals. Also contains sandbox guidance.
- `references/troubleshooting.md`: symptom-to-cause map, Chromium, Electron
  and Qt flags, toolkit limits, error codes with `platform_detail`.

Read any file from the binary:
`agent-desktop skills get agent-desktop-linux references/troubleshooting.md`.
