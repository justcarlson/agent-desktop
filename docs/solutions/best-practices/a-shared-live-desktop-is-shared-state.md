---
title: A shared live desktop is shared state - four ways one run breaks every other run
date: 2026-10-18
category: best-practices
module: scripts/remote-linux.sh, scripts/session-env.sh, crates/linux
problem_type: best_practice
component: tooling
severity: high
applies_when:
  - "Several agents or test runs drive one live GNOME session over scripts/remote-linux.sh"
  - "A run starts a second graphical or D-Bus session on the dev box"
  - "A run starts a process that can outlive the command, such as a clipboard owner"
  - "A run synthesizes keyboard or pointer input and reads the effect back"
symptoms:
  - "Every new AT-SPI client fails with 'dbind-ERROR: AT-SPI: Couldn't connect to accessibility bus' and dumps core, while GetAddress still returns the old bus_0 address."
  - "/run/user/<uid>/at-spi/ is empty although at-spi-bus-launcher is still running."
  - "Every remote run waits on the box lock for tens of minutes and no process seems to hold it."
  - "Builds on the box fail on include_str! of a committed docs/ file that exists locally."
  - "Synthetic keys and clicks land nowhere; a seat grab returns AlreadyGrabbed."
tags: [linux, at-spi, x11, wayland, remote-development, shared-state, flock, rsync, gnome-shell, input]
---

# A shared live desktop is shared state - four ways one run breaks every other run

## Context

The Linux adapter is verified against a real GNOME 46 X11 session on a dev box that several agents and test runs drive at once through `scripts/remote-linux.sh`. That session is process-wide shared state: its accessibility bus, its X server, its clipboard and its input grab belong to every run. During the Linux exploration probes, four separate mistakes by one run silently broke the box for every other run, and each looked like a product defect until it was traced. None of them is visible in the final code, which only carries the guards.

## Guidance

1. **Give every second session its own runtime directory.** A run that starts `dbus-run-session` and `gnome-shell --headless --wayland` (or any second session) with the live `XDG_RUNTIME_DIR` also starts its own `at-spi-bus-launcher`. That launcher takes the same `/run/user/<uid>/at-spi/bus_0` path and deletes it when it exits. The live session's bus daemon keeps running on an unlinked socket, `org.a11y.Bus.GetAddress` still answers with the old address, and every new AT-SPI client fails. Set `XDG_RUNTIME_DIR` to a private `0700` directory before the second session starts, and check that `/run/user/<uid>/at-spi/bus*` still exists afterwards. `scripts/remote-linux.sh` now prints a warning after every run when the socket is missing, and it refuses a command that starts `dbus-run-session` without setting `XDG_RUNTIME_DIR`. A test suite run the way CI runs it (`dbus-run-session -- xvfb-run ... cargo test`) is a second session too: on a CI runner it is the only session, but on the box it deleted the live socket a second time.
2. **Detach anything that outlives the command.** `echo hi | xclip -selection clipboard` returns at once, but the forked selection server keeps the caller's open descriptors, including the box lock, and the caller's stdio pipes. One such `xclip` held the lock for half an hour. The script now takes the lock with `flock -o`, which keeps the lock out of children, and `-w` bounds the wait. A probe that sets the clipboard starts the owner with stdin, stdout and stderr on `/dev/null` under `setsid`, records its pid, and stops it before it finishes.
3. **Sync the files git lists, not a `.gitignore` merge.** `rsync --filter=':- .gitignore'` reads `.gitignore` as rsync filter syntax, where a line that starts with `!` clears the rule list instead of re-including a path. This repository's `.gitignore` excludes `docs/*` and re-includes files with `!`, so `docs/` never reached the box and every all-targets build there failed on an `include_str!` of a committed doc. The script now builds an rsync filter from `git ls-files -co --exclude-standard`, escapes rsync pattern characters, and mirrors exactly that list.
4. **Check for a held grab before you trust undelivered input.** A GNOME Shell modal, such as a password prompt from a background updater, holds an active keyboard and pointer grab. While it is open, synthetic keys and clicks reach no window, and a GDK seat grab returns `AlreadyGrabbed`. `xdotool key XF86LogGrabInfo` makes Xorg write the grab holder to its log (`~/.local/share/xorg/Xorg.<n>.log` for a rootless Xorg), which names `gnome-shell` and its pid. A run that finds a held grab records its input rows as blocked by the environment, not as product findings. Clearing the modal needs the session's owner: a logout, or a restart of `org.gnome.Shell@x11.service`.

Every rule protects the next run, not the current one. A run that breaks one of them usually still passes itself.

## Why This Matters

Each failure surfaced far from its cause and looked like a product result. The deleted bus socket looked like an adapter that could not connect. The held lock looked like a slow box. The unsynced docs looked like a broken build. The held grab looked like an input path that does not work on GTK4. Captures taken while any of these held were wrong, and some were nearly recorded as findings about AT-SPI. A restart of the a11y bus also leaves every app that was already running, including GNOME Shell, off the new bus until that app restarts, so one mistake costs the rest of the session its shell-surface coverage.

## When to Apply

- Before a run starts any second session, compositor or bus launcher on the dev box.
- Before a run starts a process that can outlive its command: a clipboard owner, a launched app, a background helper.
- When a remote build fails on a file that exists locally, check what the sync sent before you debug the build.
- When synthetic input has no effect, check for a held grab before you conclude anything about the input path.

## Examples

A private runtime directory for a headless Wayland session:

```bash
R=$(mktemp -d "$HOME/ad-wt/rt.XXXXXX"); chmod 700 "$R"
XDG_RUNTIME_DIR=$R dbus-run-session -- bash -c 'unset DISPLAY; gnome-shell --headless --wayland --virtual-monitor 1280x800 & ...'
ls /run/user/$(id -u)/at-spi/bus*   # must still list the live socket afterwards
rm -rf "$R"
```

A clipboard owner that holds nothing it should not:

```bash
setsid xclip -selection clipboard -i < payload.txt > /dev/null 2>&1 &
owner=$!
...
kill "$owner"
```

Naming the grab holder:

```bash
log=$(ls -t ~/.local/share/xorg/Xorg.*.log | head -1)
xdotool key XF86LogGrabInfo
grep -A3 'Active grab' "$log" | tail -8
```

`scripts/private-session.sh [--wm] -- <command>` is the one way to run bus, display or app commands on the box: it builds the 0700 runtime dir, starts the private bus and Xvfb (and openbox with `--wm`), pushes `DISPLAY` into the bus activation environment once Xvfb answers and before any app starts, then unmounts the doc and gvfs mounts and removes the dir. The order matters: a bus that starts before Xvfb hands D-Bus-activated apps such as `gnome-terminal-server` whatever display the caller carried, and they map their windows on the owner's real screen while the private AT-SPI tree still reports `SHOWING` (ledger L18-17). The helper clears `AT_SPI_BUS_ADDRESS`, `NO_AT_BRIDGE` and `GTK_A11Y` before it starts the bus, because a toolkit that honours the first connects to whatever bus it names (L12-4) while the adapter reads the private one. With `--wm` it exits non-zero, with openbox's log, when openbox is missing or never becomes ready, and never runs the command without the window manager it was asked for. `scripts/test-private-session.sh` checks both on the box. The live-test launcher refuses to start without the helper's `AGENT_DESKTOP_PRIVATE_SESSION` marker.

Related: `docs/solutions/best-practices/remote-dev-boxes-over-tailscale.md` (box setup and the remote loop), `docs/solutions/best-practices/a-test-that-acts-on-its-own-runner-acts-on-every-other-test.md` (the same class of hazard inside one test process).
