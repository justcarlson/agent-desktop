---
title: Make permissioned fixture and real-app checks the adapter gate
date: 2026-07-11
category: best-practices
module: platform adapter verification
problem_type: best_practice
component: testing_framework
severity: high
applies_when:
  - "Changing a platform adapter's tree, resolution, window, input, or accessibility code"
  - "Changing release-binary integration behavior"
  - "Fixing a bug that a mock adapter could not expose"
tags: [e2e, fixture-app, accessibility, macos, windows, linux, regression, adapters]
---

# Make permissioned fixture and real-app checks the adapter gate

## Context

Core unit tests prove shared contracts, but they cannot prove that a native
accessibility API exposes the same tree, identity, or input behavior on a real
desktop. A mock cannot reproduce a bad AX-to-window bridge or disagreement
between independently implemented native readers.

## Guidance

The macOS adapter has two complementary native gates:

- `tests/e2e/run.sh` builds a temporary SwiftUI fixture, drives the release
  binary, and independently observes each harmless effect. It requires macOS,
  a release binary, and Accessibility permission; it exits with a clear
  prerequisite failure when one is unavailable.
- `src/tests/snapshot_test.rs` contains three `#[ignore]` Finder probes:
  snapshot window identity must agree with `list-windows`; a reported button
  name must be findable by that same name; and a fresh find ref must re-resolve
  through `get`. The registration test prevents these guards from silently
  disappearing.

Run normal tests first, then run the native gate on a permissioned machine for
any adapter change. Use the fixture for mutation coverage. Treat real user apps
as observation-only unless a narrowly scoped, reversible interaction is
explicitly authorized.

## Why This Matters

The fixture provides deterministic, safe interaction coverage. Finder probes
cover native seams that the fixture cannot emulate: AX window identity,
accessible-name derivation, and strict re-resolution against a real system
application. Together they catch both behavior regressions and accidental test
deletion without treating a mock as a platform oracle.

## Prevention

- For every native regression, add a deterministic unit test where possible
  and a fixture or safe real-app assertion at the affected seam.
- Verify effects independently of an `ok: true` response.
- Keep native tests opt-in and prerequisite-aware; never make them operate on
  user data or depend on an arbitrary foreground application.
- Record a skipped native gate as skipped, not green.

## Standing practice on Windows

Windows first gated on a scripted dogfood run against off-the-shelf software,
before it carried any fixture harness. It now also ships `tests/e2e-windows/`
alongside a WinForms fixture app at `tests/fixture-app-windows/`: a PowerShell
5.1 harness that stages the real, hashed `agent-desktop.exe`, drives it against
the fixture, and asserts every effect by independent re-observation — the same
discipline as macOS's `tests/e2e/run.sh`. `Run-E2E.ps1` is the one
process-terminating entry point; the hosted CI lane runs its seeded-failure
self-test (`Run-E2E.ps1 -SelfTestSeedFailure`) on every PR, proving a failing
leg reaches a non-zero exit code, while the live suite needs a real desktop and
the `DesktopLease` exclusivity guard, so it runs locally rather than on hosted
CI.

Dogfood against real off-the-shelf software remains a separate, additional gate
every adapter change still runs regardless of fixture coverage: the fixture buys
deterministic mutation coverage the way `tests/e2e/run.sh` does on macOS, while
dogfood proves the adapter against native seams — Explorer's DirectUI shell,
Chromium/Electron hosts, System Settings — that no fixture can emulate. Each
run's judged report is kept outside version control (see "Definition of Done:
Dogfood Is a Gate" in `CLAUDE.md`); the scripts that drive the runs are
`probes/windows/scratch/run-*-dogfood.ps1`, and what a run measured is recorded
as rows in `probes/windows/FINDINGS.md`. This is not a rule that recurred; it is
how each layer of the adapter enters the product.

The vocabulary run shows what the shape buys. `probes/windows/scratch/run-dogfood.ps1` drove the
`ControlType`→`Role`, action, and state vocabulary against four real UI stacks nobody in this
repository wrote — classic Notepad (Win32 `EDIT` proxy), Explorer (DirectUI shell), and
WinForms/WPF scratch fixtures. It found one real defect no unit test had: `invalid` was
emitted on every node of every target, because a UIA form-validity flag's `false` default was
read as a positive claim rather than the absence of one. It also recorded two targets —
Chromium/Electron and the modern Settings app — as **skipped with a reason** rather than
silently green, exactly this doc's "record a skipped native gate as skipped" rule. The
report and the raw per-node census JSON it was produced from are both kept outside
version control, because a census can carry a real application's on-screen text where a
report describing shapes and counts does not.

The Windows actionability-and-occlusion run sharpens the same discipline one level
up: a dogfood judgement can accept the exact defect it was built to catch when
two code paths share an error code. Its J4 judgement treated any
`PLATFORM_NOT_SUPPORTED` envelope as proof a below-fold Explorer scroll
worked — but that code is also exactly what the defect produces, because
`execute_action` was at that point legitimately unimplemented: a click
whose `scroll_into_view` override is missing and falls through to the trait
default answers with the identical code as a click that scrolled, passed the
gate, and reached dispatch. Deleting the product fix left the gate reporting
pass; three sibling judgements had the same hole. The fix is a positive
discriminator, not a broader error-code check — the judgements now require
the envelope's `message` to name `execute_action` by name, proving dispatch
was actually reached, per the header comment in
`probes/windows/scratch/run-actionability-dogfood.ps1` and its
`Test-DispatchReached` / `Test-UnsupportedSeamBeforeDispatch` predicates. It is
this doc's own "verify effects independently of `ok: true`" guidance one level
down: a structured error *code* is not enough discrimination either, when the
healthy path and the defect share one.

