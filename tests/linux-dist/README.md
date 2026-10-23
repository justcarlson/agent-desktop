# Linux distribution checks

The supported live-desktop floor is Ubuntu 24.04. CLI and FFI artifacts still
build in pinned Ubuntu 22.04 containers and must pass the glibc 2.35 floor
check. The `linux-consumer` jobs run those extracted artifacts directly on
the hosted Ubuntu 24.04 x64 and ARM64 runners, in private Openbox sessions.
The binary loader floor does not certify an older desktop stack.

Ubuntu 22.04's ATK bridge 2.38.0 aborts inside the application's
`Action.GetActions`: it declares `(sss)` entries but appends four strings.
The library's tree traversal calls that method; the app abort leaves it with
`TIMEOUT` and zero nodes. This is the app's in-process bridge, rather than
`at-spi2-registryd` or the Rust `zbus` library. Ubuntu 24.04's 2.52.0 bridge
contains the [upstream fix](https://github.com/GNOME/at-spi2-core/commit/dc0dc33).

PR #296 consumer job `119064268637` emits `dbus[5558]` with signature
`a(sss)` at byte 5, then `windows-exact: FAIL code=TIMEOUT rc=-9 nodes=0`.
An isolated local probe compiled the unmodified `impl_GetActions` handlers
from the official Ubuntu [2.38.0](https://archive.ubuntu.com/ubuntu/pool/main/a/at-spi2-atk/at-spi2-atk_2.38.0.orig.tar.xz)
and [2.52.0](https://archive.ubuntu.com/ubuntu/pool/main/a/at-spi2-core/at-spi2-core_2.52.0.orig.tar.xz) source tarballs against libdbus
1.12.20, supplying one fake ATK action. The Jammy handler exited by SIGABRT
with that exact signature error; the Noble handler exited 0 with reply
signature `a(sss)`. This measures the faulty serializer, not a full 22.04
desktop reproduction or a PID-to-command trace of the CI process. Local
Docker's daemon was unavailable; the shared box's Docker state was untouched.

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
The release dry-run installs the GTK 3 typelib, runs the fake
contracts, and stages the discovery check from that job's extracted archive.

The consumer also runs the real ctypes smoke against the extracted library
and header while Calculator is registered, requiring complete main-thread
and worker-thread snapshots with refs. A smoke failure fails the consumer.
Both C and npm click checks require a readable baseline different from `7`
before dispatch, a unique button ref, and an independent readback of `7`.
Clipboard clear accepts only the reader's no-text status; observer errors
fail the check. Ten deterministic regression cases cover these contracts,
and each was individually inverted and restored.

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

Current results are recorded in `probes/linux/FINDINGS.md` findings
`L22-16`, `L22-19` and `L22-20`. PR #296 release dry-run
`39551893675` passed the pinned-container CLI and FFI builds, glibc gates and
archive checks on both architectures. Its 22.04 consumer jobs failed on the
bridge defect above. The hosted 24.04 consumer workflow change still requires
a dry-run after publication; native x64 validation alone does not certify
that workflow or ARM64 desktop behavior.

On 2026-10-20, the actual x64 archives from that dry-run were downloaded,
their SHA-256 checks verified, and the extracted FFI archive exercised on
Ubuntu 24.04 in private Openbox sessions. Every C consumer leg passed,
including exact-window restart refusal, ctypes main/worker snapshots, click
readback, lifetime, serialization, bus recovery and clipboard. Library-adjacent
helper discovery also passed with an empty PATH. The npm leg stopped at its
Node prerequisite before running any checks because Node was absent from the
box's PATH; the hosted consumer job provisions Node 24 with `setup-node`.

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
