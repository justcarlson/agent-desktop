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

- No permission and no prompt. A window is read from its own drawable, so a window
  that another window covers, or that is minimized, is still captured.
- Targets are a window (`--app`, `--window-id`), a display by index from
  `list-displays` (`--screen`), or the whole screen with no selector. With no
  output path the image comes back as base64 in `data`.
- Under a compositing session such as GNOME on X11, a root-window grab does not
  show mapped windows. The CLI never returns it as the screen: a whole-screen
  grab on a compositing session answers `PLATFORM_NOT_SUPPORTED` with reason
  `compositing`. Capture a window or a display instead.
- On a non-compositing window manager (openbox), a root grab is exact. A covered
  window is read through a Composite redirect, and the image is returned only when
  the repaint proves the whole target is covered or the pixmap differs from the
  screen in every pixel of its content area. When the paint cannot be proven the
  call answers `TIMEOUT` and never an image that might be blank. Bring the window
  forward with `--headed focus-window` and capture again.
- The area no X window owns (wallpaper, shell chrome) cannot be read under a
  compositor. Do not expect a screenshot to show it.

## `screenshot` on Wayland

Wayland has no window position and no per-window capture. Only the no-selector
form works, through the desktop portal's `Screenshot`.

```bash
agent-desktop screenshot /tmp/desktop.png
```

- **The first call shows a consent dialog** ("Allow Applications to Take
  Screenshots?", Deny focused) that is not on the accessibility bus. A person
  must answer it. The CLI waits up to the command's deadline.
- **Allow** returns the whole desktop as one PNG. **Deny**, or a dismissed dialog,
  gives `PERM_DENIED`.
- **Nobody answers**: `TIMEOUT` whose suggestion names the dialog. The dialog
  stays open until a person dismisses it, and while it is open the next call
  answers `PERM_DENIED` at once. Do not retry in a loop: ask the user to answer
  the dialog.
- The desktop remembers the answer per user for the host process. After an Allow,
  later calls return in about 50 ms with no dialog. After a Deny, every later call
  is `PERM_DENIED` at once until the user resets it in the desktop's privacy
  settings.
- A request can take about 20 s on a headless shell that has been idle for a few
  seconds, whatever the grant. Use a long `--timeout-ms` for the first call.
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
- **The helper must sit beside the `agent-desktop` binary** in the same directory,
  owned by the user and not writable by others or by a shared group. npm installs and
  release archives ship it there. A copy of the binary moved alone cannot set the
  clipboard: `clipboard-set` answers `PLATFORM_NOT_SUPPORTED` naming the missing
  helper, `not_delivered`, `safe`.
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
