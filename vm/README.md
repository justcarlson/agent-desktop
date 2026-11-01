# Hyprland VM evidence, S2 to S5

VM evidence for the counted heads of the S2 to S5 slots in `hypr/base`. Each slot folder has its own README with every run.

Edge image `2026.10.31.471` runs Hyprland 0.57.0-2. Stable image `4.0.6` runs Hyprland 0.56.2-4. All runs used throwaway Omarchy VMs; the host ran no display server.

| Slot | PR | Runs | Passing runs | Folder |
|---|---|---|---|---|
| S2: list-displays layout on wlroots compositors | #12 | 4 | 4 | [`s2/`](s2/README.md) |
| S3a: bind one Hyprland instance per session | #16 | 5 | 4 | [`s3a/`](s3a/README.md) |
| S3b: tie each window to one Hyprland toplevel | #17 | 4 | 4 | [`s3b/`](s3b/README.md) |
| S4a: focus-window through the compositor | #18 | 4 | 4 | [`s4a/`](s4a/README.md) |
| S4b: close-app for native Wayland windows | #19 | 4 | 4 | [`s4b/`](s4b/README.md) |
| S5a: display capture through screencopy | #20 | 4 | 4 | [`s5a/`](s5a/README.md) |
| S5b: window capture in place, including hidden workspaces | #21 | 14 | 9 | [`s5b/`](s5b/README.md) |

Failed runs are kept on purpose. The slot READMEs say why each one failed.
