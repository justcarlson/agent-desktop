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

Requires Ubuntu 24.04+ with a graphical session of the signed-in user and a
running AT-SPI accessibility bus, on x86_64 or aarch64. Other distributions
need the at-spi2-core bridge fix dc0dc33; Ubuntu 22.04's bridge aborts on
`Action.GetActions`. The archive loader floor is glibc 2.35.
Run the CLI inside the same desktop session as the target apps: the
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
| Keyboard and mouse | `press`, `mouse-click`, `mouse-move`, `mouse-wheel` | `press` and text keys work on X11 and capability-detected Mutter Wayland sessions; pointer commands are X11 and `--headed`. See `references/display-servers-and-input.md` |
| Held input | `key-down`, `key-up`, `mouse-down`, `mouse-up` | Fail closed: `key-down` and `key-up` with `ACTION_NOT_SUPPORTED`; `mouse-down` and `mouse-up` with `POLICY_DENIED` headless and `ACTION_NOT_SUPPORTED` with `--headed` |
| App and window | `launch`, `close-app`, `list-windows`, `list-apps`, `focus-window`, `move-window`, `resize-window`, `minimize`, `maximize`, `restore` | Launch and listing work on X11 and Wayland; `focus-window` requires X11. Other window control and graceful close need an X11 client. See `references/windows-and-lifecycle.md` |
| Displays | `list-displays` | Works |
| Capture | `screenshot` | X11: a window without permission; a display only under a non-compositing manager. Wayland: portal consent, with display cropping. With no selector the primary display is returned. See `references/capture-and-clipboard.md` |
| Clipboard | `clipboard-get`, `clipboard-set`, `clipboard-clear` | Works; text, images and file lists. Wayland needs `wl-clipboard` |
| Wait | `wait` | Works for ms, `--element`, `--window`, `--text`, `--menu`, `--menu-closed`, `--event`. Menu waits share the menu-surface detector; `--notification` refuses because notification identity cannot be established |
| Notifications | `list-notifications`, `dismiss-notification`, `dismiss-all-notifications`, `notification-action` | Refuse: an admitted backend returns `ACTION_NOT_SUPPORTED` for unproven identity; a service with a different owner than the shell returns `PLATFORM_NOT_SUPPORTED`. See the shell reference |
| Shell surfaces | `open-system-surface` | Watcher-backed `system-tray`; `action-center` needs proven role ancestry. Unproven popup containers and openers refuse before input. See the shell reference |
| Cursor overlay | `cursor-overlay` | Requires an active session; see below. X11 enable returns `data.rendered: true`; Wayland saves the setting with `data.rendered: false` |
| System | `status`, `permissions`, `version`, `batch`, `skills`, `session`, `trace` | Works |

## First Contact

- **Run `agent-desktop status` first.** An empty `supported_surfaces` means the
  accessibility bus did not answer, not that the desktop is empty. Read
  `references/bus-and-permissions.md` and do not restart a bus you did not start.
- **`--app` takes the name `list-apps` prints** (`gnome-calculator`,
  `nautilus`) or a desktop-entry name/id backed by verified executable and
  argument identity, never a window title or an arbitrary registry label.
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
- **A window appears before its content.** `wait --window "Text Editor" --app gnome-text-editor` first, then
  `wait --text`. Web-engine detection reads the application's AT-SPI toolkit
  metadata, independent of executable names or installed payload files. Window
  observations wait for three equal full reads and a web-document role (or a
  requested depth cut), at any tree size. A shell without document evidence
  stays incomplete when the wait budget ends. `--text` matches a subtree, so
  ancestors of the text match. A `--timeout` shorter than the first read returns
  `TIMEOUT`, never a partial tree.
- **Geometry-only refs require unchanged geometry.** A changed size, even one
  pixel, cannot prove identity. Position and size must both match in the
  recorded coordinate frame; screen-coordinate refs need refreshing after a window move.
  Take a fresh snapshot after layout changes and use stable identity where
  available. Indistinguishable siblings remain `AMBIGUOUS_TARGET`. The tab
  pages of one window are such siblings when the toolkit marks every page
  showing: the text views of a Text Editor window with three tabs share one
  rectangle and one state, so a ref to any of them answers `AMBIGUOUS_TARGET`
  with `candidate_count` 3 until the extra tabs are closed. A ref belongs to the
  window it was taken in: two windows with the same title each resolve their own
  refs.
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
- **After opening a menu-role popup, inspect `data.surfaces`.** A `menu` entry leads to
  `snapshot --surface menu --app <name> -i`. If absent, use `wait --menu`
  and `list-surfaces` before assuming the opener failed. A list-backed dropdown
  is part of the window, not a menu surface; see the shell reference.
