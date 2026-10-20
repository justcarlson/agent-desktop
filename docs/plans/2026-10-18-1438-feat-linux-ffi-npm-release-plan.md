---
title: Linux FFI, npm and Release (Sub-phase 3.13) - Plan
type: feat
date: 2026-10-18
origin: docs/phases.md
artifact_contract: ce-unified-plan/v1
product_contract_source: docs/phases.md §Phase 3 sub-phase 3.13
execution: code
---

# Linux FFI, npm and Release (Sub-phase 3.13) - Plan

## Goal Capsule

- **Objective:** an agent on Ubuntu 22.04 or newer, on x86_64 or aarch64, can install agent-desktop through every channel the other platforms already have (`npm install -g`, the CLI archive, the FFI archive), and each channel is proven by running its actual packaged artifact against a real app: observe, act once, read the effect back independently.
- **Means:** distribution wiring and verification only. No new command, `ad_*` entry point, adapter method or core line. The Linux CLI joins the release and dry-run matrices on the floor host, a glibc gate reads the finished ELF files, a C consumer and an npm packed-install run the extracted dry-run archives on both architectures in a private session, `npm/lib/platform.js` flips, and the Linux platform skill is delivered through a seam the owner provides (KTD1-KTD14).
- **Authority:** `docs/phases.md` §3.13 and its "Release, Skill & Docs" checklist, as corrected here where the code or the ledger disproves them (R12). The pre-code probe rows `L22-1` to `L22-15` are the measurements this plan stands on.
- **Stop conditions:** stop if a requirement needs a line under `crates/core` (the platform skill's selection is the only known case and it is the owner's, KTD1); if 3.9-3.12 land without the pieces named in Assumptions (resolution and `click` for the action leg, the Calculator-class launch recipe, the clipboard owner helper of 3.10, the private-session harness of 3.12); or if neither `ubuntu-22.04` nor `ubuntu-22.04-arm` can run the private-session consumer (U1 decides this before any workflow edit).
- **Execution profile:** U1 first (it measures the two things the box could not and settles the readback observer); U2 (docs and ledger corrections) may run in parallel with U1 because it needs no measurement the corrections lack; U3 after U1; U4 after U3; U5 after U3; U6 after U3, U5 and 3.10; U7 after U4, U5, U6; U8 (documentation and release notes) after U7; U9 (the skill) after the owner seam exists (O1) and U8; U10 (dogfood) after U7 and U8, its skill leg after U9; U11 last, because it records every disposition.
- **Finishes and ships:** the implementer runs the box measurements, dispatches the dry-run workflow on the branch, commits, and opens one PR stacked on the 3.12 branch with `gh stack`, from `feat/linux-3.13-ffi-npm-release`, into `feat/linux-adapter`; nothing merges to `main` and nothing is published. The stack order is decided: 3.0 to 3.11, then 3.14, then 3.12, then 3.13, then 3.15, so this branch is cut from the 3.12 head, which sits on the 3.14 head. No 3.12 branch exists yet and its name is inferred from the sub-phase title; confirm it at the cut. Everything 3.14 lands (menu, tray, shell and notification surfaces, the sandbox note) is already in the base, so this sub-phase states it and does not defer it.

---

## Product Contract

### Summary

Make the Linux adapter reachable. After this sub-phase `npm install -g agent-desktop` on Ubuntu 22.04+ downloads, verifies and runs a Linux binary; the release workflow builds, gates, archives and attests Linux CLI archives next to the existing Linux FFI archives; the FFI archive's `libagent_desktop_ffi.so` is exercised by a real C consumer on both architectures; and a Linux platform skill exists, with the README and the FFI docs stating what Linux does and does not do at this point in the roadmap. Nothing is published from the branch: the acceptance vehicle is the dry-run workflow, which produces the same archives from the branch commit and has no publish job.

### Problem Frame

Distribution is already mostly wired. The CLI and FFI crates select `LinuxAdapter` by target (`src/Cargo.toml`, `src/main.rs`, `crates/ffi/src/adapter.rs`), the release workflow already builds Linux FFI archives on `ubuntu-22.04` and `ubuntu-22.04-arm`, and the npm table already holds the two Linux triples with `released: false`. What is missing is everything that proves a Linux user gets a working thing. The probes found the traps before any workflow was edited:

- **The glibc floor belongs to the build host, not the code.** A CLI and a cdylib built on the 2.39 box need `GLIBC_2.39` (`pidfd_getpid`, `pidfd_spawnp`, bound by `std`) and fail to load on a real jammy 2.35 userland with a hard, non-weak version error (`L22-2`). Both depend only on `libgcc_s`, `libm` and `libc`, so there is no libdbus, X11 or xcb to install and the roadmap's `libdbus-1-dev` has no basis (`L22-1`). The roadmap already names the FFI runners (`ubuntu-22.04`, `ubuntu-22.04-arm`) for the CLI matrix, so the floor holds only if the plan keeps those labels and adds the gate that reads the finished file; nothing in the workflows reads a glibc version today.
- **The packaged payload loads, observes and refuses honestly today; it cannot act yet.** The C consumer matched the header's ABI major and four size getters, took the handshake, snapshotted Calculator (34 refs) and then got `PLATFORM_NOT_SUPPORTED` on `click` because strict resolution and actions are not in the measured crate (`L22-4`). The installed CLI says the same. A test that accepts that result is not a Linux acceptance test, and the existing broad FFI tests do accept it (`c_abi_snapshot.rs`, the ignored roundtrip, and `smoke.py` without `AD_EXPECT_STUB`).
- **The display text is invisible to the product's own snapshot.** The Calculator display carries no name and no value (`L22-5`), so "read the effect back" needs an observer that does not go through the code under test.
- **The hosted-library facts are good.** `ad_init` is a version handshake only; no bootstrap and no thread affinity (`L22-6`); calls on one adapter serialise, so eight threads see a median of about 6x the single-call latency (`L22-7`); exact-window identity refuses a restarted target (`L22-10`); a lost accessibility bus surfaces as `INTERNAL` and the same adapter recovers without being recreated (`L22-8`).
- **The clipboard has no owner in a foreign host today.** The C clipboard calls return `PLATFORM_NOT_SUPPORTED` in the measured crate. 3.10 lands a dedicated helper executable, `agent-desktop-linux-clipboard-owner` (a second `[[bin]]` of `crates/linux`), located beside the image that holds the adapter through `/proc/self/maps`, never through `PATH`, with ownership, permission and build-identity checks; a detached helper keeps serving after the launching host exits (`L22-9`), so FFI-host clipboard-set is supported, at parity with macOS and Windows. This sub-phase packages that helper in all three channels together.
- **npm works, with two Linux-only wrinkles.** The packed install passes through the real checksum and archive path, but with `trash` absent (a stock Linux box) each install leaves a duplicate 4 MB `.extract-*` directory and a stderr warning (`L22-12`), and `scripts/ci-npm-wrapper-smoke.sh` has no Linux arm (`L22-11`). Every incomplete-install path already fails explicitly and cleans up (`L22-13`).
- **The skill cannot be committed on its own.** `skills get platform` on Linux fails with `INVALID_ARGS` (`L22-14`); the selection table and the embedded catalogue live in `crates/core/src/commands/skills.rs`, and a core test rejects any `skills/` Markdown the table does not serve. `docs/phases.md` lists that table as an owner decision for a cross-platform core change in the Phase 3 introduction, states the dependency in §3.13, and then, in the Release checklist, tells this sub-phase to register Linux in core directly. The checklist line contradicts the introduction and is corrected here (KTD1, U2).
- **Two things the box could not measure.** Native aarch64 (`L22-3`) and Ubuntu 22.04 with GNOME 42 (`L22-15`). The plan measures what a hosted runner can show (KTD4) and writes down what it cannot.

### Requirements

**Release and archives**

- R1. Every shipped Linux ELF file (CLI and cdylib, both architectures) is built on the floor host and carries no symbol above `GLIBC_2.35`, enforced by a gate that reads the finished file in `release.yml` and in `release-dry-run.yml`, and the gate itself is proven to fail on a file that breaks the floor (`L22-1`, `L22-2`).
- R2. The release workflow builds, size-checks, smoke-runs, archives, lists and checksums a Linux CLI archive for `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`; the archive lists exactly what the npm table expects (`agent-desktop` and `agent-desktop-linux-clipboard-owner`); `EXPECTED_ASSETS` becomes 13 with its explanation, and `scripts/check-npm-package.js` inspects the Linux construction so a Linux matrix row without a Linux package step cannot pass.
- R3. `release-dry-run.yml` produces the same Linux CLI and FFI archives from the branch commit, with bash staging that mirrors the release layout, keeps `contents: read`, and contains no job that can publish.

