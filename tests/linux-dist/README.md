# Linux distribution checks

Build the CLI, FFI library and clipboard owner from the same checkout. Create
the CLI and FFI archives using the release workflow layout. Run each command
inside its own private session:

```sh
scripts/private-session.sh --wm -- tests/linux-dist/run-consumer.sh <extracted-ffi-directory>
scripts/private-session.sh --wm -- scripts/ci-ffi-helper-discovery-smoke.sh
scripts/private-session.sh --wm -- tests/linux-dist/run-npm-install.sh <archives-directory>
python3 -I -m unittest discover -s tests/ffi-python -p 'test_linux_helper.py'
```

The FFI discovery script expects the library in `target/release-ffi` and the
owner in `target/release`. It copies both into a fresh directory and empties
PATH during clipboard calls. A foreign GTK reader must receive the payload
after adapter destruction, then observe an empty selection after clear. The
npm check applies the same contract after the installed CLI process exits.
The release dry-run installs the GTK 3 typelib and `trash-cli`, runs the fake
contracts, and stages the discovery check from that job's extracted archive.

To invert discovery, stage only the library, point `AD_DYLIB_PATH` at it, set
`AD_EXPECT_LINUX_HELPER=1` and `AD_HELPER_EMPTY_PATH` to an empty directory, and
run `python3 tests/ffi-python/smoke.py` in a private session. It must fail with
the missing clipboard owner error. Borrowing another checkout's helper would
invalidate this check.

## Owner rule

The C lifetime leg no longer starts a snapshot thread, sleeps five
milliseconds, then destroys its handle. Scheduling does not establish which
operation acquired the handle first. It keeps twenty create/snapshot/destroy
cycles and checks that a destroyed handle returns `INVALID_ARGS`. The Rust
test `destroy_revokes_new_calls_without_invalidating_in_flight_owners` holds
an owner explicitly and tests in-flight retention without a scheduling race.

The foreign reader explicitly selects both GTK 3 and GDK 3. Leaving GDK
unversioned selected GDK 4 on a host with both toolkits and prevented any
clipboard observation. A fake GI test rejects that mismatch. Private bus
cleanup replaces a PID list and fixed delay with PID handles, generation and
exact runtime checks before signaling, followed by an observed exit wait.
Four fake process tests reject reuse and foreign runtime membership.

## Rebase conflicts

The harness rebase onto `64ccca5a` had no conflicts. The distribution rebase
onto `631871c6` retained thirteen packaging commits and dropped seven older
core, test or documentation duplicates. Production Rust and CLI code did not
change from that base.

- `6ace8cfe`: conflicts in `crates/core/src/{lib,operation_report,operation_target}.rs`.
  Its exact patch was already present. Kept the newer base definitions.
- `ef683584`: conflicts in `crates/core/src/commands/{drag,drag_tests,hover,mouse_click,mouse_move}.rs`,
  `crates/core/src/{lib,operation_report_build,operation_report_build_tests,operation_target}.rs`,
  `crates/linux/src/adapter.rs`, `crates/macos/src/system/{adapter,window_report}.rs`,
  `crates/windows/src/system/adapter.rs`, and `docs/{json-output,phases}.md`.
  Kept the base's complete operation reports, newer delivery and close failure
  evidence, window verification and Linux clipboard owner implementation.
- `f8adfcd4`: conflicts in the macOS and Windows `system/adapter.rs` files.
  Its exact patch was already present; kept the newer implementations.
- `7b8aeed4`: conflicts in `docs/{json-output,phases}.md`. Kept current Linux
  two-pixel verification, lifecycle, capture, clipboard and harness evidence.
  The incoming Windows wording correction and pointer drag example were
  already present, so this duplicate had no remaining change.

## Validation

Current results are recorded in `docs/phases.md` under the distribution
section and in findings `L22-16`, `L22-19` and `L22-20`. Native x64 validation
does not certify the Ubuntu 22.04 glibc floor or arm64 artifacts; those require
the pinned container release dry-run, which was not dispatched.

The C consumer passed all nine legs twice in separate private sessions,
including clipboard. The library-adjacent helper smoke and installed npm
package passed with the same checkout's helper. Dropping the helper failed
specifically for the missing owner. All thirteen new fake tests failed under
targeted inversions and passed after restoration. The Mac npm suite passed
63 tests with its Linux-only GNU gzip case skipped; Linux passed all 64.
Four-package all-target Clippy passed with warnings denied. The deterministic
Rust held-owner test passed, and Mac formatting and policy checks passed.
All 2,506 retained Linux library tests passed with ignored tests enabled and
one test thread across bounded private-session calls. No native test failed
or needed deletion on this branch. Calls that reached their execution budget
continued only unfinished test names in the next private session.
