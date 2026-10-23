# jev scripts

Two entry points that drive a desktop app from plain language, using the TypeSafe
Jev model to pick elements. `run.mjs` takes a goal and drives until it is met.
`act.mjs` takes one step and hands the result back to you. Neither puts the
accessibility tree in the caller's context.

**Data leaves the machine.** Every model call posts a description of the screen to
the TypeSafe hosted API (`https://api.typesafe.ai/v1/systemone`, or whatever
`TYPESAFE_BASE_URL` points at) and needs `TYPESAFE_API_KEY`. The script exits with
`TYPESAFE_API_KEY unset` otherwise. See "What leaves the machine" before pointing a
run at anything confidential.

They call the `agent-desktop` binary, so it must be built or on `PATH`. The
clipboard-paste fallback for text entry presses `cmd+v`, so that fallback is
macOS only.

## Environment

| Variable | Meaning |
| --- | --- |
| `TYPESAFE_API_KEY` | Required. Sent as a bearer token. |
| `TYPESAFE_BASE_URL` | Decision endpoint. Default `https://api.typesafe.ai/v1/systemone`. |
| `TYPESAFE_MODEL` | Model name. Default `jev-latest`. |
| `AGENT_DESKTOP_BIN` | Binary used by `run.mjs`. Default `target/release/agent-desktop` in this repo, then `agent-desktop` on `PATH`. |
| `JEV_VERIFY_MS` | How long `run.mjs` re-reads a field after a paste before declaring the text missing. Default 2000, range 0 to 60000. |

## run.mjs: a goal, driven to the end

```sh
node scripts/jev/run.mjs --app Finder "open the Applications folder"
node scripts/jev/run.mjs --app TextEdit --cursor --text "A line to write." "write the supplied sentence into the document"
```

Each turn reads the screen, asks for one operation and a target for it in the same
request, carries it out, and reads again. It prints one JSON line per turn and one
when it stops (abridged here; real lines also carry `ref`, `steps`, `post_state`,
`history` and more):

```json
{"turn":{"step":1,"operation":"CLICK","target":"treeitem \"Applications\"","confidence":1,"ok":true,"delivery":"delivered_verified","changed":true}}
{"stop":"done","confidence":0.96}
```

| Flag | Meaning |
| --- | --- |
| `--app <name>` | Required. |
| `--window-id <id>` | Read one window, by an id from `agent-desktop list-windows --app <name>`. For apps with several windows where none is focused. The run stays on that window; if it closes, the run stops with `window_closed`. |
| `--root @ref` | Start inside one region instead of the whole window. |
| `--text "..."` | A value for `TYPE_TEXT`. Repeat once per value; they are consumed in order. |
| `--cursor` | Start a session and show a cursor that travels to each element before the operation lands. Bring the app in front of your terminal or the cursor is hidden behind it. |
| `--no-values` | Withhold the values read from the snapshot's fields. Text you supplied with `--text` is not covered: after a `TYPE_TEXT` that string is kept in the turn history and sent in `recent_actions`. |
| `--confirm-destructive` | Stop for confirmation on every step rated destructive, whatever the confidence. |

The operations are `CLICK`, `TYPE_TEXT`, `CHECK`, `UNCHECK`, `EXPAND`, `COLLAPSE`,
`SCROLL`, `DRILL`, `WIDEN`, `WAIT`, `DONE` and `BLOCKED`. An operation is offered
only when some element on screen advertises the matching action.

- **One target head per operation.** `click_target` is chosen from elements that
  advertise `Click`, `type_text_target` from those that advertise `SetValue`, and
  so on. Only the head for the chosen operation is read, so a target is never
  incompatible with its verb.
- **Looking is an operation.** The window is read with `--skeleton` first. A region
  that was cut off reports how many children it holds; `DRILL` pins it as the root
  for later turns and `WIDEN` returns to the whole window.
- **`CHECK` and `UNCHECK` are idempotent**, so a box already in the wanted state is
  left alone.
- **You supply the text.** A run never types a string you did not pass. When no
  `--text` value is left, `TYPE_TEXT` is not offered at all.
- **Text entry.** A direct `set-value` is tried first. Only when it reports that
  nothing was delivered and a retry is safe does the text go through the clipboard
  and a paste. The clipboard is read only when a paste is about to need it and put
  back when the run ends, but only if it held text: if it held an image, files or
  anything unreadable, it is cleared instead and the original is lost. After a paste
  the field is read back, and a field that does not hold the exact text stops the run with
  `TEXT_VERIFICATION_FAILED` without typing again. A field that normalizes text
  (trims, changes case or line endings) therefore reports a failure even when the
  text landed.
- **Stops.** `done` and `blocked`; a confidence too low for the risk (the stop names
  the candidate it would have acted on); 40 actions; 80 model calls; three turns in
  a row that changed nothing; `action_failed` on the first failed operation, which
  is never repeated; `window_closed`, `surface_outside_window` and
  `root_outside_window` when `--window-id` is set. The exit code is 0 only for
  `done`.
- **`WAIT`** pauses 250 ms before the screen is read again.
- The model call is retried up to twice, with backoff, on HTTP 429, 503 and 529.

### What leaves the machine

Each turn posts every offered element's role, accessible name or description, up to
sixty characters of the value it holds, its state, the window title, and the recent
actions. That is enough to send the contents of a private document or a filled
form. `--no-values` removes the values read from the snapshot; targeting gets
harder, because a value is often the only thing that tells two unnamed rows apart.
Names, descriptions and titles are still sent. It does not cover text you supply:
after a `TYPE_TEXT` the exact string is stored in the turn history, and every later
decision request carries it in `recent_actions` while that turn is among the last
eight, with or without the flag. Do not pass a secret to `--text` unless you accept
it leaving the machine.

