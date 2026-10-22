# Display Servers and Input on Linux

Observation and semantic ref actions use AT-SPI and behave the same on X11 and
Wayland. Input, window control and capture differ.

## Which display server

The CLI reads `WAYLAND_DISPLAY`, `DISPLAY` and `XDG_RUNTIME_DIR` (an empty value
counts as unset). A Wayland socket that accepts a connection wins, because X11
calls cannot list Xwayland windows. A stale `WAYLAND_DISPLAY` falls through to a
working `DISPLAY`. With neither reachable, display commands fail with cause
`not_configured`, `missing` or `refused` and list the variables they tried.

## X11 versus Wayland (GNOME)

| Capability | X11 | Wayland |
|------------|-----|---------|
| Observation, semantic ref actions | AT-SPI | AT-SPI |
| Keys, `press`, text | XTEST, live keymap | Mutter RemoteDesktop keysyms |
| `hover`, `drag`, `mouse-*`, physical click, wheel | XTEST, `--headed` | `ACTION_NOT_SUPPORTED`, `physical_delivery_started: false` |
| Window ops | EWMH | Xwayland clients only; native windows and un-minimize refused |
| `focus-window`, headed scoped `press` | EWMH activation | `ACTION_NOT_SUPPORTED`, cause `wayland_session`, Xwayland clients included |
| Headless scoped `press` | Live display and frame focus guards, no activation | Live frame focus guard, no activation |
| `screenshot` | Window or display | Portal consent; `--screen` picks a monitor |
| Clipboard | Native selection owner | `wl-clipboard` |
| GNOME Shell ref click, popup opener | `--headed`, receiver proof from the shell's tree | Ref click: `wayland_session`; popup opener: unproven (shell reference) |

X11 needs no outside tool; `xdotool`, `ydotool`, `xclip` and `wmctrl` are not
used. `wl-clipboard` is the one outside tool, on Wayland only.

## Headed and headless

Ref actions are headless and semantic: the pointer and the active window never
change. Pointer commands (`hover`, `drag`, `double-click`, `triple-click`,
`mouse-move`, `mouse-click`, `mouse-wheel`) need global `--headed`, because every
X11 synthetic pointer path moves the user's cursor.

- Under `--headed`, ref `click`, `right-click` and `type` try physical input
  first. Success is `physical_synthetic` with `verified: false` (XTEST has no
  read-back).
- The semantic action runs only after `ACTION_NOT_SUPPORTED` or a geometry
  refusal (`check: geometry`), both `not_delivered`; `details.physical_fallback`
  records the skipped `physical_input` step. Focus, policy, timeout, uncertain
  and partial-delivery refusals never fall back.
- On Wayland, headed pointer commands (`click`, `hover`, `double-click`,
  `mouse-wheel`, `mouse-down`, `select`) are refused at once: `ACTION_NOT_SUPPORTED`,
  cause `wayland_session`, `physical_delivery_started: false`. A headed `type`
  sends keys through `RemoteDesktop.type_text` once the Mutter capability probe
  succeeds.
- Windows works the other way round (semantic first).

Physical keys need the target frame `active` within a short wait, else
`check: window_active`. Headed `type` and ref key presses also need the target,
or its proven internal editor, `focused`: before each X11 text plan and, on
Wayland, before the bootstrap Shift and each key-down. GNOME 46's GTK4 bridge
rejects `Component.GrabFocus`, so the tool cannot restore a document's focus:
focus it yourself or use headless `type`. A `target_focus` refusal sent no keys.

Headless semantic actions need no active frame: a headless `type` into a
background window works. Scoped `press` rules are in
`references/windows-and-lifecycle.md`. The active-frame gate is necessary, not
sufficient: while a popup menu holds the grab, keys reach the menu, so key steps
stay unverified.

