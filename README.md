<h1 align="center">agent-desktop</h1>

<p align="center">Computer use for AI agents: read and operate desktop apps through the OS accessibility tree.</p>

<p align="center">
  <a href="https://github.com/lahfir/agent-desktop/actions/workflows/ci.yml?query=branch%3Amain"><img src="https://img.shields.io/github/actions/workflow/status/lahfir/agent-desktop/ci.yml?branch=main&style=for-the-badge" alt="CI status"></a>
  <a href="https://github.com/lahfir/agent-desktop/releases"><img src="https://img.shields.io/github/v/release/lahfir/agent-desktop?include_prereleases&style=for-the-badge" alt="GitHub release"></a>
  <a href="https://www.npmjs.com/package/agent-desktop"><img src="https://img.shields.io/npm/v/agent-desktop?label=npm&style=for-the-badge" alt="npm version"></a>
  <a href="https://clawhub.ai/lahfir/agent-desktop"><img src="https://img.shields.io/badge/ClawHub-skill-f97316?style=for-the-badge" alt="ClawHub skill"></a>
  <a href="https://skills.sh/lahfir/agent-desktop/agent-desktop"><img src="https://img.shields.io/badge/skills.sh-listed-8b5cf6?style=for-the-badge" alt="skills.sh listing"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-Apache--2.0-blue.svg?style=for-the-badge" alt="Apache-2.0 License"></a>
</p>

<p align="center">
  <video src="https://github.com/user-attachments/assets/9b2c9f8c-a49d-4b69-b6cf-11d9e0d40ceb" controls width="800"></video>
</p>

agent-desktop is a CLI and C library for macOS, Windows and Linux. An agent takes a snapshot of an app and gets JSON
with a ref for every element it can act on. It acts on a ref, then snapshots again to check the result. There is no
pixel matching, and the observe-act loop stays in your agent.

## Installation

```bash
npm install -g agent-desktop
agent-desktop status            # platform, permissions, supported surfaces
```