**FFI**

- R4. A real C consumer, built against the packaged header and linked or loaded from the extracted FFI archive, proves on x86_64 and aarch64 (on every runner U1 shows can host the private session; a runner that cannot is a written deferral to 3.15 with the evidence, never a pass): the ABI major and size getters equal the header; `ad_init` before and after adapter creation; a snapshot that is `ok:true` with qualified refs; exactly one action with an independent readback of its effect; exact-window identity refusing after the target restarts; destroy semantics; overlapping calls on one adapter completing; and a lost-and-restored accessibility bus recovered by the same adapter. The action leg fails on `PLATFORM_NOT_SUPPORTED`, and a version call or an `ok:false` envelope never counts as a pass (`L22-4`, `L22-6` to `L22-8`, `L22-10`).
- R5. Anything the C API cannot do on Linux is stated, not hidden: text-only clipboard, the restricted `ad_snapshot` arguments, serialised calls per adapter, no SONAME.

**npm**

- R6. `linux-x64` and `linux-arm64` are `released: true`; the postinstall advertises the Linux skill only once the skill is served (R9); on Linux, staging cleanup removes the directory itself when `trash` is absent; the tests that pinned Linux as unreleased pin the released contract; the Linux wrapper smoke runs; and a packed install from the actual dry-run archives, through the installer's real checksum and extraction path, runs the installed command through both the JS wrapper and the global symlink and performs a live snapshot and one independently verified action, on both architectures (`L22-11` to `L22-13`).
- R7. Install failures stay explicit: the missing `curl`, `tar`, checksum entry, corrupt archive, unexpected member and scripts-disabled cases keep their messages, and the README states the `curl`, `tar` and script-permitting npm requirements.

**Clipboard channel parity**

- R8. The clipboard owner helper `agent-desktop-linux-clipboard-owner` ships beside `agent-desktop` in the CLI archive, beside `libagent_desktop_ffi.so` in the FFI archive, and in the npm package, discovered beside the image and not through `PATH`, and one archive-contents check over all three fails when any channel omits it (KTD7).

**Skill, docs, honesty**

- R9. `skills/agent-desktop-linux/SKILL.md` and its references exist, are served by `skills get platform` and listed `current` on Linux through an owner-provided seam, carry a version marker release-please bumps, and state only facts the landed code and the ledger support. Core Rust is untouched by this sub-phase.
- R10. The README platform table, install instructions, permissions section, FFI docs and release notes template state Linux support with capability-specific claims and footnotes, the measured glibc floor, and no tool, build dependency or check the product does not use or the ledger disproves.
- R11. The Linux perf method is applied and reported: min-of-seven with the warm-up discarded (min, median and max), release size on both architectures against 15 MiB, FFI call serialisation, wrapper-versus-symlink start-up.
- R12. `docs/phases.md` states what was built and no longer states what the evidence disproved; the `L22` rows enter `probes/linux/FINDINGS.md` and every row naming 3.13 is disposed of; every deferral is written into the receiving sub-phase's scope in the same PR.
- R13. Isolation: `git diff -- crates/core` is empty, no crate or npm dependency is added, no shipped source names a plan, a phase or a decision id.
- R14. A stranger dogfood is run from three perspectives and its judged report committed with every finding disposed of.

### Key Decisions

- KD1. **Core stays platform-neutral.** (session-settled: user-directed — chosen over adding the Linux arm to `skills.rs` here: core is the shared surface.) Governs R9, R13. `docs/phases.md` does **not** authorise the arm: the Phase 3 introduction (F7, J6) lists it as an owner decision, §3.13 says it "needs the cross-platform core change", and only the checklist line "Register the Linux skill in `crates/core/src/commands/skills.rs`" says to build it, which contradicts the first two and is corrected in U2.
- KD2. **Tests fail for a real reason, and real runs outrank suites.** (session-settled: user-directed — chosen over broad mocked suites and over accepting `ok:false` or a version call as proof: probes and captures are never read by tests and never committed.) Governs R1, R4, R6. Every test added here is invert-verified.
- KD3. **All box access goes through `scripts/remote-linux.sh`, never `sudo`, and live runs happen in `scripts/private-session.sh`.** (session-settled: user-directed — chosen over the live bus and over a hand-rolled session.) Governs R4, R6, R11.
- KD4. **Nothing is published.** (session-settled: user-directed — chosen over exercising `release.yml` end to end: its publish jobs fire only from `main`.) Governs R2, R3. No verification step in this plan runs `npm publish`, `gh release create` or `gh release upload`; the dry-run workflow, the archives it uploads as artifacts and a local `npm pack` are the vehicle.
- KD5. **The PR stacks on the 3.12 branch into `feat/linux-adapter`; nothing merges to `main`.** (session-settled: user-directed — chosen over a PR against `main`.)
- KD6. **Exact dependency pins; no dependency is added.** (session-settled: user-directed — chosen over a convenience crate or npm package for any of the new checks.) Governs R13. The glibc gate is `readelf` and shell; the readback observer is the system `Atspi` typelib, installed by the runner's package manager, never vendored.
- KD7. **Every `docs/phases.md` statement the evidence disproves is corrected in place, as its own unit, citing rows.** (session-settled: user-directed — chosen over leaving it for the next planner.) Governs R12.
- KD8. **The stranger dogfood is a gate.** (session-settled: user-directed — chosen over an author-run check.) Governs R14.

### Scope Boundaries

- No new command, no new `ad_*` entry point, no new adapter method, no `Action` variant, no change to the C ABI or to `agent_desktop.h` (the header-drift job stays untouched). No `watch` (`P3-O9`), no new FFI wrappers for text commands (`P3-O10`) or portal capture (`P3-O11`): they did not ship in core, so Phase 3 has nothing to wrap.
- No Linux code, name, cfg or dependency in `crates/core`, including the skill selection (KTD1).
- No new notification, tray, shell, Flatpak or Snap behaviour: 3.14 lands first and owns it. The skill and README state what 3.14 landed (menu surfaces, the GNOME shell surfaces, the tray shape, notification commands and the sandbox note, `L12-8`/`L12-9` as 3.14 disposed of them) at the landed head, and nothing is deferred to 3.14.
- Considered and not built:
  - **A base-URL override in the npm installer** so tests can point it at local archives. It would let an environment variable redirect the download source of a checksum-verified binary. The tests use a PATH-first `curl` double instead (`L22-11` did), which exercises the real archive and checksum code and ships nothing. Evidence that would change the call: the owner asking for a mirror feature.
  - **A SONAME on the cdylib** (`L22-1`: none). A linking consumer records the file name, which works for `-L`/`-rpath` use; macOS needed `install_name` because Swift/SPM resolves by it, which has no Linux analogue in the supported consumers. Documented in the FFI archive README; evidence that would change the call: a distro-packaging request.
  - **Typed C clipboard and richer `ad_snapshot` arguments.** Existing ABI limits (`ad_get_clipboard`/`ad_set_clipboard` are text; `ad_snapshot` takes no window id or root) are owner scope, not Linux work.
  - **A weakened skills-catalogue test** to land the Linux Markdown before the seam exists.
