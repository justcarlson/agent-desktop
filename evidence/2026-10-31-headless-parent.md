# M0 probe: headless parent compositor (2026-10-31, the test host)

Question (proposal section 6): can the `hyprland` E2E tier run Hyprland nested in a
headless parent, so that CI can run it?

Method: upstream `scripts/private-session.sh` (private runtime dir, bus, Xvfb),
then a headless parent, then `Hyprland -c <minimal conf>` with
`WAYLAND_DISPLAY` set to the parent socket.

| Parent | Result |
|---|---|
| weston 15.0.1 `--backend=headless --renderer=gl` | Hyprland aborts: `invalid version for global wl_compositor: expected at most 5, got 6`, then `CBackend::create() failed` |
| sway 1.12 `WLR_BACKENDS=headless WLR_RENDERER=gles2` | Hyprland aborts: `invalid version for global xdg_wm_base: expected at most 5, got 6`, then `CBackend::create() failed` |
| Hyprland alone (no parent) | Aquamarine has no headless-only mode; `CBackend::create() failed` (baseline) |
| Host Hyprland 0.56.2 | Works (nested window on a hidden special workspace) |

Result: Aquamarine 0.15 binds `wl_compositor` v6 and `xdg_wm_base` v6. The
packaged headless parents do not offer them. The proposal's fallback applies: the
parent is the host compositor, on a special workspace. The `hyprland` tier
therefore runs on a Hyprland host (self-hosted), not on hosted CI.

Second finding: in upstream `private-session.sh` on Arch, AT-SPI registry
activation fails (`Could not activate remote peer 'org.a11y.atspi.Registry':
unit failed`). Arch's `at-spi-bus-launcher` uses `dbus-broker-launch --scope
user`, which activates through systemd. Starting `/usr/lib/at-spi2-registryd`
inside the private session fixes it (`list-apps` then returns `ok: true`). This
is an upstream harness defect on Arch, separate from Hyprland.

Teardown: `private-session.sh` left no process with the session runtime dir in
its environment after either run.
