# Capture and Clipboard on Linux

`screenshot` and the clipboard commands split on the display server. X11 needs
no permission and no outside tool. Wayland asks a person once and needs
`wl-clipboard`. Run `agent-desktop permissions` to see which case you are in:
`screen_recording` is `not_required` on X11 and `unknown` on Wayland.

## `screenshot` on X11

```bash
agent-desktop screenshot --app gnome-calculator /tmp/calc.png
agent-desktop screenshot --screen 0 /tmp/screen0.png
agent-desktop screenshot /tmp/everything.png
```

- No permission and no prompt. Targets are a window (`--app`, `--window-id`), a
  display by index from `list-displays` (`--screen`), or the primary display with
  no selector (the first display if none is marked primary). With no output path
  the image comes back as base64 in `data`.
- A window is read from its own drawable. Under a compositing window manager
  (mutter) a covered window and a minimized window are both captured; each
  measured 360x504 with a 900x700 editor over the calculator.
- Two windows of one process with the same title: `--window-id` captures the one
  the toolkit reports active, and its twin when exactly two windows share the title.
  Any other tie answers `AMBIGUOUS_TARGET` ("More than one X11 window matches this
  window") with `candidates` and the suggestion to give the windows distinct titles.
- The tool temporarily hides its own cursor overlays during capture and restores
  them afterward. They neither block the screenshot nor appear in its pixels.
- Under a non-compositing window manager (openbox) that read is not possible. A
  covered window answers `ACTION_FAILED` with `details.reason: "covered"` and the
  suggestion to raise it with `--headed focus-window`; the same capture after the
  focus returns the window. A minimized window answers `INVALID_ARGS`, "The window
  is not viewable and the window manager does not composite, so it has no pixels
  to capture", with the suggestion to `restore` it. Both are `not_delivered` and
  `retry: safe`.
- A whole-screen or `--screen` grab on a non-compositing manager is exact
  (1280x800 on the measured display). Under a compositor such as GNOME on X11 the
  root window holds only the manager's own stage, so both forms answer
  `PLATFORM_NOT_SUPPORTED` with `details.reason: "compositing"` and the
  suggestion to capture one window with `--window-id`.
- The area no X window owns (wallpaper, shell chrome) cannot be read under a
  compositor. Do not expect a screenshot to show it.
- `screenshot` has no timeout flag.
- A display-server or caller-deadline timeout stays `TIMEOUT`, including when
  capturing a covered window. It does not prove coverage or advise focus.
- Redirected capture preserves resource, connection and visibility failures;
  only unproven window paint receives the `covered` advice.

## `screenshot` on Wayland

Wayland has no window position and no per-window capture. The portal's
`Screenshot` captures the desktop; the CLI returns the primary display by
default, cropping the image on a multi-monitor desktop. `--screen N` selects
another monitor. Cropping requires dimensions that confirm the monitor layout.

```bash
agent-desktop screenshot /tmp/desktop.png
```

- **The first call shows a consent dialog** ("Allow Applications to Take
  Screenshots?", Deny focused) that is not on the accessibility bus. A person
  must answer it. `screenshot` has no timeout flag: the CLI waits for the portal's
  answer, bounded by the longer of the command's own budget and 30 s.
- **Allow** returns the selected display as a PNG. **Deny**, or a dismissed dialog,
  gives `PERM_DENIED`.
- **Nobody answers**: the portal stops waiting after about 25 s (measured 25.0 s
  twice) and the call answers `TIMEOUT` whose suggestion names the dialog. The
  dialog stays open, and a later call answers `PERM_DENIED` at once, without a second
  dialog, until a person dismisses it. Do not retry in a loop: ask the user to
  answer the dialog.
- The desktop remembers the answer per user for the host process. After an Allow,
  later calls return in about 50 ms with no dialog. After a Deny, every later call
  is `PERM_DENIED` until the user resets it in the desktop's privacy
  settings.
- `--screen N` and a display target crop the whole-desktop image to that monitor.
  When the image does not confirm the monitor layout the call answers
  `PLATFORM_NOT_SUPPORTED` with reason `display_mapping`.
- A known display layout is checked again after consent, including when it had
  only one monitor. A changed layout returns `ACTION_FAILED` without pixels.
- A window target (`--app`, `--window-id`) is `PLATFORM_NOT_SUPPORTED` on Wayland
  for good, not a gap to retry. Capture a display, or read the window
  with `snapshot`.
- The portal writes the PNG under the user's home. The CLI accepts the file only
  inside the temp, runtime or home directory, checks it is a regular file of the
  user made for this request, reads it and removes it.

## Clipboard

```bash
agent-desktop clipboard-set "hello from agent-desktop"
agent-desktop clipboard-get
agent-desktop clipboard-clear
```

`clipboard-get` reads `text` by default; `--format` takes `auto`, `image` or
`file-urls`. `clipboard-set` takes text, `--image <png>` or `--file-url <path>`.
Image content is written to a private file (`--out` picks the path) and the result
carries its `path`, `width` and `height`. An empty clipboard answers `found: false`.

### X11

- `clipboard-set` hands the selection to a detached owner process, the helper
  `agent-desktop-linux-clipboard-owner`, which serves the content until another
  client takes the selection or `clipboard-clear` ends it. The CLI exits first;
  the content stays available while the owner lives.
- **The helper must sit beside the `agent-desktop` binary** in the same directory
  and be owned by the user or root. The helper must be a regular executable file,
  not a symlink. Neither it nor its directory may be writable by a shared group;
  the file must not be world-writable. A group-writable helper is accepted when
  its group is the user's own private group. A world-writable directory is
  accepted only with the sticky bit. A 777
  copy is refused with `PLATFORM_NOT_SUPPORTED`, "writable by others". npm
  installs and release archives ship it in place. A copy of the binary moved alone
  cannot set the clipboard: `clipboard-set` answers `PLATFORM_NOT_SUPPORTED`
  naming the missing helper. Both refusals are `not_delivered`, `safe`.
- Without a clipboard manager the content ends with the owner. GNOME's shell keeps
  only text and PNG after the owner dies, caps text at 4 MiB, and does not take
  over a file copy or a custom target: do not rely on content outliving the owner.
- A file copy serves both `x-special/gnome-copied-files` and `text/uri-list`. A
  paste of a plain-text path pastes nothing in a file manager.
- Transfers above the server's single-request limit (262140 bytes measured) use
  `INCR`; large content works.
- "Nothing there" and "target not available" are different answers: an absent
  format is `absent`, not an error.

### Wayland

- The clipboard goes through `wl-clipboard` (`wl-copy`, `wl-paste`). Install the
  `wl-clipboard` package. Text, images and file lists round-trip against foreign
  readers and writers under GNOME.
- Clipboard routing follows the display-server detector: an empty
  `WAYLAND_DISPLAY` counts as unset; a stale socket falls through to a reachable
  `DISPLAY`. Once Wayland is selected, clipboard failures never fall back to X11.
  Missing `wl-copy` or `wl-paste` returns `PLATFORM_NOT_SUPPORTED` with a
  `wl-clipboard` install hint, even when `DISPLAY` is set.
- After Wayland is selected, connection failure ("Failed to connect to a Wayland
  server"), a seat with no keyboard ("This seat has no keyboard"), and a timeout
  from a shell that no longer serves clients are distinct errors, never an empty
  clipboard.
- `clipboard-clear` uses `wl-copy --clear`.

### Clipboard through FFI

The C library reaches the same code. Its clipboard API is text-only. Set and
clear acquire the cross-process interaction lease; reads have a deadline but
do not acquire that mutation lease. For an FFI host, the matching owner helper
can sit beside `libagent_desktop_ffi.so`; it need not sit beside the host's
executable. There is no stable SONAME yet.
