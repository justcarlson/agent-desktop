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
  - "The remote box clock runs ahead of the machine that edits the sources"
symptoms:
  - "A test run on the box shows the behaviour from before a source edit, with no rebuild."
  - "A test that passes locally fails on the box, or the reverse, and the failing assertion does not match the code."
  - "cargo prints no Compiling line for a crate that changed."
tags: [linux, remote-development, cargo, rsync, mtime, shared-state]
---

# A shared build dir serves another tree's binary - touch the sources before a remote test run

## Context

`scripts/remote-linux.sh` syncs each worktree to its own folder on the box, and every worktree builds into one `~/ad-target` so the disk holds one set of dependencies. Cargo decides whether a crate is fresh by comparing source modification times with its build outputs. `rsync -a` keeps the workstation's modification times, and the box clock runs ahead of the workstation (L27-28). Nothing in the final code shows this: the script only names the shared directory.

## Guidance

1. **Treat a missing recompile as a stale binary, not as a passing build.** An edit made on the workstation carries a modification time that is older than an output another worktree built on the box a moment before. Cargo then reuses that output, and the test binary comes from other sources. The run reports the old behaviour and no error.
2. **Touch the sources on the box before the run.** `scripts/remote-linux.sh -c 'find crates src -name "*.rs" -exec touch {} +; cargo test -p agent-desktop-linux'` forces a rebuild of what the sync changed. Do this whenever the target dir is shared and the run follows an edit.
3. **Read the Compiling lines.** A run that changed a crate and shows no `Compiling` line for it did not test the change.
4. **Prefer a per-worktree `CARGO_TARGET_DIR` when the disk allows it.** A separate directory removes the cross-tree reuse; it does not remove the clock skew against that tree's own earlier builds, so the touch stays.

## Why This Matters

A stale binary makes a fix look broken or a defect look fixed, and the cause sits in the build dir rather than the product. A finding recorded from such a run is wrong in both directions.

## When to Apply

- Before a remote test run that follows a source edit, with a shared target dir.
- When a result on the box contradicts the code you just read.

Related: [develop platform adapters against remote boxes over Tailscale](remote-dev-boxes-over-tailscale.md), [a shared live desktop is shared state](a-shared-live-desktop-is-shared-state.md).
