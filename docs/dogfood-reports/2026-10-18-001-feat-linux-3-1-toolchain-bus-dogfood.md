# Dogfood report - Linux toolchain and bus bootstrap (sub-phase 3.1)

Date: 2026-10-18 | Branch: `feat/linux-3.1-toolchain-bus`

Operator: a stranger holding only the shipped skill (SKILL.md plus 4 references) and the built binary. No source was read.

## Environment

| Item | Value |
|------|-------|
| OS | Ubuntu 24.04, x86_64 |
| Desktop | GNOME 46 on X11 |
| Binary | `{"version":"2.4","ok":true,"command":"version","data":{"os":"linux","target":"x86_64","version":"0.10.1"}}` |
| Session A (live, bus DOWN) | Real desktop session; AT-SPI socket `<runtime-dir>/at-spi/bus_0` missing. Not touched or repaired. |
| Session B (private, bus UP) | Private `XDG_RUNTIME_DIR`, `dbus-run-session`, `xvfb-run`; `gnome-calculator` started and polled for (up to 30 s loop on `status`). The a11y bus auto-activated on first `agent-desktop status`. |
| How run | Every command in bash through the transport, verbatim from the skill where the skill gives an example. Each failing result was reproduced in a second, separate invocation (bus-down twice, bus-up twice). |
| Caveat | The default state root (`$HOME/.agent-desktop`) is outside the permitted write area; `session start` wrote there because the skill documents no alternative for the first run. I ended every session I created. The skill documents `AGENT_DESKTOP_HOME` but it is not mentioned as a first-run step. |

## Judgements

**J1. `agent-desktop version` (both sessions)**
Observed: `ok:true`, `data:{os:"linux",target:"x86_64",version:"0.10.1"}`, exit 0. Matches the skill (`{ version, target, os }`). Verdict: pass.

