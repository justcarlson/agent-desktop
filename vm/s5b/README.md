# S5b: window capture in place, including hidden workspaces

Fork PR #21 into `hypr/base`. Each run is one Omarchy VM boot that ran the x11-openbox, wayland and Hyprland tiers with the binary built from the head shown.

| Run | Channel | Image | Hyprland | Head | Binary sha256 | rc | x11-openbox | wayland | hyprland |
|---|---|---|---|---|---|---|---|---|---|
| 79da750f/edge-1 | edge | 2026.10.31.471 | 0.57.0-2 | `79da750f` | `48f0155aa427` | 0 | pass 95 | pass 10 | pass 16 |
| 79da750f/edge-2 | edge | 2026.10.31.471 | 0.57.0-2 | `79da750f` | `48f0155aa427` | 0 | pass 95 | pass 10 | pass 16 |
| 79da750f/edge-3 | edge | 2026.10.31.471 | 0.57.0-2 | `79da750f` | `48f0155aa427` | 0 | pass 95 | pass 10 | pass 16 |
| 79da750f/stable-1 | stable | 4.0.6 | 0.56.2-4 | `79da750f` | `48f0155aa427` | 0 | pass 95 | pass 10 | pass 16 |
| invert-check/break-gl-edge-1 | edge | 2026.10.31.471 | 0.57.0-2 | `1a45a312` | `d0ee0168d5d1` | 1 | not run | not run | fail 1 |
| invert-check/iso2-gl-edge-1 | edge | 2026.10.31.471 | 0.57.0-2 | `1a45a312` | `902b015d9e05` | 0 | not run | not run | pass 1 |
| stack-1a45a312/r2-edge-1 | edge | 2026.10.31.471 | 0.57.0-2 | `1a45a312` | `902b015d9e05` | 0 | pass 95 | pass 11 | pass 30 |
| stack-1a45a312/r2-edge-2 | edge | 2026.10.31.471 | 0.57.0-2 | `1a45a312` | `902b015d9e05` | 0 | pass 95 | pass 11 | pass 30 |
| stack-1a45a312/r2-edge-3 | edge | 2026.10.31.471 | 0.57.0-2 | `1a45a312` | `902b015d9e05` | 0 | pass 95 | pass 11 | pass 30 |
| stack-1a45a312/r2-stable-1 | stable | 4.0.6 | 0.56.2-4 | `1a45a312` | `902b015d9e05` | 0 | pass 95 | pass 11 | pass 30 |
| stack-8b957f56/r1-edge-1 | edge | 2026.10.31.471 | 0.57.0-2 | `8b957f56` | `902b015d9e05` | 1 | pass 95 | pass 11 | fail 1, pass 29 |
| stack-8b957f56/r1-edge-2 | edge | 2026.10.31.471 | 0.57.0-2 | `8b957f56` | `902b015d9e05` | 1 | pass 95 | pass 11 | fail 1, pass 29 |
| stack-8b957f56/r1-edge-3 | edge | 2026.10.31.471 | 0.57.0-2 | `8b957f56` | `902b015d9e05` | 1 | pass 95 | pass 11 | fail 1, pass 29 |
| stack-8b957f56/r1-stable-1 | stable | 4.0.6 | 0.56.2-4 | `8b957f56` | `902b015d9e05` | 1 | pass 95 | pass 11 | fail 1, pass 29 |

## Notes

- `79da750f/`: the S5a and S5b stack head, counted once for both slots.
- `stack-8b957f56/`: the whole stack before the session-count oracle. All four runs failed one leg, `capture.hyprland_in_view_first_frame`. Each failed the old 250 ms timing budget for an unchanged window in view under virgl. `r1-edge-2` also failed in a second way: the slow-redraw rounds 1 and 2 returned `WINDOW_NOT_FOUND` for a live window identified through accessibility, after round 0 captured it. That second failure did not recur in the later runs. It is open and under investigation.
- `stack-1a45a312/`: the whole stack at the merged head, with the in-view leg counting capture sessions in the Wayland protocol log instead of timing. All four runs pass.
- `invert-check/`: the in-view leg alone. `iso2-gl-edge-1` ran the merged binary and passed. `break-gl-edge-1` ran a build with the in-view check disabled (not committed) and failed with 11 capture sessions instead of 1. This shows the leg detects the defect it guards.

## Files

- `versions.txt`: channel, image, Omarchy, kernel, head, binary hash and package versions.
- `rc`: exit code of the whole run.
- `*.log`: tier logs. Each passing leg prints its `finding:` lines; `summary:` gives the counts.
- `failed-evidence/`: JSON evidence kept only for failing legs.
