#!/usr/bin/env bash
# The privileged uinput lab: measure whether a ydotool route exists on Wayland
# without reaching a seat a live session uses. The owner runs it with root.
# Usage: scripts/uinput-lab.sh [--dry-run] [--owner]
#
# A private compositor does not isolate uinput: the device is kernel-global.
# The script identifies its device and its seat before it sends any event, and
# stops when isolation is unproven.
set -euo pipefail

legs() {
  cat <<'TABLE'
UINPUT-01	device-node-mode-owner-groups	L24-31
UINPUT-02	privileged-ydotoold-start	L5-2 L5-12
UINPUT-03	created-device-identification	L13-15
UINPUT-04	libinput-udev-seat-property	L1-17
UINPUT-05	seat-versus-live-sessions	L1-17
UINPUT-06	keys-into-wayland-gtk4	L13-15
UINPUT-07	pointer-route-verdict	L13-17
TABLE
}

dry_run=0
owner=0
while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) dry_run=1; shift ;;
    --owner) owner=1; shift ;;
    -h|--help)
      echo "usage: scripts/uinput-lab.sh [--dry-run] [--owner]" >&2
      exit 0
      ;;
    *)
      echo "uinput-lab: unknown argument $1" >&2
      exit 2
      ;;
  esac
done

if [ "$dry_run" = 1 ]; then
  printf 'leg\tname\tsources\n'
  legs
  exit 0
fi

if [ "$owner" != 1 ]; then
  echo "uinput-lab: the lab needs the owner and root; pass --owner" >&2
  exit 2
fi

if [ "$(id -u)" != 0 ]; then
  echo "uinput-lab: run with root; /dev/uinput is root-only on a stock install" >&2
  if [ -e /dev/uinput ]; then
    stat -c 'device=%n mode=%a owner=%U group=%G' /dev/uinput >&2 || true
  fi
  id -nG | sed 's/^/groups=/' >&2 || true
  exit 2
fi

echo "uinput-lab: the leg bodies are not implemented in this stub" >&2
legs >&2
exit 3
