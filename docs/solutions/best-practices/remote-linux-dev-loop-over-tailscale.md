---
title: Develop the Linux adapter against a remote Ubuntu box over Tailscale
date: 2026-08-31
category: best-practices
module: crates/linux, scripts/remote-linux.sh, scripts/session-env.sh
problem_type: best_practice
component: tooling
symptoms:
  - "AT-SPI returns an empty tree when a command runs from an SSH shell."
  - "DISPLAY=:0 talks to nothing on a box reached through AnyDesk or Chrome Remote Desktop."
  - "AnyDesk closes with 'Status: display_server_not_supported'."
  - "cargo is not found when a command runs through ssh without a login shell."
  - "Tailscale refuses to authorize with 'device with nodekey ... already exists'."
severity: high
tags: [linux, at-spi, tailscale, remote-development, phase-3, x11, wayland]
---

# Develop the Linux adapter against a remote Ubuntu box over Tailscale

## Problem

Phase 3 needs a real Ubuntu desktop. AT-SPI2 is a D-Bus protocol, so the tree exists only
inside a logged-in graphical session. A macOS workstation cannot host one, and a container
cannot either.

Three constraints break the obvious setups:

1. **An SSH shell is not a desktop session.** It has no `DISPLAY` and no
   `DBUS_SESSION_BUS_ADDRESS`. Every AT-SPI call returns an empty tree, with no error that
   names the cause.
2. **The display number is not `:0`.** A remote-desktop product starts its own X server.
   The box in this repository's setup runs on `:1`. A hardcoded `:0` fails silently.
3. **The box sits behind NAT.** Port 22 is filtered on both the public and the LAN address.
   A router port-forward exposes `sshd` to the internet and breaks when the ISP rotates the
   public IP.

## Resolution

Tailscale for transport, `rsync` + `ssh` for the loop, and a stolen session environment for
AT-SPI. Two scripts, 23 lines total.

`scripts/session-env.sh` reads the graphical environment from the running session process
instead of a guess:

```bash
for proc in gnome-shell xfce4-session plasmashell mate-session cinnamon-session lxqt-session; do
  pid=$(pgrep -u "$(id -un)" -x "$proc" | head -1)
  [ -n "$pid" ] && break
done
eval "$(tr '\0' '\n' < "/proc/$pid/environ" | grep -E '^(DISPLAY|...)=' | sed 's/.../export .../')"
```

`scripts/remote-linux.sh` syncs the tree, then runs a command inside that environment.
`target/` stays on the box, so builds stay incremental.

```bash
scripts/remote-linux.sh                                  # cargo test -p agent-desktop-linux
scripts/remote-linux.sh cargo clippy -p agent-desktop-linux --all-targets
```

## Setup — on the Ubuntu box

Run these through the remote-desktop console. The box needs no SSH yet.

```bash
sudo apt update
sudo apt install -y curl git rsync build-essential pkg-config openssh-server at-spi2-core

curl -fsSL https://tailscale.com/install.sh -o /tmp/ts.sh
sudo sh /tmp/ts.sh
sudo tailscale up          # open the printed URL in the box browser
tailscale ip -4            # note this address

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup.sh
sh /tmp/rustup.sh -y

gsettings set org.gnome.desktop.interface toolkit-accessibility true
gsettings set org.gnome.desktop.session idle-delay 0
gsettings set org.gnome.desktop.screensaver lock-enabled false
```

Download the install scripts to a file. A `curl | sh` pipe is easy to mangle in a
remote-desktop paste, and the failure looks like a missing command.

## Setup — force Xorg and auto-login

Warning: this reboots the box. Auto-login leaves the console unlocked, so keep the box off
the open internet.

Ubuntu 24.04 defaults to Wayland. AT-SPI reads the tree on Wayland, but XTest input
synthesis does not work there, and AnyDesk refuses the session outright. Xorg is required
for Phase 3, not a preference.

```bash
sudo sed -i 's/^#*WaylandEnable=.*/WaylandEnable=false/' /etc/gdm3/custom.conf
sudo sed -i 's/^#  AutomaticLoginEnable = true/AutomaticLoginEnable=true/' /etc/gdm3/custom.conf
sudo sed -i 's/^#  AutomaticLogin = user1/AutomaticLogin=<user>/' /etc/gdm3/custom.conf
sudo reboot
```

After the reboot, `echo $XDG_SESSION_TYPE` on the console must print `x11`.

Auto-login is not optional. Without it the box stops at the GDM greeter after every reboot,
no user session exists, and `session-env.sh` fails until a person logs in by hand.

## Setup — on the workstation

```bash
brew install --cask tailscale-app && open -a Tailscale     # sign in with the same account
ssh-keygen -t ed25519 -f ~/.ssh/linuxbox -N ""
ssh-copy-id -i ~/.ssh/linuxbox.pub -o IdentitiesOnly=yes <user>@<tailscale-ip>
```

Add the alias to `~/.ssh/config`. Connection reuse keeps each `rsync` + `ssh` pair to one
handshake:

```
Host linuxbox
  HostName <tailscale-ip>
  User <user>
  IdentityFile ~/.ssh/linuxbox
  ControlMaster auto
  ControlPath ~/.ssh/cm-%r@%h:%p
  ControlPersist 10m
  ServerAliveInterval 30
  ServerAliveCountMax 3
```

## Verify

```bash
ssh linuxbox 'pgrep -a at-spi2-registryd'
ssh linuxbox 'tr "\0" "\n" < /proc/$(pgrep -x gnome-shell | head -1)/environ | grep DISPLAY'
scripts/remote-linux.sh
```

The second command prints the real display number. If it is not `:0`, that is the proof the
env-steal is load-bearing.

## Rules

- Never hardcode `DISPLAY=:0`. Read it from a live session process.
- Keep the Linux box on Xorg for the whole of Phase 3. Revisit Wayland only when input
  synthesis moves to libei or a portal.
- Source `~/.cargo/env` explicitly in any `ssh` command. Ubuntu's `.bashrc` returns early
  for a non-interactive shell, so rustup's PATH line never runs.
- Keep `cargo check --target x86_64-unknown-linux-gnu` on the workstation. Only tests need
  the roundtrip; a type error does not.
- Leave `target/` on the box and exclude it from `rsync`. A full rebuild per sync makes the
  loop useless.

## Gotchas that cost time

| Symptom | Cause |
|---|---|
| `display_server_not_supported` in AnyDesk | The session is Wayland, or the GDM greeter is. AnyDesk supports X11 only. |
| Tailscale: `device with nodekey ... already exists` | Stale state from an earlier install. Run `tailscale logout`, then log in again. |
| `/usr/local/bin/tailscale` is a broken stub on macOS | The cask ships the CLI as `Tailscale` with a capital T. Repoint the stub. |
| `sudo: a terminal is required to read the password` | A non-TTY shell. Run the command on the box console, or add a `sudoers.d` entry. |
| Empty AT-SPI tree, no error | The command ran without `session-env.sh`. |