A run substitutes for the fixture app only while it keeps the fixture app's discipline — real
software, effects verified independently, skips recorded honestly, and raw captures kept out of
the repository while the judgement drawn from them is kept in it.

## Standing practice on Linux

The Linux adapter has no single fixture-app gate; it uses live tests with controlled GTK/Qt helper applications and dogfood against off-the-shelf software (GNOME apps, VS Code) on the remote box. Live tests run only inside `scripts/private-session.sh`, which gives them a private bus, Xvfb display and runtime directory: the launcher in `crates/linux/src/tree/live_launch.rs` asserts the `AGENT_DESKTOP_PRIVATE_SESSION` marker and refuses to start without it. Tests that depend on stacking or window activation pass `--wm` to get openbox; mutter coverage nests `scripts/private-mutter-shell.sh` inside the private session. Both managers are required evidence for lifecycle behavior: they disagree on activation, restoring minimized windows, placement and animation. A fresh private mutter also opens the Activities Overview, which refuses input until `Escape` closes it (L28-2, L28-17); the helper checks for an already-open Overview, and a test that maps the first window must check it again.

The lifecycle gate must cover the identities and final states that a successful native request alone cannot establish:

- **Launch and close share application identity.** A launcher can exit while its server owns the window: the desktop entry `Name=Terminal`, `Exec=gnome-terminal` runs as `gnome-terminal-server`. A name or alias counts only when executable evidence backs it: `crates/linux/src/system/app/app_alias.rs` maps an entry to a process only when the entry's resolved executable and full argv match it, `crates/linux/src/system/launch/launch_attach.rs` attaches only on a verified executable identity, and `app/app_name.rs::name_matches` compares the query with the listed name and those aliases, so a name alone never selects a process (L31-40). Exercise a running app with no window and a forwarded launch: reopening must actually start the launcher, and forwarding must wait for a window absent before the spawn rather than report the old window (L31-63, L31-66). Name resolution must also exclude processes of another session (L31-61).
- **A close request is not process exit.** Observe the expected process instance leaving, preserve a resident process with no windows, and report save dialogs without answering them. A dialog opened by this call carries `delivered_unverified`; a dialog already open blocks the repeated close before dispatch and carries `not_delivered` (L31-62, L31-65). Graceful close without a Wayland/Xwayland route returns `ACTION_NOT_SUPPORTED` with `details.cause = "wayland_session"`, including when Xwayland is absent (L31-66).
- **Focus is an observed postcondition.** On X11, lift a minimized window and judge the hidden flag, then require the display's active client and the toolkit's active frame to agree. An ignored stamped activation gets one stamp-free pager resend (source 2, timestamp 0), never a click; a manager that already names the window gets time for the toolkit's frame report. A modal frame that never reports active instead requires the active client plus X input focus inside it. Test Overview and modal refusals, and the Wayland `ACTION_NOT_SUPPORTED` / `wayland_session` refusal (L31-1, L31-5, L31-64; `system/focus/focus_window.rs`).
- **Window commands share one rectangle.** Move, resize, maximize and restore must speak the visible bounds reported by `list-windows`: X11 client geometry minus the GTK client-side shadow. Compare against independent X11 geometry with that conversion, keep resize position where the window fits a display, and judge placement only after a full confirmation pause. See [A matching window rectangle can be mid-animation](../runtime-errors/a-matching-window-rectangle-can-be-mid-animation.md) (L31-60, L31-67).
- **Stopped is not missing.** Preserve the listed window ID per client and process instance through `SIGSTOP`, and check every lifecycle/window target route for `APP_UNRESPONSIVE` / `not_delivered` rather than `WINDOW_NOT_FOUND`. Process-state tests separately distinguish exit, a stopped process, a blocked loop, one timeout and unavailable evidence (L31-17, L31-18, L31-59; `system/process/process_state_classify.rs`).

Box setup and the remote loop are in [remote-dev-boxes-over-tailscale.md](remote-dev-boxes-over-tailscale.md); the hazards of the shared session, including the nested mutter display's D-Bus activation environment, are in [a-shared-live-desktop-is-shared-state.md](a-shared-live-desktop-is-shared-state.md). Measurements are recorded as rows in `probes/linux/FINDINGS.md`; judged dogfood reports are kept outside version control.

## Related

- [Build desktop actions as an observe-resolve-preflight-dispatch contract](playwright-grade-desktop-reliability-2026-06-02.md)
- [Guard OS-reordered resources with an identity fingerprint](identity-fingerprint-against-os-reorder-2026-04-16.md)
- [Never ship platform code that CI cannot execute](never-ship-platform-code-that-ci-cannot-execute.md) — the complementary gate. This doc covers permissioned native verification of the macOS adapter against real applications; that one covers running platform-conditional core code on real Windows and Linux runners.
