<h1 align="center">agent-desktop</h1>

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

agent-desktop is a command-line tool and C library that lets an AI agent read and operate desktop apps on macOS and Windows through the operating system's accessibility tree. It prints JSON. Each element the agent can act on gets a ref such as `@s8f3k2p9:e3`, which names the snapshot it came from. Linux is planned.

It ships 60 command names, 56 operational commands. The other four (`key-down`, `key-up`, `mouse-down`, `mouse-up`) refuse to run until a daemon can release a held key or button.

On macOS, CLI calls reuse a private command host for each snapshot namespace and binary build. It retains native accessibility objects so a removed row cannot silently resolve to a surviving row with the same identifier. The host exits after 15 idle minutes. `TMPDIR` must be owned by the current user with mode `0700`; the standard macOS per-user temporary directory satisfies this requirement. Headless calls still prohibit foreground activation and synthesized input.

## Installation

```bash
npm install -g agent-desktop
```

This works on macOS (ARM64, x64) and Windows (x64, ARM64). The npm package includes all four released CLI binaries and the matching macOS helpers. Installation works with lifecycle scripts disabled, without a separate GitHub download. Release packaging verifies each archive against the release's `checksums.txt` before bundling it.

When postinstall runs on macOS, it links the command to the native binary to avoid Node startup. When scripts are disabled, the JavaScript launcher selects the bundled binary instead. On Windows, postinstall optimizes owned npm/Yarn command shims to call the native executable directly. Blocked scripts use the JavaScript launcher, and pnpm may retain its own shim. FFI libraries remain separate GitHub release assets; Linux CLI binaries are not released yet.

Release archives are also on the [releases page](https://github.com/lahfir/agent-desktop/releases). The checksums come from the same release as the archives, so they catch corruption but do not prove origin. Check origin with:

```bash
gh attestation verify <downloaded-archive> --repo lahfir/agent-desktop
```

To build from source you need Rust 1.89 or later, plus the Xcode command-line tools on macOS 13+ or the MSVC build tools on Windows:

```bash
git clone https://github.com/lahfir/agent-desktop && cd agent-desktop
cargo build --release -p agent-desktop
```

### Permissions

macOS needs Accessibility permission. Screenshots also need Screen Recording, and opening Notification Center needs Automation permission for System Events. `agent-desktop permissions` reports the state without prompting; `agent-desktop permissions --request` asks for what is missing; `agent-desktop permissions --guide` opens System Settings at the missing permission with an on-screen guide that shows what to drag.

Windows needs no grant for apps at your terminal's integrity level. An app running as administrator can be read, but only an elevated agent can send it input. The binary is unsigned, so SmartScreen may warn on a browser download, antivirus may quarantine it, and Smart App Control on a clean Windows 11 install blocks it. The npm install path does not trigger SmartScreen.

## Language bindings

Each release also ships `libagent_desktop_ffi` for macOS, Linux and Windows, so a program can call the tool in process instead of starting the CLI for each command.

```python
import ctypes
lib = ctypes.CDLL("./lib/libagent_desktop_ffi.dylib")
lib.ad_init(5)  # checks the header's AD_ABI_VERSION_MAJOR against the library
adapter = lib.ad_adapter_create()
lib.ad_adapter_destroy(adapter)
```

See [`skills/agent-desktop-ffi/`](skills/agent-desktop-ffi/) for the entry points, memory ownership and threading rules.

## Usage

Take a snapshot, act on a ref from it, then snapshot again to check the result:

```bash
agent-desktop snapshot --app Finder -i            # interactive elements with refs
agent-desktop click @s8f3k2p9:e3
agent-desktop type @s8f3k2p9:e5 "quarterly report"
agent-desktop press cmd+s
agent-desktop snapshot --app Finder -i
```

For a dense app, start with a shallow map and expand only the part you need. On Slack this cut one snapshot from 30,743 tokens to 383:

```bash
agent-desktop snapshot --skeleton --app Slack -i --compact
agent-desktop snapshot --root @s8f3k2p9:e3 -i --compact
```

A ref belongs to the snapshot that printed it. A bare `@e3` is rejected with `INVALID_ARGS`. If the UI changes, an action returns `STALE_REF` or `AMBIGUOUS_TARGET`; take a new snapshot and use its refs.

Ref actions run headless by default, through the accessibility API, without moving the pointer or taking focus. `--headed` allows physical input. In PowerShell, quote every ref (`'@s8f3k2p9:e1'`), because a bare `@token` is the splatting operator and the argument disappears.

### Commands

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

`agent-desktop <command> --help` lists a command's flags. The JSON envelope, error codes and exit codes are in [docs/json-output.md](docs/json-output.md).

The skills are bundled in the binary and match its version. `agent-desktop skills get desktop` prints the core guide, and `agent-desktop skills get platform` prints the guide for the OS you run on. They cover sessions and traces, the agent cursor overlay, multi-agent sessions and Chromium apps through `launch --cdp`.

## Platform support

| | macOS | Windows | Linux |
|---|:---:|:---:|:---:|
| Accessibility tree | **Yes** | Yes\* | Planned |
| Click / type / keyboard | **Yes** | **Yes** | Planned |
| Mouse input | **Yes** | **Yes** | Planned |
| Screenshot | **Yes** | **Yes** | Planned |
| Clipboard | **Yes** | **Yes** | Planned |
| App & window management | **Yes** | Yes\*\* | Planned |
| Notifications | **Yes** | **Yes** | Planned |
| Shell surfaces (`open-system-surface`) | Planned | **Yes** | Planned |

\* On Windows, `list-surfaces` lists each process's `window`, `focused`, `sheet` and `menu` surfaces. `snapshot --surface` also reads the shell surfaces `taskbar`, `system-tray`, `system-tray-overflow`, `start-menu` and `action-center` without `--app`. `quick-settings` returns `PLATFORM_NOT_SUPPORTED` on every Windows build and names `action-center`, which carries those controls.
\*\* `launch` on Windows takes an absolute path, or a bare name under System32 or the Windows directory, with or without `.exe`. It cannot resolve a display name such as "Google Chrome".

## Development

Each platform adapter builds only on its own OS, so scope cargo to the host's packages. On macOS:

```bash
HOST_PKGS="-p agent-desktop-core -p agent-desktop-macos -p agent-desktop-linux -p agent-desktop -p agent-desktop-ffi"
cargo test $HOST_PKGS
cargo clippy $HOST_PKGS --all-targets -- -D warnings
```

On Windows the set is `-p agent-desktop-core -p agent-desktop-windows -p agent-desktop -p agent-desktop-ffi`. [CONTRIBUTING.md](CONTRIBUTING.md) lists every gate, and [docs/faq.md](docs/faq.md) answers common questions.

## License

Apache-2.0
