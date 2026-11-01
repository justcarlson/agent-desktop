# Display Servers and Input on Linux

Observation and semantic ref actions use AT-SPI and behave the same on X11 and
Wayland. Input, window control and capture differ.

## How the display server is chosen

The CLI reads `WAYLAND_DISPLAY`, `DISPLAY` and `XDG_RUNTIME_DIR` (an empty value
counts as unset). A Wayland socket that accepts a connection wins, because an
Xwayland client lives on a Wayland desktop whose windows X11 calls cannot list.
A stale `WAYLAND_DISPLAY` from another session falls through to a working
`DISPLAY`. With neither reachable, display-dependent commands fail with cause
`not_configured`, `missing` or `refused` and list the variables they tried.

### Hyprland

On Wayland the CLI binds one Hyprland instance before it reads Hyprland's own
state. It checks `$XDG_RUNTIME_DIR/hypr/<signature>/.socket.sock` and binds the
instance only when that IPC socket answers and the Wayland socket is served by
the same compositor process. The compositor must also advertise
`hyprland_toplevel_mapping_manager_v1`. The bind has a 1 s budget across every
socket it tries, and the CLI never runs `hyprctl`.

- With `HYPRLAND_INSTANCE_SIGNATURE` set, only that instance is read, even
  when others run. A signature whose instance has exited refuses with
  `hyprland_mismatch`; the CLI does not search for another.
- With no display variables set, as in an SSH shell, the CLI searches the
  user's runtime directory (`/run/user/<uid>` when `XDG_RUNTIME_DIR` is
  unset). One live instance is bound and the session is treated as Wayland.
  Stale `hypr/` folders are skipped.
- A Wayland session whose socket another compositor serves keeps the generic
  Wayland behavior, and so does a Hyprland without the mapping global.

The bind refuses instead of guessing. `list-windows`, `list-displays` and every
command that resolves a window id report the refusal:

| Cause in `details.display_unavailable` | Code | When |
|---|---|---|
| `hyprland_ambiguous` | `PLATFORM_NOT_SUPPORTED` | Several instances answer, none is named, and `WAYLAND_DISPLAY` is unset or served by one of them. `details.signatures` lists them; set `HYPRLAND_INSTANCE_SIGNATURE` to choose |
| `hyprland_mismatch` | `PLATFORM_NOT_SUPPORTED` | The named instance is not live, the named instance and `WAYLAND_DISPLAY` belong to different compositors, the instance serves no Wayland socket, or the bound compositor was replaced |
| `silent` | `TIMEOUT`, retryable | An IPC or Wayland socket accepted the connection and did not answer in time. `details.detail` names the socket |

