# Capture and Clipboard on Linux

X11 needs no permission and no outside tool. A bound Hyprland captures through
the compositor and asks nobody. Other Wayland desktops ask a person once for
`screenshot`. The Wayland clipboard needs `wl-clipboard`. `agent-desktop permissions` shows the
case: `screen_recording` is `not_required` on X11 and `unknown` on Wayland.
`screenshot` has no timeout flag.

## `screenshot` on X11

```bash
agent-desktop screenshot --app gnome-calculator /tmp/calc.png
agent-desktop screenshot --screen 0 /tmp/screen0.png
agent-desktop screenshot /tmp/everything.png
```

- No permission and no prompt. Targets: a window (`--app`, `--window-id`), a
  display by `list-displays` index (`--screen`), or with no selector the primary
  display (the first if none is marked primary). With no output path the image
  returns as base64 in `data`.
- A window is read from its own drawable. Under a compositing window manager
  (mutter), covered and minimized windows are captured.
- Under a non-compositing window manager (openbox) that read is not possible. A
  covered window answers `ACTION_FAILED` with `details.reason: "covered"` and
  the suggestion to raise it with `--headed focus-window`; the same capture
  after the focus returns the window. A minimized window answers `INVALID_ARGS`
  ("The window is not viewable and the window manager does not composite, so it
  has no pixels to capture") with the suggestion to `restore` it. Both are
  `not_delivered` and `retry: safe`. On a non-compositing manager, whole-screen
  and `--screen` captures are exact.
- Under a compositor such as GNOME on X11 the root window holds only the
  manager's own stage, so whole-screen and `--screen` captures answer
  `PLATFORM_NOT_SUPPORTED` with `details.reason: "compositing"` and the
  suggestion to capture one window with `--window-id`. The area no X window owns
  (wallpaper, shell chrome) cannot be read under a compositor.
- Two windows of one process with the same title: `--window-id` captures the one
  the toolkit reports active, and its twin when exactly two windows share the
  title. Any other tie answers `AMBIGUOUS_TARGET` ("More than one X11 window
  matches this window") with `candidates` and the suggestion to give the windows
  distinct titles.
- The tool hides its own cursor overlays during capture and restores them; they
  neither block the screenshot nor appear in its pixels.
- A display-server or caller-deadline timeout stays `TIMEOUT`, including when
  capturing a covered window. It does not prove coverage or advise focus.
- Resource, connection and visibility failures keep their own errors. Only a
  window whose paint cannot be proven gets the `covered` advice.

## `screenshot` on Hyprland

On a bound Hyprland, `screenshot` uses the compositor's own capture protocols.
It never asks the portal, so no dialog appears.

```bash
agent-desktop list-windows --app foot
agent-desktop screenshot --window-id w-hypr-658-344-18000006 /tmp/foot.png
agent-desktop screenshot --screen 1 /tmp/second.png
```

- A window target binds to exactly one toplevel, the way `focus-window` does.
  A window on a hidden special workspace, on a workspace out of view, or with
  no accessibility tree is captured where it is. Nothing is activated, shown
  or moved.
- A window in view is captured from one frame, in about 45 ms. In view
  means it is on the workspace its monitor shows, or on the special workspace
  open over it, and no other window on its workspace is fullscreen or
  maximized. The frame holds the newest drawing the window committed before
  the request.
- Any other window, such as one out of view or behind a fullscreen window, is
  drawn from the buffer it last drew, which can be seconds old. The capture
  makes the window draw, then returns the first frame whose pixels changed;
  those pixels were drawn after the request. A window whose pixels stay the
  same for at least 250 ms returns that unchanged frame, which can be older
  than the request. A drawing that lands later is not waited for, so capture
  again to see it. An unchanged window out of view takes about 430 ms to
  capture.
- `--screen N` captures the output `list-displays` names at index `N`, at its
  full pixel size. `data.scale_factor` is the output's scale; fractional
  scales and outputs at negative origins work.
- With `ecosystem:enforce_permissions` on, Hyprland withholds capture from a
  client it does not list, without asking anyone. The CLI reads the setting
  first and answers `PERM_DENIED` with `details.option` and
  `details.enforced: true`. Nothing is requested and no dialog appears. The
  refusal holds even for a client the config allows. Set the option to false
  to capture.
- No frame before the deadline, or a deadline that ends the wait for a new
  drawing or for a refused buffer's retry, is `TIMEOUT` with `details.kind:
  "capture_frame"`. Retry with a longer budget, or once the compositor draws
  again.
- A frame the compositor refuses is `ACTION_FAILED` with `details.kind:
  "capture_refused"` and `details.reason`. Right after a window moves,
  Hyprland 0.56 can refuse frames until the size settles. The CLI retries on
  new sessions for up to 2 s first.
- A window that closes before or during the capture is `WINDOW_NOT_FOUND`. A
  display that is removed or changes during the capture is `INVALID_ARGS`.
- Every error before the image is written is `not_delivered` with
  `retry: safe`, and no file is created. `docs/json-output.md` lists every
  code.
- `permissions` reports `screen_recording` as `unknown`. Each capture makes
  Hyprland send `screencast` events on its event socket.

## `screenshot` on other Wayland desktops

Without a bound Hyprland, Wayland offers no window position and no per-window
capture. The portal's
`Screenshot` captures the desktop; the CLI returns the primary display by
default, cropping the image on a multi-monitor desktop. `--screen N` selects
another monitor.

```bash
agent-desktop screenshot /tmp/desktop.png
```

- The first call shows a consent dialog ("Allow Applications to Take
  Screenshots?", Deny focused) that is not on the accessibility bus. A person
  must answer it. The CLI waits for the portal's answer, bounded by the longer
  of the command's own budget and 30 s.
- Allow returns the selected display as a PNG. Deny, or a dismissed dialog,
  gives `PERM_DENIED`.
- Nobody answers: the portal gives up after about 25 s and the call returns
  `TIMEOUT`, with the dialog named in its suggestion. The
  dialog stays open, and later calls return `PERM_DENIED` at once, with no
  second dialog, until a person dismisses it. Do not retry in a loop: ask the
  user to answer it.
- The desktop remembers the answer per user for the host process. After Allow,
  later calls return in about 50 ms with no dialog. After Deny, every call is
  `PERM_DENIED` until the user resets it in the desktop's privacy settings.
- `--screen N` and a display target crop the whole-desktop image to that
  monitor. Cropping requires dimensions that confirm the monitor layout;
  otherwise the call answers `PLATFORM_NOT_SUPPORTED` with reason
  `display_mapping`.
- A known display layout is rechecked after consent, including when it had only
  one monitor. A changed layout returns `ACTION_FAILED` without pixels.
- A window target (`--app`, `--window-id`) is `PLATFORM_NOT_SUPPORTED` on
  these desktops, permanently. Capture a display, or read the window with `snapshot`.
- The portal writes the PNG under the user's home. The CLI accepts the file only
  inside the temp, runtime or home directory, checks it is a regular file of the
  user made for this request, reads it and removes it.

## Clipboard

```bash
agent-desktop clipboard-set "hello from agent-desktop"
agent-desktop clipboard-get
agent-desktop clipboard-clear
```

Formats and output follow the core `commands-system` reference. Text, images and
file lists all work on Linux.

### X11

- `clipboard-set` hands the selection to a detached owner process, the helper
  `agent-desktop-linux-clipboard-owner`, which serves the content until another
  client takes the selection or `clipboard-clear` ends it. The CLI exits first;
  the content stays available while the owner lives.
- The helper must sit beside the `agent-desktop` binary and be a regular
  executable file, not a symlink, owned by the user or root. Neither it nor its
  directory may be group-writable, even when the group appears private; the file
  must not be world-writable. Remove group-write permission from both the helper
  and its directory before using a build made with a group-writable umask. A
  world-writable directory is accepted only with the sticky bit and without
  group-write permission. A 777 copy is refused with `PLATFORM_NOT_SUPPORTED`, "writable by
  others". npm installs and release archives ship it in place. A binary copied
  alone cannot set the clipboard: `PLATFORM_NOT_SUPPORTED` names the missing
  helper. Both refusals are `not_delivered`, `safe`.
- Without a clipboard manager the content ends with the owner. GNOME's shell
  keeps only text and PNG after the owner dies, caps text at 4 MiB, and does not
  take over a file copy or a custom target: do not rely on content outliving the
  owner.
- A file copy serves `x-special/gnome-copied-files` and `text/uri-list`. A
  plain-text path pastes nothing in a file manager.
- Content above the X server's single-request limit transfers with `INCR`, so
  large content works.
- "Nothing there" and "target not available" are different answers: an absent
  format is `absent`, not an error.

### Wayland

- The clipboard goes through `wl-clipboard` (`wl-copy`, `wl-paste`): install the
  package. Text, images and file lists round-trip against foreign readers and
  writers under GNOME. `clipboard-clear` runs `wl-copy --clear`.
- Routing follows the display-server detector: an empty `WAYLAND_DISPLAY` counts
  as unset, and a stale socket falls through to a reachable `DISPLAY`. Once
  Wayland is selected, clipboard failures never fall back to X11. A missing
  `wl-copy` or `wl-paste` returns `PLATFORM_NOT_SUPPORTED` with a `wl-clipboard`
  install hint, even when `DISPLAY` is set.
- After Wayland is selected, a failed connection ("Failed to connect to a
  Wayland server"), a seat with no keyboard ("This seat has no keyboard") and a
  timeout from a shell that has stopped serving clients are distinct errors,
  never an empty clipboard.

### Clipboard through FFI

The C library reaches the same code; its clipboard API is text-only. Set and
clear take the cross-process interaction lease; reads have a deadline but do not
take that mutation lease. The matching owner helper can sit beside
`libagent_desktop_ffi.so`; it need not sit beside the host's executable. The
library has no stable SONAME.
