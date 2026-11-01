# S2: list-displays layout on wlroots compositors

Fork PR #12 into `hypr/base`. Each run is one Omarchy VM boot that ran the x11-openbox, wayland and Hyprland tiers with the binary built from the head shown.

| Run | Channel | Image | Hyprland | Head | Binary sha256 | rc | x11-openbox | wayland | hyprland |
|---|---|---|---|---|---|---|---|---|---|
| s2b-edge-1 | edge | 2026.10.31.471 | 0.57.0-2 | `b66d96c5` | `b84cf47803f4` | 0 | pass 95 | pass 11 | pass 10 |
| s2b-edge-2 | edge | 2026.10.31.471 | 0.57.0-2 | `b66d96c5` | `b84cf47803f4` | 0 | pass 95 | pass 11 | pass 10 |
| s2b-edge-3 | edge | 2026.10.31.471 | 0.57.0-2 | `b66d96c5` | `b84cf47803f4` | 0 | pass 95 | pass 11 | pass 10 |
| s2b-stable-1 | stable | 4.0.6 | 0.56.2-4 | `b66d96c5` | `b84cf47803f4` | 0 | pass 95 | pass 11 | pass 10 |

## Notes

- Counted head `b66d96c5`. The merged commit `c17a710d` has the same patch-id.

## Files

- `versions.txt`: channel, image, Omarchy, kernel, head, binary hash and package versions.
- `rc`: exit code of the whole run.
- `*.log`: tier logs. Each passing leg prints its `finding:` lines; `summary:` gives the counts.
- `failed-evidence/`: JSON evidence kept only for failing legs.
