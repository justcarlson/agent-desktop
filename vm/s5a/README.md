# S5a: display capture through screencopy

Fork PR #20 into `hypr/base`. Each run is one Omarchy VM boot that ran the x11-openbox, wayland and Hyprland tiers with the binary built from the head shown.

| Run | Channel | Image | Hyprland | Head | Binary sha256 | rc | x11-openbox | wayland | hyprland |
|---|---|---|---|---|---|---|---|---|---|
| edge-1 | edge | 2026.10.31.471 | 0.57.0-2 | `79da750f` | `48f0155aa427` | 0 | pass 95 | pass 10 | pass 16 |
| edge-2 | edge | 2026.10.31.471 | 0.57.0-2 | `79da750f` | `48f0155aa427` | 0 | pass 95 | pass 10 | pass 16 |
| edge-3 | edge | 2026.10.31.471 | 0.57.0-2 | `79da750f` | `48f0155aa427` | 0 | pass 95 | pass 10 | pass 16 |
| stable-1 | stable | 4.0.6 | 0.56.2-4 | `79da750f` | `48f0155aa427` | 0 | pass 95 | pass 10 | pass 16 |

## Notes

- S5a and S5b were counted on the same stack head `79da750f`. The same runs are copied under `s5b/79da750f/`.

## Files

- `versions.txt`: channel, image, Omarchy, kernel, head, binary hash and package versions.
- `rc`: exit code of the whole run.
- `*.log`: tier logs. Each passing leg prints its `finding:` lines; `summary:` gives the counts.
- `failed-evidence/`: JSON evidence kept only for failing legs.
