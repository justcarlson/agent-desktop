---
title: A shared build dir serves another tree's binary - touch the sources before a remote test run
date: 2026-10-19
category: best-practices
module: scripts/remote-linux.sh
problem_type: best_practice
component: tooling
severity: medium
applies_when:
  - "Several worktrees build into one CARGO_TARGET_DIR on a remote box"
  - "A run reports Finished within a fraction of a second after a source edit"
symptoms:
  - "A test run on the box shows the behaviour from before a source edit, with no rebuild."
  - "A test that passes locally fails on the box, or the reverse, and the failing assertion does not match the code."
  - "cargo prints no Compiling line for a crate that changed."
  - "A test run lists test names that belong to another worktree."
tags: [linux, remote-development, cargo, rsync, mtime, shared-state]
---

# A shared build dir serves another tree's binary - touch the sources before a remote test run

## Context

`scripts/remote-linux.sh` syncs each worktree to its own folder on the box, and every worktree builds into one `~/ad-target` so the disk holds one set of dependencies. Cargo names a workspace member's artifacts and fingerprints the same way in every copy of the tree, so all worktrees share one set of outputs. A fingerprint is judged against source modification times, and `rsync -a` keeps the workstation's times. A build that another worktree made later therefore looks fresh to this one, even though it came from other sources. Runs in two lanes saw another worktree's test names in their own output, and `Finished` came back in 0.05 s after an edit. The script now touches the sources before a cargo command (Guidance 2); nothing in it says what that guard prevents.

## Guidance

1. **Treat a missing recompile as a stale binary, not as a passing build.** The run reports the behaviour of the other worktree's sources and prints no error.
2. **`scripts/remote-linux.sh` touches the synced sources before a cargo command.** When the command text contains `cargo` or `linux-live-gate`, the script runs `find crates src -name '*.rs' -exec touch {} +` inside the box slot, so cargo rebuilds this worktree's own sources (`scripts/test-remote-linux.sh` pins this). A command that reaches cargo without naming it skips the touch, so run that `find` yourself first.
3. **Read the Compiling lines.** A run that changed a crate and shows no `Compiling` line for it did not test the change.
4. **Copy a binary you will drive live to a private path right after you build it.** `~/ad-target/release/agent-desktop` is overwritten by any lane's build, and a live run that used it between two builds got another lane's binary, which answered `wait --menu` with the stub `TIMEOUT` (L33-45).
5. **A per-worktree `CARGO_TARGET_DIR` removes the cross-worktree reuse** when the disk allows it.

## Why This Matters

A stale binary makes a fix look broken or a defect look fixed, and the cause sits in the build dir rather than the product. A finding recorded from such a run is wrong in both directions.

## When to Apply

- Before a remote test run that follows a source edit, with a shared target dir.
- When a result on the box contradicts the code you just read.

Related: [develop platform adapters against remote boxes over Tailscale](remote-dev-boxes-over-tailscale.md), [a shared live desktop is shared state](a-shared-live-desktop-is-shared-state.md).
