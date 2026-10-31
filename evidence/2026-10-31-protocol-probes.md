# M0 protocol probes: R2, R3, R4, R7, R8 (2026-10-31, the test host)

Host: the test host, Hyprland 0.56.2. Client: `probes/wlprobe`, a throwaway Rust
crate on `wayland-client` 0.31 with the pure-Rust backend (no libwayland link),
`wayland-protocols` 0.32, `-wlr` 0.3, `-misc` 0.3 (virtual keyboard), `png`.
All dependencies are MIT, Apache-2.0 or Zlib. The `hyprland_toplevel_mapping_v1`
binding is generated with `wayland-scanner` from the vendored BSD-3 XML in
`probes/wlprobe/protocols/`; the `wayland-protocols-hyprland` crate is
LGPL-3.0, so it is not used.

Method: every probe that sends input, activates or draws ran in the nested
private session (`private-session.sh` → `private-hyprland-shell.sh` → `probes/pN-*.sh`).
On the host, the probes only read (`hyprctl -j`, toplevel list).
Host active window, active workspace and cursor position were identical before
and after the runs. After teardown, no session dirs or probe processes remained.
Raw logs and PNGs are in `probes/out/`.

## Harness finding (affects proposal section 6 and every capture leg)

The nested compositor's Wayland output sits on a hidden host special workspace.
The host sends no frame callbacks to it, so the nested Hyprland never renders.
`ext_image_copy_capture` frames got a `damage` event but no `ready` within 2–3 s
in every case (`HEADLESS=0 probes/p4-capture.sh`).

Fix, nested only: `hyprctl output create headless PROBE`, then
`monitor = PROBE,1280x800@60,0x0,1` and `monitor = WAYLAND-1,disable`
(`use_headless` in `probes/lib.sh`). The nested session then renders on its own
timer, and all capture and screencopy legs below pass. The host is unchanged.
This also suggests a way to run the tier without a visible parent. That is not
proven yet, because Aquamarine still needs the parent at startup.

## 1. Identity (R2)

Nested: 2 `ADTest` processes, 1 `ADOther`, and one process with 2 `ADTwin`
windows (`probes/two_windows.py`).

| Check | Result |
|---|---|
| `ext_foreign_toplevel_list_v1` → `hyprland_toplevel_mapping_manager_v1` | Every toplevel mapped to its `hyprctl` address; no `failed` events (5/5) |
| Identifier | Opaque string (`18000002`…), unique per toplevel, the same across 2 client connections |
| wlr handles → address (`get_window_for_toplevel_wlr`) | 5/5 mapped. This joins the identity handle to the activation handle |
| 2 `ADTest`, different pids | pid disambiguates. Start time does not (both `8828759` ticks); pid + start time does |
| Same pid + title, tiled | Hyprland sizes differed by chance (452×241 and 452×235). AT-SPI `bounds` gave the matching size, so size disambiguated |
| Same pid + title, floating 400×300 at the same position and workspace | **Ambiguous.** Address and identifier differ, but pid, title, size, position and workspace are equal in `j/clients`. AT-SPI gives `x=0,y=0`, 400×300 for both frames, and both report `is_focused: false` |

Host (read-only): 3 toplevels (a terminal, an Electron app, one browser window). Each one
mapped. Ambiguous groups (same pid + title): **0**. A notes app and a file manager were
not running, so the M0 refusal-rate measurement is still open for them and for
a multi-window browser. Host reads are in `probes/out/host/`, with titles and classes generalized.

Verdict: **holds.** The identifier → address chain is reliable. The evidence
chain cannot separate a frame from its twin when the twins match in pid, title,
size and workspace, because GTK on Wayland reports no global position over
AT-SPI. Refusal is correct there. Focus state is the only extra discriminator,
and it only covers the active twin.

## 2. Activate (R3)

`zwlr_foreign_toplevel_handle_v1.activate(seat)`, with A on workspace 1 (active),
B on workspace 2 and C on `special:hid`.

| Target | `j/activewindow` | Workspace | wlr `activated` (2) | AT-SPI focused |
|---|---|---|---|---|
| B on non-active workspace 2 | B | switched to 2 | B `[2]`, A `[]` | ADOther |
| A back on 1 | A | 1 | A `[2]` | ADTest |
| C on hidden `special:hid` | C | active stays 1; monitor `specialWorkspace` = `special:hid` (**revealed**) | C `[2]` | ADSpecial |
| A again | A | 1; special closed (`-`) | A `[2]` | ADTest |

The state events arrived in the first dispatch after the request (under 1 ms).

Verdict: **holds.** All three oracles agree, and `activate` reveals a hidden
special workspace, as R3 expects for `--headed`. It also switches normal
workspaces and closes an open special workspace when focus leaves it. The
disposition must report both side effects. Separately, agent-desktop 1.0.2
refuses element `focus` on Wayland (`ACTION_NOT_SUPPORTED`, `wayland_session`),
even with `--headed`.

## 3. Keys (R4)

App: `probes/keylog_app.py`, the harness `gtk3_app.py` plus a key and
entry-change log. The nested layout was `de`, so a layout change is visible.

