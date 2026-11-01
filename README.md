# Hyprland evidence for agent-desktop

M0 protocol probe evidence and acceptance legs for proposed Hyprland support in
[vercel-labs/agent-desktop](https://github.com/vercel-labs/agent-desktop).
Baseline: agent-desktop 1.0.2 on Hyprland 0.56.2.

Probes that send input, activate windows or draw ran inside a nested private
Hyprland session. On the host they only read (`hyprctl -j`, toplevel lists).
Requirement IDs (R1–R11, D4, M0–M6) refer to the proposal text, which is not
part of this branch.

| Path | Contents |
|---|---|
| `evidence/` | Dated write-ups: baseline CLI behavior, headless parent compositor, protocol probes (identity, activate, keys, capture, cursor), independent pointers |
| `parity-matrix.md` | Mapping from the computer-use-linux MCP tools to agent-desktop commands, with gaps on Hyprland |
| `legs.md` | Planned acceptance legs per milestone, with oracles, dispositions and negative controls |
| `probes/` | Probe scripts `p1`–`p5`, helpers, and raw output |
| `probes/wlprobe/` | Throwaway Rust Wayland client used by the probes (`Cargo.toml`, `Cargo.lock`, `protocols/`, `src/`) |
| `probes/out/` | Logs, JSON and PNG captures from the nested session; `host/` holds read-only host reads with titles and classes generalized |
| `vm/` | Omarchy VM runs for the counted heads of slots S2 to S5 (fork PRs #12 and #16 to #21): one folder per slot with head SHA, image versions, tier logs and leg summaries |

The probe scripts expect a GTK3 test app (button "Increment", entry, checkbox
"Opt", window title `ADTest`) at `$APP`; use `APP=probes/gtk3_app.py`. Build the
probe client with `cargo build` in `probes/wlprobe/`, and set `AD` to the
`agent-desktop` binary if it is not on `PATH`.
