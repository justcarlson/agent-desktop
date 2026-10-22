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
| App pointer: `hover`, `drag`, `mouse-*`, physical click, wheel | XTEST, `--headed` | None: `ACTION_NOT_SUPPORTED` with `physical_delivery_started: false` |
| Window ops | EWMH on the X11 display | Xwayland clients only; native windows and un-minimize refused |
| `focus-window`, headed scoped `press` | EWMH activation, then keys for `press` | Refused with `ACTION_NOT_SUPPORTED`, cause `wayland_session`, including Xwayland clients |
| Headless scoped `press` | Live display and frame focus guards; no activation | Live frame focus guard; no activation |
| `screenshot` | Window or display; primary display by default | Portal consent; primary display by default, `--screen` selects a monitor |
| Clipboard | Native selection owner | `wl-clipboard` |
| GNOME Shell ref click / popup opener | `--headed`, shared structural receiver proof required | Refused: receiver cannot be proven; see shell reference |

No tool is needed on X11 beyond the binary and its clipboard helper. `xdotool`,
`ydotool`, `xclip` and `wmctrl` are not used. `wl-clipboard` is the one outside
tool, and only on Wayland. AT-SPI actions need no input device on either server.

## Headed and headless

Ref actions are headless and semantic by default: the pointer and the active
window never change. Every X11 synthetic pointer path moves the user's real
cursor, so pointer commands (`hover`, `drag`, `double-click`, `triple-click`,
`mouse-move`, `mouse-click`, `mouse-wheel`) require global `--headed`.

Under `--headed`, ref `click`, `right-click` and `type` attempt physical input
first. Physical success reports `physical_synthetic` with `verified: false`,
because XTEST has no read-back. The adapter falls back to the semantic action
only when the physical attempt returns `ACTION_NOT_SUPPORTED` or a geometry
refusal (`check: geometry`) with `not_delivered`; it records the skipped
`physical_input` step and refusal in `details.physical_fallback`. Focus, policy,
timeout, uncertain and partial-delivery refusals never authorize fallback. The
CLI's Wayland pointer gate runs before that fallback. On
Wayland ordinary-app headed pointer commands are refused at once with
`ACTION_NOT_SUPPORTED`, cause `wayland_session` and
`physical_delivery_started: false` (measured for `click`, `hover`,
`double-click`, `mouse-wheel`, `mouse-down` and `select`). A headed `type` sends Wayland
keys through `RemoteDesktop.type_text` after the Mutter capability probe succeeds. Headless `type` writes
through `EditableText`. A headless action reports `semantic_api` and leaves the
pointer still. Windows works the other way round (semantic first), so do not
carry that assumption across platforms.

Before a physical ref-addressed key, the target frame must report
`active` within a short wait; a frame that never does fails with
`check: window_active`. Headed `type` and ref-addressed key presses also require
the target or its proven internal editor to report `focused`: before each X11 text
plan, and on Wayland before the bootstrap Shift and each subsequent key-down. The
internal-editor proof requires an editable target, a same-owner hit, a parent chain
back to the target, and a focused editor that advertises `EditableText`.
Closing a GTK4 menu can leave the document unfocused even while its window is
active. GNOME 46's GTK4 bridge rejects AT-SPI `Component.GrabFocus` as unsupported.
When neither the target nor its proven internal editor is focused, the tool
cannot restore that document's focus automatically: focus the document yourself,
or use headless `type` through `EditableText`. A `target_focus` refusal with
`physical_delivery_started: false` sent no keys; waiting does not supply the
missing focus action. Headless semantic actions do not need the frame active:
a headless `type` into a background window succeeds. A headless `press --app` to
a background app on X11 fails with `display_active_window`. When headless
`press --window-id` cannot confirm the named window's exact focus, it returns
`STALE_REF`, `check: exact_window_focus`, `cause: exact_window_focus_unconfirmed`.
Bring that window forward, refresh `list-windows`, and retry.
The gate is necessary, not sufficient: while a popup menu holds the grab, keys
reach the menu even if the frame reads active, so key steps stay unverified.
Pointer forms aim at the centre of the ref's bounds, which the adapter withholds
where the position is not proved. Read the result with a fresh `get`.
Ref-addressed double and triple clicks prove the pointer receiver before every
press, so a window raised by the first click is seen before the second. The
checks share a 300 ms press-to-press budget, inside the 400 ms in which GTK, Qt
and GNOME read two presses as one gesture. A changed receiver, or a check that
outruns the budget (`details.kind: multi_click_window`), stops the gesture with
`ACTION_FAILED`, the clicks already sent in `delivered_clicks` and
`delivered_unverified`; inspect the app before retrying.
`hover @ref` proves the receiver the same way before the move. A covered target
or an unproven receiver is refused with `ACTION_FAILED` and `not_delivered`, and
the cursor does not move. `hover --xy` has no target element to prove.

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
  A bare upper-case letter (`H`) is the shifted key (`shift+h`); with other
  modifiers the case is ignored (`ctrl+A` is `ctrl+a`).