Pointer forms aim at the centre of the ref's bounds and refuse where the
position is not proved. Double and triple clicks prove the receiver before every
press, within a 300 ms press-to-press budget (GTK, Qt and GNOME read presses
within 400 ms as one gesture). A changed receiver, or a check over the budget
(`details.kind: multi_click_window`), stops the gesture with `ACTION_FAILED`,
`details.delivered_clicks` and `delivered_unverified`: inspect the app before
retrying. `hover @ref` proves the receiver before the move; a covered target or
unproven receiver is refused (`ACTION_FAILED`, `not_delivered`, cursor
unmoved). `hover --xy` has nothing to prove.

## Keys and `press`

```bash
agent-desktop press ctrl+a
agent-desktop press return --app gnome-text-editor
```

- Key names: the macOS and Windows names (`return`, `escape`, `tab`, `space`,
  `home`, `pageup`, `f1`-`f24`), punctuation names (`comma`, `period`,
  `greater`, `leftbracket`) and X keysym names (`Page_Up`). `delete` is
  Backspace; `forwarddelete` is Delete. A single character is its own key. An
  unknown name is `INVALID_ARGS`, `not_delivered`, and lists the accepted forms.
- A bare upper-case letter (`H`) is `shift+h` on every platform; with other
  modifiers the case is ignored.
- `meta`, `cmd`, `command`, `super` and `win` name the Super modifier in combos;
  for a bare key use `super` or `meta`.
- `press` without `--app` or `--window-id` sends to the focused window.
- On X11 `press` takes only keys the active layout maps; on Wayland it takes
  printable ASCII, function and modifier keysyms without reading the layout. An
  unmapped character (`é`, `你`) is refused, `not_delivered`: `INVALID_ARGS` on
  X11, `ACTION_NOT_SUPPORTED` with `character_outside_layout` on Wayland. Use
  headless `type` or `set-value` for accents and non-Latin text; only X11
  `--headed type` types them, through spare keycodes.

### Blocked combos

`press` refuses these with `POLICY_DENIED`, `not_delivered`; `--force` sends them:
`alt+f4`, `alt+tab`, `alt+f2`, `meta+l`, `meta+d`, `meta+tab`, a bare `super`,
`ctrl+alt+t`, `ctrl+alt+delete`, `ctrl+alt+forwarddelete`, `ctrl+alt+backspace`,
`ctrl+alt+l`, `ctrl+alt+f1` to `ctrl+alt+f12`. App shortcuts such as `ctrl+q`
are sent. The CLI help does not list the blocked combos.

## `type`

Empty text sends no keys and reports `not_delivered`.

- Headless `type @ref text` writes through `EditableText.InsertText`. Without
  `--timeout-ms` it waits 5000 ms plus 30 ms per character; an explicit
  `--timeout-ms` replaces that.
- Under `--headed` it sends physical keys first (`XTEST.type_text` on X11,
  `RemoteDesktop.type_text` on Wayland, `physical_synthetic`) and falls back to
  `EditableText` only where keys cannot be sent. It needs the field, or its
  proven internal editor, `focused`; focus elsewhere in the window does not
  count. Verify the value afterwards.
- A terminal's text node (`textfield` named `Terminal`, only `RightClick`) takes
  keys only under `--headed`; headless `type` there is `POLICY_DENIED`, check
  `supported_action`. Its value is null: verify terminal output through files
  or another app.
- X11 `--headed type` refuses control characters, Tab and newline included,
  before sending anything. Use headless `type` or `set-value` for such text, or
  an explicit `press tab` or `press return`. On Wayland a newline is sent as
  `Return` and a tab as `Tab`; other control characters are refused with
  `character_outside_layout`.
- X11 text checks the live keyboard state before each chunk. An unsupported
  state refuses before that chunk's keys; the semantic fallback applies only if
  no key was sent, since earlier chunks report `delivered_unverified`.
- The first `type` into a fresh document changes the window title; the ref still
  resolves.

## Wayland keys

