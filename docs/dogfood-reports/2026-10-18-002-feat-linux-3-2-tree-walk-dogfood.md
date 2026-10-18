# Dogfood: the Linux accessible tree walk

- **Date:** 2026-10-18
- **Branch:** `feat/linux-3.2-tree-walk`, cut from and merging back into `feat/linux-adapter`.
- **Surface:** the internal tree walk only. This sub-phase has no CLI surface: the walk is reachable
  through tests until `snapshot` is wired to it in the observation sub-phase. The stranger-operator
  rule (shipped skill and built binary only) therefore cannot apply here, and the first stranger
  dogfood of the walk runs with the observation sub-phase. What ran instead is the `#[ignore]` live
  tests against real applications, judged from their printed numbers and from the tree dumps they
  produce on failure, never from a test merely passing.
- **Capture safety:** counts, depths, timings and role names only. No titles, paths, pids, host or
  user names.

## Environment

| Item | Value |
|---|---|
| OS | Ubuntu 24.04 LTS, x86_64, GNOME 46 |
| Display | Xvfb virtual display, no window manager |
| Session | private `dbus-run-session` with a private 0700 `XDG_RUNTIME_DIR` per run; the machine's own AT-SPI bus was never touched |
| Toolchain | Rust 1.89.0, zbus (blocking-free `block_on` with a deadline race) |
| Applications | Files 46.4, Terminal 3.52, Calculator 46.3, a PyQt6 window (Qt 6), VS Code (Electron, `--no-sandbox`, no extensions) |
| Harness | `walker_live_real_apps` and `walker_live_stopped_app_returns_a_partial_walk_at_the_deadline`, `--test-threads=1` |

## Judgements

A walk is judged good only when it is complete, has zero unexplained read errors, and a timed run
(min-of-seven after one discarded warm-up) never returns a partial tree.

| Id | Subject | Observed | Verdict |
|---|---|---|---|
| J1 | Files | 367 nodes, depth 27, complete, 0 read errors, 266/276/296 ms (min/median/max) | pass |
| J2 | Terminal | 49 nodes, depth 7, complete, 0 read errors, 34/35/37 ms | pass |
| J3 | Calculator | 98 nodes, depth 16, complete, 0 read errors, 83/87/93 ms; action lists carry machine names (`win.close`, `default.activate`); an exited app classifies as `AppNotFound` | pass |
| J4 | Qt window | 18 nodes, depth 4, complete, 0 read errors, 16/17/23 ms; push button found by its label. Qt answers no `GetAll`, so every property comes from the per-property fallback | pass |
| J5 | VS Code | 402 nodes, depth 33, complete, 0 read errors, 321/330/340 ms. Across four full runs it never produced a read error, so the former blanket waiver had nothing to waive | pass |
| J6 | Stopped application | Calculator stopped with SIGSTOP after the third level was read, deadline 3000 ms: the walk returned at 3000 ms (not deadline plus the 5 s per-call timeout), 3 of 98 nodes, `complete=false`, one node marked truncated; after SIGCONT the same frame walked again to 98 nodes, complete | pass |

J6 is the measurement the deadline design exists for. It stops only the pid the test started, and
resumes it before asserting.

## Findings

| Id | Finding | Disposition |
|---|---|---|
| F1 | A child whose bus name vanished between `GetChildren` and the process-id lookup aborted the whole walk with `PLATFORM_NOT_SUPPORTED`. | Fixed here. A failed lookup other than a timeout leaves the name unrecorded; the child is kept and read as an unreadable node. Tests `a_failed_process_lookup_leaves_the_bus_unrecorded_without_failing_the_level`, `a_process_lookup_that_ran_out_of_budget_fails_the_level`, `a_child_on_a_bus_with_no_known_process_is_still_listed`. Inverted: each of the three fails when its fix is reverted. |
| F2 | A non-root node that was gone discarded the tree already read. | Fixed here. Only the root being gone returns the error; below it the node is kept, marked truncated, its parent marked truncated, and the walk incomplete. Test `a_child_that_vanished_is_kept_and_does_not_abort_the_walk`. Inverted: fails without the parent marking. |
| F3 | A failed `GetChildren` produced no children yet the walk reported `complete=true`. | Fixed here. The node is truncated and the walk incomplete. Test `a_node_whose_children_could_not_be_read_is_truncated_and_the_walk_incomplete`. Inverted: fails when either the truncation or the completeness flag is dropped. |
| F4 | When the interface list failed, extents and actions were reported Absent, claiming no Component or Action interface. | Fixed here: they are Failed. Test `an_unreadable_interface_list_makes_the_dependent_field_a_failure`. Inverted: fails when Failed is turned back into Absent. |
| F5 | Timeout behaviour was untested: a timeout after the first level, a timeout on the first level, and the depth limit. | Fixed here: `a_timeout_after_the_first_level_returns_the_partial_tree_marked_incomplete`, `a_timeout_on_the_first_level_is_an_error`, `a_depth_limit_marks_the_node_it_stops_at_and_the_result_incomplete`. Each inverted and fails. |
| F6 | The live harness accepted a partial walk after a timeout as a pass: timed runs never read `complete`. | Fixed here: a timed run that is incomplete fails the app. Verified by reading the code path and by the complete runs in J1 to J5; not inverted against a real slow application. |
| F7 | The VS Code case waived every read error whenever the walk was complete. | Fixed here: no app tolerates a read error, and every read error is printed with its role, field and detail. Four runs showed none, so no named class of tolerated errors is needed. |
| F8 | The Terminal cleanup killed every `gnome-terminal-server` by process name when none had existed before the test, which can kill a server the test did not launch. | Fixed here: all name-based killing is removed (`processes_named` and the extra-pid list are gone). The terminal child now exits by itself when its guard directory is removed or the test process ends; the only kill left is the process group of a child the harness itself spawned. Audit: no `pkill`, `killall` or name match remains in the three files. |
| F9 | The module-wide `allow(dead_code, unused_imports)` on the tree module hid all new dead code. | Fixed here: item-level allows on `walk_tree`, `atspi_element`, `into_native_handle` and the two not-yet-used re-exports; test-only helpers are `cfg(test)`. Clippy with `-D warnings` passes. |
| F10 | Running a version probe of the VS Code binary while checking the environment started a full VS Code on the shared machine and blocked the run lock. | Accepted as an operator slip, not a product defect: the stray process was identified by its own command line and age and killed by pid. Noted so the next run does not probe app versions by running them. |
| F11 | The walk has no CLI surface, so no stranger run of the shipped skill was possible. | Owned elsewhere: the observation sub-phase wires `snapshot` to the walk and runs the first stranger dogfood; written into that sub-phase's scope in `docs/phases.md`. |

## Not covered

The live tests cannot make a bus name vanish between two calls, so F1 and F2 are proven by unit tests
over the pure pieces (`record_pids`, `child_elements`, `walk_levels`) and not by a live reproduction.
