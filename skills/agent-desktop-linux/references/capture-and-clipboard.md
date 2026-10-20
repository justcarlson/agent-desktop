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
  display by index from `list-displays` (`--screen`), or the whole screen with no
  selector. With no output path the image comes back as base64 in `data`.
- A window is read from its own drawable. Under a compositing window manager
  (mutter) a covered window and a minimized window are both captured; each
  measured 360x504 with a 900x700 editor over the calculator.
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

## `screenshot` on Wayland

Wayland has no window position and no per-window capture. Only the no-selector
form works, through the desktop portal's `Screenshot`.

```bash
agent-desktop screenshot /tmp/desktop.png
```

- **The first call shows a consent dialog** ("Allow Applications to Take
  Screenshots?", Deny focused) that is not on the accessibility bus. A person
  must answer it. `screenshot` has no timeout flag: the CLI waits for the portal's
  answer, bounded by the longer of the command's own budget and 30 s.
- **Allow** returns the whole desktop as one PNG. **Deny**, or a dismissed dialog,
  gives `PERM_DENIED`.
- **Nobody answers**: the portal stops waiting after about 25 s (measured 25.0 s
  twice) and the call answers `TIMEOUT` whose suggestion names the dialog. The
  dialog stays open, and a later call answers `PERM_DENIED` without a dialog; the
  call made right after the timeout can spend about 16 s while the portal refuses
  a second dialog. Do not retry in a loop: ask the user to answer the dialog.
- The desktop remembers the answer per user for the host process. After an Allow,
  later calls return in about 50 ms with no dialog. After a Deny, every later call
  is `PERM_DENIED` until the user resets it in the desktop's privacy
  settings.
- `--screen N` and a display target crop the whole-desktop image to that monitor.
  When the image does not confirm the monitor layout the call answers
  `PLATFORM_NOT_SUPPORTED` with reason `display_mapping`.
- A window target (`--app`, `--window-id`) is `PLATFORM_NOT_SUPPORTED` on Wayland
  for good, not a gap to retry. Use a whole-desktop capture, or read the window
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
  and be owned by the user. It must not be world-writable or writable by a shared
  group. A group-writable helper is accepted when its group is the user's own
  private group: the shipped helper is mode 775 in that group and works. A 777
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
  `INCR`; large content works but is slower.
- "Nothing there" and "target not available" are different answers: an absent
  format is `absent`, not an error.

### Wayland

- The clipboard goes through `wl-clipboard` (`wl-copy`, `wl-paste`). Install the
  `wl-clipboard` package. Text, images and file lists round-trip against foreign
  readers and writers under GNOME.
- When `wl-copy` or `wl-paste` is missing and `DISPLAY` is set, the call uses the X11
  route; an Xwayland client reads and writes the same clipboard once the seat has a
  keyboard. With no `DISPLAY` the error names the `wl-clipboard` package.
- Three failures are distinct and never read as an empty clipboard: an empty or
  stale `WAYLAND_DISPLAY` ("Failed to connect to a Wayland server"), a seat with no
  keyboard ("This seat has no keyboard"), and a timeout from a shell that no
  longer serves clients.
- `clipboard-clear` uses `wl-copy --clear`.

### Clipboard through FFI

The C library reaches the same code. Calls are text-only through the FFI and are
serialized. There is no stable SONAME yet.