- Deferred to follow-up work, written into `docs/phases.md` in U2/U11 with the evidence: GNOME 42 Mutter/shell behaviour on real 22.04 hardware (`L22-15`; receiver 3.15's full-branch live e2e unless U1 shows a runner can host it); native-arm64 desktop behaviour beyond what the runner shows (`L22-3`; receiver 3.15); the skill, if O1 has not landed (receiver: the owner's seam, then this sub-phase's U8 re-opens).

---

## Planning Contract

### Assumptions

- **3.9 (lifecycle)** lands `launch` by name, id and path, `close`, `focus-window` and `process_state`, and fixes the app identifiers `list-apps` returns; the consumer and the dogfood start and stop their own targets through them or through the 3.12 harness helpers. The skill's launch, close and window claims come from 3.9's landed behaviour and its per-window-manager un-minimize scope.
- **3.10 (capture and clipboard)** lands the screenshot routes and the clipboard owner as a dedicated helper binary, `agent-desktop-linux-clipboard-owner` (KTD10, R10/R10a, U10, U16, U17 there), supported for FFI hosts as well as the CLI. 3.10 needs nothing from `crates/core`. Its handoff to this sub-phase is the packaging of that helper in the CLI archive, the FFI archive and the npm package, with an archive-contents check over all three.
- **3.11 (signals and wait)** lands `capture_signal_baseline` and `wait_for_menu`; the consumer and npm legs use core's `wait` for appearance and disappearance of the target instead of sleeps, and the skill repeats 3.11's named limits (no `ACTIVE` on Wayland, the `--menu` before `--menu-closed` caveat, the window-owning app population rule).
- **3.12 (live e2e harness)** lands the private-session launch recipe for real GNOME apps, the target catalogue (Calculator's addressable controls, restoration, readback) and the `linux-e2e` job; this sub-phase reuses its launch helpers and does not build a second harness. If 3.12's Calculator recipe differs from `L22`'s (`7` button, display), 3.12's wins.
- **3.14 (shell and notifications)** lands before 3.12 and 3.13: `menu` in `supported_surfaces`, the shell surfaces, the tray shape, notification commands, the sandbox note. Its own plan lists the 3.13 skill seam as a dependency of its skill leg, which is circular with this stack order; the seam (O1) must therefore exist below 3.14's skill leg, or 3.14's skill leg is listed open. 3.9 (lifecycle) is not yet cut as a branch and has written no §3.13 text in `docs/phases.md`; its skill facts are the dispositions of its dogfood (collected in `plans/3.13-split.md`).
- **3.5/3.7** land strict resolution and semantic actions; the action leg of R4 and R6 cannot pass without them (`L22-4`). They are reached through 3.12's dependency chain.
- The socket-probing display-server detector and the session-bus facility of earlier drafts exist when the consumer runs under Xvfb and a private bus.
- `ubuntu-22.04` and `ubuntu-22.04-arm` hosted runners offer `apt` for `at-spi2-core`, `dbus-x11`, `xvfb`, `openbox`, `gnome-calculator`, `python3-gi`, `gir1.2-atspi-2.0` and `gcc`; the release workflow already uses both labels for the FFI legs. This is checked in U1, not trusted.
- Ledger area `L22` was assigned at the pre-code probe. Sibling plans take "the next free area" at their cuts: if `L22` is taken when this branch is cut, renumber the probe rows into the next free area and rewrite every `L22-n` citation in shipped docs and this plan's Definition of Done together.

### Key Technical Decisions

- KTD1. **The Linux platform skill is delivered only through an owner-provided seam.** (Evidence: `skills.rs` holds the `CURRENT_PLATFORM` branches, the include list and the `SKILLS` table; `skills_tests.rs` rejects unserved Markdown under `skills/`; `L22-14`.) The code offers no neutral seam: `skills::list/get` read a `const SKILLS` table with `include_str!` bodies and a `cfg!(target_os)` `CURRENT_PLATFORM`, so a binary-owned or injected catalogue would change core's API and is larger than the edit it avoids. The real choice is the minimal approved edit (O1): in `skills.rs` the Linux `include_str!` constants and `SKILL_LINUX_REFS`, a `linux` branch of `CURRENT_PLATFORM`, one `SKILLS` entry with `platform: Some("linux")`; in `skills_tests.rs` the Linux branch of `the_platform_alias_serves_this_build_s_platform_skill` (it asserts the "no platform skill" error on Linux today) and a Linux entry in the list test. `include_str!` needs the files to exist and `every_skill_markdown_on_disk_is_served_from_the_skills_table` rejects them when unregistered, so the Markdown and the arm are one commit and never two. Either is delivered outside this PR's diff, as a commit below it in the stack or a separate owner PR, so `git diff <base> -- crates/core` stays empty here; an edit that lands inside this diff is an explicit owner override of KD1 that the owner states and that waives only the skill-related part of the Isolation check. U9 does not start until the seam exists. Governs R9, R13. No half-state is committed: `skills/agent-desktop-linux/` is added in the commit that is served.
- KTD2. **Build host equals floor host, and the finished file is read.** (Evidence: `L22-2`.) Linux CLI and FFI legs run on `ubuntu-22.04` and `ubuntu-22.04-arm`. `scripts/check-elf-glibc-floor.sh <file> <max>` reads `readelf --version-info` (and `--dyn-syms`), extracts every `GLIBC_x.y`, compares with `sort -V` and exits non-zero naming the symbol and file when any exceeds the maximum; it also fails when the file has no version info (a static or non-ELF file would otherwise pass vacuously). A self-test builds two tiny C objects on the Linux lane, one calling `pidfd_getpid` (needs 2.39 on the 24.04 CI lane) and one calling `puts`, and requires the gate to fail the first and pass the second; it skips with a loud message, not a pass, when the host libc cannot produce the failing object. Governs R1.
- KTD3. **Acceptance is the extracted dry-run archive, not a rebuilt tree.** (Evidence: `L22-11`: `AGENT_DESKTOP_BINARY_PATH` alone makes zero download calls.) A job downloads the dry-run artifacts for its own architecture, extracts them into clean directories, and runs the consumer and the packed npm install against those files. Governs R3, R4, R6.
- KTD4. **ARM64 and GNOME 42 are measured on the hosted runners; what they cannot show is stated.** (Evidence: `L22-3`, `L22-15`; the box is x86_64 24.04.) The same job runs on `ubuntu-22.04` and `ubuntu-22.04-arm` inside `scripts/private-session.sh` with apt-installed GNOME apps. That measures the aarch64 ELF, the loader closure, the FFI and npm paths and the toolkit-era behaviour of jammy's GTK and libadwaita; it does not measure gnome-shell/Mutter, which U1 tries (jammy `gnome-shell --x11` rootless under Xvfb, as `L16-6` did on 24.04) and records either way. Whatever U1 cannot show on a runner is written into `docs/phases.md` as an owner infrastructure item with its receiver, never as a pass.
- KTD5. **The readback observer does not go through the code under test.** (Evidence: `L22-5`: the display has no name or value in the product's snapshot.) Default: a short Python script using the system `Atspi` typelib reads the display's `Text` interface directly. U1 confirms it runs on both runners and on the box; if 3.7 exposes the display value through the product and that read is itself verified against the typelib read once, the product read may replace it. Governs R4, R6.
- KTD6. **Exactly one consumer, two entry points.** The consumer is a small C program with sub-commands (`abi`, `snapshot <app>`, `click <ref>`, `windows-exact <app>`, `serialise`, `lifetime`), driven by `tests/linux-dist/run-consumer.sh`; `tests/ffi-python/smoke.py` gains an `AD_EXPECT_REAL=1` mode that requires `ok:true` (the hosted-ctypes case, `L22-6`). The existing stub passthrough, header-drift, layout and panic tests keep their contracts. Governs R4.
- KTD7. **The clipboard owner helper ships in all three channels at once.** (Evidence: `L22-9`; dossier §2.4; 3.10 KTD10.) The helper `agent-desktop-linux-clipboard-owner` joins the CLI archive listing beside `agent-desktop`, the FFI archive's `lib/` beside the library (the adapter locates it through `/proc/self/maps`, never `PATH`, with ownership, permission and build-identity checks), `entries` in `npm/lib/platform.js` (the macOS helper constant generalises to a per-platform list), `postinstall.js` and wrapper helper handling, the dry-run and release listing checks, and `check-npm-package.js`. One archive-contents check covers all three channels and fails when any omits the helper. The helper is a second `[[bin]]` of `crates/linux`, and cargo builds another package's bin only with `-p agent-desktop-linux --bins`, so the CLI and FFI build steps add `cargo build --locked --release -p agent-desktop-linux --bins --target ...` (or fold `--bins` into a package-scoped build) and the build-identity check requires the helper and the packaged adapter image to come from the same build. FFI-host clipboard-set is supported, not refused. Governs R8.
- KTD8. **No new product surface in the installer.** `cleanupStaging` already removes staging with `rmSync` on Windows; Linux does the same when `trash` is absent (`L22-12`), keeping `trash` when present. The Linux prompt mapping in `promptSkillInstall` is added only with the skill. Governs R6.
- KTD9. **The flip never precedes the matrix.** `scripts/check-npm-package.js` fails when a released npm target has no CLI matrix row and derives the asset count from both matrices; a matrix row ahead of the flag passes, so U3 adds the matrix and U5 flips. Its tarball-construction check only knows macOS and Windows. It gains the Linux step, with a fixture proving a Linux matrix row with no Linux package step is rejected. Governs R2.
- KTD10. **A lost bus is `INTERNAL` and is documented, not remapped.** (Evidence: `L22-8`: a connection lost after being established returns `INTERNAL`, a cold-start failure returns the denied/unavailable mapping.) The two are different faults, and the same adapter recovers when the bus returns; the test asserts the recovery, the skill states both codes. Remapping is a bus-mapping decision in the Linux crate the owner of that mapping may overturn (O4); it is not core and not needed to ship. Governs R4, R9.
- KTD11. **Document the serialisation, do not widen it.** (Evidence: `L22-7`: 24 overlapping snapshots, median ~6x one call.) Calls on one adapter serialise by design (the interaction lease and bus holder); a foreign host should not expect parallel speed-up. The consumer pins "completes, all `ok:true`", not a latency. Governs R5.
- KTD12. **Claims are capability-specific.** The README table row for Linux is "Yes" per capability with footnotes where 3.14 landed a limit (notifications, tray, shell, Flatpak/Snap: stated at the landed head), where Wayland differs (`L13-5`, `L13-14`), and where a toolkit limits (`L3-30`). The Release checklist's `busctl --user list | grep a11y` check is replaced by the measured cold-activation route (`L1-5`), `libdbus-1-dev` is removed (`L22-1`), and mandatory `xdotool`/`ydotool`/`xclip` are replaced by the landed native routes of 3.8 and 3.10. Governs R10, R12.
- KTD13. **Host-scoped commands everywhere.** The workspace names three platform crates, so every documented and workflow `cargo` command uses the host's package set (`CLAUDE.md`). Governs R10.
- KTD14. **Stranger dogfood has three operators and one honest blocker.** npm installation, foreign FFI embedding, and skill-only operation. The skill-only leg cannot run until O1; the plan states that and does not fake it with the author's knowledge (`L22-14`). Governs R14.

### High-Level Technical Design

*This illustrates the intended approach and is directional guidance for review, not implementation specification.*

```
release-dry-run.yml (contents: read, no publish job)
  cli  [x86_64 on ubuntu-22.04, aarch64 on ubuntu-22.04-arm, + existing macOS/Windows]
     build --release -p agent-desktop -> check-elf-glibc-floor <=2.35 -> size -> version smoke
     -> tar (exact listing) -> sha256 -> upload dry-run-binary-<target>
  ffi  [same two Linux runners + existing Windows rows; the dry-run ffi matrix has no macOS rows today]
     build --profile release-ffi -> check-elf-glibc-floor <=2.35 -> stage lib/ include/ LICENSE README SHA256SUMS
     -> tar -> sha256 -> upload dry-run-ffi-<target>
  linux-consumer [needs cli, ffi with `if: ${{ !cancelled() }}` so an unrelated macOS/Windows row cannot skip it; one job per Linux runner; first step downloads this arch's two artifacts and fails if either is absent]
     download both artifacts for this arch -> extract into clean dirs
     apt: at-spi2-core dbus-x11 xvfb x11-utils openbox gnome-calculator python3-gi gir1.2-atspi-2.0 gcc
     AGENT_DESKTOP_WM_ROOT=/ private-session --wm --   (apt layout; fails closed if no window manager answers)
        run-consumer.sh   (C consumer vs extracted .so + packaged header; Atspi readback)
        run-npm-install.sh (npm pack -> install -g --prefix scratch, PATH-first curl double serves the
                            real archives + checksums.txt -> wrapper + symlink -> snapshot + click + readback)
     ELF/loader closure printed (readelf, ldd) into the job log
```

The skill seam (KTD1) is the one switch; everything else is fixed, including the helper packaging (KTD7).

### Sequencing

U1 settles runner capability, the observer and the jammy Calculator addressability. U2 is documentation and can proceed alongside; U8 (documentation) follows U7 and U9 (the skill) waits for O1; U10 is the dogfood and U11 closes. U3 (gate + workflows) precedes U4 (consumer) and U5 (npm) because both consume its archives. U6 packages the clipboard owner helper in all three channels. U7 is the live run on the box and the hosted runners. The PR is one unit unless the owner splits it at the seam after U8 (release + FFI + npm + docs) from U9 (skill). See `plans/3.13-split.md` for the lane cut: the skill content is written first as its own docs lane.

---

## Implementation Units

### U1. Measurements before workflows

- **Goal:** settle by measurement what the plan would otherwise assume, as new ledger rows.
- **Requirements:** R1, R4, R6, R11.
- **Dependencies:** 3.12's private-session launch recipe; `scripts/remote-linux.sh`, `scripts/private-session.sh`; `L22` rows.
- **Files:** none in the repository; a probe script under the owner's probes directory, and its rows go to `probes/linux/FINDINGS.md` in U11. A throwaway branch run of the dry-run workflow with a scratch job is the vehicle for items 1-3; it is deleted before the PR.
- **Approach:** measure each item at least twice, in both directions where a claim has one:
  1. Runner capability: on `ubuntu-22.04` and `ubuntu-22.04-arm`, `apt-get install` the package set of the High-Level Design, start `AGENT_DESKTOP_WM_ROOT=/ scripts/private-session.sh --wm` (the script otherwise looks for a user-space openbox root a runner lacks, and its window-manager wait must fail the run, not proceed without one), choose the GSK renderer that lets Calculator register without a GPU, launch Calculator, and record: package versions, whether the bus and registry answer, the Calculator window and ref count, whether `Atspi` imports (closes the aarch64 half of `L22-3`).
  2. Floor (closes `L22-2`'s open half): build the CLI and cdylib on both runners, record `file`, `readelf -h/-l/-d`, `readelf --version-info` max `GLIBC_`, and `ldd`; require max <= 2.35 or record what exceeds it and which dependency causes it. Run the artifacts unmodified on the runner.
  3. GNOME 42 reach: can jammy's `gnome-shell` run rootless with `--x11` under Xvfb on the runner (package size, time, whether Calculator and Files register)? Record yes/no and the app versions. This decides whether `L22-15` is closed in part or stays an owner infrastructure item.
  4. Observer: does the `Atspi` typelib read the Calculator display's text on jammy 41.x and on 24.04 46.x, and does the `7` button resolve by name on both (it may be labelled differently on 41)? The answer fixes the target and the observer per KTD5.
  5. Hosted `ctypes` order and a worker-thread first read on aarch64 (the `L22-6` result re-checked).
  6. `npm install -g` of a packed package with the PATH-first `curl` double on both runners; confirm the installer's exact `curl` invocation the double must accept, and that `npm`'s `allow-scripts` warning does not stop the postinstall (`L22-13`).
  7. Wrapper start-up: JS wrapper versus the replaced global symlink, min-of-seven (feeds R11).
- **Test expectation:** none -- a measurement run.
- **Verification:** each item has a row with a verdict; any item contradicting this plan is reported and the affected KTD is rewritten before its unit starts.

### U2. Roadmap and ledger corrections

- **Goal:** `docs/phases.md` and the ledger say what is true after this PR (KD7).
- **Requirements:** R9, R10, R12.
- **Dependencies:** none for the corrections the evidence already forces; U7 and U10 for the dispositions they produce (finished in U11).
- **Files:** `docs/phases.md`, `probes/linux/FINDINGS.md`.
- **Approach:** correct in place, no "previously" text, each citing its row or source:
  - §3.13 Scope: the glibc floor is verified by the gate on the finished files (`L22-2`), the CLI matrix rows name `ubuntu-22.04`/`-arm` explicitly and the gate reads the finished file, the skill requires the owner seam (KTD1), and the empty-`supported_surfaces` clause is replaced by what is advertised at the 3.14 head (Window, Sheet, Menu, and the shell surfaces when the shell tree is present) plus the explanation of an empty list in general. The §3.13 bullets that other sub-phases wrote (3.4 to 3.8, 3.10, 3.14) stay; each later sub-phase's bullet is only checked against its landed code. The 3.13 "Depends on" line gains 3.12 and 3.14.
  - §3.13 Depends on: add 3.12, the owner seam O1 and the 3.10 payload decision; Exit criteria: replace "npm install -g works on Ubuntu; release dry-run artifacts verified" with the falsifiable form of R4-R6 (extracted archives, both architectures, live snapshot and one independently verified action, installed through the wrapper and the symlink), restated as a branch-verifiable criterion in the way the Windows packaging criterion was, because publication fires only from `main`.
  - Release, Skill & Docs checklist: the `ad_*` line from `P3-O9`/`O10`/`O11` (Phase 3 adds no FFI wrapper; existing exports are tested); the core-registration line becomes the owner prerequisite (KTD1); mandatory `xdotool`/`ydotool`/`xclip` become the landed native routes (3.8, 3.10); the unscoped `cargo build --release` and `libdbus-1-dev` become the host-scoped build with no native bus library (`L22-1`); `busctl --user list | grep a11y` becomes cold activation (`L1-5`); the blanket Flatpak/Snap sentence becomes a 3.14 pointer (`L12-8`, `L12-9`); the README "Yes" becomes capability-specific.
  - The Phase 3 introduction's F7/J6 bullet gains the statement that 3.13 cannot commit the skill before the owner delivers the seam.
  - Receiving scopes for deferrals (R12): the GNOME 42 shell/Mutter and native-arm64 desktop residue (3.15), none for clipboard parity (3.10 supports FFI-host clipboard-set).
  - Ledger: add `L22-1` to `L22-15` (rows only, no captures, no box identity); U1's rows are added in U11; append a 3.13 clause (never replace a disposition) to rows several sub-phases claim.
- **Test expectation:** none -- documentation; `python3 scripts/check-linux-findings.py` and `scripts/check-no-phase-references.sh` run in the Verification Contract.
- **Verification:** the checks pass, and a reader of §3.13 can plan 3.14 and 3.15 from it without opening this PR.

### U3. The glibc gate and the Linux release and dry-run legs

- **Goal:** Linux CLI and FFI archives exist from the branch commit and from a release, built on the floor host and gated on the finished file.
- **Requirements:** R1, R2, R3, R13.
- **Dependencies:** U1 (runner behaviour, floor result).
- **Files:** `scripts/check-elf-glibc-floor.sh`, `scripts/check-elf-glibc-floor-selftest.sh`, `.github/workflows/release.yml`, `.github/workflows/release-dry-run.yml`, `scripts/check-npm-package.js` (+ its fixtures).
- **Approach:** KTD2, KTD3, KTD9.
  - Gate script and self-test as in KTD2; the self-test runs in `ci.yml`'s `test-linux` job (ubuntu-latest, whose newer libc produces the failing object) so every PR exercises it; in CI it exits non-zero when it cannot produce that object, and only a documented local override skips.
  - `release.yml` `build` matrix gains `x86_64-unknown-linux-gnu` on `ubuntu-22.04` and `aarch64-unknown-linux-gnu` on `ubuntu-22.04-arm`, with Linux steps in the same style as the others: build (`cargo build --locked --release -p agent-desktop --target ...`, plus the helper through `cargo build --locked --release -p agent-desktop-linux --bins --target ...`, since cargo builds another package's bin only with `-p agent-desktop-linux --bins`), size check with `stat -c%s` against 15 MiB, the glibc gate, a `version` smoke, tarball creation with an exact-listing check (`agent-desktop` and `agent-desktop-linux-clipboard-owner`) and `.sha256`. `build-ffi` gains the glibc gate on the `.so` and a Linux README paragraph (glibc 2.35 floor, `dlopen` by path, no SONAME so link with `-L`/`-rpath`, calls on one adapter serialise, text-only clipboard and its X11 owner note (the helper sits beside the library, found through the image and not `PATH`; set from a host works and persists after the host exits, per KTD7), and that `ad_snapshot` takes no window id or root), with a content assertion over the staged README. `EXPECTED_ASSETS` becomes 13 with the explanation `(6 CLI tarballs + 4 FFI tarballs + 2 FFI zips + checksums.txt)`. The `ffi-release-panic-guard` Linux rows stay.
  - `release-dry-run.yml`: the `cli` matrix gains both Linux rows with the same steps; the `ffi` matrix gains both Linux rows with a bash staging step that mirrors `release.yml`'s Unix path (library, header, licence, README, `SHA256SUMS`, archive, checksum check); path filters add the new scripts and `tests/linux-dist/**`. No job gains write permission and none can publish.
  - `check-npm-package.js` learns the Linux tarball construction in `tarballConstructionsByOs` (the `runner.os == 'Linux'` branch), and its fixture suite gains a case where a Linux matrix row has no Linux package step and is rejected.
- **Execution note:** the workflows cannot run on the box. Lint with the repository's workflow checks where present; the proof is U7's dispatch of the dry-run on the branch.
- **Test scenarios:**
  - The gate fails a file requiring `GLIBC_2.39`, passes one requiring at most 2.34, fails a file with no version info, and names the symbol in its message.
  - `check-npm-package.js`: a released Linux target with no Linux matrix row fails; a Linux row with no Linux package construction fails; the real workflow passes with the asset count at 13.
  - A text test over `release-dry-run.yml` fails on any write permission scope or any `npm publish`, `gh release create` or `gh release upload` step (R3).
  - The release workflow's `build` and `build-ffi` Linux rows run on the floor hosts (a text test over the workflow, in the existing style, fails if a Linux row names `ubuntu-latest`).
- **Verification:** `node scripts/check-npm-package.js` passes; the self-test passes on the Linux lane; each rule broken in turn (gate always exits 0, Linux construction branch removed, count left at 11, a Linux row moved to `ubuntu-latest`, a write permission added to the dry run) makes its check fail.

### U4. The real FFI consumer on both architectures

- **Goal:** the packaged `.so` is proven against a real app: observe, act once, read the effect back independently, refuse stale identity, survive a lost bus.
- **Requirements:** R4, R5, R13.
- **Dependencies:** U1 (observer and target), U3 (archives); 3.5/3.7 for the action leg; 3.9/3.12 for launch helpers; 3.11 waits for appearance.
- **Files:** `tests/linux-dist/consumer.c`, `tests/linux-dist/run-consumer.sh`, `tests/linux-dist/atspi_read.py`, `tests/ffi-python/smoke.py` (the `AD_EXPECT_REAL` arm), the `linux-consumer` job in `.github/workflows/release-dry-run.yml`.
- **Approach:** KTD3, KTD5, KTD6, KTD10, KTD11. `run-consumer.sh` takes the extracted archive directory, compiles `consumer.c` against `include/agent_desktop.h` and links or loads `lib/libagent_desktop_ffi.so`, then runs, inside a private session with a launched target:
  1. `abi`: `ad_abi_version()` equals `AD_ABI_VERSION_MAJOR`; the four size getters (`ad_action_size`, `ad_action_result_size`, `ad_ref_entry_size`, `ad_exact_window_info_size`) equal the header macros.
  2. Handshake: `ad_init` correct and wrong major, with creation before and after (`L22-6`); `ad_check_permissions` result recorded.
  3. `snapshot <app>`: return code 0, `"ok":true`, `ref_count` above a floor taken from U1, a qualified ref for the named control; the JSON is freed.
  4. `click <ref>`: exactly one execution through `ad_execute_by_ref_timeout`; the effect (the display text) is read by `atspi_read.py`, never by the snapshot; the display before and after is compared and equals the expected string once. A `PLATFORM_NOT_SUPPORTED` result fails the leg.
  5. `windows-exact <app>`: list, keep the opaque id and `process_instance`, read the raw tree, stop the owned target, require `WINDOW_NOT_FOUND` for the old id, start a fresh target and require a different id and instance and that the old id still refuses (`L22-10`).
  6. `lifetime`: 20 create/snapshot/destroy cycles; destroy during an in-flight call completes and a later call returns the invalid-handle code (`L22-7`).
  7. `serialise`: eight threads by three snapshots on one adapter all return `ok:true`; no latency assertion.
  8. Lost bus: stop only the session's own accessibility bus processes (by recorded pid), require the same adapter's snapshots to return `INTERNAL`, relaunch the target and require the same adapter to snapshot `ok:true` again (`L22-8`, KTD10).
  9. Clipboard per KTD7: `ad_set_clipboard` text from the C host, the host exits, then an independent foreign reader (not the library) reads the value; the helper is found beside the library and not through `PATH` (the leg repeats with `PATH` stripped); `ad_clear_clipboard` from a second process ends it.
  `smoke.py` with `AD_EXPECT_REAL=1` requires `ok:true` for a named app, covers the ctypes worker-thread read, and keeps its stub behaviour when the variable is unset. The job runs on both Linux runners, downloads that architecture's dry-run artifacts and never rebuilds.
- **Test scenarios:** each numbered leg above is one assertion with a message naming the code returned; the whole run fails closed when no target registers (no skip-as-pass). The consumer never kills by name, owns every process it starts and stops them by pid.
- **Verification:** passes on the box (x86_64, 24.04, against a locally built `release-ffi` library for the loop) and in the hosted job on both architectures; invert-verified by: wrong header size macro, a click that targets a different ref (readback unchanged), a stubbed `PLATFORM_NOT_SUPPORTED`, a stale id that binds the new process, a bus kill that is not followed by relaunch, a smoke run with the real adapter swapped for the stub build.

### U5. npm activation and installed-package verification

- **Goal:** `npm install -g` on Linux installs a checksum-verified binary that runs and acts, on both architectures.
- **Requirements:** R6, R7, R13.
- **Dependencies:** U1 (curl invocation, `allow-scripts`), U3 (archives); the target as U4.
- **Files:** `npm/lib/platform.js`, `npm/scripts/postinstall.js`, `tests/npm/postinstall.test.js`, `tests/npm/wrapper.test.js`, `tests/npm/helpers.js`, `scripts/ci-npm-wrapper-smoke.sh`, `tests/linux-dist/run-npm-install.sh`, `tests/linux-dist/curl-double.sh`, `.github/workflows/ci.yml` (Linux wrapper-smoke step), `.github/workflows/release-dry-run.yml` (the install leg of `linux-consumer`).
- **Approach:** KTD3, KTD8, KTD9.
  - Flip both Linux `released` flags. `cleanupStaging` removes staging with `rmSync` when `trash` is unavailable (all platforms keep their current behaviour where `trash` works), so a normal Linux install leaves no `.extract-*` and no warning (`L22-12`).
  - Tests that pinned Linux as unreleased now pin the released table (`linux-x64`, `linux-arm64`, archive entries, binary names, prompt mapping once the skill is served), and unsupported-architecture tests use genuinely unsupported keys (for example `linux-riscv64`, `freebsd-x64`). New cases: Linux archive extraction with the exact member list, digest reuse (second install makes zero downloads), corrupt archive, checksum mismatch, missing checksum entry, unexpected member, missing `curl`, missing `tar`, scripts disabled (the wrapper's message), missing `trash` (no leftover directory).
  - `node --test tests/npm/*.test.js`, `check-npm-package.js` and `ci-npm-wrapper-smoke.sh` run only in the macOS job of `ci.yml` today (`ci.yml` lines 224-229); the Linux `test-linux` job (ubuntu-latest) runs none of them, so the Linux job gains the npm tests and the check as well as the smoke, and its existing release build gains the helper build.
  - `ci-npm-wrapper-smoke.sh` (its macOS helper handling is restructured per platform, not only a new case arm; `check-npm-package.js` is edited in its construction loop, regex and self-test fixture) gets `Linux-x86_64` and `Linux-aarch64` arms (`agent-desktop-linux-x64`/`-arm64`, two entries (`agent-desktop` and `agent-desktop-linux-clipboard-owner`), `sha256sum`), and `ci.yml`'s `test-linux` job runs it after its release build.
  - `run-npm-install.sh`: `npm pack` in `npm/`, `npm install -g <tgz> --prefix <scratch>` with `curl-double.sh` first on `PATH` mapping the hard-coded release URLs to the extracted dry-run directory (archive and `checksums.txt` assembled the way `publish-github` does), `curl-double.sh` serves the extracted dry-run CLI archive renamed to `agent-desktop-v<npm/package.json version>-<target>.tar.gz` with a regenerated `checksums.txt` line for that name, and accepts the installer's exact argv (recorded in U1 item 6); then: the installed `agent-desktop version` through the symlink and `node <prefix>/lib/node_modules/.../bin/agent-desktop.js version` through the wrapper both report `os: linux` and the right `target`; a live `snapshot` of the launched target; one `click` with the effect read back by `atspi_read.py`; exit codes propagate (`snapshot --app nonexistent` exits 1). `AGENT_DESKTOP_BINARY_PATH` is never set in this script (it makes zero downloads, `L22-11`).
- **Test scenarios:** as above; the installed-package leg fails if the double served nothing (the double records each request and the script requires both the archive and `checksums.txt` requests on first install, zero on second).
- **Verification:** `node --test tests/npm/*.test.js` and `node scripts/check-npm-package.js` pass; the hosted install leg passes on both runners; invert-verified by: `released: false` restored (the install prints the unreleased message and the leg fails), the `rmSync` fallback removed (the leftover directory is found), a corrupted archive with a matching checksum, a double serving a stale archive.

### U6. Clipboard payload parity across the three channels

- **Goal:** the clipboard owner helper is in every channel, with one check over all three.
- **Requirements:** R8, R5.
- **Dependencies:** 3.10 landed; U3, U5.
- **Files:** `npm/lib/platform.js` (helper list), `npm/scripts/postinstall.js`, `npm/bin/agent-desktop.js`, `scripts/check-npm-package.js` (and `EXPECTED_ASSETS` accounting where it counts files), both workflows' Linux archive steps, `scripts/ci-npm-wrapper-smoke.sh`, `scripts/ci-ffi-helper-discovery-smoke.sh` (macOS-only today: it copies the dylib and `agent-desktop-macos-helper` into a scratch directory and runs `tests/ffi-python/smoke.py` with `AD_EXPECT_MACOS_HELPER=1`; the Linux twin copies the `.so` and the owner helper and uses the same smoke arm, and is the precedent for the discovery leg), `tests/linux-dist/run-consumer.sh` (clipboard leg).
- **Approach:** KTD7. Generalise the macOS helper constant to a per-platform helper list, add the helper to the CLI listing check, the FFI `lib/`, the npm `entries`, the postinstall and wrapper handling, and the manual-download fallback text, build it with `-p agent-desktop-linux --bins`, and add one archive-contents check over the three channels. `ci-npm-wrapper-smoke.sh` and `check-npm-package.js` account for the helper file (the wrapper smoke asserts it is present, executable and used by a clipboard set and read). The helper's build identity must match the packaged adapter.
- **Test scenarios:** a CLI archive missing the helper fails the listing check; an FFI archive missing it fails; `entries` missing it fails `check-npm-package.js`; from the host process, `ad_set_clipboard` marks the selection, a foreign requestor reads it before and after the host exits, `ad_clear_clipboard` clears it, and the helper is discovered beside the library image and not through `PATH`.
- **Verification:** the scenarios pass; invert-verified by removing the helper from one channel at a time.

### U7. Live verification, performance and size

- **Goal:** prove R1-R8 on the box and on the hosted runners from the branch commit, and price them.
- **Requirements:** R1-R8, R11.
- **Dependencies:** U3-U6.
- **Files:** none new; the dry-run dispatch and the box runs.
- **Approach:** (1) on the box through `scripts/remote-linux.sh`, in a private session: build `release-ffi` and the release CLI, run `run-consumer.sh` and `run-npm-install.sh` against locally staged archives (the gate runs on the box build as a negative control: it must fail on 24.04, `L22-2`); (2) dispatch `release-dry-run.yml` on the branch (it can reach no publish job) and read both Linux `linux-consumer` jobs; (3) record the dry-run run identifier in the PR description. The perf method is the Linux one (`scripts/perf-baseline-compare.sh` is macOS-bound): min of seven with the warm-up discarded, reporting min, median and max for the FFI `ad_snapshot` against the CLI `snapshot` on the same target, the eight-way overlap per-call figures, the wrapper versus symlink start-up, and the release size of the CLI and cdylib on both architectures against 15 MiB. The numbers go in the PR description.
- **Test expectation:** none -- a run; its failures are fixed in the unit that owns them.
- **Verification:** all legs green on the box and on both runners; the glibc maximum of each of the four files is recorded; any residue (GNOME 42 shell, arm64 desktop) is the written deferral of U2.

### U8. Documentation and release notes

- **Goal:** the README, the FFI docs and the release text state what Linux does, from landed behaviour, independent of the skill seam.
- **Requirements:** R5, R7, R10.
- **Dependencies:** U7 (measured facts); the landed facts of 3.8-3.11.
- **Files:** `README.md`, `crates/ffi/README.md`, `skills/agent-desktop-ffi/` (the Linux load and link examples), `docs/phases.md` (the release-notes text for the promotion).
- **Approach:** KTD12, KTD13. README: the platform table with capability-specific Linux entries and footnotes; install instructions (npm, direct archive, from source with the host-scoped build and no native bus library); the permissions section (cold bus activation, not `busctl`); the install requirements `curl`, `tar` and a script-permitting npm (`allow-scripts`, `L22-13`); Ubuntu 22.04+ stated as verified, Fedora 38+ as loadable by the 2.35 floor but unverified. FFI docs: `dlopen` by path, `-L`/`-rpath`, no SONAME, serialised calls, text-only clipboard, the restricted `ad_snapshot` arguments, and the clipboard note per KTD7. Release notes: `release.yml` creates the release with a fixed `Release <tag>` body and release-please writes the changelog from commit messages, so the Linux support, glibc floor, display-server and Flatpak/Snap-to-3.14 statements are carried by the commit body of the integration promotion; this unit drafts that text in the PR description and writes it into `docs/phases.md` for the promotion (owner item O6). No workflow change.
- **Test expectation:** none beyond the dogfood executing every example; `scripts/check-no-phase-references.sh` for shipped source.
- **Verification:** every command in the README Linux sections runs verbatim in the default shell on the private session; the glibc figure equals U7's measured maximum.

### U9. The Linux platform skill (gated on O1)

- **Goal:** `agent-desktop skills get platform` on Linux returns the Linux skill, and every claim in it is one a stranger can execute verbatim.
- **Requirements:** R9, R10, R13.
- **Dependencies:** O1 delivered; 3.4-3.12 landed (the facts below are theirs); U8.
- **Files:** `skills/agent-desktop-linux/SKILL.md` and `skills/agent-desktop-linux/references/*.md`, `release-please-config.json` (a generic extra-file entry for the new `SKILL.md`), `skills/agent-desktop/SKILL.md` line 30 (says "Linux has no platform skill yet") and `skills/agent-desktop/references/workflows.md` First-Time Setup (macOS and Windows branches only; add the Linux branch, the core test pins that both existing names stay), `skills/agent-desktop/references/commands-*.md` Linux differences (Markdown only), `npm/scripts/postinstall.js` (`platformSkill` map gains `linux: 'agent-desktop-linux'`) and `tests/npm/postinstall.test.js`, `CONTRIBUTING.md` line 166 and `.github/PULL_REQUEST_TEMPLATE.md` line 40 (name the Linux skill). The release `publish-skills` job loops `skills/*` and `scripts/link-skills.sh` loops the directory, so neither needs an edit. The seam commit (O1) is the only commit that touches `skills.rs`.
- **Approach:** KTD1, KTD10, KTD12. The full content list, with the branch each item came from, is in `plans/3.13-split.md` lane S; this unit writes only what the landed code and the ledger support at the cut. If no seam exists when this unit's turn comes, stop and report: the PR is marked as awaiting O1 with U9 and the skill-only dogfood leg named as open, and nothing from this unit is committed. The content is aggregated from the handoffs the earlier plans assign to this sub-phase, rewritten against the landed code: first-run state-root relocation (`AGENT_DESKTOP_HOME`); bus activation and the three states (answered and healthy, answered and empty, unavailable) and why `busctl --user list` is not a health check (`L1-5`, `L1-9`, `L1-13`, `L1-16`); the lost-bus `INTERNAL` versus cold-start mapping (KTD10); the names `list-apps` actually returns and qualified refs from a fresh observation (`L16-1`, no `Finder`); the X11/Wayland capability table (`L5-5`, `L5-6`, `L13-5`, `L13-8`, `L13-14`); semantic versus physical policy and delivery verification (3.7, 3.8); launch, close, window and focus claims with the per-window-manager un-minimize scope (3.9); capture and clipboard ownership, the Wayland tool requirement and what a refused covered window means (3.10); the wait limits (3.11); toolkit and menu limits (`L3-30`, `L18` rows); the FFI notes of R5; held input and cursor overlay status; the shell/sandbox boundary stated as 3.14's. The empty-`supported_surfaces` explanation covers the general condition, not the current Linux list. The core guide's cross-platform workflows gain the Linux differences in the same PR; Rust core is untouched.
- **Test expectation:** none for the Markdown beyond the seam's own catalogue test (owned by O1's deliverable); `scripts/check-no-phase-references.sh` covers shipped source, and the dogfood executes every example.
- **Verification:** `skills`, `skills get platform` and `skills get platform --full` serve the Linux skill on Linux and list it `current`, existing aliases, references and version matching stay coherent, `git diff -- crates/core` is empty, and the version marker is bumped by a release-please dry check.

### U10. Stranger dogfood

- **Goal:** find what a new agent hits using only the shipped artifacts and docs.
- **Requirements:** R14.
- **Dependencies:** U7, U8; U9 for leg 3.
- **Files:** `docs/dogfood-reports/<date>-<serial>-feat-linux-3-13-ffi-npm-release-dogfood.md`.
- **Approach:** KD8, KTD14. Each operator is a subagent briefed that it holds only the artifact for its leg and the shipped docs, repository withheld, working in `scripts/private-session.sh --wm`; every example runs verbatim in the default shell; every step's `ok` is read; waiting uses the tool's own primitives; any undocumented flag, variable or ordering is a finding; a failure counts after a second independent observation; a behavioural claim is measured in both directions. Legs:
  1. **npm:** a clean environment with only the dry-run archives and the packed package; the README install section verbatim; first command, first snapshot, first action; missing-`curl`/`tar`, read-only prefix, `--ignore-scripts` as negatives (`L22-13`).
  2. **Foreign FFI embedding:** only the extracted FFI archive and header; a C and a Python consumer written from the docs alone; ABI mismatch, destroy-then-use, two threads, bus loss, stale id, clipboard per KTD7.
  3. **Skill-only:** only the installed CLI and `skills get`; blocked until O1 and said so; once served, every example verbatim on X11 and on the Wayland tier of 3.12.
  4. Adversarial pass: an unsupported architecture key, an old glibc userland (the jammy userland of `L22-2` extracted unprivileged), a path with spaces, a stale `SHA256SUMS`.
  The report is judged: at least one finding, else the targets are made harder.
- **Test expectation:** none -- a run and a report; each finding is fixed (a named test that fails when the fix is broken), owned elsewhere (the receiving scope written into `docs/phases.md` in this PR) or accepted with a reason.
- **Verification:** the report is committed, every finding has one disposition, every fixed finding's test was seen to fail with the fix removed.

### U11. Close-out

- **Goal:** the roadmap, ledger and PR description record exactly what shipped.
- **Requirements:** R12, R13.
- **Dependencies:** U7, U10.
- **Files:** `docs/phases.md`, `probes/linux/FINDINGS.md`.
- **Approach:** add U1's rows, dispose of every `L22` row (below), finish U2's receiving-scope text with the run results, restate §3.13's `Est. PR size` with the measured size, and run the Verification Contract.
- **Test expectation:** none -- documentation.
- **Verification:** `python3 scripts/check-linux-findings.py` exits 0 and `--sub-phase 3.13` lists no `DEFERRED` row.

---

## Verification Contract

| Check | Command or method | Proves |
|---|---|---|
| Lint | `cargo clippy --locked -p agent-desktop-core -p agent-desktop-linux -p agent-desktop -p agent-desktop-ffi --all-targets -- -D warnings` on the box | quality gate on the Linux set |
| Unit | `scripts/cargo-test-isolated-home.sh test --locked -p agent-desktop-core -p agent-desktop-linux -p agent-desktop -p agent-desktop-ffi` under `dbus-run-session -- xvfb-run -a` | the existing suites still pass |
| ELF gate | `scripts/check-elf-glibc-floor-selftest.sh` on the Linux lane; the gate on each shipped file in the dry run | R1 |
| npm | `node --test tests/npm/*.test.js` and `node scripts/check-npm-package.js` | R2, R6, R7 |
| Consumer, box | via `scripts/remote-linux.sh`: `scripts/private-session.sh --wm -- tests/linux-dist/run-consumer.sh <staged archive dir>` | R4 on x86_64 24.04 |
| Install, box | the same for `run-npm-install.sh` | R6 on x86_64 24.04 |
| Dry run | `gh workflow run release-dry-run.yml --ref <branch> -f version=0.0.0-dryrun` (no publish job exists in it), then both `linux-consumer` jobs green | R2-R4, R6 on x86_64 and aarch64 from the branch commit |
| Invert | break each rule named in U3-U6 (gate always passes, Linux package step missing, count 11, `released: false`, `rmSync` fallback removed, header size macro wrong, click on a wrong ref, stale id binds the new process, no relaunch after bus loss, helper dropped from one channel), watch its check fail, restore | the checks guard what they name |
| Isolation | `cargo tree -p agent-desktop-core` shows no platform crate and `git diff <base> -- crates/core` is empty | R13, KD1 |
| Dependency | `git diff -- Cargo.lock Cargo.toml npm/package.json` shows no added dependency | KD6 |
| Header | `bash scripts/update-ffi-header.sh` check / the `ffi-release-gates` verify shows no drift | the ABI is untouched |
| Source rules | `scripts/check-rust-file-size.sh`, `scripts/check-no-phase-references.sh`, `scripts/check-stale-ref-constructor-misuse.sh`, `cargo fmt --all -- --check`, `scripts/check-release-consistency.sh` | 400-line cap, no plan ids in source, version coherence |
| Ledger | `python3 scripts/check-linux-findings.py` exits 0, then `--sub-phase 3.13` lists no `DEFERRED` row | R12 |
| Perf | min-of-seven with the warm-up discarded (min, median, max); size on both architectures against 15 MiB | R11 |
| Skill | `skills`, `skills get platform`, `--full` on Linux; every example verbatim | R9, once O1 lands |
| Dogfood | the committed report with every finding disposed of | R14 |

Requirement-to-check map: R1 → U3 gate and self-test, U7 per-file maxima; R2 → U3 workflow and `check-npm-package.js` rows; R3 → U3 dry-run legs, U7 dispatch; R4 → U4 legs 1-9 on the box and both runners; R5 → U3 README paragraph, U4 clipboard and serialise legs, U8; R6 → U5 tests and install leg; R7 → U5 failure-mode cases, U8 README; R8 → U6 archive-contents test and clipboard leg; R9 → U9 and the Skill row; R10 → U2, U8, dogfood legs; R11 → U7 Perf; R12 → U2, U10, Ledger; R13 → Isolation, Dependency, Source rules; R14 → U10.

## Definition of Done

- Every Verification Contract row passes, the invert check has been run for each rule it names, and the dry-run run identifier and its two Linux consumer results are in the PR description. No row was satisfied by `ok:false`, a version call or `AGENT_DESKTOP_BINARY_PATH`.
- Every ledger row naming 3.13 is disposed of in the PR description:
  - `L22-1`, `L22-2`: implemented as the floor host and the gate (U3); the 22.04-built artifact's real maximum is recorded from the dry run (U7).
  - `L22-3`: closed by the aarch64 runner legs (U1, U4, U5, U7) for the ELF, loader, FFI and npm paths; native arm64 desktop residue written into 3.15's scope.
  - `L22-4`: implemented (U4 legs 1-4), the action leg failing on `PLATFORM_NOT_SUPPORTED`.
  - `L22-5`: implemented as the independent observer (KTD5, U4).
  - `L22-6`, `L22-7`, `L22-10`: implemented as consumer legs (U4); serialisation documented (KTD11).
  - `L22-8`: recovery asserted (U4 leg 8); the classification question accepted as documented behaviour with its reason, owner item O4 for the bus-mapping owner.
  - `L22-9`: implemented by 3.10's helper and packaged here (KTD7, U6).
  - `L22-11`, `L22-12`, `L22-13`: implemented (U5).
  - `L22-14`: implemented when O1 lands (U9); otherwise owned elsewhere (O1) with the receiving scope written into `docs/phases.md`.
  - `L22-15`: closed in part or owned by 3.15, per U1 item 3.
  - U1's new rows: each with its verdict and a disposition.
- The dogfood report is committed with every finding disposed of and every fixed finding's test invert-verified; a skill-only leg that could not run because O1 is open is listed as open, not passed.
- `crates/core` diff is empty, no dependency was added, no shipped source names a plan, phase or decision id, and no probe script, capture or box identity is committed.
- `docs/phases.md` states what was built and no longer states what the evidence disproved; every deferral is written into the receiving sub-phase's scope.
- Nothing was published, nothing merged to `main`, and no abandoned experiment code remains (the throwaway runner-probe branch is deleted).

## Risks & Dependencies

| Risk | Mitigation |
|---|---|
| O1 (skill seam) does not land in time | U9 stops before committing; the PR names U9 and the skill-only dogfood leg as open; the rest ships; reducing scope is the owner's call |
| A 22.04-built artifact still exceeds 2.35 through a dependency | U1 item 2 measures it before the workflow edits; the gate names the symbol; the fix is in the dependency or the floor is restated with the evidence |
| Hosted runners cannot run the GNOME app set or gnome-shell | U1 items 1 and 3 decide; the consumer falls back to the box for x86_64 and the arm64 residue is owned by 3.15 with the evidence, never passed |
| Jammy Calculator 41 differs from 46 (button names, display exposure) | U1 item 4; the target and the observer are fixed per architecture from the measurement, and 3.12's recipe wins if it differs |
| The helper is missing or stale in one channel | one archive-contents check over the CLI archive, the FFI archive and the npm package; the adapter's build-identity check fails closed with the named `PLATFORM_NOT_SUPPORTED` |
| The PATH-first `curl` double drifts from the installer's invocation | U1 item 6 records the invocation; the double fails loudly on an argument it does not recognise |
| The workflows cannot be run locally | the dry-run dispatch is the proof; text tests over the workflow guard the rules that can be guarded offline |
| PR size exceeds the ~2k guideline | about 1.2k lines of scripts, workflow YAML, tests and Markdown plus the skill; the seam after U8 (release, FFI, npm, docs) from U9 (skill) is named and the owner decides on a split |
| `L22` rows renumbered at the cut | the renumber clause in Assumptions; citations rewritten together |

## Open Questions

### Deferred to implementation

- The exact packages and versions the hosted runners install, and the Calculator control names on jammy (U1 items 1 and 4).
- Whether the product's own `get`/`find` exposes the display value once 3.7 lands, replacing the typelib observer (KTD5).
- The `curl` argument shape the double must accept (U1 item 6).
- The 3.12 branch and harness helper names this PR stacks on and reuses.

### For the owner

- **O1 (blocking for the skill, and for 3.14's skill leg):** the platform skill's distribution. The code has no neutral seam to deliver; the answer is an approved minimal Linux arm confined to `skills.rs` and `skills_tests.rs` (about 30 lines: constants, `CURRENT_PLATFORM`, one `SKILLS` entry, two test edits), committed together with the Markdown. `docs/phases.md` currently lists it as an owner decision and, in one checklist line, tells 3.13 to build it; U2 rewrites that line.
- **O6:** the GitHub Release notes text for the integration promotion (release-please builds the changelog from the promotion commit body; U8 drafts it).
- **O4:** whether a bus connection lost after being established should keep `INTERNAL` or map like a cold-start failure (`L22-8`); this plan documents the current behaviour.
- **O5:** infrastructure for real Ubuntu 22.04 GNOME 42 (shell/Mutter) and a native arm64 desktop if the hosted-runner measurement cannot cover them (`L22-3`, `L22-15`); receiver 3.15.
- Whether to split this sub-phase at the named seam into two stacked PRs.

## Sources & Research

- `probes/linux/FINDINGS.md` rows `L1-1`, `L1-2`, `L1-5`, `L1-9`, `L1-13`, `L1-16`, `L3-30`, `L5-5`, `L5-6`, `L8-4`, `L12-2`, `L12-8`, `L12-9`, `L13-5`, `L13-8`, `L13-14`, `L16-1`, `L16-6`, `L18` rows; pre-code probe rows `L22-1` to `L22-15` (scripts under the owner's probes directory, captures never committed).
- `.github/workflows/release.yml` (CLI `build` matrix, `build-ffi`, `ffi-release-gates`, `ffi-release-panic-guard`, `publish-github` and `EXPECTED_ASSETS`, `publish-npm`, `publish-skills`), `.github/workflows/release-dry-run.yml`, `.github/workflows/ci.yml` (`test-linux`, npm and FFI steps), `release-please-config.json`.
- `npm/lib/platform.js`, `npm/scripts/postinstall.js`, `npm/bin/agent-desktop.js`, `tests/npm/`, `scripts/check-npm-package.js`, `scripts/ci-npm-wrapper-smoke.sh`, `scripts/ci-ffi-helper-discovery-smoke.sh`, `scripts/check-release-consistency.sh`, `scripts/link-skills.sh`, `scripts/private-session.sh`, `scripts/remote-linux.sh`.
- `crates/ffi/` (`src/adapter.rs`, `abi_version.rs`, `operation.rs`, `error.rs`, `commands/snapshot.rs`, `commands/execute_by_ref_timeout.rs`, `input/clipboard.rs`, `convert/window.rs`, `include/agent_desktop.h`, `tests/c_abi_windows_live_round_trip.rs`, `tests/c_abi_windows_bootstrap.rs`), `tests/ffi-python/smoke.py`, `tests/e2e/ffi_live.py` for the contract and the precedents.
- `crates/core/src/commands/skills.rs` and `skills_tests.rs` for the selection table and the catalogue-completeness test (read, not edited).
- `docs/phases.md` §Phase 3 introduction (F7, J6), §3.13, "Release, Skill & Docs", the Windows packaging exit criterion for the branch-verifiable form, and `docs/dogfood-reports/2026-10-18-001-feat-linux-3-1-toolchain-bus-dogfood.md`.
- Plans 3.4 (observation), 3.5 (resolution), 3.6 (actionability), 3.7 (semantic actions), 3.8 (input), 3.9 (lifecycle), 3.10 (capture and clipboard), 3.11 (signals and wait) and the 3.12 research dossier for what lands before this sub-phase and the skill handoffs each assigns here; the cross-plan audit of 2026-10-18.
- `docs/solutions/best-practices/never-ship-platform-code-that-ci-cannot-execute.md`, `docs/solutions/best-practices/real-app-tests-are-the-platform-adapter-gate.md`, `docs/solutions/best-practices/a-fixed-sleep-is-not-a-synchronization-primitive.md`.
