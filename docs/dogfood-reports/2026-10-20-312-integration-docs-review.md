# Linux harness stranger documentation review

Scope: the operator received only the harness README and shipped core skill,
with product source and plans withheld. This is a documentation review, not a
live dogfood receipt. The shared box lock prevented the build/live commands
from starting. U13 remains incomplete until real private-session commands and
cleanup are observed.

| Finding | Disposition | Reviewable result |
| --- | --- | --- |
| The shared-host instructions name a wrapper and timeout but provide no complete invocation, fresh suite creation, or distinction between lock-wait and command timeout. | Fixed here | README gives separate build and single-leg calls, whole-second 720/60 budgets, mktemp under ~/ad-wt, binary/helper adjacency, automatic tool PATH and explicit Qt venv launch. |
| The KDE table looks like executable steps despite unresolved apps/refs, omitted executable prefix/headed policy and an X11-only active-window oracle. | Fixed here | README calls it an acceptance checklist owned by 3.15, states executable/headed/fingerprint requirements and separates X11 and Wayland oracles. The receiving roadmap requires concrete hardware/session setup and per-cell outcomes; no Plasma pass is claimed. |
| An operator cannot discover a tier's IDs/modes from the run examples. | Fixed here | README supplies an isolated Python inventory command and explains that a selected ID runs every registered mode. The command was executed locally and listed 136 Openbox IDs with modes/dispositions. |
| Negative-control, failure-reading and cancellation guidance omit the runnable selection, evidence fields and distinction between wrapper PID and runner PID. | Fixed here | README supplies a bounded negative-control payload, JSON report-reading guidance, same-repro/independent-read triage rules, and PID/start-token verification through the private harness-owner marker. Live cancellation/cleanup still requires measurement; the instructions themselves are not a passed cancellation test. |

Validation: the inventory command ran verbatim. Documentation diff checks
pass. The portable suites validate the existing registry/verdict/cleanup
contracts separately; they do not establish that a stranger completed a live
run, portal consent, KDE walk, or cleanup on the box.