**J2. `agent-desktop status` and `permissions`, bus DOWN (the skill's "first run" step 1 and the workflows "First-Time Setup")**
Observed, both commands, identical, exit 1: `ok:false`, `error.code:"PLATFORM_NOT_SUPPORTED"`, message "The AT-SPI accessibility bus is not available in this session", `platform_detail` "Failed to connect to address `unix:path=<runtime-dir>/at-spi/bus_0,guid=...`: No such file or directory", `disposition:{delivery:"not_delivered",retry:"safe"}`. Suggestion: "Run inside a desktop session with a D-Bus session bus. On GNOME run 'gsettings set org.gnome.desktop.interface toolkit-accessibility true'. Elsewhere install at-spi2-core and run at-spi-bus-launcher. In Flatpak ... In snap ...".
Verdict: disappointing. The error is honest (no false success) and names the cause and the missing path, which is genuinely useful. But (a) the code `PLATFORM_NOT_SUPPORTED` says the platform can never do this, while the platform is fine and the session is broken; the skill documents no code for "environment not ready". (b) `retry:"safe"` invites a blind retry; retrying cannot succeed until something outside the tool changes. (c) The advice is a generic menu for four environments. In this exact case the user is already in a desktop session, on GNOME, and the toolkit-accessibility setting is probably already on; the socket is gone, so setting it is unlikely to bring it back (not run, because the live session was off-limits). The useful advice here would be "the bus launcher is not running or its socket is gone; log out and in, or restart `at-spi-bus-launcher`". (d) `status` is the diagnostic command the skill tells you to run first and it fails outright instead of reporting `accessibility: denied/unknown` with the suggestion inside `data`, which is how the skill says `permissions` behaves ("`state`... with a `suggestion` when denied").

**J3. `status`, `permissions`, bus UP**
Observed: `ok:true`; `permissions:{accessibility:{state:"granted"},automation:{state:"not_required"},screen_recording:{state:"unknown"}}`, `platform:"linux"`, `supported_surfaces:[]`, `snapshot_id:null`, `session_id:null`, `tracing:false`, `state_root:"<home>/.agent-desktop"`. Verdict: pass for shape; the skill's states vocabulary (granted/not_required/unknown) is respected. Note `supported_surfaces:[]` with `accessibility:"granted"` means "bus up" and "nothing usable" look the same to an agent apart from the empty list; the skill never says an empty list means "no snapshots at all".

**J4. Bus-down versus bus-up contrast (same command, both directions)**
`status`: down gives PLATFORM_NOT_SUPPORTED exit 1; up gives ok. `permissions`: same. `wait 100`: down gives the same bus error; up gives `{"waited_ms":100}`. `batch '[{"command":"wait","args":{"ms":50}}]' --stop-on-error` (the skill example, trimmed to one entry): down gives the bus error; up gives `ok:true`, `completed_entries:1`. Verdict: disappointing. A pure time wait and a batch of time waits need no accessibility bus, yet are blocked by it. Meanwhile `session start/list/end`, `trace show/export`, `skills`, `version` work with the bus down, so the gate is inconsistent: some bus-free commands are gated, others are not.

**J5. `agent-desktop skills`, `skills get desktop`, `skills get desktop workflows`, `skills get desktop nonexistent` (both sessions)**
Observed: list returns 5 skills (core, macos, windows, ffi, jev) with `current:false` on the platform ones, no Linux entry; `get desktop workflows` returns `data.content` markdown; unknown reference returns `INVALID_ARGS` listing the available references. Verdict: pass. Note the skill list text calls the core guide "for every OS" and it does contain a Linux line (see J6).

**J6. `agent-desktop skills get platform` (both sessions)**
Observed: exit 1, `INVALID_ARGS`, "This build's operating system has no platform skill yet", suggestion "Use `agent-desktop skills get desktop`; it covers the loop on every OS", retry safe. Verdict: disappointing. The behaviour matches SKILL.md ("Linux has no platform skill yet") and the suggestion is actionable. But the skill instructs this as step 2 of "First run", so a Linux agent's second step is a guaranteed error, and `INVALID_ARGS` implies the caller typed something wrong. A stranger learns nothing about Linux (bus, what works).

**J7. `session start --name "invoice-bot"`, `export AGENT_DESKTOP_SESSION=<id>`, `status`, `trace show --limit 5`, `session end "$AGENT_DESKTOP_SESSION"`, `session list`, `session gc` (skill "Trace a multi-step run" and "Sessions and traces")**
Observed: start returns `session_id`, `name`, `trace:"on"`, `created_at`, plus `activation` and `next` hints (both work bus-down and bus-up). With the env var exported, `status` reports `session_id` and `tracing:true`; `trace show` returns events including `trace.meta` with `os:"linux"`; `trace export --out t.html` ok (57 events, one warning `unpaired_command` for the in-flight `trace` command itself); `session end` ok; `session gc` returned `removed: []` immediately after ending (no grace-period explanation). `trace show` without activation returns INVALID_ARGS "No active session for trace command" with a good suggestion. Verdict: pass overall; the lifecycle is the most complete part of the build. Small disappointments: the `unpaired_command` warning on the exporter's own run is noise; `session start` run twice creates two live sessions with the same name and no warning; sessions persist until ended (list grows).

**J8. `status` with a session, bus DOWN**
Not observable: with the bus down, `status` never reaches session reporting (J2), so the skill's "`status` confirms `session_id` and `tracing`" cannot be used to verify tracing in that state. Verdict: disappointing (follows from J2).

**J9. Observation commands, bus UP: `snapshot --app "Calculator" -i --compact`, `--skeleton`, `--surface menu`, `find --app ... --count`**
Observed: `PLATFORM_NOT_SUPPORTED`, "Snapshot surface 'window' is not supported on this platform", suggestion "Choose one of the supported snapshot surfaces", `details:{requested_surface:"window",supported_surfaces:[]}`, `disposition:{delivery:"unknown",retry:"unknown"}`. Verdict: fail/disappointing. Honest, right code, and the skill promises this code for unsupported surfaces. But: the suggestion tells the agent to choose a supported surface and the list is empty, so there is no choice to make; `window` is the default and it is the only thing a stranger asked for. `disposition` is `unknown/unknown` although nothing was attempted and the skill says observation commands carry no disposition. The honest message is "snapshot is not implemented on Linux yet". `--app Calculator` was never even resolved, so a typo'd app would give the same answer.

**J10. Commands that fail through unimplemented adapter methods, bus UP: `list-apps`, `list-windows`, `list-displays`, `list-surfaces`, `close-app`, `clipboard-get`, `clipboard-set`, `list-notifications`, `wait --window`**
Observed (representative): `list-apps`: message "list_apps is not supported on this platform", suggestion "Use a platform/build that advertises this capability or choose a supported command". `list-surfaces --app` and `close-app` also say "list_apps is not supported"; `wait --window` says "list_windows ..."; `screenshot --screen 0 out.png` says "list_displays is not supported"; `launch`, `clipboard-set`, `press`, `open-system-surface`, `permissions --request` say "acquire_interaction_lease is not supported on this platform". All `not_delivered`/`safe`, exit 1.
Verdict: fail on usefulness, pass on honesty. No false success. But the messages name internal adapter methods, often of a different operation than the command I typed (screenshot reports list_displays; clipboard-set and press report an "interaction lease"). A stranger cannot tell which command is unsupported or that it is a permanent condition. The suggestion is a non-answer: nothing says which commands do work on Linux, and `status.supported_surfaces` is empty. `permissions --request` is a diagnostic that fails with an interaction-lease message, which is baffling.

**J11. Commands, bus DOWN: `list-apps`, `list-windows`, `list-displays`, `clipboard-get` vs `snapshot`, `launch`, `screenshot`, `wait`, `click`, `find`, `batch`**
Observed: the first group returns "X is not supported on this platform"; the second group returns the AT-SPI bus error. Bus UP: the second group returns "surface not supported" / "acquire_interaction_lease..." / "list_displays...". Verdict: disappointing. Gate ordering differs per command, so the message depends on which internal method a command happens to call first. `screenshot` and `launch` under bus-down blame the bus even though, bus-up, they are unsupported regardless; fixing the bus per the advice would only reveal the next error. An agent following the bus advice gets no signal that it will still fail.

**J12. `click @s8f3k2p9:e1` and `is @s8f3k2p9:e1` (a fabricated ref from the skill example), bus UP; `click @e1` bus DOWN**
Observed bus UP: `SNAPSHOT_NOT_FOUND`, a long suggestion about session scoping and FFI (`ad_snapshot`, `ad_execute_by_ref`), `recovery.strategy:"refresh_snapshot_then_retry_original"`, `retryable:true`. Observed bus DOWN: both `click @s8f3k2p9:e1` and bare `click @e1` return the bus error, not the INVALID_ARGS the skill promises for a bare `@e1`. Verdict: mixed. Bus UP is correct per the skill, but the suggestion leaks FFI instructions to a CLI user and `retryable:true` tells the agent to snapshot, which cannot succeed on Linux yet (J9), producing a retry loop. Bus DOWN: argument validation (bare ref) should precede the platform gate; the skill says a bare `@eN` fails with `INVALID_ARGS`.

**J13. Unknown subcommand and help: `agent-desktop bogus`, `status --help`**
Observed: `INVALID_ARGS`, command `"unknown"`, exit 2 (matches CLAUDE-documented/skill codes); `--help` prints. Verdict: pass. `status --help` lists `--wait-for`, `--debug`, `--screenshot` for a status command (irrelevant global flags), low-value noise.

**J14. `cursor-overlay enable` without a session, bus UP**
Observed: `INVALID_ARGS` "Cursor overlay settings require an active session", suggestion to run `session start` and pass its id. Verdict: pass (clear and actionable).

## Findings

| id | severity | what is wrong | evidence | proposed disposition |
|----|----------|---------------|----------|----------------------|
| F1 | high | With the bus missing, `status` and `permissions` (the skill's mandated first diagnostics) fail outright with `PLATFORM_NOT_SUPPORTED` and `retry:"safe"`. The code says the platform is unsupported, the retry flag invites a loop, and the diagnostic gives no data. | J2, J4 | Fix here: report the condition inside `status` data (`accessibility` state `unknown`/`denied` with the suggestion) or use a distinct code (an environment-not-ready code, or PERM_DENIED-like) with `retry` not `safe`. |
| F2 | high | The bus-missing suggestion is a four-environment menu. For "already in a GNOME session, socket gone" it does not say the launcher/socket is gone, nor that the next error will be different. The gsettings advice would likely not bring back a deleted socket (not verified in the live session by design). | J2, J11 | Fix here: detect session type and a present-but-dead socket path, and give one targeted next step (restart the a11y bus launcher or re-login); include which commands will still fail afterwards. |
| F3 | high | Nearly every UI command is a stub whose message names an internal adapter method (`acquire_interaction_lease`, `list_apps`, `list_displays`) and often a different one than the command run. Suggestion is a generic non-answer. | J10, J11 | Fix here (small): per-command message "`<command>` is not implemented on Linux yet"; owned by later Linux sub-phases for the implementations. |
| F4 | medium | Gate ordering is inconsistent: some commands report the bus error, others "unsupported", for the same unsupported feature (e.g. `screenshot`, `launch` blame the bus when down). Following the bus advice only exposes the next error. | J11 | Fix here: check support before the bus, or say both. |
| F5 | medium | Bus-free commands are blocked by the bus gate: `wait <ms>` and `batch` of time waits fail bus-down but succeed bus-up; session/trace/skills work without it. | J4 | Fix here: only gate commands that touch the bus. |
| F6 | medium | Snapshot on Linux returns "surface 'window' not supported" with `supported_surfaces: []`, a suggestion to "choose one" from an empty list, and `disposition: unknown/unknown` although the skill says observation commands carry none. `--app` is never validated. | J9 | Owned by the Linux snapshot sub-phase for the feature; fix here the message/disposition (say snapshot is not implemented; omit disposition). |
| F7 | medium | Skill gives a Linux agent no usable start-up path: its "First run" step 2 (`skills get platform`) is a guaranteed `INVALID_ARGS` error; the frontmatter and prose list macOS and Windows only; no mention of the AT-SPI bus, no list of what works on Linux; `skills` lists no Linux entry. | J5, J6, J1 | Fix here: add a short Linux section or Linux platform skill (bus bootstrap, working commands, empty `supported_surfaces` meaning) and make `skills get platform` return it or a non-error. |
| F8 | medium | Argument validation does not precede the platform gate: bare `click @e1` bus-down returns the bus error, not the documented `INVALID_ARGS`. | J12 | Fix here: validate args first. |
| F9 | low | `SNAPSHOT_NOT_FOUND` suggestion embeds FFI call names (`ad_snapshot`, `ad_execute_by_ref`) for a CLI caller, and `retryable:true` sends the agent to a snapshot that cannot succeed on Linux. | J12 | Accept FFI wording (shared text); owned by the Linux snapshot sub-phase for the retry loop. |
| F10 | low | `session start` twice with one name creates two live sessions silently; `session gc` returns `removed: []` right after `session end` with no reason; `trace export` warns `unpaired_command` about its own in-flight command. | J7 | Accept / low-priority; document the gc grace rule. |
| F11 | low | `status` reports `state_root` as an absolute home path (identifying data in output); first-run guidance never says the default root writes under the home directory or how to relocate for sandboxed runs. | J3, Environment | Accept; mention `AGENT_DESKTOP_HOME` in first-run text. |
| F12 | low | `status` says `screen_recording: unknown` and `supported_surfaces: []` without explanation; an agent cannot distinguish "unimplemented" from "none present". | J3 | Owned by later Linux sub-phase; document meaning meanwhile. |
| F13 | low | `permissions --request` fails with "acquire_interaction_lease not supported" though the skill describes it as the prompt path; on Linux there is no prompt, which should be stated (`not_required`) rather than errored. | J10 | Fix here with F3. |


## Dispositions

Every finding takes exactly one disposition. "Fixed" rows name the test and the result of breaking the fix (invert) and restoring it. The Linux adapter is the only code changed; `crates/core` and the shared binary policy are untouched.

| id | disposition | where |
|----|-------------|-------|
| F1 | fixed here | `permission_report` now reports an unreachable bus as `PermissionState::Denied` with the cause-specific suggestion instead of returning an error, so `status` and `permissions` succeed. Tests in `crates/linux/src/system/permissions_tests.rs`: `a_missing_accessibility_socket_reports_denied_with_the_restart_step`, `a_missing_session_bus_reports_denied_with_the_desktop_session_step`, `an_absent_service_reports_denied_with_the_install_step`, `a_failed_launcher_spawn_reports_denied_with_the_launch_error_step`, `a_bus_timeout_stays_an_error_rather_than_a_denied_report`. Invert: making the mapping return the error again fails 4 of them; restored, all pass. The bus error also carries explicit non-retryable evidence (`retryable: false`, the core mechanism from `explicit_non_retryable_evidence_overrides_code_defaults`): `the_bus_error_carries_explicit_non_retryable_evidence`; invert: removing the evidence fails it. The envelope's `disposition.retry` is set by the binary for pre-dispatch errors, not by the adapter, and a missing bus no longer reaches that path; gated commands now answer `PERM_DENIED`, whose `not_delivered`/`safe` disposition is core's shared preflight and the same as a denied permission on macOS. |
| F2 | fixed here | The single four-environment suggestion is replaced by one suggestion per classified fault (`crates/linux/src/system/bus_fault.rs`): no session bus, service absent (install `at-spi2-core`, Flatpak and snap lines), launcher cannot start, socket gone (`systemctl --user restart at-spi-dbus-bus.service` or log out and in). The GNOME `gsettings` line is offered for none of them (L1-16). Tests: the four suggestion tests above, which assert the specific step and, for the socket case, the absence of `gsettings`. Invert: restoring the generic socket-gone text fails `a_missing_accessibility_socket_reports_denied_with_the_restart_step`; restored, it passes. `docs/phases.md` section 3.1 (scope, exit criteria, AT-SPI2 Bus Detection) and the plan (R5, R12, KTD5, KTD6, Definition of Done) are rewritten to match. |
| F3 | owned elsewhere | The method names come from core's `AdapterError::not_supported(method)` text. Written to the Phase 3 introduction in `docs/phases.md` as a cross-platform core follow-up for the owner. The `acquire_interaction_lease` variant of the message no longer appears for `launch`, `press` and `clipboard-set` because Linux now implements the lease (see F13); they proceed to the next unimplemented method. |
| F4 | accepted | Gate ordering follows the shared command policy: commands marked as needing Accessibility are refused by the permission preflight when the report is denied; list, clipboard and display commands need no permission and reach the adapter. macOS behaves identically with Accessibility denied. After F1 the refusal is `PERM_DENIED` with the cause-specific step. Which "unsupported" error a stub then returns is F3. |
| F5 | accepted | `wait` and every `batch` entry are policy-gated on Accessibility (`src/command_policy`), so with the report denied they fail on macOS the same way. Changing which commands are gated is a shared-policy change, outside Phase 3. Re-run: `wait 100` and a one-entry `batch` of a `wait` answer `PERM_DENIED` with the suggestion when the bus is down, and succeed when it is up. |
| F6 | owned elsewhere | Section 3.4 scope in `docs/phases.md` now requires `snapshot`/`find` without a surface flag to succeed and `supported_surfaces` to list what Linux supports (J9). The wording and `unknown`/`unknown` disposition of the surface error are core's and are listed in the Phase 3 introduction. |
| F7 | owned elsewhere | `skills get platform` selects from a `cfg!(target_os)` table inside `crates/core/src/commands/skills.rs` that has no Linux arm; a Linux skill needs a Linux name in core, which Phase 3 may not change. Section 3.13 scope now requires the Linux skill and the selection, and the Phase 3 introduction lists the core change. |
| F8 | fixed here | Bare `click @e1` with the bus down returned the bus error because the permission report failed before argument validation. With F1 the report succeeds, so the binary's preflight validates arguments first. Tests: the existing `command_policy::tests::invalid_ref_args_are_rejected_before_permission_preflight` (policy half) together with F1's denied-report tests (adapter half). Invert: the F1 invert (returning the error) restores the bus error ahead of validation; restored, the re-run below shows `INVALID_ARGS`. |
| F9 | owned elsewhere | Shared `SNAPSHOT_NOT_FOUND` text and `retryable` flag; listed in the Phase 3 introduction. The retry loop disappears when `snapshot` works (section 3.4). |
| F10 | owned elsewhere | Session naming, `session gc` reasons and the exporter's own `unpaired_command` warning are shared core behaviour; listed in the Phase 3 introduction. |
| F11 | owned elsewhere | The first-run guidance is the shared core skill; the missing `AGENT_DESKTOP_HOME` note is listed in the Phase 3 introduction. The absolute `state_root` in `status` is by design: it is the resolved directory a caller relocates with `AGENT_DESKTOP_HOME`. |
| F12 | accepted | `supported_surfaces: []` and `screen_recording: unknown` are true today: the adapter implements no surface and does not probe screen capture. Section 3.4 owns the surfaces; `screen_recording` stays `unknown` until the capture sub-phase can probe it. The empty list means the adapter implements no surface yet, which section 3.4 changes. |
| F13 | fixed here | `permissions --request` failed because core acquires an interaction lease first and `LinuxAdapter` left `acquire_interaction_lease` at its not-supported default. Linux has no permission prompt, so it now acquires the same Unix lease as macOS (`crates/linux/src/system/interaction_lease.rs`, including the inherited-descriptor path) and the request returns the current report. Tests: `the_adapter_grants_the_lease_a_permission_request_needs` and `an_inherited_descriptor_must_be_a_nonnegative_integer`. Invert: removing the override fails the first; restored, it passes. |

Both macOS and Windows implement the lease; neither has a prompt-less request path of its own beyond returning the report, so Linux matches them.

## Re-run after the fixes

Fresh release build; the live session still has no AT-SPI socket (it was not touched or repaired), and the private session has the bus up. `<tmp>` stands for a throwaway state root.

Bus down, live session (before: every row below except `list-apps` failed with `PLATFORM_NOT_SUPPORTED`, `retry: safe`):

| command | before | after |
|---------|--------|-------|
| `status` | exit 1, `PLATFORM_NOT_SUPPORTED` | exit 0, `ok:true` |
| `permissions` | exit 1, `PLATFORM_NOT_SUPPORTED` | exit 0, `ok:true` |
| `permissions --request` | exit 1, "acquire_interaction_lease is not supported" | exit 0, same report as `permissions` |
| `click @e1` | the bus error | `INVALID_ARGS` (documented bare-ref error) |
| `click @s8f3k2p9:e1` | the bus error | `PERM_DENIED` with the restart step |
| `wait 100` | the bus error | `PERM_DENIED` with the restart step |
| `batch` of one `wait` | the bus error | `PERM_DENIED` naming the entry, with the restart step |

After, `status` (live session, bus down):

```json
{"version":"2.4","ok":true,"command":"status","data":{"permissions":{"accessibility":{"state":"denied","suggestion":"The accessibility bus daemon's socket is missing or refusing connections. Restart it with 'systemctl --user restart at-spi-dbus-bus.service', or log out and back in."},"automation":{"state":"not_required"},"screen_recording":{"state":"unknown"}},"platform":"linux","ref_count":null,"session_id":null,"snapshot_id":null,"state_root":"<tmp>","supported_surfaces":[],"tracing":false,"version":"0.10.1"}}
```

After, `click @s8f3k2p9:e1` (live session, bus down):

```json
{"version":"2.4","ok":false,"command":"click","error":{"code":"PERM_DENIED","message":"Accessibility permission not granted","suggestion":"The accessibility bus daemon's socket is missing or refusing connections. Restart it with 'systemctl --user restart at-spi-dbus-bus.service', or log out and back in.","disposition":{"delivery":"not_delivered","retry":"safe"}}}
```

No session bus at all (`DBUS_SESSION_BUS_ADDRESS` pointing at a missing socket), `status`: `ok:true`, accessibility `denied`, suggestion "No D-Bus session bus is reachable. Run inside a desktop session, or start one with 'dbus-run-session -- <command>'.". `click @s8f3k2p9:e1` answers `PERM_DENIED` with the same suggestion.

Bus up, private session (unchanged by the fixes, confirmed in the other direction):

| command | result |
|---------|--------|
| `status` | `ok:true`, accessibility `granted` |
| `permissions` | `ok:true`, accessibility `granted` |
| `permissions --request` | `ok:true`, accessibility `granted` (before: unsupported lease error) |
| `click @e1` | `INVALID_ARGS` |
| `wait 100` | `ok:true`, `waited_ms: 100` |

Residual, stated plainly: with the bus down a gated command's `disposition.retry` is `safe`, because the shared preflight sets it for any `PERM_DENIED`; macOS shows the same for a denied permission. The recovery step in the suggestion is what tells the agent a blind retry will not help.
