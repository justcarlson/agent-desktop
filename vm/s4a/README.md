# S4a: focus-window through the compositor

Fork PR #18 into `hypr/base`. Each run is one Omarchy VM boot that ran the x11-openbox, wayland and Hyprland tiers with the binary built from the head shown.

| Run | Channel | Image | Hyprland | Head | Binary sha256 | rc | x11-openbox | wayland | hyprland |
|---|---|---|---|---|---|---|---|---|---|
| edge-2 | edge | 2026.10.31.471 | 0.57.0-2 | `48faceda` | `a5d0c21e7d44` | 0 | pass 95 | pass 10 | pass 13 |
| r2-edge-1 | edge | 2026.10.31.471 | 0.57.0-2 | `48faceda` | `a5d0c21e7d44` | 0 | pass 95 | pass 10 | pass 13 |
| r2-edge-3 | edge | 2026.10.31.471 | 0.57.0-2 | `48faceda` | `a5d0c21e7d44` | 0 | pass 95 | pass 10 | pass 13 |
| stable-1 | stable | 4.0.6 | 0.56.2-4 | `48faceda` | `a5d0c21e7d44` | 0 | pass 95 | pass 10 | pass 13 |

## Notes

- No notes.

## Files

- `versions.txt`: channel, image, Omarchy, kernel, head, binary hash and package versions.
- `rc`: exit code of the whole run.
- `*.log`: tier logs. Each passing leg prints its `finding:` lines; `summary:` gives the counts.
- `failed-evidence/`: JSON evidence kept only for failing legs.
