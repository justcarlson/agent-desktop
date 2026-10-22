# Bus and Permissions on Linux

Observation and semantic actions need no permission dialog. What gates them is
whether the AT-SPI accessibility bus answers.

## Find the bus

The CLI connects to the session bus, calls `GetAddress` on `org.a11y.Bus` (this
starts the bus launcher), then reads `ChildCount` on the AT-SPI registry root.
Later commands reuse that connection and reconnect after a connection-level
failure.

Judge health with `status`, never with `busctl --user list`: on a cold session
`org.a11y.Bus` has no owner until the first `GetAddress`.

```bash
agent-desktop status
agent-desktop permissions
```

| State | `status` shows | Meaning |
|-------|----------------|---------|
| Healthy | `accessibility: granted`, `supported_surfaces` listed, `list-apps` rows | Normal |
| Empty | `accessibility: granted`, no `list-apps` rows | No app registered. Not a fault: start an app, or restart one that started before the bus |
| Unavailable | `accessibility: denied` with a suggestion, empty `supported_surfaces`; commands fail `PLATFORM_NOT_SUPPORTED` | The session bus or accessibility bus cannot be reached |

An app that started before the accessibility bus never registers on a later one:
restart it. A false `toolkit-accessibility` GSettings key does not stop apps from
working, and `IsEnabled` is not a diagnostic.

On a hand-started session the registry prints `SpiRegistry daemon is running`
on the terminal of the process that started the bus. That is not CLI output: on
success the CLI writes one JSON envelope to stdout and nothing to stderr.

| `permissions` field | Linux value |
|---------------------|-------------|
| `accessibility` | `granted` when the bus answered, else `denied` with a cause-specific suggestion |
| `screen_recording` | `not_required` on X11; `unknown` on Wayland (only a screenshot request shows the answer) |
| `automation` | `not_required` |

`permissions --request` has nothing to request.

## Unavailable bus

Each cause returns `PLATFORM_NOT_SUPPORTED`, `details.retryable: false`, a
suggestion naming the next step, and the raw D-Bus error in `platform_detail`.

| Cause | Suggestion | Meaning |
|-------|------------|---------|
| No session bus | Run inside a desktop session, or start one with `dbus-run-session -- <command>` | `DBUS_SESSION_BUS_ADDRESS` and the runtime socket are missing |
| AT-SPI not installed | Install `at-spi2-core`; Flatpak: `--talk-name=org.a11y.Bus`; snap: connect `desktop-legacy` or `unity7` | `GetAddress` answered `ServiceUnknown` or `NameHasNoOwner` |
| Launcher cannot start | Read `systemctl --user status at-spi-dbus-bus.service` | The launcher failed to spawn |
| Socket gone | Restart `at-spi-dbus-bus.service`, or log out and back in | The accessibility bus socket is missing or refuses connections |

A timeout is `TIMEOUT` and proves nothing about the bus. A bus that dies
mid-session returns `INTERNAL` with a retry suggestion, and stays `INTERNAL`
until the accessibility service runs again: an established adapter does not
start a replacement bus, a new process does. The loss does not show whether an
action was delivered. Retry an action only when its disposition says retry is
safe.

Do not run the restart suggestion where another process owns the bus. On a
shared host a second launcher that shares the runtime directory can delete the
live bus socket and break every other client. Use a private session; `mktemp -d`
makes the runtime directory mode 0700:

```bash
XDG_RUNTIME_DIR=$(mktemp -d) dbus-run-session -- agent-desktop status
```

## Session requirements

- Run the CLI as the user who owns the session. A process of another user or
  session is not matched by name.
- The target app must run on the same session bus. A process whose environment
  names another bus counts as another session's.
- A sandboxed app (Flatpak, snap) is on the bus only when the sandbox shares it:
  `references/shell-tray-and-notifications.md`.

## State root

All persisted state lives under `~/.agent-desktop`, or under `AGENT_DESKTOP_HOME`
when set (the value is the root itself). Set it to move the root, for example
into a private session. A root the tool creates has mode 0700.

The root must be private to the user. A group-writable or open root makes
commands that write there (`snapshot` for one) fail with `INTERNAL`, "private
file parent is accessible by group or other users". `status` still runs. Fix an
existing root:

```bash
chmod 700 "${AGENT_DESKTOP_HOME:-$HOME/.agent-desktop}"
```

Do not pre-create it with `mkdir -p` under a umask of 002.

The interaction lock lives under `/tmp/agent-desktop-<uid>/`, outside the state
root. It serializes input per user. `TIMEOUT` with `lock_timeout` and a high
`contention_count` means another agent of the same user is driving input.