- **Shell pointer input requires receiver proof.** Headed shell clicks use the
  same structural hit test as app clicks. An unknown receiver or overlapping
  sibling refuses before any pointer event; Wayland shell clicks therefore refuse.
  Notification commands refuse unproven protocol identity; tray snapshots also need a StatusNotifierWatcher.
- **The state root must be private.** A group-writable or world-accessible
  `AGENT_DESKTOP_HOME` or `~/.agent-desktop` makes `snapshot` fail with
  `INTERNAL`; run `chmod 700` on it or let the tool create it.

## Wait for a window

Wait for an app window to appear (`--window` requires a title or title substring):

```bash
agent-desktop wait --window "Text Editor" --app gnome-text-editor --timeout 10000
```

Wait for an app window to close after the next close action. This example uses
Calculator so no unsaved-document prompt interrupts the close. Batch records
the window baseline before closing it:

```bash
agent-desktop batch '[{"command":"close-app","args":{"app":"gnome-calculator"}},{"command":"wait","args":{"event":"window-closed","app":"gnome-calculator","timeout":10000}}]' --stop-on-error
```

To observe a close performed by someone else, start
`agent-desktop wait --event window-closed --app gnome-text-editor --timeout 10000`
before they close the window. There is no `--window-closed` flag.

## Cursor overlay

Start a session and pass its returned ID to every overlay command:

```bash
session_id=$(agent-desktop session start --name docs-example | jq -r '.data.session_id')
agent-desktop cursor-overlay enable --session "$session_id"
agent-desktop cursor-overlay disable --session "$session_id"
agent-desktop session end "$session_id"
```

Without a session,
`cursor-overlay enable` and `disable` return `INVALID_ARGS`. On X11, enable
returns `data.rendered: true`; on Wayland it saves the setting and returns
`data.rendered: false` without drawing an overlay.

## What differs on Linux

- Firefox/Gecko `select` can change the selected value without firing `change`
  or `input` events, so dependent page handlers do not run. The result stays
  `delivered_unverified` even when value readback matches. Check the dependent
  application effect before continuing; value readback alone is no commit proof.
- Ref actions are semantic AT-SPI calls that never move the pointer or change
  the active window. `--headed` enables physical-first input for ref `click`,
  `right-click` and `type`, with the required focus permission.
- Under `--headed`, these actions attempt physical input first. The adapter
  falls back to the semantic action only after `ACTION_NOT_SUPPORTED` or a
  geometry refusal (`check: geometry`) with `not_delivered`; it records the
  skipped physical step and refusal in `details.physical_fallback`. Focus,
  policy, timeout, uncertain or partial-delivery refusals never fall back.
  Physical success reports `physical_synthetic` and is unverified.
  On Wayland the CLI rejects headed pointer actions before that
  adapter fallback: ordinary-app headed
  pointer commands (`click`, `hover`, `double-click`, `mouse-*`, wheel) are refused
  at once with `ACTION_NOT_SUPPORTED`, cause `wayland_session`,
  `physical_delivery_started: false`. A headed `type` sends Wayland keys through
  `RemoteDesktop.type_text` only after detecting the Mutter RemoteDesktop
  capability; other compositors return `PLATFORM_NOT_SUPPORTED` before input.
  Windows works the other way round. Read the step in
  `data` to see which ran.
- Headless `type` writes through `EditableText`. A terminal's text node is a
  `textfield` named `Terminal` whose only action is `RightClick`; headless `type`
  on it is `POLICY_DENIED` with check `supported_action`, and its value is null.
  It receives keys only under `--headed`.
- In key combos `cmd` and `meta` are the Super key. Key names are the macOS and
  Windows names plus X keysym names.
- X11 and Wayland differ in input, capture and window control; the Wayland
  limits are stated where they apply.
- Window ops act on X11 clients; `focus-window` and headed scoped `press` require
  an X11 session. Headless scoped `press` requires live target focus on either
  display server. Focusing a window on another workspace can switch the user's
  current desktop.
- Ref geometry compares position and size only in its recorded coordinate frame.
  A window move can make screen-relative geometry unable to distinguish twins;
  refresh the snapshot rather than retrying the old ref. Linux exact FFI refs
  carrying bounds or a bounds hash require window-relative geometry, even when
  the ref also carries a name or identifier. Geometry-free identity refs remain
  supported.
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
