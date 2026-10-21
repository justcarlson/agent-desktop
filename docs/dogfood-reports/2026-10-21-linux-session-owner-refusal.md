# Linux session-owner refusal

2026-10-21. Branch `feat/linux-3.15-fix-session`, baseline `f0366d90`.

## Finding and disposition

**P2, fixed here:** the supplied final Linux dogfood reported two private-session
attempts returning `resolve_owning_process_exited` while `process_state` was
`running`. Linux treated a missing accessibility-bus owner as proof of process
exit. Bus-name absence proves only that the current bus cannot reach that owner.

The shared resolver boundary now rechecks `/proc` on bus-owner absence. Only
`Lookup::Gone` produces `resolve_owning_process_exited`. Otherwise the settled
`STALE_REF` is `resolve_owner_unreachable`, with this message and suggestion:

> Stored ref's owning application is not reachable on this session's accessibility bus
>
> Return to the desktop session where this ref was captured, run 'snapshot', then retry with the new ref.

An unreadable process is never declared exited. The error remains not delivered,
safe for caller recovery, and nonretryable inside the resolver's polling loop.
Frame, menu, owner-query and live-element paths use the same classification.

## Deterministic proof

`tree::resolve::refusal_tests::an_owner_unreachable_on_this_bus_is_refused_without_claiming_it_exited`
uses a fake bus departure and the test process's live PID. It checks the public
code, kind, message, suggestion, delivery and retry semantics, and proves no
path, evidence or candidate-search reads follow the refusal.

`tree::resolve_gate::tests::a_missing_bus_owner_is_only_exited_when_its_process_is_gone`
checks both a live PID and the impossible Linux PID `u32::MAX`.

Inversion restored the old `AppNotFound => app_exited()` mapping: the regression
failed at its kind assertion, receiving `resolve_owning_process_exited` instead
of `resolve_owner_unreachable`. Restoring the fix passed the same test.

## Live proof

A real GTK3 button fixture ran on private Openbox/Xvfb desktop A. Two fresh
private B desktops used distinct runtimes and buses with the same CLI state root as A. A's AT-SPI bus unique
name was advanced before fixture registration, so the foreign name was absent
on each B bus rather than accidentally identifying a different local owner.

Independent Gio D-Bus queries identified A's bus name by its Unix process ID;
B0 and B1 both observed `NameHasOwner=false` for that name while `/proc` still
contained A's PID. GTK click callbacks wrote file counters and pipe events;
CLI success alone was not the delivery oracle. Readiness used `wait` and callback
pipe readiness, with no fixed sleep as proof.

| Observation | B0 | B1 |
|---|---|---|
| A PID alive; A bus name absent from B bus | confirmed | confirmed |
| Foreign A ref | `STALE_REF / resolve_owner_unreachable` | same |
| Error process state and delivery | `running / not_delivered` | same |
| A counter after foreign attempt | 1 | 1 |
| B counter after foreign attempt | 0 | 0 |
| B counter after its own valid ref | 1 | 1 |

A stayed at 1 across both foreign attempts. Returning to A, taking a fresh
snapshot and clicking changed its native counter to 2. Terminating and reaping
A then made the saved ref return `STALE_REF / resolve_owning_process_exited`
with `process_state: exited` and `not_delivered`.

Frozen debug CLI SHA-256:
`94d3ef2881e57139108bfd5380d170a92033c072d5a48257d4c58972df76a113`.
Evidence on the box: `~/ad-wt/session-owner-proof-f0366d90/`
(`proof.py`, `live.log`, `invert.log`, counter files, `binary.sha256`).
The scratch binary and shared snapshot state are removed after proof; no private
Cargo target was created, and the shared target is preserved.

## Gates and review

- Box clippy: core, Linux, binary and FFI; all targets; `-D warnings`: passed.
- Box `cargo test -p agent-desktop-linux --lib`: 1,794 passed, zero failed;
  197 marked live tests excluded by the default invocation; no `--ignored` run.
- Targeted regression: passed before and after inversion restoration.
- `cargo fmt --all -- --check`, Rust file-size, phase-reference,
  stale-ref-constructor and diff whitespace checks: passed.
- Read-only source and final-diff reviews: no remaining findings.
- Core is unchanged; its conditional cross-platform gate was not required.

Scope is this diagnostic refusal and its controls, using a GTK3 fixture. The
broader application matrix from the original dogfood was not rerun. The live
harness initially used the wrong snapshot selector flag; it was corrected to
`--window-id` before either claimed live pass. No flaky-test repair or deletion
was needed.
