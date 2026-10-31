"""The fail-closed guard for the attached `hyprland` tier.

The attached tier drives the desktop session it runs in, so it must never
start on a workstation. It refuses unless all three hold:

- the caller opted in with `AGENT_DESKTOP_E2E_ATTACHED_HYPRLAND=1`;
- `test/omarchy-vm` wrote the marker inside the guest: a regular file owned
  by root, in root-owned directories outside `$HOME` and `$XDG_RUNTIME_DIR`,
  holding this boot's id, so no user process and no earlier boot can make it;
- the machine is a virtual machine: `systemd-detect-virt --vm` names a
  hypervisor, or, when that tool cannot run, the DMI vendor names one.

The marker path is fixed here and never read from the environment.
"""

from __future__ import annotations

import os
import stat
import subprocess
import sys
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from runner.session import TierUnavailable

OPT_IN = "AGENT_DESKTOP_E2E_ATTACHED_HYPRLAND"
MARKER = Path("/var/lib/agent-desktop-e2e/attached-hyprland-vm")
BOOT_ID = Path("/proc/sys/kernel/random/boot_id")
DMI_ROOT = Path("/sys/class/dmi/id")
DMI_FIELDS = ("sys_vendor", "product_name", "bios_vendor")
HYPERVISORS = ("qemu", "kvm", "virtualbox", "innotek", "vmware", "xen", "bochs", "parallels",
               "bhyve", "cloud hypervisor", "virtual machine", "amazon ec2", "google compute engine")
GROUP_OR_OTHER_WRITE = stat.S_IWGRP | stat.S_IWOTH


class Refused(TierUnavailable):
    pass


def _lstat(path: Path):
    return os.lstat(path)


def _read(path: Path) -> str:
    return Path(path).read_text()


def _detect_virt() -> subprocess.CompletedProcess:
    return subprocess.run(["systemd-detect-virt", "--vm"], capture_output=True, text=True,
                          timeout=5, check=False)


def _dmi_field(name: str) -> str:
    try:
        return (DMI_ROOT / name).read_text().strip()
    except OSError:
        return ""


def opt_in_refusal(env: dict[str, str]) -> str | None:
    if env.get(OPT_IN) != "1":
        return f"{OPT_IN} is not 1; only test/omarchy-vm sets it inside the guest"
    return None


def _root_only(path: Path, kind: str) -> str | None:
    try:
        info = _lstat(path)
    except FileNotFoundError:
        return f"the VM marker {MARKER} is missing; only test/omarchy-vm writes it inside the guest"
    except OSError as error:
        return f"the VM marker {MARKER} cannot be read: {error}"
    test = stat.S_ISREG if kind == "regular file" else stat.S_ISDIR
    if not test(info.st_mode):
        return f"the VM marker path {path} is not a {kind}"
    if info.st_uid != 0:
        return f"the VM marker path {path} is not owned by root"
    if info.st_mode & GROUP_OR_OTHER_WRITE:
        return f"the VM marker path {path} is writable by group or others"
    return None


def marker_refusal(env: dict[str, str]) -> str | None:
    for key in ("HOME", "XDG_RUNTIME_DIR"):
        value = env.get(key)
        if value and (Path(value) == MARKER or Path(value) in MARKER.parents):
            return f"the VM marker {MARKER} lies inside {key}"
    problem = _root_only(MARKER, "regular file")
    for parent in MARKER.parents:
        problem = problem or _root_only(parent, "directory")
    if problem:
        return problem
    try:
        marked, booted = _read(MARKER).strip(), _read(BOOT_ID).strip()
    except OSError as error:
        return f"the VM marker {MARKER} cannot be read: {error}"
    if not booted or marked != booted:
        return f"the VM marker {MARKER} was written in another boot"
    return None


def vm_refusal() -> str | None:
    try:
        result = _detect_virt()
    except (OSError, subprocess.SubprocessError):
        result = None
    if result is not None:
        found = result.stdout.strip()
        if result.returncode == 0 and found and found != "none":
            return None
        if found == "none":
            return "this machine is not a virtual machine: systemd-detect-virt --vm says none"
    vendor = " ".join(_dmi_field(name) for name in DMI_FIELDS).lower()
    if any(name in vendor for name in HYPERVISORS):
        return None
    return f"this machine is not a virtual machine: DMI names {vendor.strip() or 'nothing'}"


def refusals(env: dict[str, str]) -> list[str]:
    """Every reason the attached tier may not start here; empty when it may."""
    checks = (opt_in_refusal(env), marker_refusal(env), vm_refusal())
    return [reason for reason in checks if reason]


def require(env: dict[str, str]) -> None:
    reasons = refusals(env)
    if reasons:
        raise Refused("hyprland: the attached tier refused to start: " + "; ".join(reasons))


def main() -> int:
    reasons = refusals(dict(os.environ))
    for reason in reasons:
        print(f"attached hyprland guard: {reason}", file=sys.stderr)
    return 2 if reasons else 0


if __name__ == "__main__":
    sys.exit(main())
