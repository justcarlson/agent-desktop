# Dogfood: the Linux role, state, name and identity vocabulary

- **Date:** 2026-10-18
- **Branch:** `feat/linux-3.3-vocabulary`, cut from and merging back into `feat/linux-adapter`.
- **Surface:** the internal vocabulary only (role, states, name evidence, identifiers, descriptors).
  This sub-phase has no CLI surface: the vocabulary is reached only through tests until the
  observation sub-phase wires `snapshot` to the walk. The stranger-operator rule (shipped skill and
  built binary only) therefore cannot apply here. The stranger dogfood of the vocabulary runs with
  the observation sub-phase: `docs/phases.md` §3.4 states that its dogfood re-reads the vocabulary
  through `snapshot` on Files, Calculator, a Qt 6 app and VS Code and judges role, state, name,
  `native_id`, DOM id, descriptors and placeholder against the AT-SPI tree. What ran instead is the
  `#[ignore]` live test `vocabulary_live_real_apps` against real applications, judged from the
  numbers it prints and from the checks that fail it, never from a test merely passing.
- **Capture safety:** counts, role names, state names and control labels only. No titles, paths,
  pids, host or user names.

## Environment

| Item | Value |
|---|---|
| OS | Ubuntu 24.04 LTS, x86_64, GNOME 46 |
| Display | Xvfb virtual display, no window manager |
| Session | private `dbus-run-session` with a private 0700 `XDG_RUNTIME_DIR`; the machine's own AT-SPI bus was never touched |
| Applications | GNOME Files, GNOME Terminal, GNOME Calculator, a Qt 6 window (the harness script in `crates/linux/src/tree/live_qt.rs`), VS Code (Electron, `--no-sandbox`, no extensions), Firefox |
| Harness | `vocabulary_live_real_apps` (`vocabulary_live_tests.rs`, `vocabulary_live_analysis.rs`), `--test-threads=1`, one fresh run through `scripts/remote-linux.sh` |

Each app is walked by the 3.2 walker, every node is resolved through the 3.3 vocabulary, and the
test fails on any unknown role, unknown states, unknown name, incomplete identifier set, failed
read, or incomplete walk.

## Judgements

A vocabulary run is judged good only when the walk is complete, every node resolves a role, states
and name with zero unknown, there are zero read errors, and the app-specific rule it exists to
prove was exercised by real nodes rather than satisfied vacuously.

| Id | Subject | Observed | Verdict |
|---|---|---|---|
| J1 | Files | 358 nodes, unknown role/states/name 0/0/0, 0 read errors, root resolves `window`. 258 label reads, 0 returned text, 70 ms. Disabled: `Back`, `Forward` (GTK4 sensitive-only nodes are not reported disabled). 5 `switch`, 3 `menubutton`. 0 blank-name nodes resolved from a `LabelledBy` target | pass |
| J2 | Terminal | 49 nodes, 0/0/0, 0 read errors. 14 label reads, 0 text, 3 ms. 5 disabled menu items (`Copy`, `Copy as HTML`, `Find Next`, `Find Previous`, `Clear Highlight`); the text area resolves `document` because GTK3 sets neither `editable` nor `read-only` on it (L3-29) | pass |
| J3 | Calculator | 98 nodes, 0/0/0, 0 read errors, root resolves `window`. 28 buttons: 27 enabled, 1 disabled (`Undo`), so the live check `enabled >= 20` and `disabled == {Undo}` held on real data. 39 label reads, 10 ms; one node with a `LabelledBy` target read the label `"4"` and kept its own `Name`. Display resolves `textfield` with `readonly`, 2 `switch` | pass |
| J4 | Qt window | 18 nodes, 0/0/0, 0 read errors. The button resolves its Qt `AccessibleId` ending `okButton` (kept as the dotted chain); the line edit resolves name `Type name`, placeholder `Type name`, description absent. 6 label reads, 0 text | pass |
| J5 | VS Code | 402 nodes, complete, 0/0/0, 0 read errors. DOM ids: 10 inside the web document, 0 outside it, so the web-document scoping rule had real nodes on both sides to be checked against. 2 `alert` nodes whose `xml-roles` agree. 4 disabled, 16 `haspopup`. 198 label reads, 0 text, 43 ms. Vocabulary pass over the 402 nodes 1414/1434/1437 us (min/median/max) | pass |
| J6 | Firefox | 947 nodes, 0/0/0, 0 read errors. 394 label reads, 2 returned text, 98 ms; 2 blank-name nodes resolved from a `LabelledBy` target. 3 disabled (`Back`, `Forward`, `Reload`), 28 `haspopup` | pass |
| J7 | Window role for roots | Files and Calculator top-level frames resolve `window`, the GTK4 frame whose role name is `application` included | pass |

