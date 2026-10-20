# Display Servers and Input on Linux

Observation and semantic ref actions go through AT-SPI and behave the same on
X11 and Wayland. Input, window control and capture differ. This file states
how the CLI picks the display server, then what each input command does.

## How the display server is chosen

The CLI reads `WAYLAND_DISPLAY`, `DISPLAY` and `XDG_RUNTIME_DIR` (an empty value
counts as unset). A Wayland socket that accepts a connection wins, because an
Xwayland client lives on a Wayland desktop whose windows X11 calls cannot list.
A stale `WAYLAND_DISPLAY` from another session falls through to a working
`DISPLAY`. With neither reachable, display-dependent commands fail with a cause
of `not_configured`, `missing` or `refused` and list the variables they tried.

## X11 versus Wayland

| Capability | X11 | Wayland (GNOME) |
|------------|-----|-----------------|
| Observation and semantic ref actions | AT-SPI, no tool | AT-SPI, no tool |
| Keys, `press`, text keys | XTEST in process, live keymap | Mutter RemoteDesktop, keysyms |
| Pointer: `hover`, `drag`, `mouse-*`, physical click, wheel | XTEST, `--headed` | None: `ACTION_NOT_SUPPORTED` with `physical_delivery_started: false` |
| Window ops, `focus-window`, `press --app` | EWMH on the X11 display | Xwayland clients only; native windows refused |
| `screenshot` | Window, screen, whole screen | Whole desktop through a consent dialog |
| Clipboard | Native selection owner | `wl-clipboard` |

No tool is needed on X11 beyond the binary and its clipboard helper. `xdotool`,
`ydotool`, `xclip` and `wmctrl` are not used. `wl-clipboard` is the one outside
tool, and only on Wayland. AT-SPI actions need no input device on either server.

## Headed and headless

Ref actions are headless and semantic by default: the pointer and the active
window never change. Every X11 synthetic pointer path moves the user's real
cursor, so pointer commands (`hover`, `drag`, `double-click`, `triple-click`,
`mouse-move`, `mouse-click`, `mouse-wheel`) require global `--headed`.

Under `--headed`, `click` and `type` try physical input first and fall back to
the semantic call only when the pointer is unsupported (Wayland) or the target
cannot be aimed. Read the step in `data`: `physical_synthetic` means a real
event with `verified: false`, because XTEST has no read-back; `semantic_api` means
the node's own action ran and the pointer did not move. Windows works the other
way round (semantic first), so do not carry that assumption across platforms.

Before a physical ref-addressed key or click, the target frame must report
`active` within a short wait; a frame that never does fails with
`check: window_active`. Headless semantic actions do not need the frame active:
a headless `type` into a background window succeeds. A headless `press --app` to
a background app fails with `display_active_window`, and `press --window-id` on
another window of the active process fails with `other_window_of_target`.
The gate is necessary, not sufficient: while a popup menu holds the grab, keys
reach the menu even if the frame reads active, so key steps stay unverified.
Pointer forms aim at the centre of the ref's bounds, which the adapter withholds
where the position is not proved. Read the result with a fresh `get`.

## Keys and `press`

```bash
agent-desktop press ctrl+a
agent-desktop press return --app gnome-text-editor
```

- Key names are the macOS and Windows names (`return`, `escape`, `tab`,
  `space`, `home`, `pageup`, `f1`-`f24`) plus punctuation names (`comma`,
  `period`, `greater`, `leftbracket`) and X keysym names such as `Page_Up`.
  `delete` is the backspace key; `forwarddelete` is the forward Delete key.
  A single character is its own key. An unknown name lists the accepted forms.
- `cmd` and `meta` are the Super key.
- `press` with no `--app` sends to the focused window. `press --app` or
  `--window-id` delivers only while the target process owns the active window;
  a headless press to a background or minimized target is refused. See
  `references/windows-and-lifecycle.md`.
- Keys carry the Shift level the live keymap needs. A character the layout
  cannot produce is typed through spare keycodes, so `type` handles accents and
  non-Latin text.

### Blocked combos

These shortcuts take the foreground, close a window or switch the session, so
`press` refuses them with `POLICY_DENIED` and `not_delivered` before any key is
sent. `--force` sends them:

- `alt+f4`, `alt+tab`, `alt+f2`
- `meta+l`, `meta+d`, `meta+tab`, and a bare `super`
- `ctrl+alt+t`, `ctrl+alt+delete`, `ctrl+alt+forwarddelete`, `ctrl+alt+backspace`,
  `ctrl+alt+l`
- `ctrl+alt+f1` through `ctrl+alt+f12`

