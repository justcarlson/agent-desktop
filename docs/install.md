# Installing agent-desktop

Native packages are the default on macOS, Windows and Linux, on x64 and ARM64. They install the released
executable without a JavaScript launcher or an agent-desktop postinstall script.

## macOS and Linux: Homebrew

With [Homebrew](https://brew.sh) installed:

```bash
brew tap lahfir/agent-desktop https://github.com/lahfir/agent-desktop.git
brew install lahfir/agent-desktop/agent-desktop
agent-desktop version
```

Homebrew creates a symlink to the native executable and installs its platform helper beside it. Upgrade with
`brew update && brew upgrade lahfir/agent-desktop/agent-desktop`; uninstall with
`brew uninstall lahfir/agent-desktop/agent-desktop`.

## Windows: Scoop

With [Scoop](https://scoop.sh) installed, run in a non-administrator PowerShell:

```powershell
scoop bucket add agent-desktop https://github.com/lahfir/agent-desktop.git
scoop install agent-desktop/agent-desktop
agent-desktop version
(Get-Command agent-desktop).Source
```

The command resolves to `apps\agent-desktop\current\agent-desktop.exe`. Scoop manages the `current` directory
junction and adds it to PATH. This package creates no `.cmd`, `.ps1`, or executable shim and has no install hooks.
Upgrade with `scoop update; scoop update agent-desktop`; uninstall with `scoop uninstall agent-desktop`.

Both managers check the archive against a SHA-256 pinned in the repository. The manual release workflow reads
`Cargo.toml` and opens a package-update PR using the published archives and their verified checksums. If a prior npm-family installation shadows the native command, uninstall
it through the manager that installed it, then open a new terminal. Check resolution with `command -v agent-desktop`
on macOS/Linux or `Get-Command agent-desktop` on Windows.

## Blocked install scripts

The npm package includes the native binaries and helpers. With install scripts blocked, its launcher selects the
bundled binary. When postinstall runs, it links POSIX commands directly to the native binary and optimizes owned
Windows npm/Yarn commands to invoke the executable. Some package managers retain their own launcher.

## Release archives

Release archives are on the [releases page](https://github.com/lahfir/agent-desktop/releases). Their checksums come
from the same release, so they catch corruption but do not prove origin. Check origin with:

```bash
gh attestation verify <downloaded-archive> --repo lahfir/agent-desktop
```

## From source

You need Rust 1.89 or later, plus the Xcode command-line tools on macOS 13+, the MSVC build tools on Windows, or a C
linker on Linux:

```bash
git clone https://github.com/lahfir/agent-desktop && cd agent-desktop
cargo build --release -p agent-desktop
```

On Linux, also run `cargo build --release -p agent-desktop-linux --bins` and keep
`agent-desktop-linux-clipboard-owner` beside the CLI. npm and the release archives include it.

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

## Platform notes

**Windows.** `list-surfaces` lists each process's `window`, `focused`, `sheet` and `menu` surfaces. `snapshot
--surface` also reads `taskbar`, `system-tray`, `system-tray-overflow`, `start-menu` and `action-center` without
`--app`. `quick-settings` returns `PLATFORM_NOT_SUPPORTED` and names `action-center`, which carries those controls.

`launch` on Windows takes an absolute path, or a bare name under System32 or the Windows directory, with or
without `.exe`. It cannot resolve a display name such as "Google Chrome".

**Linux.** Semantic actions work on X11 and Wayland. On Wayland, app pointer input refuses and native window control
is unsupported.
