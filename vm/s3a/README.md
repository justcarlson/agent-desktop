# S3a: bind one Hyprland instance per session

Fork PR #16 into `hypr/base`. Each run is one Omarchy VM boot that ran the x11-openbox, wayland and Hyprland tiers with the binary built from the head shown.

| Run | Channel | Image | Hyprland | Head | Binary sha256 | rc | x11-openbox | wayland | hyprland |
|---|---|---|---|---|---|---|---|---|---|
| r2-edge-1 | edge | 2026.10.31.471 | 0.57.0-2 | `c033f345` | `097b448d3d14` | 0 | pass 95 | pass 11 | pass 13 |
| r2-edge-2 | edge | 2026.10.31.471 | 0.57.0-2 | `c033f345` | `097b448d3d14` | 0 | pass 95 | pass 11 | pass 13 |
| r2-edge-3 | edge | 2026.10.31.471 | 0.57.0-2 | `c033f345` | `097b448d3d14` | 0 | pass 95 | pass 11 | pass 13 |
| r2-stable-1 | stable | 4.0.6 | 0.56.2-4 | `c033f345` | `097b448d3d14` | 1 | fail 1, pass 94 | pass 11 | pass 13 |
| r3-stable-1 | stable | 4.0.6 | 0.56.2-4 | `c033f345` | `097b448d3d14` | 0 | pass 95 | pass 11 | pass 13 |

## Notes

- `r2-stable-1` failed one generic leg (`qualified_refs`, x11-openbox tier). The same leg passed on every other run of this head. `r3-stable-1` is the one retry, and it passed. Under the flake rule this is reported as a flake, not fixed.

## Files

- `versions.txt`: channel, image, Omarchy, kernel, head, binary hash and package versions.
- `rc`: exit code of the whole run.
- `*.log`: tier logs. Each passing leg prints its `finding:` lines; `summary:` gives the counts.
- `failed-evidence/`: JSON evidence kept only for failing legs.