- In combos, `meta`, `cmd`, `command`, `super` and `win` name the Super modifier.
  For a bare key use `super` or `meta`, not `cmd`, `command` or `win`.
- `press` without `--app` or `--window-id` sends to the focused window.
  Headless `press --app` requires the target process to own the active window;
  `press --window-id` also requires the exact window's focus. A background or
  minimized target is refused. See
  `references/windows-and-lifecycle.md`.
- Keys carry the Shift level the live keymap needs. `press` takes only keys the
  active layout maps and the named keysyms above. A character the layout cannot
  produce (`é`, `你`) is refused and not delivered (`INVALID_ARGS` on X11,
  `ACTION_NOT_SUPPORTED` with `character_outside_layout` on Wayland); `type`
  sends it through spare keycodes, so use `type` for accents and non-Latin text.

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

Empty typing sends no keys and does not request focus on the target. It reports
`not_delivered`; `--headed` can still activate the source window before the
empty insertion is dispatched.

Headed typing proves focus on the field or its proven internal editor. Focus
elsewhere in the window does not authorize ref typing. Verify the resulting
value or application effect after typing.

Headless `type @ref text` writes through `EditableText.InsertText`. Under `--headed` it
sends physical keys first (step `XTEST.type_text` on X11, `RemoteDesktop.type_text` on
Wayland, `physical_synthetic`; the result still reads the field back when it can)
and falls back to `EditableText` only where keys cannot be sent. A terminal's
text node is a `textfield` named `Terminal` with only
`RightClick`: headless `type` on it answers `POLICY_DENIED` with check
`supported_action`, and it takes keys only under `--headed`. Its value is null, so
terminal output cannot be read through a ref; verify what a terminal command did
through files or another app.

X11 physical `type` refuses control characters, including Tab and newline, before
sending any text: those keys can move focus or submit a form inside a text burst.
Use headless `type` or `set-value` for literal text containing those characters;
use an explicit `press tab` or `press return` when navigation or submission is
intended.

X11 physical text checks the live keyboard state before sending each chunk. If
that state is unsupported, the physical route refuses before input; an editable
field may then use the documented semantic fallback.

The first `type` into a fresh document changes the window title. The ref from the
snapshot still resolves: `get` on it reads the typed value. Take a fresh snapshot
only when the UI you address has changed for another reason.

## Wayland keys

Wayland keys use one detected backend: `org.gnome.Mutter.RemoteDesktop` on the
session bus. Before creating a session or emitting an event, the adapter probes
that interface's typed `Version` property. A missing service or capability
returns `PLATFORM_NOT_SUPPORTED`, `not_delivered` and
`physical_delivery_started: false`; another compositor never falls back to X11.
A probe timeout remains `TIMEOUT`.
Calls send keysyms and wait 150 ms so the compositor drains them. A long-lived
host can retain the keyboard session for later deliveries; a new CLI process
opens its own session. Ending a session releases its held keys.

- **The lead Shift.** A newly opened key session sends a lone Shift tap and
  waits 100 ms before the real keys so the compositor can attach its keyboard.
  It inserts no text, but a handler that watches bare Shift can observe it.
  Reusing a still-live session skips the lead tap.
- **Characters.** Only printable ASCII, function, modifier and vendor keysyms are
  accepted. Anything else returns `ACTION_NOT_SUPPORTED` with cause
  `character_outside_layout`; set the value directly with `set-value` instead.
  The rule is measured on a US layout, so a character your keymap holds can be
  refused. A refusal is safe: nothing was sent.
