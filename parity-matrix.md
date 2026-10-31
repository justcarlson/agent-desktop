# Swap parity matrix (M0)

Audit: 2026-10-30. Scope: section 4.1 of the proposal (swap inventory, M0 artifact; the proposal text is not part of this branch).
This is a source audit. It does not certify the Hyprland route.

## Sources

| Source | Version or location |
|---|---|
| Requirements | Proposal requirements R1–R11, approved draft 5, dated 2026-10-31 |
| computer-use-linux | Commit `ab17e47`: MCP server implementation and tool schemas, skill, and Pi adapter |
| agent-desktop | Tag `v1.0.2`; debug binary reports `1.0.2`. [README](https://github.com/vercel-labs/agent-desktop/blob/v1.0.2/README.md), [CLI definitions](https://github.com/vercel-labs/agent-desktop/blob/v1.0.2/src/cli/mod.rs), [arguments](https://github.com/vercel-labs/agent-desktop/blob/v1.0.2/src/cli_args/mod.rs), [Linux skill](https://github.com/vercel-labs/agent-desktop/blob/v1.0.2/skills/agent-desktop-linux/SKILL.md), [JSON contract](https://github.com/vercel-labs/agent-desktop/blob/v1.0.2/docs/json-output.md) |
| Hyprland behavior | [Recorded baseline](evidence/2026-10-31-baseline.md). Its date is later than this audit date. No desktop action was run for this matrix. |

CLI code is in `src/cli` and `src/cli_args`. The existing binary supplied help
output; no build was needed.

## MCP parameters

The current server defines **21 tools**. Proposal section 4.1 names 15. The six
additional tools are included below. All parameters are optional unless marked
**required**. These groups expand to individual MCP fields; they are not nested
objects.

| Group | Fields and types |
|---|---|
| W: window selectors | `window_id: integer`, `pid: integer`, `tty: string`, `terminal_pid: integer`, `terminal_command: string`, `terminal_cwd: string`, `app_id: string`, `wm_class: string`, `title: string` |
| V: pointer window selectors | `window_id: integer`, `pid: integer`, `app_id: string`, `wm_class: string`, `window_title: string` |
| S: screenshot selectors | `window_id: integer`, `pid: integer`, `app_id: string`, `wm_class: string`, `title: string` |
| E: element selectors | `element_index: integer`, `role: string`, `name: string`, `text: string`, `states: string[]` (default empty) |
| I: image options | `max_width: integer`, `max_height: integer` (default 1920 each); `max_bytes: integer` (default 2 MiB); `scale: number` (0–1); `format: png or jpeg` (default png); `quality: integer` (1–95, default 80; JPEG only) |
| P: post-action observation | `screenshot_after: boolean or "screen" or "window" or null`; all I fields; `include_tree: boolean` (default false) |

| MCP tool | Complete input surface |
|---|---|
| `get_app_state` | W + I; `app_name_or_bundle_identifier: string`, `max_nodes: integer` (default 1000, cap 10000), `max_depth: integer` (default 32, cap 64), `include_screenshot: boolean` (default true), `verbose: boolean` (default false) |
| `list_windows` | None |
| `focused_window` | None |
| `activate_window` | W + P |
| `screenshot` | S + I; `raise_window: boolean` (default true), `full_screen: boolean` (default false). No terminal selectors. |
| `click` | E + V + P; `x: integer`, `y: integer`, `coordinate_space: capture or atspi` (default capture), `button: string`, `click_count: integer`, `relative: boolean` |
| `type_text` | W + P; **`text: string`** |
| `press_key` | W + P; **`key: string`** |
| `scroll` | V + P; **`direction: string`**; `element_index: integer`, `x: integer`, `y: integer`, `pages: number` (default 1, absolute value at most 100), `relative: boolean` |
| `drag` | P; **`start_x: integer`, `start_y: integer`, `end_x: integer`, `end_y: integer`** |
| `set_value` | E + P; `element_identifier: string`; **`value: string`** |
| `perform_action` | E + P; `element_identifier: string`, `action: string` (omission selects primary action) |
| `get_tree_page` | `offset: integer` (default 0) |
| `move_window` | W + P; **`x: integer`, `y: integer`** |
| `resize_window` | W + P; **`width: integer`, `height: integer`** |
| `list_apps` | None |
| `doctor` | None |
| `setup_accessibility` | None |
| `setup_window_targeting` | None |
| `complete_interaction` | None. Exposed only with `COMPUTER_USE_LINUX_NOTIFY_ON_COMPLETE=1`. |
| `run_shell` | **`command: string`**; `cwd: string`, `env: object<string,string>` (default empty), `timeout_seconds: integer` (default 30, max 120). Exposed only with `COMPUTER_USE_LINUX_ENABLE_SHELL=1`. |

## Command and requirement mapping

Commands below omit the `agent-desktop` executable prefix. `REF` means a
qualified ref from a fresh snapshot, such as `@s8f3k2p9:e8`. `ID` means the
opaque string from agent-desktop `list-windows`, not a computer-use-linux
numeric window id.

R1 session binding, R2 identity and R9 concurrency apply across the route.
R10 governs every JSON result, in addition to the R1–R9 mapping.

| computer-use-linux tool | agent-desktop command and flags | Requirement | Gaps on Hyprland 1.0.2 |
|---|---|---|---|
| `get_app_state` | `snapshot --window-id ID --include-bounds --max-depth N`; optionally `-i --compact`; then `screenshot --window-id ID PATH.png`; `status` for readiness | R1, R2, R2b, R7, R9 | Tree works. Window capture refuses. No combined tree/image call, raw node cap, image compression or matching diagnostics schema. |
| `list_windows` | `list-windows [--app APP] [--timeout-ms MS]` | R1, R2, R2b | AT-SPI listing works. Compositor-only terminals and compositor identity need R2b. |
| `focused_window` | `list-windows`; select the unique row with `is_focused: true` | R2, R2b, R3 | No dedicated command. AT-SPI state alone does not provide the required compositor focus proof. |
| `activate_window` | `--headed focus-window --window-id ID`; alternatively `--app APP --title TITLE` | R2, R2b, R3 | Refuses with `wayland_session`. R3 remains a required gate and supports targeted physical workflows. |
| `screenshot` | `screenshot --window-id ID PATH.png`; `screenshot --screen N PATH.png`; no selector captures primary display | R2, R2b, R7, R8 | Window capture refuses. Display capture uses portal consent. R7 needs bound-toplevel capture and screen capture without a dialog. No image-size/quality flags or implicit raise. |
| `click` | `click REF`; `right-click REF`; `--headed mouse-click --xy=X,Y --button BUTTON --count N`; `--headed double-click REF` or `triple-click REF` | R2, R3, R6, R9 | Semantic click works. Physical pointer route is X11 only. No numeric-index bridge or relative-crop flag. Double/triple click are outside G-swap absent observed use of those variants. |
| `type_text` | `type REF TEXT [--timeout-ms MS]`; use `--headed type REF TEXT` for physical delivery | R2, R2b, R3, R4, R9 | CLI requires an editable ref; MCP can type to focused or window-targeted terminals without a tree. R2b terminal typing needs a defined CLI path. Hyprland physical keys need R4. |
| `press_key` | `press COMBO [--app APP or --window-id ID] [--force]`; `--headed` permits activation | R2, R2b, R3, R4, R9 | Exact scoped focus cannot be proved; headed scope refuses. Chord spelling must be translated. R4 still ships before swap. |
| `scroll` | `--headed scroll REF --direction D --amount N`; or `--headed mouse-wheel --x X --y Y --dy DY --dx DX` | R2, R3, R6, R9 | Physical wheel is X11 only. MCP pages map to wheel units, not CLI amount 1:1. No relative-crop option. |
| `drag` | `--headed drag --from-xy=X,Y --to-xy=X,Y [--duration MS] [--drop-delay MS]`; refs use `--from REF --to REF` | R2, R3, R6, R9 | X11 only. R6 still includes drag before swap. Coordinate-only drag gives no receiver proof. |
| `set_value` | `set-value REF VALUE [--timeout-ms MS]` | R2, R9 | Semantic route works where AT-SPI supports it. Explicit identifiers and numeric indices must become refs. |
| `perform_action` | Capability-specific `click`, `right-click`, `toggle`, `check`, `uncheck`, `expand`, `collapse`, `select REF VALUE` | R2, R9 | No generic named AT-SPI action command. Inspect `available_actions`; unsupported arbitrary names have no exact CLI equivalent. |
| `get_tree_page` | `snapshot --root REF [-i --compact]`; `find --window-id ID --root REF --limit N` for searches | R2, R9 | No cached offset pagination. Root snapshot is a new live subtree read; it does not reproduce a cached page. |
| `move_window` | `move-window --window-id ID --x=X --y=Y` (or `--app APP`) | R2, R5 | Wayland control refuses. IPC dispatch and config-language detection remain M6 work. |
| `resize_window` | `resize-window --window-id ID --width W --height H` (or `--app APP`) | R2, R5 | Wayland control refuses. IPC dispatch and verified geometry remain M6 work. |
| `list_apps` | `list-apps [--app APP] [--timeout-ms MS]` | R1, R2 | Works through AT-SPI; does not reproduce the MCP process-candidate/accessible-root split. |
| `doctor` | `status`; `permissions`; `version` | R1, R2, R7 | CLI health works, but lacks the plugin's portal/input/backend diagnostic schema. R1 must diagnose binding failures. |
| `setup_accessibility` | `status`; `permissions` can inspect state | R1 (setup prerequisite) | No command that sets GNOME accessibility and verifies the saved setting. Host setup remains separate. |
| `setup_window_targeting` | None | R2, R3 (GNOME-specific setup) | No GNOME extension installer; outside Hyprland scope. |
| `complete_interaction` | None | No direct R1–R9 requirement | No best-effort completion-notification command. Ending a session is not a notification equivalent. |
| `run_shell` | Calling client's shell tool, outside agent-desktop | No direct R1–R9 requirement | CLI has no general shell subcommand. Client owns cwd, environment, deadline and output limits. |

The following required operation has no computer-use-linux tool:

| Operation | agent-desktop command | Requirement | Gap |
|---|---|---|---|
| Graceful app close | `close-app APP` | R5 | Wayland graceful close needs implementation. Prove process exit or report the blocking dialog. |

## Results the agent consumes

computer-use-linux returns MCP text/structured JSON, with separate inline
image content blocks when requested. agent-desktop returns JSON on stdout.
Its envelope is `{version:"2.5", ok, command, data}` on success and
`{version, ok:false, command, error}` on failure. Exit codes are 0, 1 and 2
(success, structured error, parse error). Read `disposition.delivery` and
`disposition.retry` before retrying a mutation; `ok:true` alone is not effect
verification.

| Tool | computer-use-linux result | agent-desktop result consumed by the agent |
|---|---|---|
| `get_app_state` | `window_context`, `accessibility_tree`, `accessibility_tree_raw_count`, `accessibility_tree_spill` (`path`, `node_count`, `bytes`, `inline_node_count`), `tree_scoped`, `accessibility_tree_truncated`, `readiness`; optional `diagnostics`, `screenshot` metadata, inline image; separate error fields | Snapshot `data.app`, `window`, `snapshot_id`, `ref_count`, `complete`, `tree`; partial reads can include `truncated`, `nodes_observed`, node `subtree_truncated`. Capture is a separate result. |
| `list_windows` | `backend`, `windows[]`, `error`, `permissions_hint`, `note` | `data` is an array: `id`, `title`, `app_name`, `pid`, optional `process_instance`, `bounds`, plus `is_focused`, `accessible`, optional `minimized`, `visible`. |
| `focused_window` | `backend`, nullable `focused_window`, `error`, `permissions_hint`, `message` | Same window array; agent filters `is_focused`. An empty selection does not prove compositor focus. |
| `activate_window` | `ok`, `implemented`, `backend`, `focus`, `error`, `permissions_hint`, `received`; optional P observation | `data.focused` contains the confirmed window; failure carries `error.code`, `details` and `disposition`. |
| `screenshot` | Inline PNG/JPEG image and JSON caption: `width`, `height`, `coordinate_width`, `coordinate_height`, `scale`, `resized`, `bytes`, `original_bytes`, `max_bytes`, `format`, `quality`, `source`, `cropped_to_window`, `window_title`; optional off-screen warning | With output path: `data.path`, `format`, `width`, `height`, `scale_factor`. Without path: base64 in `data.data`, same metadata. No inline MCP image block. The agent must load the PNG through its image tool. |
| `click` | `ok`, `implemented`, `action`, `message`, `received`; optional P observation | Ref action: `data.action`, `steps`, `disposition`. Coordinate action also reports `target` and request context `x`, `y`, `count`; coordinates do not prove receiver identity. |
| `type_text` | Same action fields; optional P observation | `data.action`, `steps`, `disposition`; readback contradiction fails. Successful unverified delivery does not prove persistence. |
| `press_key` | Same action fields; optional P observation | JSON press result and `disposition`; focus or delivery failure is a structured error. Verify app state separately. |
| `scroll` | Same action fields; optional P observation | Ref action report or wheel operation report with `action`, `steps`, `target`, `disposition`; wheel context includes `dx`, `dy`, `x`, `y`. |
| `drag` | Same action fields; optional P observation | `data.action`, `steps`, `target`, `from`, `to`, `disposition`. Dispatch completion alone does not prove the drop. |
| `set_value` | Same action fields; optional `verified` for editable-text readback; optional P observation | `data.action`, `steps`, `disposition`, verification details where available; not the MCP `verified` boolean. |
| `perform_action` | Same action fields; optional P observation | Result of the chosen capability-specific command: `action`, `steps`, `disposition`. |
| `get_tree_page` | `nodes[]`, `offset`, nullable `next_offset`, `node_count`, `message` from cached snapshot | New subtree snapshot or find matches with qualified refs; no `next_offset` contract. |
| `move_window` | `ok`, `implemented`, `backend`, post-operation `window`, `message`, `permissions_hint`, `received`; optional P observation | `data.action`, `steps`, `target`, `disposition`; verified report names `details.verification_scope: window_position`, tolerance and units. |
| `resize_window` | Same geometry fields; optional P observation | Same operation report; verified scope is `window_size`. |
| `list_apps` | `apps[]` process candidates, `accessible_apps[]`, `accessibility_error`, `note` | `data.apps[]`: `name`, `pid`, optional `bundle_id`, `process_instance`, `presentation`; incomplete inventory adds `complete:false`, `skipped`. |
| `doctor` | `platform`, `portals`, `accessibility`, `windowing`, `input`, `readiness`, `capabilities` | Separate status/permissions/version envelopes; no schema-preserving conversion. |
| `setup_accessibility` | `before`, `accessibility_command`, `after`, `changed_accessibility`, `requires_target_app_restart`, `message` | Health inspection only; no setup result equivalent. |
| `setup_window_targeting` | `extension_dir`, `wrote_files`, `enable_command`, `windows`, `windows_error`, `permissions_hint`, `requires_shell_reload`, `message` | No equivalent. |
| `complete_interaction` | `ok`, `cue`, `note` | No equivalent. |
| `run_shell` | `ok`, `command_sha256`, `cwd`, `timeout_seconds`, `exit_code`, `stdout`, `stderr`, truncation flags, `error` | Client shell result, outside the CLI JSON contract. |

## Options across tools

| Option or selector | Mapping and remaining gap |
|---|---|
| `screenshot_after` | Available on activate, click, type, press, scroll, drag, set-value, perform-action, move and resize. `true` or `"screen"` captures the desktop; `"window"` crops the action target or focused window; false/null disables it. CLI needs an explicit action followed by `screenshot`. `batch` can sequence commands but is non-atomic; inspect each entry's `ok`. It does not fuse a PNG into the action result. |
| `include_tree` | Same ten tools. MCP refreshes its compacted tree and cached indices. CLI needs a fresh `snapshot`; global `--wait-for role:text` or `--wait-for-gone` can return a post-action snapshot for supported ref actions. A fresh observation alone is not independent proof of every requested effect. |
| P result failures | MCP can keep action `ok:true` while reporting `screenshot_error` or `accessibility_error`. CLI separates action and observation envelopes; preserve both results and do not repeat the action because capture failed. |
| Image sizing | I fields apply to app-state, screenshot and P. CLI has no `max_width`, `max_height`, `max_bytes`, `scale`, `format` or `quality` flags; it emits PNG. Keep the original coordinate frame when displaying a resized image. |
| `window_id`, `pid`, `app_id`, `wm_class`, `title` | Re-enumerate agent-desktop ids. Use `--window-id` for an exact target or `--app` for the listed app name. Only `focus-window` offers `--title`. Other title/pid selection requires filtering inventory and checking a unique process instance. There are no equivalent pid/app_id/wm_class selector flags. Never pass the old numeric id as a CLI id. |
| `tty`, `terminal_pid`, `terminal_command`, `terminal_cwd` | W only: app-state, activate, type, press, move, resize. CLI has no terminal discovery selectors. Resolve a unique compositor window externally or add support; an AT-SPI-only inventory cannot replace this for Ghostty/Alacritty. R2b is required. |
| `element_index`, `element_identifier`, role/name/text/states | Indices and raw AT-SPI identifiers are not CLI refs. Use `snapshot`, or `find --window-id ID --role ROLE --name NAME --text TEXT --state TOKEN=BOOL`, then a qualified ref. Use `--exact` when needed. Require a unique match; do not select the first ambiguous result. |
| Relative coordinates | MCP click/scroll `relative:true` uses the clipped window-capture origin, before preview scaling. CLI physical commands accept absolute coordinates only. Translate using validated geometry and image scale; R6 must handle fractional scale and negative origins. `coordinate_space:atspi` is a KWin-specific MCP route, not a Hyprland mapping. |
| Wheel units | MCP uses `round(max(abs(pages),0.1) * 5)`, at least one unit. CLI `scroll --amount` defaults to 3. `mouse-wheel` uses signed lines: positive dy up, positive dx right. Check direction and units explicitly. |
| Focus side effects | MCP targeted screenshot normally raises the window; `raise_window:false` disables that request. Targeted input also activates. CLI requires explicit `--headed` for focus changes; headless calls must preserve focus/workspace/pointer. Capture under R7 must work on hidden windows without activation. |
| Shared sessions | Pass `--session SESSION --agent-id AGENT` consistently. R8 requires `session start --cursor --multi-agent`; R9 requires four agents, concurrent semantic delivery and serialized physical input. CLI 1.0.2 saves a Wayland overlay with `rendered:false`; semantic ref actions still take the interaction lease. Neither gate is satisfied yet. |

## Clients

agent-desktop 1.0.2 ships a **CLI and C FFI library, no MCP server**. Its help
lists no MCP/serve command, and its source has no MCP transport implementation.
A custom MCP wrapper would be additional code and is not part of this parity
claim. Any agent client with a shell tool can use the CLI after loading the core
and Linux skills:

```sh
agent-desktop skills get desktop
agent-desktop skills get platform
agent-desktop list-windows
agent-desktop snapshot --window-id ID -i --compact
```

A client saves a PNG and reads it with its own image reader. A shell call that
prints a PNG path does not establish that the agent can see the image. M5 must
verify the route in a fresh client session with only the shipped binary and skill.

## Window operations required before swap (R5 decision)

`move_window` and `resize_window` stay in M6 with minimize, maximize and
restore. Required window operations before swap are:

- `close-app APP`: explicit R5/M4 requirement, although the MCP server has no
  matching tool. Success must prove process exit or report the blocking dialog.
- `--headed focus-window --window-id ID`: explicit R3/M1 requirement and the
  activation dependency of targeted input.