### Risk sets the bar

An ordinary step needs 0.70 confidence in its target. One rated destructive (a
score of 0.50 or more on a separate "hard to undo" question) needs 0.90. Below 0.55
nothing runs. The destructive question is asked in its own request, only when the
target confidence is at least 0.70 and either below 0.90 or `--confirm-destructive`
is set.

## act.mjs: one step at a time

You keep the goal, the plan and the memory. You send one sentence and get back one
small object.

```sh
node scripts/jev/act.mjs --app TextEdit "the button that saves the document"
node scripts/jev/act.mjs --app TextEdit --execute --text "Morning over the dock." "type this text into the main writing area"
```

| Flag | Meaning |
| --- | --- |
| `--app <name>` | Required. |
| `--window-id <id>` | As for `run.mjs`. |
| `--root @ref` | Resolve inside one container instead of the whole window. |
| `--text "..."` | Text the intent needs. The model returns choices, never strings. |
| `--execute` | Run the command when `decision` is `act`. Without it nothing runs. |
| `--confirm-destructive` | Return `confirm` for any step rated destructive. |
| `--bin <path>` | agent-desktop binary. Default `target/release/agent-desktop` in this repo, then `PATH`. (`act.mjs` does not read `AGENT_DESKTOP_BIN`.) |

The output is a single JSON object with `ok`, `intent`, `app`, `window`,
`window_id`, `surface`, `element` (`ref`, `role`, `name`, `where`), `command`,
`argv`, `decision`, `why`, `confidence` (`target`, `command`), `gates` (`present`,
`destructive`, `needs_text`), `runner_up`, `notes` and, with `--execute`,
`executed`. The exit code is 1 when `--execute` was given and the command did not run or failed.

Phrase the intent as an action, not an element: `"type this text into the main
writing area"` resolves to `type`, while `"the main writing area"` names no
operation.

| `decision` | Meaning | What to do |
| --- | --- | --- |
| `act` | One element matches and the confidence clears the bar for the risk. | Nothing; with `--execute` it already ran. |
| `confirm` | Plausible but under the bar, or hard to undo with `--confirm-destructive`. | Ask the user, or re-phrase and call again. |
| `abstain` | The model answered `none`, the element is probably not on this screen, or two elements fit equally. | Open the surface you need, then call again. |
| `needs_text` | The command takes text and none was supplied. | Call again with `--text`. |

What code decides, not the model:

- The same bars as above: 0.70, 0.90 for a destructive rating of 0.50 or more,
  nothing below 0.55.
- One request carries five questions, answered in parallel: `target`, `command`
  (one of 16 verbs), `present`, `destructive` and `needs_text`. The chosen verb is
  then checked against the element's advertised actions: a readonly combobox gets
  `click`, never `set-value`, and `--text` with a verb that takes no text corrects
  the verb to `set-value` or `type`.
- When a sheet, alert, menu or popover is open, that surface is read instead of the
  window.
- Disabled and hidden elements, and elements that advertise no action, are never
  offered.
- A Choice accepts 255 options, so up to 254 elements go in one pass. A first pass
  under 0.70 re-asks over its top five. A screen with more than 254 elements says
  so in `notes`; use `--root @ref`.
- A failed model call is not retried.

## Deciding through OpenRouter

Point `TYPESAFE_BASE_URL` at OpenRouter's Decisions router to use an OpenRouter key:

```sh
TYPESAFE_BASE_URL=https://openrouter.ai/api/alpha/decisions \
TYPESAFE_MODEL=typesafe/jev-1.13 \
TYPESAFE_API_KEY=sk-or-... \
node scripts/jev/run.mjs --app Finder "open the Applications folder"
```

A bare `TYPESAFE_MODEL` such as `jev-latest` is scoped to `~typesafe/jev-latest`
automatically when the URL contains `openrouter.ai`. A pinned `typesafe/...` id is
sent as given, and `typesafe/jev-latest` is rejected by OpenRouter. The OpenRouter
endpoint is alpha and may change.

## Limits

- A full-window snapshot takes about 3 s on a dense app; `--root` is far cheaper.
  `act.mjs` takes one snapshot, plus a second of the open surface when a sheet,
  alert, menu or popover is up. `run.mjs` reads the screen before each action and
  again after it, with the same extra surface read, so a run costs several
  snapshots per turn.
- Held input is unavailable, so no sustained key and no drag with hold.
- `press` is not resolved here. It needs no element, so call it directly.

## Files

| File | Holds |
| --- | --- |
| `policy.mjs` | The operations, how a screen is described, the bars, and when to stop. Touches no application. |
| `screen.mjs` | Which elements exist, which a command can reach, and how each is described. Shared by both entry points. |
| `desktop.mjs` | The code that speaks to agent-desktop for `run.mjs`: observe, execute, text entry, cursor. |
| `run.mjs` | The loop and its command line. |
| `act.mjs` | The single-step resolver. |

## Tests

```sh
node scripts/jev/act.test.mjs
node scripts/jev/run.test.mjs
node scripts/jev/run.integration.test.mjs
node scripts/jev/evidence.test.mjs
```

`run.integration.test.mjs` drives `run.mjs` against a fake binary and model, so none
of them needs the real app or an API key.

Turns keep native `steps`, `post_state` and `details` for the caller; these
evidence fields are excluded from the history sent to the model. A verified
`AlreadyInState` step with `outcome: "skipped"` and `delivery: "not_delivered"`
means no input was needed.