What a bound Hyprland adds to `list-windows`, to resolving a window id, to
`focus-window` and to `close-app` is in `windows-and-lifecycle.md`. Keys go
through a virtual keyboard; see [Wayland keys](#wayland-keys). Pointer input
is in [Pointer on Hyprland](#pointer-on-hyprland). Other commands on Hyprland
behave as on a generic Wayland session.

## X11 versus Wayland

| Capability | X11 | Wayland (GNOME) |
|------------|-----|-----------------|
| Observation, semantic ref actions | AT-SPI, no tool | AT-SPI, no tool |
| Keys, `press`, text keys | XTEST in process, live keymap | Mutter RemoteDesktop, keysyms; a bound Hyprland: a virtual keyboard |
| App pointer: `hover`, `drag`, `mouse-*`, physical click, wheel | XTEST, `--headed` | Bound Hyprland: a virtual pointer, `--headed`; see [Pointer on Hyprland](#pointer-on-hyprland). Other compositors: none, `ACTION_NOT_SUPPORTED` with `physical_delivery_started: false` |
| Window ops | EWMH on the X11 display | Bound Hyprland: every window through the compositor, move and resize floating only. Other compositors: Xwayland clients only; native windows and un-minimize refused |
| `focus-window`, headed scoped `press` | EWMH activation, then keys for `press` | Refused with `ACTION_NOT_SUPPORTED`, cause `wayland_session`, Xwayland clients included. On a bound Hyprland, `--headed focus-window`, headed scoped `press`, headed ref actions and `launch --activate` activate through the compositor |
| Headless scoped `press` | Live display and frame focus guards, no activation | Live frame focus guard, no activation |
| `screenshot` | Window or display; primary display by default | Bound Hyprland: window or display, no dialog. Other compositors: portal consent, display only. Primary display by default, `--screen` picks a monitor |
| `list-displays` | RandR monitors | Mutter `DisplayConfig`; see [Displays on Wayland](#displays-on-wayland) for other compositors |
| Clipboard | Native selection owner | `wl-clipboard` |
| GNOME Shell ref click, popup opener | `--headed`, receiver proof from the shell's own tree | Ref click refused with `wayland_session`, popup opener refused as unproven; see shell reference |

X11 needs no tool beyond the binary and its clipboard helper. `xdotool`,
`ydotool`, `xclip` and `wmctrl` are not used. `wl-clipboard` is the one outside
tool, on Wayland only. AT-SPI actions need no input device on either server.

## Headed and headless

Ref actions are headless and semantic by default: the pointer and the active
window never change, and a headless action reports `semantic_api`. Pointer
commands (`hover`, `drag`, `double-click`, `triple-click`, `mouse-move`,
`mouse-click`, `mouse-wheel`) need global `--headed`, because every X11
synthetic pointer path moves the user's real cursor.

- Under `--headed`, ref `click`, `right-click` and `type` try physical input
  first. Physical success is `physical_synthetic` with `verified: false` (XTEST
  has no read-back).
- The adapter falls back to the semantic action only when the physical attempt
  returns `ACTION_NOT_SUPPORTED` or a geometry refusal (`check: geometry`) with
  `not_delivered`; `details.physical_fallback` records the skipped
  `physical_input` step and refusal. Focus, policy, timeout, uncertain and
  partial-delivery refusals never fall back.
- On a Wayland session that is not a bound Hyprland, the CLI's pointer gate
  runs before that fallback: ordinary-app headed pointer commands are refused at
  once with `ACTION_NOT_SUPPORTED`, cause
  `wayland_session` and `physical_delivery_started: false` (`click`, `hover`,
  `double-click`, `mouse-wheel`, `mouse-down` and `select` included). A
  headed `type` sends keys through `VirtualKeyboard.type_text` on a bound
  Hyprland, and elsewhere through `RemoteDesktop.type_text` once the Mutter
  capability probe succeeds.
- Windows works the other way round (semantic first); do not carry that
  assumption across platforms.

A physical ref-addressed key needs the target frame `active` within a short
wait, else `check: window_active`. Headed `type` and ref-addressed key presses
also need the target, or its proven internal editor, `focused`: before each X11
text plan and, on Wayland, before the bootstrap Shift and each subsequent
key-down. The internal-editor proof needs an editable target, a same-owner hit,
a parent chain back to the target and a focused editor that advertises
`EditableText`.

Closing a GTK4 menu can leave the document unfocused even while its window is
active. GNOME 46's GTK4 bridge rejects AT-SPI `Component.GrabFocus` as
unsupported. If neither the target nor its proven internal editor is focused,
the tool cannot restore that document's focus automatically: focus the document
yourself, or use headless `type` through `EditableText`. A `target_focus`
refusal with `physical_delivery_started: false` sent no keys, and waiting does
not supply the missing focus action.

Headless semantic actions need no active frame: a headless `type` into a
background window works. A headless `press --app` to a background app on X11
fails with `display_active_window`. When headless `press --window-id` cannot
confirm the named window's exact focus, it returns `STALE_REF`,
`check: exact_window_focus`, `cause: exact_window_focus_unconfirmed`. Bring that
window forward, refresh `list-windows`, and retry. The active-frame gate is
necessary, not sufficient: while a popup menu holds the grab, keys reach the
menu even if the frame reads active, so key steps stay unverified.

Pointer forms aim at the centre of the ref's bounds, which the adapter withholds
where the position is not proved. Read the result with a fresh `get`.

Ref-addressed double and triple clicks prove the pointer receiver before every
press, so a window raised by the first click is seen before the second. The
checks share a 300 ms press-to-press budget, inside the 400 ms in which GTK, Qt
and GNOME read two presses as one gesture. A changed receiver, or a check that
outruns the budget (`details.kind: multi_click_window`), stops the gesture with
`ACTION_FAILED`; `details.delivered_clicks` counts the clicks already sent and
the disposition is `delivered_unverified`. Inspect the app before retrying.

`hover @ref` proves the receiver the same way before the move; a covered target
or an unproven receiver is refused (`ACTION_FAILED`, `not_delivered`, cursor
unmoved). `hover --xy` has no target element to prove.

## Keys and `press`

```bash
agent-desktop press ctrl+a
agent-desktop press return --app gnome-text-editor
```

- Key names: the macOS and Windows names (`return`, `escape`, `tab`, `space`,
  `home`, `pageup`, `f1`-`f24`), punctuation names (`comma`, `period`,
  `greater`, `leftbracket`) and X keysym names such as `Page_Up`. `delete` is
  Backspace; `forwarddelete` is the forward Delete key. A single character is
  its own key. An unknown name is `INVALID_ARGS`, `not_delivered`, and lists the
  accepted forms.
- A bare upper-case letter (`H`) is `shift+h` on every platform; with other
  modifiers the case is ignored (`ctrl+A` is `ctrl+a`).
- `meta`, `cmd`, `command`, `super` and `win` name the Super modifier in combos;
  for a bare key use `super` or `meta`, not `cmd`, `command` or `win`.
- `press` without `--app` or `--window-id` sends to the focused window. Headless
  `press --app` requires the target process to own the active window;
  `press --window-id` also requires the exact window's focus. A background or
  minimized target is refused. See `references/windows-and-lifecycle.md`.
- Keys carry the Shift level the live keymap needs. On X11 `press` takes only
  keys the active layout maps and the named keysyms above; on Wayland it takes
  printable ASCII, function and modifier keysyms without reading the layout. A
  character the layout cannot produce (`é`, `你`) is refused and not delivered
  (`INVALID_ARGS` on X11, `ACTION_NOT_SUPPORTED` with `character_outside_layout`
  on Wayland). On X11, `--headed type` sends such a character through spare
  keycodes; on Wayland it refuses it, so use headless `type` or `set-value` for
  accents and non-Latin text.

### Blocked combos

`press` refuses these shortcuts with `POLICY_DENIED`, `not_delivered`, before
any key is sent, because they take the foreground, close a window or switch the
session. `--force` sends them: `alt+f4`, `alt+tab`, `alt+f2`, `meta+l`,
`meta+d`, `meta+tab`, a bare `super`, `ctrl+alt+t`, `ctrl+alt+delete`,
`ctrl+alt+forwarddelete`, `ctrl+alt+backspace`, `ctrl+alt+l`, `ctrl+alt+f1` to
`ctrl+alt+f12`. App shortcuts such as `ctrl+q` are sent. The list is in the
adapter, not in the CLI help; `agent-desktop press <combo>` shows the refusal
per combo.

## `type`

For literal newlines in Bash, set `TEXT_REF` to the qualified ref of a
multiline text field from the latest snapshot, then use headless typing:

```bash
agent-desktop type "$TEXT_REF" $'First line\nSecond line'
```

Empty text sends no keys, requests no focus on the target and reports
`not_delivered`; `--headed` can still activate the source window before the
empty insertion is dispatched.

- Headless `type @ref text` writes through `EditableText.InsertText`. Without
  `--timeout-ms`, `type` waits 5000 ms plus 30 ms per character; an explicit
  `--timeout-ms` replaces that.
- Under `--headed` it sends physical keys first (step `XTEST.type_text` on X11,
  `VirtualKeyboard.type_text` on a bound Hyprland, `RemoteDesktop.type_text` on
  other Wayland sessions, `physical_synthetic`; the result still
  reads the field back when it can) and falls back to `EditableText` only where
  keys cannot be sent. It proves focus on the field or its proven internal
  editor; focus elsewhere in the window does not authorize ref typing. Verify
  the value or application effect afterwards.
- A terminal's text node (`textfield` named `Terminal`, only `RightClick`) takes
  keys only under `--headed`; headless `type` there is `POLICY_DENIED`, check
  `supported_action`. Its value is null, so a ref cannot read terminal output:
  verify what a terminal command did through files or another app.
- X11 physical `type` refuses control characters, Tab and newline included,
  before sending anything, because those keys can move focus or submit a form
  inside a text burst. Use headless `type` or `set-value` for literal text with
  those characters, and an explicit `press tab` or `press return` for
  navigation or submission. On Wayland, `--headed type` sends a newline as
  `Return` and a tab as `Tab`, which can submit a form or move focus; other
  control characters are refused with `character_outside_layout`.
- X11 physical text checks the live keyboard state before each chunk. If that
  state is unsupported, the physical route refuses before that chunk's keys; the
  semantic fallback applies only if no key was sent, since earlier chunks report
  `delivered_unverified`.
- The first `type` into a fresh document changes the window title, but the ref
  from the snapshot still resolves: `get` on it reads the typed value. Take a
  fresh snapshot only when the UI you address has changed for another reason.

## Displays on Wayland

`list-displays` asks GNOME's Mutter `DisplayConfig` interface first. When no
Mutter answers, it connects to the session's Wayland socket and reads the
compositor's `wl_output` and `zxdg_output_manager_v1` globals. wlroots-family
compositors such as Sway and Hyprland advertise both. On a bound Hyprland it
reads the bound instance's Wayland socket, which also covers a session found
by discovery without `WAYLAND_DISPLAY`.

```bash
agent-desktop list-displays
```

- `id` is the output's connector name, such as `eDP-1` or `DP-2`. It does not
  change while the monitor stays connected.
- `bounds` is the output's logical position and size in the compositor's
  layout.
- `scale` is the current mode's width (its height on an output turned a
  quarter) over the logical width, rounded to 1/120. An output with no mode
  reports its integer `wl_output` scale.
- Wayland names no primary output. The output at the layout origin is
  primary; with none there, the top-left one is. The list is primary first,
  then by `id`, so `screenshot --screen N` indexes stay stable.
- A compositor that advertises neither global, or returns an output without a
  name, position or size, gives `PLATFORM_NOT_SUPPORTED`.
  `platform_detail` names what was missing.
- A compositor that accepts the connection but does not answer within the
  command's budget gives a retryable `TIMEOUT`. `details.detail` names the
  socket.
- A socket that closes the connection gives `PLATFORM_NOT_SUPPORTED` with
  cause `missing`.

## Wayland keys

On a bound Hyprland, keys go through a virtual keyboard on that compositor; see
[Hyprland keys](#hyprland-keys). Other Wayland sessions use one detected
backend, `org.gnome.Mutter.RemoteDesktop` on the session bus. Before creating a
session or emitting an event, the adapter probes
the interface's typed `Version` property. A missing service or capability is
`PLATFORM_NOT_SUPPORTED`, `not_delivered`, `physical_delivery_started: false`;
another compositor never falls back to X11. A probe timeout is `TIMEOUT`. Calls
send keysyms and wait 150 ms for the compositor to drain them. A long-lived host
can retain the keyboard session for later deliveries; a new CLI process opens
its own. Ending a session releases its held keys.

- **Lead Shift.** A new session sends a lone Shift tap and waits 100 ms before
  the real keys so the compositor can attach its keyboard. It inserts no text,
  but a handler that watches bare Shift can observe it. Reusing a still-live
  session skips the lead tap.
- **Characters.** Through Mutter, only printable ASCII, function, modifier and vendor keysyms;
  anything else is `ACTION_NOT_SUPPORTED`, cause `character_outside_layout`.
  Set the value directly with `set-value` instead. On a non-US layout, a
  character your keymap holds can still be refused. A refusal is safe: nothing
  was sent.
- **App pointer.** AT-SPI gives no global window origin on Wayland, so ordinary
  app pointer commands cannot be aimed (see Headed and headless above). A bound
  Hyprland is the exception; see [Pointer on Hyprland](#pointer-on-hyprland). GNOME
  Shell ref clicks and popup openers refuse too; see
  `references/shell-tray-and-notifications.md`.
- **No active window.** On a Wayland session with no seat keyboard no window
  carries `ACTIVE`, so `focus-changed` waits time out.

### Hyprland keys

A bound Hyprland takes keys through `zwp_virtual_keyboard_v1`. Step labels are
`VirtualKeyboard.type_text` and `VirtualKeyboard.press_key`. Each delivery
creates one virtual keyboard and destroys it at the end, so none stays in the
compositor's device list.

- **Keymap.** The CLI reads the seat's keymap, adds a key for each character
  that keymap lacks, and sends the result before the first key. The layout is
  the compositor's main keyboard's active layout; the CLI sends that layout
  group and every held modifier with the keys. It does not switch the user's
  layout.
- **Characters.** Any Unicode character, Tab and newline included. Other
  control characters, such as `\u0007` or `\r`, are `ACTION_NOT_SUPPORTED`,
  cause `character_outside_layout`, before any key. Use `set-value` instead.
- **Focus.** The focus gate runs before each key-down, as on other Wayland
  sessions. If focus moves mid-text, the CLI stops, releases every held key and
  modifier, and returns `delivery_uncertain` with `delivered_events`. Read the
  field before you retry. Global `press` has no target, so it goes to whichever
  window holds keyboard focus.
- **Lead Shift.** Each delivery starts with the lone Shift tap described above.
- **Refusals.** Each is `ACTION_NOT_SUPPORTED`, `not_delivered`,
  `physical_delivery_started: false`, and no key was sent:

| `details.cause` | When |
|---|---|
| `active_layout_unknown` | The compositor's main keyboard names no layout of the seat's keymap |
| `owner_keymap_unreadable` | The seat's keymap could not be read |
| `owner_keymap_invalid`, `owner_keymap_unrecognized` | The seat's keymap does not compile, or lacks the keycodes or symbols section the CLI adds keys to |
| `keysym_invalid` | A requested key has no keysym name a keymap can carry |
| `no_free_keycode` | The text needs more new keys than the keymap has free codes |
| `extended_keymap_invalid`, `extended_keymap_too_large` | The keymap with the added keys does not compile, or is too large to send |

Without libxkbcommon, keys refuse with `PLATFORM_NOT_SUPPORTED`, cause
`xkb_unavailable`, `not_delivered`.

## Pointer on Hyprland

On a bound Hyprland, `--headed` pointer commands go through a
`zwlr_virtual_pointer_v1` bound to one output. They never pass through the
GNOME portal.

```bash
agent-desktop list-displays
agent-desktop --headed mouse-click --xy 640,400 --count 2
agent-desktop --headed mouse-wheel --x 640 --y 400 --dy -2
agent-desktop --headed drag --from-xy 600,400 --to-xy 800,400
```

- **Coordinates** are global logical layout points, the bounds `list-displays`
  reports. Negative origins and fractional scales work. A point on no output is
  `INVALID_ARGS`, cause `point_off_output`, `not_delivered`.
- **The first motion** of each command starts one logical pixel left of the
  aim (right of it at an output's left edge), then moves onto it. Hyprland
  skips a motion to the pixel the cursor already holds, and 0.56.2 gives no
  pointer focus to a window that maps under a still cursor. Without the step,
  input aimed at the cursor's own pixel goes to the window that held focus
  before.
- **Coordinate commands** (`mouse-move`, `mouse-click`, `mouse-wheel`,
  `drag --from-xy`, `hover --xy`) prove nothing about the receiver. The report
  says `details.receiver_proof: false` and counts `details.delivered_events`.
  Read the effect from the app afterwards.
- **Ref commands** (`click`, `double-click`, `triple-click`, `scroll`) aim at
  the element inside its window and prove the receiver from the compositor
  before each burst. The window must be mapped and shown, and the point must be
  inside it. No `top` or `overlay` layer and no floating, pinned or fullscreen
  window that takes the pointer before the target may cover the point.
  Hyprland gives the pointer first to pinned windows, then to floating windows
  allowed over fullscreen, then to the fullscreen window, then to other
  floating windows, then to tiled ones; within a group the later window in
  `hyprctl clients` wins. A fullscreen window takes every point on its
  workspace. When `hyprctl clients` reports neither `allowedOverFullscreen`
  nor the older `overFullscreen` and that decides the order, the cause is `evidence_missing` with
  `evidence: "over_fullscreen"`.
  A headed ref action first activates the window, as `focus-window` does, so a
  window on a hidden special workspace or another workspace is revealed before
  any check. A cover can undo the activation: Hyprland moves the cursor into
  the window, onto a floating or pinned cover that then stays active
  (`activation_refused`), and a layer that takes the keyboard, such as slurp's,
  keeps the window's frame inactive (`frame_not_active`). Both refusals are
  `ACTION_FAILED`, `not_delivered`, with `activation_sent: true`. A cover the pre-action hit test sees then fails core's
  `receives_events` check: `ACTION_FAILED`, `not_delivered`, reason
  `occluded by layer` or `occluded by window`, with the layer namespace in
  `occluder.name`. Anything that check passes or cannot read meets the
  receiver proof before each burst, which refuses before any event with
  `ACTION_FAILED`, kind `receiver_evidence`, and one of the causes
  `evidence_missing`, `target_not_shown`, `point_outside_target`,
  `popup_surface`, `layer_cover` or `window_cover`. A read that runs out of time is `TIMEOUT`, cause
  `evidence_timed_out`.
- **Every layer counts.** The tool trusts no layer namespace, its own cursor
  namespace included. A shell bar or dock that covers the whole output, as the
  stock Omarchy bar does, refuses every ref pointer action under it. Use a
  semantic action, or a coordinate command and read the effect.
- **Bounds and hit tests** come from the compositor. A ref's live bounds and
  its `find` and `snapshot` bounds are its window rectangle at the window's
  Hyprland origin, in the same layout points. `hyprctl` lists only toplevel
  windows, so an element inside a popup surface (a menu or a GTK 4 popover)
  has no bounds, and a pointer action on it refuses at once with cause
  `popup_surface`. Its semantic actions, such as `click` through the
  accessibility action, still work. The pre-action hit test hides the
  agent's cursors, then reads the same window stack and layers; a cover answers
  `InterceptedBy` with its rectangle, evidence it cannot read answers unknown,
  and a read that runs out of time is `TIMEOUT`.
- **By ref, `hover` refuses.** `hover @ref` is `ACTION_NOT_SUPPORTED`, cause
  `hover_unsupported`, and sends nothing. Use `hover --xy`.
- **`drag` by ref** proves both endpoints as a ref click does, before it starts.
  Each must be shown, inside its window and uncovered, or the drag refuses
  before any event with the same causes. The move itself carries no further
  proof. `drag --from-xy` and `--to-xy` stay coordinate commands.
- **Modifiers** are not held: `--modifiers` is `ACTION_NOT_SUPPORTED`, cause
  `modifiers_unsupported`, `not_delivered`.
- **Focus gate.** The active window is read before every button press and
  every wheel step. If it changes mid-sequence, delivery stops with
  `ACTION_FAILED`, kind `focus_gate`, cause `focus_changed`.
- **Partial delivery.** A sequence that stops after its first event reports
  `delivery_uncertain` with `retry: unsafe`, `physical_delivery_started: true`,
  `delivered_events` and `buttons_released`. Pressed buttons are always
  released. `buttons_released: true` means the compositor confirmed every
  release. `false` means it went away or did not confirm within 1 s. Take a
  fresh snapshot before any retry.
- **Signals.** SIGINT, SIGTERM or SIGHUP during pointer input stops the next
  event, releases held buttons, waits up to 1 s for the compositor to confirm
  them, then ends the process with that signal.

## Wheel and scroll

```bash
agent-desktop --headed mouse-wheel --x 640 --y 400 --dy 5
```

- Positive `--dy` scrolls up, negative down; positive `--dx` right, negative
  left, on all three platforms. Write a negative value as `--dy -5` or
  `--dy=-5`.
- With no `--dy`, the default is three lines down when `--dx` is zero (explicit
  `--dx 0` included) and no vertical scroll when `--dx` is nonzero. Zero on both
  axes, or more than 1000 clicks on an axis, is `INVALID_ARGS`, `not_delivered`.
- The result carries `dy`, `dx`, `target`, `steps` and `disposition`, with no
  `scrolled` success flag. Wheel delivery is `delivered_unverified`: read the
  content afterwards.
- `scroll @ref --headed` sends wheel clicks at the centre of the target's
  bounds, only for a node with the `Scroll` capability. A wheel click is
  page-size dependent (about two rows on GTK, exactly three items on Qt), so
  `--amount` counts clicks, not rows. A `scrollarea` without a ref cannot take
  `scroll @ref`; use `scroll-to` on a child or `mouse-wheel` at coordinates.
  Each wheel burst uses click's pointer gate at that point: a hit-test result
  naming an interceptor refuses the burst; an unknown receiver refuses with
  `pointer_hit_unproven`. If earlier bursts were sent, the error reports their
  `delivered_clicks`, `delivered_unverified` and unsafe retry. Inspect the
  content before the next step.
- `scroll-to` scrolls through the bar on GTK3 and GTK4 and through `ScrollTo` on
  Chromium. It does not work on Qt: use `scroll --headed`. A container with no
  scrollable axis has no ref. If target visibility is unknown, `scroll-to`
  refuses before scrolling; equally ranked scroll bars refuse with
  `AMBIGUOUS_TARGET` before a value write. A scroll that moved the viewport
  without revealing the target is unsafe to retry: inspect the target and
  disposition first.

## Drag

`drag` picks up, moves along an interpolated path, holds still for a dwell, then
releases. The dwell is deliberate: a hovered folder or drop target can open
under the pointer, and any later motion lands inside it and copies instead of
moving. The default duration is 300 ms; the minimum is 100 ms, and a shorter
`--duration` is `INVALID_ARGS`, `not_delivered`, because a toolkit such as GTK
needs several motion events to start a drag. The default drop
delay on Linux is 150 ms; set `--drop-delay` only to give a slow drop target
time to arm. The result reports the gesture, not the outcome: check the effect
(a file moved, a list changed) with a fresh `snapshot`.

## Interrupted input

The X server keeps XTEST-pressed buttons when the client dies. The CLI journals
what it holds under `$XDG_RUNTIME_DIR/agent-desktop-input`, and the next call on
that display releases what a killed process left before it sends its own events.
If a button seems stuck after you killed a command, run any input command
with valid arguments, such as `agent-desktop --headed mouse-move --xy 640,400`,
to release it before the new input.
