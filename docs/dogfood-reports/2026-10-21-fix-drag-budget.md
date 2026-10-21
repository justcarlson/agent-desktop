# Drag duration and action budget

2026-10-21, branch `feat/linux-3.15-fix-drag`, base `f0366d90`.

## F1 — immediate timeout for an impossible duration/budget combination

Disposition: **fixed here**. The final Linux dogfood found that `--headed drag
--from-xy 100,200 --to-xy 400,500 --duration 5000` returned an immediate
`TIMEOUT` with busy-app recovery under the default 5000 ms budget. Core already
uses the same deadline for preparation and physical delivery. It now rejects
explicit timing at or above that budget as `INVALID_ARGS`, naming `--duration`,
`--drop-delay` and `--timeout-ms`, before endpoint resolution or focus. Shared
`DragParams` validation applies the same rule to its other callers. Actual
preparation can still consume a sufficient budget and return `TIMEOUT`.

The CLI help and shipped interaction guide describe the action budget, including
platform pickup/default delays and completion. Optional post-action waits keep
their separate budget.

## Deterministic regression and inversion

`commands::drag::tests::timing_tests::impossible_drag_timing_is_invalid_before_target_work`
failed on the original product with `TIMEOUT` instead of `INVALID_ARGS`. Removing
only the early timing check from the fix made it fail with four endpoint
resolutions instead of zero. Restoring the check passed. The test also checks
that focus, overlay and drag delivery were untouched. The companion headroom
test passes the requested 5000 ms duration and 500 ms drop delay to the fake
adapter with a 10000 ms budget, without sleeping.

## Private X11 proof

One isolated Xvfb session, a native X11 oracle window, and a frozen debug binary:
SHA-256 `462d897456f02e2be9ae36413cda2f20d8585c9ea15bf415c374acede1c338b6`.
The oracle reads X11 button events and `XQueryPointer`, independently of CLI JSON.

| Repetition | Default budget | 10000 ms budget control |
| --- | --- | --- |
| 1 | `INVALID_ARGS`, `not_delivered`; pointer 640,400 unchanged; zero button events | Button1 press at 100,200, release at 400,500 after 5432 ms; final mask 0 |
| 2 | `INVALID_ARGS`, `not_delivered`; pointer 400,500 unchanged; zero button events | Button1 press at 100,200, release at 400,500 after 5429 ms; final mask 0 |

Controls returned `delivered_unverified`; the oracle proves physical delivery,
duration and release. Raw receipts are retained locally in
`.claude/drag-fix/live.log`. An initial stdin-based invocation ran no oracle and
is not counted; the successful run used an oracle file in owned scratch.

## Targeted gates

- Box clippy: core, Linux, binary and FFI, all targets, `-D warnings`: passed.
- Box Linux library: 1793 passed, 0 failed; 197 ignored live tests not run.
- Both new core tests on Linux: 2 passed.
- Mac `cargo test -p agent-desktop-core`: 1453 passed across its test suites.
- Core all-target cross-checks for Windows MSVC and Linux GNU: passed; Linux
  was also checked on the box. These are compile checks, not Windows execution.
- Formatting, Rust file-size, phase-reference and diff-whitespace checks: passed.

The private-session launcher and outer trap remove their owned runtime and
binary/oracle scratch. Independent shell checks confirmed both proof directories
absent after exit. Builds used the shared `~/ad-target`; no separate
`CARGO_TARGET_DIR` was created.