- **App pointer.** AT-SPI gives no global window origin on Wayland, so ordinary
  app pointer commands refuse at once with `ACTION_NOT_SUPPORTED`, cause `wayland_session`,
  before anything is sent. A `--headed` ref `type` sends keys through
  `RemoteDesktop.type_text`. GNOME Shell ref clicks and
  popup openers also require the shared structural receiver proof and refuse
  when that proof is unavailable; see
  `references/shell-tray-and-notifications.md`.
- **No active window.** On a Wayland session with no seat keyboard no window carries
  `ACTIVE`, so `focus-changed` waits time out.

## Wheel and scroll

```bash
agent-desktop --headed mouse-wheel --x 640 --y 400 --dy 5
```

- A positive `--dy` scrolls up and a negative one down; a positive `--dx` scrolls
  right and a negative one left, on all three platforms. A negative value
  works as `--dy -5` or `--dy=-5`.
- With no `--dy`, the default is three lines down when `--dx` is zero (including
  explicit `--dx 0`), and zero vertical lines when `--dx` is nonzero.
- The result carries `dy`, `dx`, `target`, `steps` and `disposition`, with no
  `scrolled` success flag. Wheel delivery is `delivered_unverified`; read the
  scrolled content afterwards.
- `scroll @ref --headed` sends wheel clicks at the centre of the target's bounds,
  and only for a node that carries the `Scroll` capability. A wheel click is
  page-size dependent (about two rows on GTK, exactly three items on Qt), so
  `--amount` counts clicks, not rows. A `scrollarea` with no ref cannot take
  `scroll @ref`; use `scroll-to` on a child or `mouse-wheel` at coordinates.
  Each wheel burst uses click's pointer gate at that point. A hit-test result
  naming an interceptor refuses the burst; an unknown receiver refuses with
  `pointer_hit_unproven`. If earlier bursts were sent, the error reports their
  `delivered_clicks`, `delivered_unverified` and unsafe retry. Inspect the content
  before deciding what to do next.
- `scroll-to` scrolls through the bar on GTK3 and GTK4 and through `ScrollTo` on
  Chromium. It does not work on Qt: use `scroll --headed`. A container with no
  scrollable axis has no ref. If target visibility is unknown, `scroll-to`
  refuses before scrolling. Equally ranked scroll bars refuse with
  `AMBIGUOUS_TARGET` before a value write. A scroll that moved the viewport but failed to reveal the target
  is unsafe to retry; inspect the target and disposition first.

## Drag

`drag` picks up, moves along an interpolated path, holds still for a dwell, and
releases. The dwell holds still on purpose: a hovered folder or drop target can
open under the pointer, and any later motion then lands inside it and copies
instead of moving. The default duration is 300 ms and the minimum is 100 ms: a
shorter `--duration` is refused with `INVALID_ARGS` and `not_delivered`, because
a toolkit such as GTK needs several motion events to start a drag (Files and a
GTK3 drop target both ignored a 0 ms or 30 ms gesture and accepted 60 ms and up). The default drop delay on Linux is 150 ms; pass
`--drop-delay` only to give a slow drop target time to arm. The result reports the
gesture, not the outcome: verify the effect (a file moved, a list changed) with
a fresh `snapshot`.

## Interrupted input

The X server keeps XTEST-pressed buttons when the client dies. The CLI journals
what it holds under `$XDG_RUNTIME_DIR/agent-desktop-input`, and the next call on
that display releases what a killed process left before it sends its own events.
If a button seems stuck after you killed a command, run any input command
with valid arguments, such as `agent-desktop --headed mouse-move --xy 640,400`,
to release it before the new input.

## Held input

`key-down` and `key-up` fail closed with `ACTION_NOT_SUPPORTED` and
`delivery: unknown`. `mouse-down` and `mouse-up` without `--headed` fail with
`POLICY_DENIED` and `not_delivered`; with `--headed` they fail with
`ACTION_NOT_SUPPORTED`. No event is emitted. Compose `press` with modifiers
(`ctrl+shift+a`) and `drag` instead.