| Step | Result |
|---|---|
| Tab, minimal generated keymap (wtype style, Tab at keycode 9) | GTK gets `press Tab`; **focus does not move** (also with a 500 ms hold before destroy) |
| Tab, full `us` keymap (`xkbcli compile-keymap --layout us`), evdev `KEY_TAB` | Focus moves to the Entry (AT-SPI `states: ['focused']`) |
| "Hello é", minimal keymap, 300 ms and 5 ms per key | AT-SPI `value: "Hello é"` both times; app log shows `eacute` |
| `j/devices` during delivery | `hl-virtual-keyboard-wlprobe` is `main=True`, `active_keymap='error'` (no named layout; the keymap compiles cleanly); the physical keyboard is `main=False` |
| `j/devices` after | Physical keyboard `main=True`, `layout=de`, `German` |
| Client `wl_keyboard.keymap` sizes | 37532 (de) → virtual keymap → 37532 after each destroy. Hyprland re-sends the physical keymap |
| Host `j/devices` | `us`, English (US), unchanged |

Verdict: **holds, with one qualification.** Text, including a non-ASCII
character, arrives and is AT-SPI-verified, and the user's layout is restored.
A minimal per-text keymap breaks GTK key bindings: the keysym arrives, but Tab
navigation does not run. `press` must use a keymap built from the user's layout
(as R4 states), with extra keysyms added only for characters the layout lacks.
While a virtual keyboard exists, it is the seat's main keyboard. A physical key
pressed during delivery is interpreted with the virtual keymap, which is a
reason to keep batches short.

## 4. Capture (R7)

`ext_foreign_toplevel_image_capture_source_manager_v1.create_source(ext handle)`
→ `ext_image_copy_capture` session (SHM, XRGB8888). Frames: f1, then f2 with no
change, then an AT-SPI `click` on Increment, then f3. Nested headless output (see the harness finding).

| Case | f1 | f2 (static) | f3 (after click) | Pixel diff |
|---|---|---|---|---|
| Visible, workspace 1, 613×758 | ready, 14 ms | ready, 7 ms | ready, 4 ms | f1=f2: 0; f1≠f3: 328 px (box = the `count 0` → `count 1` label) |
| Hidden `special:hid`, 1238×758 | ready, 15 ms | ready, 9 ms | ready, 9 ms | f1=f2: 0; f1≠f3: 360 px |
| Hidden, static, 3 frames | ready, 2 ms | ready, 10 ms | ready, 9 ms | 0, 0 |
| Any, without the headless output | damage event, no `ready` in 2–3 s | — | — | — |

Damage is always the full buffer. The session buffer size is the window's
current size; the hidden window was re-laid out to 1238×758 on the special
workspace. PNGs: `probes/out/p4-*.png`.

Verdict: **holds.** Capture works on a hidden special workspace, and a static
window gets a new frame in about 10 ms. A frame's arrival and its damage do not
prove fresh pixels (static frames are identical, and damage is always full).
The ticking-counter test in R7 is needed. A capture command needs a deadline,
because a compositor that does not render (as in the nested case) never answers.

## 5. Cursor renderer (R8)

`wlprobe cursor`: one process, 2 surfaces of 32×32 (red, blue) on the overlay layer,
namespace `agent-desktop-cursor`, empty input region, keyboard interactivity
none, exclusive zone −1, placed over the ADTest window.

| Check | Result |
|---|---|
| `j/layers` while running | 2 entries, level 3 (overlay), the client's pid, at 200,10 and 400,40, 32×32 |
| Focus | `j/activewindow` stays ADTest; a virtual-keyboard Tab still focuses the Entry |
| `ext_image_copy_capture` of the window under the cursors | No cursor colours |
| `zwlr_screencopy` full output, `overlay_cursor=0` | 1024 red + 1024 blue px (both cursors) |
| After TERM to the exact client pid | `j/layers` empty; the next screencopy has no cursor colours |

Verdict: **holds.** The surfaces are listed, take no focus, stay out of window
captures, appear in output captures (so `--screen` must hide them first) and
vanish when the client exits. Not measured: pointer pass-through of the empty
input region (R6 receiver check). That needs `zwlr_virtual_pointer_v1` and
belongs in the M4 legs.

## Summary

The proposal's protocol assumptions held in Hyprland 0.56.2. `ext_foreign_toplevel`
identifiers map one-to-one to Hyprland addresses and to wlr activation handles.
`activate` focuses windows on other workspaces and reveals a hidden special
workspace, and three independent oracles confirm it. The virtual keyboard
delivers text, including `é`, and the user's layout comes back after delivery.
Toplevel capture works on hidden special workspaces. The layer-shell cursor
design behaves as R8 states. Four results were new or did not match the proposal:
(1) the nested E2E session cannot render or capture until it adds its own
headless output, so proposal section 6 needs that step; (2) twins with equal pid, title,
size and workspace cannot be told apart through AT-SPI, so R2 refuses them, as
designed; (3) a minimal per-text keymap breaks GTK key bindings, so R4's
layout-based keymap is required, not optional; (4) capture frames arrive at
once for static windows with full damage, so only the ticking-counter test
proves freshness, and a capture deadline is required. The host refusal-rate
count is still open, because only 3 unambiguous windows were open, and a notes app
and a file manager were not running.