Keys go through `org.gnome.Mutter.RemoteDesktop` on the session bus, after a
probe of its `Version` property. A missing service or capability is
`PLATFORM_NOT_SUPPORTED`, `not_delivered`, `physical_delivery_started: false`;
there is no X11 fallback. A probe timeout is `TIMEOUT`. Calls send keysyms and
wait 150 ms for the compositor to drain them. A long-lived host keeps the
session; a new CLI process opens its own.

- **Lead Shift.** A new session sends a lone Shift tap and waits 100 ms so the
  compositor can attach its keyboard. It inserts no text, but a handler that
  watches bare Shift sees it. A reused session skips it.
- **Characters.** Only printable ASCII, function, modifier and vendor keysyms;
  anything else is `ACTION_NOT_SUPPORTED`, cause `character_outside_layout`,
  `not_delivered`. The rule was measured on a US layout, so a character your
  keymap holds can be refused. Use `set-value` instead.
- **No active window.** With no seat keyboard no window carries `ACTIVE`, so
  `focus-changed` waits time out.

## Wheel and scroll

```bash
agent-desktop --headed mouse-wheel --x 640 --y 400 --dy 5
```

- Positive `--dy` scrolls up, negative down; positive `--dx` right, negative
  left, on all platforms. Write a negative value as `--dy -5` or `--dy=-5`.
- With no `--dy`, the default is three lines down when `--dx` is zero (explicit
  `--dx 0` included) and no vertical scroll when `--dx` is nonzero. Zero on both
  axes, or more than 1000 clicks on one axis, is `INVALID_ARGS`, `not_delivered`.
- The result carries `dy`, `dx`, `target`, `steps` and `disposition`. Wheel
  delivery is `delivered_unverified`: read the content afterwards.
- `scroll @ref --headed` sends wheel clicks at the centre of the target's
  bounds, only for a node with the `Scroll` capability. `--amount` counts
  clicks, not rows (a click is about two rows on GTK, three items on Qt). A
  `scrollarea` without a ref cannot take `scroll @ref`; use `scroll-to` on a
  child or `mouse-wheel`.
- Each burst uses click's pointer gate: an interceptor refuses it, an unknown
  receiver refuses with `pointer_hit_unproven`. After earlier bursts the error
  reports `delivered_clicks`, `delivered_unverified` and unsafe retry.
- `scroll-to` scrolls through the bar on GTK3 and GTK4 and through `ScrollTo` on
  Chromium. It does not work on Qt: use `scroll --headed`. A container with no
  scrollable axis has no ref. Unknown target visibility refuses before
  scrolling; equally ranked scroll bars refuse with `AMBIGUOUS_TARGET`. A scroll
  that moved the viewport without revealing the target is unsafe to retry:
  inspect first.

## Drag

`drag` picks up, moves along an interpolated path, holds still for a dwell, then
releases. The dwell is deliberate: motion over a hovered drop target would land
inside it and copy instead of move. The default duration is 300 ms; the minimum
is 100 ms, and a shorter `--duration` is `INVALID_ARGS`, `not_delivered`, because
GTK needs several motion events to start a drag. The default drop delay is
150 ms; set `--drop-delay` only for a slow drop target. The result reports the
gesture, not the outcome: check the effect with a fresh `snapshot`.

## Interrupted input

The X server keeps XTEST-pressed buttons when the client dies. The CLI journals
what it holds under `$XDG_RUNTIME_DIR/agent-desktop-input`, and the next input
call on that display releases it first. After killing a command, run any valid
input command, for example `agent-desktop --headed mouse-move --xy 640,400`.

## Held input

`key-down` and `key-up` fail with `ACTION_NOT_SUPPORTED`, `delivery: unknown`.
`mouse-down` and `mouse-up` fail with `POLICY_DENIED`, `not_delivered`, or with
`--headed` `ACTION_NOT_SUPPORTED`. No event is sent. Use `press` with modifiers
(`ctrl+shift+a`) and `drag`.