The vocabulary pass is a small fraction of the walk it follows (VS Code: about 1.4 ms of vocabulary
against 321 to 340 ms of walk in 3.2), so no latency finding arises from the resolver.

The `switch` counts in J1, J3 and J5 are role 62 toggle buttons, which the table maps to `switch`
(plan KTD2); none of them is the GTK role 7 pair of F1.

## Findings

| Id | Finding | Disposition |
|---|---|---|
| F1 | A GTK4 switch is role 7 with role name `switch` on toolkit `GTK`, yet resolved `checkbox` while macOS and Windows emit `switch` for a switch control. | Fixed here. The pair resolves `switch`; role 7 on any other toolkit, a different role name on GTK, and role 43 named `switch` stay as they were. Test `a_gtk_switch_resolves_switch_and_every_other_check_box_stays_checkbox`. Inverted: the test fails when the rule is removed. |
| F2 | The Calculator disabled-set check was vacuous: it passed whether or not a disabled button existed. | Fixed here. The live check requires the disabled buttons to equal exactly `{Undo}` and at least 20 enabled. Inverted: with the requirement broken the run printed `[calculator] 28 enabled buttons, disabled buttons []` and failed. |
| F3 | A failed `Name` read could be overridden by a readable `LabelledBy` label, reporting a confident name for a node whose own name was never read. | Fixed here. A failed name read stays unknown. Test `a_failed_name_read_is_not_overridden_by_a_readable_label`. Inverted: fails when the override is allowed. |
| F4 | The lazy label path (`LabelledBy` read only when the own `Name` is blank) was unproved live on GTK, Qt and VS Code: no blank-name node had a label target there (0 in J1 to J5). | Fixed here by adding Firefox to the live run, where 2 blank-name nodes resolve from a `LabelledBy` target, and the run fails if none does. Not inverted against a real browser; the unit tests cover the slot logic and the live check covers the transport. |
| F5 | `offscreen` cannot be derived from `VISIBLE` without `SHOWING` (L3-27): VS Code has 12 of 402 such nodes and Firefox 886 of 947, 173 of them inside the window frame. | Owned elsewhere: §3.6 derives `offscreen` from extents against the window frame and scroll container, treating the INT32_MIN sentinel as unknown bounds. Written in `docs/phases.md` §3.3 and §3.6. 3.3 emits no `offscreen`. |
| F6 | GTK4 popover menu items have blank `Name` and `Description` and nothing readable in their subtree (L3-30), so their name resolves absent. | Owned elsewhere: §3.4 owns it (addressable by position and action count; its dogfood states that for the agent), written in `docs/phases.md` §3.4. |
| F7 | No GTK role 7 named `switch` was observed on the box in this run, so the F1 rule is exercised by unit tests only; the live `switch` nodes are role 62. | Accepted: no installed GTK app on the reference box produces the pair, and building a fixture for it would test the fixture. The unit test pins every neighbouring case. |
| F8 | The Qt class-name-tail heuristic (a trailing `QPushButton`-style segment is non-identifying) can drop an explicit `objectName` that happens to look like a class name. | Accepted: bounded by L3-9, which measured the real shapes (full dotted `objectName` chain kept; class tails and list-row duplicates are the non-identifying cases). Row-level duplicates are owned by §3.5 and §3.4. |
| F9 | The vocabulary has no CLI surface, so no stranger run of the shipped skill was possible. | Owned elsewhere: §3.4's stranger dogfood re-reads the vocabulary through `snapshot` (the bullet beginning "Vocabulary inputs the walk supplies" in `docs/phases.md` §3.4). |

## Not covered

The 86 AT-SPI role numbers no app here produced are proved by the totality test only (live apps
produced 44 of 130), and 15 of the 44 state bits are synthetic-only (L3-1, L3-5). Off-English role
names are covered by the L3-26 measurement, not by this run. Wayland was not run. The Qt window in
J4 is a single small form, so Qt coverage is the identity and placeholder rules, not breadth.
