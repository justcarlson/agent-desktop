# Capture and Clipboard on Linux

X11 needs no permission and no outside tool. Wayland asks a person once for
`screenshot` and needs `wl-clipboard`. `permissions` shows the case:
`screen_recording` is `not_required` on X11 and `unknown` on Wayland.

## `screenshot` on X11

```bash
agent-desktop screenshot --app gnome-calculator /tmp/calc.png
agent-desktop screenshot --screen 0 /tmp/screen0.png
agent-desktop screenshot /tmp/everything.png
```

- Targets: a window (`--app`, `--window-id`), a display by `list-displays` index
  (`--screen`), or with no selector the primary display (the first if none is
  primary). With no output path the image returns as base64 in `data`. There is
  no timeout flag.
- Under a compositing window manager (mutter) a window is read from its own
  drawable, so covered and minimized windows are captured. Whole-screen and
  `--screen` captures answer `PLATFORM_NOT_SUPPORTED`, `details.reason:
  "compositing"`: capture one window with `--window-id`. The area no X window
  owns (wallpaper, shell chrome) cannot be read.
- Under a non-compositing manager (openbox) a covered window answers
  `ACTION_FAILED`, `details.reason: "covered"`: raise it with
  `--headed focus-window` and capture again. A minimized window answers
  `INVALID_ARGS` ("not viewable and the window manager does not composite"):
  `restore` it. Both are `not_delivered`, `retry: safe`. Whole-screen and
  `--screen` captures are exact.
- Two windows of one process with the same title: `--window-id` captures the one
  the toolkit reports active, and its twin when exactly two share the title. Any
  other tie answers `AMBIGUOUS_TARGET` with `candidates`: give the windows
  distinct titles.
- The tool hides its own cursor overlays during capture and restores them; they
  are not in the pixels.
- A display-server or caller-deadline `TIMEOUT` proves no coverage and gives no
  focus advice.

## `screenshot` on Wayland

Wayland has no per-window capture. The portal's `Screenshot` captures the
desktop; the CLI returns the primary display by default, cropping on a
multi-monitor desktop. `--screen N` picks another monitor.

```bash
agent-desktop screenshot /tmp/desktop.png
```

- The first call shows a consent dialog ("Allow Applications to Take
  Screenshots?", Deny focused) that is not on the accessibility bus. A person
  must answer it. The CLI waits for the portal's answer for the longer of the
  command's own budget and 30 s.
- Allow returns the display as a PNG. Deny, or a dismissed dialog, gives
  `PERM_DENIED`.
- Nobody answers: the portal gives up after about 25 s and the call returns
  `TIMEOUT`, with the dialog named in its suggestion. The dialog stays open, and
  later calls return `PERM_DENIED` at once, with no second dialog, until a person
  dismisses it. Do not retry in a loop: ask the user to answer it.
- The desktop remembers the answer per user for the host process. After Allow,
  later calls show no dialog. After Deny, every call is `PERM_DENIED` until the
  user resets it in the desktop's privacy settings.
- A window target (`--app`, `--window-id`) is `PLATFORM_NOT_SUPPORTED`,
  permanently. Capture a display, or read the window with `snapshot`.
- If the image does not confirm the monitor layout, `--screen N` answers
  `PLATFORM_NOT_SUPPORTED`, reason `display_mapping`. A layout that changed during
  consent answers `ACTION_FAILED` without pixels: run `list-displays` and capture
  again.

## Clipboard

```bash
agent-desktop clipboard-set "hello from agent-desktop"
agent-desktop clipboard-get
agent-desktop clipboard-clear
```

`clipboard-get` reads `text` by default; `--format` takes `auto`, `image` or
`file-urls`. `clipboard-set` takes text, `--image <png>` or `--file-url <path>`.
Image content is written to a private file (`--out` picks the path) and the
result carries its `path`, `width` and `height`. An empty clipboard answers
`found: false`; an absent format is `absent`, not an error.

### X11

- `clipboard-set` hands the selection to a detached owner,
  `agent-desktop-linux-clipboard-owner`, which serves it until another client
  takes the selection or `clipboard-clear` ends it. The CLI exits first.
- The helper must sit beside the `agent-desktop` binary: a regular file, not a
  symlink, owned by the user or root, and not writable by a shared group or by
  others. A 777 copy is refused with `PLATFORM_NOT_SUPPORTED`, "writable by
  others". npm installs and release archives ship it in place. A binary copied
  alone cannot set the clipboard: `PLATFORM_NOT_SUPPORTED` names the missing
  helper. Both refusals are `not_delivered`, `retry: safe`.
- Without a clipboard manager the content ends with the owner. GNOME's shell keeps
  only text and PNG, so do not rely on content outliving the owner.
- A file copy serves `x-special/gnome-copied-files` and `text/uri-list`. A
  plain-text path pastes nothing in a file manager.

### Wayland

- The clipboard goes through `wl-clipboard` (`wl-copy`, `wl-paste`): install the
  package. Text, images and file lists round-trip under GNOME. `clipboard-clear`
  runs `wl-copy --clear`.
- Routing follows the display-server detector: an empty `WAYLAND_DISPLAY` counts
  as unset, and a stale socket falls through to a reachable `DISPLAY`. Once
  Wayland is selected there is no fallback to X11: a missing `wl-copy` or
  `wl-paste` returns `PLATFORM_NOT_SUPPORTED` with an install hint, even when
  `DISPLAY` is set.
- A failed Wayland connection ("Failed to connect to a Wayland server"), a seat
  with no keyboard ("This seat has no keyboard") and a timeout from a shell that
  no longer serves clients are distinct errors, never an empty clipboard.

### FFI

The C library's clipboard API is text-only. Set and clear take the cross-process
interaction lease; reads have a deadline and take none. The owner helper can sit
beside `libagent_desktop_ffi.so`, not beside the host's executable. There is no
stable SONAME yet.