The package works on macOS, Windows and Linux, on x64 and ARM64. On macOS, grant Accessibility permission next; see
[Permissions](#permissions).

**Blocked install scripts.** The package's `postinstall` script downloads the binary from the GitHub release and
checks its SHA-256. A package manager that blocks install scripts leaves no binary, and the first run fails with
binary-not-found. npm 11 runs the script but warns that it is not covered by `allowScripts`. On Linux the script also
needs `curl`, `tar` and `gzip`. To allow it:

```bash
npm install -g --allow-scripts=agent-desktop agent-desktop
npm config set allow-scripts=agent-desktop --location=user
pnpm add -g --allow-build=agent-desktop agent-desktop
bun add -g --trust agent-desktop
```

**Release archives** are on the [releases page](https://github.com/lahfir/agent-desktop/releases). Their checksums
come from the same release, so they catch corruption but do not prove origin. Check origin with:

```bash
gh attestation verify <downloaded-archive> --repo lahfir/agent-desktop
```

**From source**, you need Rust 1.89 or later, plus the Xcode command-line tools on macOS 13+, the MSVC build tools on
Windows, or a C linker on Linux:

```bash
git clone https://github.com/lahfir/agent-desktop && cd agent-desktop
cargo build --release -p agent-desktop
```

On Linux, also run `cargo build --release -p agent-desktop-linux --bins` and keep
`agent-desktop-linux-clipboard-owner` beside the CLI. npm and the release archives include it.

## Quick start

Observe, act on a ref, observe again. The example uses Finder on macOS; any name from `agent-desktop list-apps` works.

```bash
agent-desktop snapshot --app Finder -i      # interactive elements, each with a ref
agent-desktop click @s8f3k2p9:e8            # act on a ref from that snapshot
agent-desktop snapshot --app Finder -i      # observe again to verify
```

A snapshot prints one JSON envelope. Trimmed:

```json
{
  "version": "2.5",
  "ok": true,
  "command": "snapshot",
  "data": {
    "app": "Finder",
    "window": { "id": "w-204749", "title": "Downloads" },
    "snapshot_id": "s8f3k2p9",
    "ref_count": 59,
    "complete": true,
    "tree": {
      "role": "window",
      "name": "Downloads",
      "children": [
        { "role": "treeitem", "name": "Documents", "ref_id": "@s8f3k2p9:e8", "available_actions": ["Click"] }
      ]
    }
  }
}
```

- **A ref belongs to the snapshot that printed it.** A bare `@e8` is rejected with `INVALID_ARGS`. When the UI
  changes, an action returns `STALE_REF` or `AMBIGUOUS_TARGET`: take a new snapshot and use its refs.
- **One window at a time.** If the app has several windows, `snapshot` returns `AMBIGUOUS_TARGET` and lists the
  candidate window ids in `error.details`. Pass one with `--window-id`.
- **Dense apps.** Map first, then drill into one region:
  `snapshot --skeleton --app Slack -i --compact`, then `snapshot --root @s8f3k2p9:e3 -i --compact`.
- **Headless by default.** Ref actions go through the accessibility API without moving the pointer or taking focus.
  The global `--headed` flag allows physical input.
- **PowerShell.** Quote every ref (`'@s8f3k2p9:e8'`), because a bare `@token` is the splatting operator.

## For AI agents

The binary carries its own docs, matched to its version:

```bash
agent-desktop skills get desktop      # the loop, refs, verification and retry rules
agent-desktop skills get platform     # what differs on the OS you run on
agent-desktop <command> --help        # flags for one command
```

Install the skill into an agent harness from [skills.sh](https://skills.sh/lahfir/agent-desktop/agent-desktop):

```bash
npx skills add https://github.com/lahfir/agent-desktop --skill agent-desktop
```

Or add this to your `AGENTS.md` or `CLAUDE.md`:

```markdown
Use `agent-desktop` to see and operate desktop apps. Before the first use, run
`agent-desktop skills get desktop` and `agent-desktop skills get platform`, and follow them.
```

Every command prints one envelope with `ok` and either `data` or `error`. An error carries a `code`, a `disposition`
and usually a `suggestion`; retry only when `error.disposition.retry` is `safe`. The full contract, error codes and exit codes
are in [docs/json-output.md](docs/json-output.md).

## Commands

It ships 60 command names, 56 operational commands. The other four (`key-down`, `key-up`, `mouse-down`, `mouse-up`)
refuse to run until a daemon can release a held key or button.

| Group | Commands |
|---|---|
| Observe | `snapshot`, `find`, `get`, `is`, `screenshot`, `list-surfaces` |
| Act on a ref | `click`, `double-click`, `triple-click`, `right-click`, `type`, `set-value`, `clear`, `focus`, `select`, `toggle`, `check`, `uncheck`, `expand`, `collapse`, `scroll`, `scroll-to` |
| Keyboard and mouse | `press`, `hover`, `drag`, `mouse-move`, `mouse-click`, `mouse-wheel`, `key-down`, `key-up`, `mouse-down`, `mouse-up` |
| Apps and windows | `launch`, `close-app`, `list-apps`, `list-windows`, `list-displays`, `focus-window`, `resize-window`, `move-window`, `minimize`, `maximize`, `restore`, `open-system-surface` |
| Notifications | `list-notifications`, `dismiss-notification`, `dismiss-all-notifications`, `notification-action` |
| Clipboard | `clipboard-get`, `clipboard-set`, `clipboard-clear` |
| Wait | `wait` |
| Sessions | `session`, `trace`, `cursor-overlay` |
| System | `status`, `permissions`, `version`, `batch`, `skills` |

## Platform support

| | macOS | Windows | Linux |
|---|:---:|:---:|:---:|
| Accessibility tree | **Yes** | Yes\* | Yes (AT-SPI) |
| Click / type / keyboard | **Yes** | **Yes** | AT-SPI; X11 / GNOME keys |
| Mouse input | **Yes** | **Yes** | X11, `--headed` |
| Screenshot | **Yes** | **Yes** | X11 windows; Wayland portal |
| Clipboard | **Yes** | **Yes** | Yes; Wayland needs `wl-clipboard` |
| App & window management | **Yes** | Yes\*\* | Launch/list; X11 window control |
| Notifications | **Yes** | **Yes** | GNOME Shell |
| Shell surfaces (`open-system-surface`) | Planned | **Yes** | GNOME tray, Action Center |

\* On Windows, `list-surfaces` lists each process's `window`, `focused`, `sheet` and `menu` surfaces. `snapshot
--surface` also reads `taskbar`, `system-tray`, `system-tray-overflow`, `start-menu` and `action-center` without
`--app`. `quick-settings` returns `PLATFORM_NOT_SUPPORTED` and names `action-center`, which carries those controls.
\*\* `launch` on Windows takes an absolute path, or a bare name under System32 or the Windows directory, with or
without `.exe`. It cannot resolve a display name such as "Google Chrome".

On Linux, semantic actions work on X11 and Wayland. On Wayland, app pointer input refuses and native window control
is unsupported. `agent-desktop skills get platform` prints the full capability table for the OS you run on.

## Permissions

**macOS** needs Accessibility permission. Screenshots also need Screen Recording, and opening Notification Center
needs Automation permission for System Events. `agent-desktop permissions` reports the state without prompting,
`--request` asks for what is missing, and `--guide` opens System Settings at the missing permission with an on-screen
guide.

**Windows** needs no grant for apps at your terminal's integrity level. An app that runs as administrator can be read,
but only an elevated agent can send it input. The binary is unsigned, so SmartScreen may warn on a browser download,
antivirus may quarantine it, and Smart App Control on a clean Windows 11 install blocks it. The npm install path does
not trigger SmartScreen.

**Linux** supports Ubuntu 24.04+ with GNOME 46 as the primary target and a glibc 2.35 floor; Ubuntu 22.04's ATK bridge
aborts on `Action.GetActions`. Run inside the user's graphical session with `at-spi2-core` installed. A TTY or SSH
shell without that session's bus, display and runtime environment cannot see its apps. `agent-desktop status`
activates and checks the accessibility bus. X11 keys need XTEST,
Wayland keys need GNOME Mutter RemoteDesktop, and the Wayland clipboard needs `wl-clipboard`. The Linux skill covers
the rest: `agent-desktop skills get linux`.

## Language bindings

Each release also ships `libagent_desktop_ffi` for macOS, Linux and Windows, so a program can call the tool in
process instead of starting the CLI for each command:

```python
import ctypes
lib = ctypes.CDLL("./lib/libagent_desktop_ffi.dylib")
lib.ad_adapter_create.restype = ctypes.c_void_p
lib.ad_adapter_destroy.argtypes = [ctypes.c_void_p]
assert lib.ad_init(6) == 0  # AD_RESULT_OK when the header's AD_ABI_VERSION_MAJOR matches the library
adapter = lib.ad_adapter_create()
lib.ad_adapter_destroy(adapter)
```

`agent-desktop skills get ffi` covers the entry points, memory ownership and threading rules.

## Development

Each platform adapter builds only on its own OS, so scope cargo to the host's packages. On macOS:

```bash
HOST_PKGS=(-p agent-desktop-core -p agent-desktop-macos -p agent-desktop -p agent-desktop-ffi)
cargo test "${HOST_PKGS[@]}"
cargo clippy "${HOST_PKGS[@]}" --all-targets -- -D warnings
```

On Windows, replace `agent-desktop-macos` in that list with `agent-desktop-windows`; on Linux, with
`agent-desktop-linux`.
[CONTRIBUTING.md](CONTRIBUTING.md) lists every gate, [CONCEPTS.md](CONCEPTS.md) defines the vocabulary, and
[docs/faq.md](docs/faq.md) answers common questions.

## License

Apache-2.0