`ctrl+q` and similar app shortcuts are sent. The list is in the adapter, not
in the CLI help; `agent-desktop press <combo>` shows the refusal per combo.

## `type`

Headless `type @ref text` writes through `EditableText.InsertText`. Under `--headed` it
sends physical keys first (step `XTEST.type_text`, `physical_synthetic`; the result still
reads the field back when it can) and falls back to `EditableText` only where keys
cannot be sent. A terminal's text node is a `textfield` named `Terminal` with only
`RightClick`: headless `type` on it answers `POLICY_DENIED` with check
`supported_action`, and it takes keys only under `--headed`. Its value is null, so
terminal output cannot be read through a ref; verify what a terminal command did
through files or another app.

The first `type` into a fresh document changes the window title. The ref from the
snapshot still resolves: `get` on it reads the typed value. Take a fresh snapshot
only when the UI you address has changed for another reason.

## Wayland keys

Wayland keys go through `org.gnome.Mutter.RemoteDesktop` on the session bus.
Each call opens its own session, sends keysyms, waits 150 ms so the compositor
drains them, and stops the session, releasing any key still held.

- **The lead Shift.** The compositor drops the first key of a session, so the
  first key press of a call sends a lone Shift tap before the real keys. The tap
  does not reach the application: a GTK3 entry logged only the real key, not the
  Shift. Send the keys you mean; the lead tap is not an extra shortcut.
- **Characters.** Only printable ASCII, function, modifier and vendor keysyms are
  accepted. Anything else returns `ACTION_NOT_SUPPORTED` with cause
  `character_outside_layout`; set the value directly with `set-value` instead.
  The rule is measured on a US layout, so a character your keymap holds can be
  refused. A refusal is safe: nothing was sent.
- **Pointer.** AT-SPI gives no global window origin on Wayland, so every pointer
  command refuses. A `--headed` ref `type` or `press` stops earlier: core waits for
  the node's bounds, finds none, and ends in `TIMEOUT` (`actionability_timeout`)
  after 5 s with no keys sent. Use headless `type` or `set-value`, or a global
  `press`, which sends keys with no element gate.
- **No active window.** On a Wayland session with no seat keyboard no window carries
  `ACTIVE`, so `focus-changed` waits time out.

## Wheel and scroll

```bash
agent-desktop --headed mouse-wheel --x 640 --y 400 --dy 5
```

- A positive `--dy` scrolls up and a negative one down; a positive `--dx` scrolls
  right and a negative one left, on all three platforms. A negative value
  works as `--dy -5` or `--dy=-5`.
- With neither `--dx` nor `--dy` the default is three lines down. The vertical
  default applies only when `--dx` is absent.
- `scrolled: true` is not a read-back. The adapter cannot tell whether the page
  moved, so read the scrolled content afterwards.
- `scroll @ref --headed` sends wheel clicks at the centre of the target's bounds,
  and only for a node that carries the `Scroll` capability. A wheel click is
  page-size dependent (about two rows on GTK, exactly three items on Qt), so
  `--amount` counts clicks, not rows. A `scrollarea` with no ref cannot take
  `scroll @ref`; use `scroll-to` on a child or `mouse-wheel` at coordinates.
- `scroll-to` scrolls through the bar on GTK3 and GTK4 and through `ScrollTo` on
  Chromium. It does not work on Qt: use `scroll --headed`. A container with no
  scrollable axis has no ref.

## Drag

`drag` picks up, moves along an interpolated path, holds still for a dwell, and
releases. The dwell holds still on purpose: a hovered folder or drop target can
open under the pointer, and any later motion then lands inside it and copies
instead of moving. The default drop delay on Linux is 150 ms; pass
`--drop-delay` only to give a slow drop target time to arm. The result reports the
gesture, not the outcome: verify the effect (a file moved, a list changed) with
a fresh `snapshot`.

## Interrupted input

The X server keeps XTEST-pressed buttons when the client dies. The CLI journals
what it holds under `$XDG_RUNTIME_DIR/agent-desktop-input`, and the next call on
that display releases what a killed process left before it sends its own events.
If a button seems stuck after you killed a command, run any input command
(`mouse-move --headed`, for instance) and it is released.

## Held input

`key-down` and `key-up` fail closed with `ACTION_NOT_SUPPORTED` and
`delivery: unknown`. `mouse-down` and `mouse-up` without `--headed` fail with
`POLICY_DENIED` and `not_delivered`; with `--headed` they fail with
`ACTION_NOT_SUPPORTED`. No event is emitted. Compose `press` with modifiers
(`ctrl+shift+a`) and `drag` instead.
