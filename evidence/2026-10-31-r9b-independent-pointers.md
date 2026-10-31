# M0 probe: independent pointers (R9b), 2026-10-31, the test host

## Nested compositor per agent

Two private sessions ran at the same time (`private-session.sh` with
`private-hyprland-shell.sh`), each with a GTK3 app. Each moved its own pointer
three times (`hyprctl dispatch movecursor`, nested instance only).

| Time | Agent A cursor | Agent B cursor |
|---|---|---|
| t1 | 110, 110 | 510, 510 |
| t2 | 120, 120 | 520, 520 |
| t3 | 130, 130 | 530, 530 |

The host cursor stayed at 527, 485 from start to end. Both sessions were torn
down with no processes left.

Verdict: each nested compositor has its own seat, so agents get truly
independent pointers and keyboards, but only for apps started inside their own
session. A nested session cannot drive the owner's existing windows. Use it
for E2E and for agent work on apps the agent launches itself. It is not a
replacement for R8 + R9 on the owner's desktop.

## Per-agent compositor seats (considered, rejected)

Per-agent seats on the owner's existing windows exist only as a compositor
plugin. [trycua/cua#4257](https://github.com/trycua/cua/issues/4257) (closed)
documents the limits of that approach:

- The plugin adds agent seats as ordinary `wl_seat` globals, so every client
  sees the extra seats.
- Single-seat apps can bind an agent seat. imv crashes on the agent keyboard's
  `repeat_info(0, 0)`, and Zed loses the user's input.
- Isolated input only works when the target app binds the agent seat, so the
  seats can't be hidden from other clients.
- For single-seat apps, isolated agent input and normal user input exclude
  each other.

agent-desktop is a CLI against stock compositor protocols and can't ship such a
plugin upstream. Per D4, R8 (rendered per-agent cursors) plus R9
(lease-serialized physical input) is the accepted model.
