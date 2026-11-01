#!/usr/bin/env bash
#
# Builds the fork's Linux x86_64 release archive on a workstation (release
# path B), for use while GitHub Actions is off on the fork. It repeats the
# Linux CLI steps of .github/workflows/release.yml: the same pinned
# ubuntu:22.04 image, rustup checked against its published checksum, the
# toolchain from rust-toolchain.toml, the same cargo commands, the glibc
# floor and size checks, and the same archive members and modes.
#
# The image runs rootless under bubblewrap, so the build needs no Docker
# group and no root. The source is a `git archive` export of one commit, so
# no local edit can reach the archive. The script never pushes, tags,
# publishes or uploads anything; it only writes files to the output directory.
#
# Usage:
#   fork/release-archive.sh [--ref REF] [--repo DIR] [--out DIR] [--work DIR] [--check] VERSION
#
#   VERSION  for example 1.0.2-hypr.0-rc1 (no leading v)
#   --ref    commit to build; tried as given, then as origin/REF (default hypr/base)
#   --repo   Git checkout to read the ref from (default: the checkout holding this script)
#   --out    output directory (default ~/.cache/ad-release/vVERSION)
#   --work   build directory, removed after a good build (default ~/.cache/ad-release/build/vVERSION)
#   --check  validate the arguments and the checkout, print the plan, build nothing
#
# Exit: 0 success, 1 refused or failed, 2 bad arguments

set -euo pipefail

TARGET=x86_64-unknown-linux-gnu
IMAGE_REPO=library/ubuntu
IMAGE_TAG=22.04
IMAGE_DIGEST=sha256:0a9887931b0e38af2e3487ed39e29436b0535c4f3f52e57745baaf79d1898956
GLIBC_FLOOR=2.35
SIZE_LIMIT=$((15 * 1024 * 1024))
MIN_FREE_KIB=$((10 * 1024 * 1024))
REGISTRY=https://registry-1.docker.io
CACHE_ROOT="${HOME}/.cache/ad-release"
WORK_MARKER=.release-archive-work

usage() {
  sed -n '15,24p' "$0" | sed 's/^# \{0,1\}//' >&2
  exit 2
}

die() {
  echo "release-archive: $1" >&2
  exit "${2:-1}"
}

REF=hypr/base
REPO=
OUT=
WORK=
CHECK=0
VERSION=

while [ "$#" -gt 0 ]; do
  case "$1" in
    --ref) [ "$#" -ge 2 ] || usage; REF=$2; shift 2 ;;
    --repo) [ "$#" -ge 2 ] || usage; REPO=$2; shift 2 ;;
    --out) [ "$#" -ge 2 ] || usage; OUT=$2; shift 2 ;;
    --work) [ "$#" -ge 2 ] || usage; WORK=$2; shift 2 ;;
    --check) CHECK=1; shift ;;
    -h|--help) usage ;;
    -*) echo "release-archive: unknown option $1" >&2; usage ;;
    *) [ -z "$VERSION" ] || usage; VERSION=$1; shift ;;
  esac
done
[ -n "$VERSION" ] || usage

if ! printf '%s\n' "$VERSION" | grep -Eqx '[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z][0-9A-Za-z.-]*)?'; then
  die "version '$VERSION' is not of the form 1.0.2 or 1.0.2-hypr.0-rc1 (no leading v)" 2
fi

[ -n "$REPO" ] || REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO=$(git -C "$REPO" rev-parse --show-toplevel 2>/dev/null) || die "'$REPO' is not a Git checkout" 2

DIRTY=$(git -C "$REPO" status --porcelain=v1 --untracked-files=all)
if [ -n "$DIRTY" ]; then
  printf '%s\n' "$DIRTY" | head -n 20 >&2
  die "the checkout at $REPO is dirty; commit, stash or remove these changes first"
fi

SOURCE_REF=$REF
SOURCE_SHA=$(git -C "$REPO" rev-parse --verify --quiet "${REF}^{commit}") || SOURCE_SHA=
if [ -z "$SOURCE_SHA" ]; then
  SOURCE_REF="origin/$REF"
  SOURCE_SHA=$(git -C "$REPO" rev-parse --verify --quiet "${SOURCE_REF}^{commit}") || die "ref '$REF' names no commit in $REPO" 2
fi
SCRIPT_SHA=$(git -C "$REPO" rev-parse HEAD)

ARCHIVE="agent-desktop-v${VERSION}-${TARGET}.tar.gz"
[ -n "$OUT" ] || OUT="${CACHE_ROOT}/v${VERSION}"
[ -n "$WORK" ] || WORK="${CACHE_ROOT}/build/v${VERSION}"
for FILE in "$ARCHIVE" "$ARCHIVE.sha256" SHA256SUMS BUILDINFO.txt; do
  [ ! -e "$OUT/$FILE" ] || die "$OUT/$FILE already exists; this script never overwrites a release file"
done

echo "version:      $VERSION"
echo "source ref:   $SOURCE_REF"
echo "source SHA:   $SOURCE_SHA"
echo "image:        ubuntu:${IMAGE_TAG}@${IMAGE_DIGEST}"
echo "target:       $TARGET"
echo "archive:      $OUT/$ARCHIVE"
echo "build dir:    $WORK"
if [ "$CHECK" -eq 1 ]; then
  echo "check only: nothing built"
  exit 0
fi

for TOOL in bwrap curl tar gzip sha256sum python3 git df; do
  command -v "$TOOL" >/dev/null 2>&1 || die "$TOOL is not installed"
done

if [ -e "$WORK" ]; then
  if [ -n "$(ls -A "$WORK")" ] && [ ! -e "$WORK/$WORK_MARKER" ]; then
    die "$WORK exists, is not empty and was not made by this script; choose another --work"
  fi
  chmod -R u+rwX "$WORK"
  rm -rf -- "$WORK"
fi
mkdir -p "$WORK" "$OUT" "${CACHE_ROOT}/oci"
: > "$WORK/$WORK_MARKER"

free_kib() {
  df -Pk "$1" | awk 'NR == 2 { print $4 }'
}
for DIR in "$WORK" "$OUT"; do
  FREE=$(free_kib "$DIR")
  [ "$FREE" -ge "$MIN_FREE_KIB" ] || die "only ${FREE} KiB free under $DIR; the build needs 10 GiB"
done

json_field() {
  python3 -I -c '
import json, sys
doc = json.load(open(sys.argv[1]))
mode = sys.argv[2]
if mode == "token":
    print(doc["token"])
elif mode == "platform":
    picks = [m["digest"] for m in doc.get("manifests", [])
             if m.get("platform", {}).get("os") == "linux"
             and m.get("platform", {}).get("architecture") == "amd64"
             and not m.get("platform", {}).get("variant")]
    if len(picks) != 1:
        sys.exit("expected one linux/amd64 manifest, found %d" % len(picks))
    print(picks[0])
elif mode == "layer":
    layers = doc.get("layers", [])
    if len(layers) != 1 or layers[0]["mediaType"] not in ("application/vnd.oci.image.layer.v1.tar+gzip", "application/vnd.docker.image.rootfs.diff.tar.gzip"):
        sys.exit("expected one gzip layer, found %d" % len(layers))
    print(layers[0]["digest"])
elif mode == "config":
    print(doc["config"]["digest"])
' "$@"
}

HEADERS="$WORK/registry-headers"
fetch_verified() {
  local kind=$1 digest=$2 dest=$3 accept=${4:-}
  local hex=${digest#sha256:}
  if [ ! -f "$dest" ] || ! printf '%s  %s\n' "$hex" "$dest" | sha256sum -c --status -; then
    rm -f -- "$dest"
    if [ -n "$accept" ]; then
      curl --proto '=https' --tlsv1.2 -fsSL -H @"$HEADERS" -H "Accept: $accept" -o "$dest.part" "${REGISTRY}/v2/${IMAGE_REPO}/${kind}/${digest}"
    else
      curl --proto '=https' --tlsv1.2 -fsSL -H @"$HEADERS" -o "$dest.part" "${REGISTRY}/v2/${IMAGE_REPO}/${kind}/${digest}"
    fi
    printf '%s  %s\n' "$hex" "$dest.part" | sha256sum -c --status - || die "registry returned content that does not match $digest"
    mv -- "$dest.part" "$dest"
  fi
}

echo "==> fetching ubuntu:${IMAGE_TAG}@${IMAGE_DIGEST}"
umask 077
curl --proto '=https' --tlsv1.2 -fsSL -o "$WORK/token.json" \
  "https://auth.docker.io/token?service=registry.docker.io&scope=repository:${IMAGE_REPO}:pull"
printf 'Authorization: Bearer %s\n' "$(json_field "$WORK/token.json" token)" > "$HEADERS"
rm -f -- "$WORK/token.json"
umask 022
INDEX_TYPES="application/vnd.oci.image.index.v1+json, application/vnd.docker.distribution.manifest.list.v2+json"
MANIFEST_TYPES="application/vnd.oci.image.manifest.v1+json, application/vnd.docker.distribution.manifest.v2+json"
OCI="${CACHE_ROOT}/oci"
fetch_verified manifests "$IMAGE_DIGEST" "$OCI/${IMAGE_DIGEST#sha256:}" "$INDEX_TYPES"
PLATFORM_DIGEST=$(json_field "$OCI/${IMAGE_DIGEST#sha256:}" platform)
fetch_verified manifests "$PLATFORM_DIGEST" "$OCI/${PLATFORM_DIGEST#sha256:}" "$MANIFEST_TYPES"
LAYER_DIGEST=$(json_field "$OCI/${PLATFORM_DIGEST#sha256:}" layer)
CONFIG_DIGEST=$(json_field "$OCI/${PLATFORM_DIGEST#sha256:}" config)
fetch_verified blobs "$LAYER_DIGEST" "$OCI/${LAYER_DIGEST#sha256:}"
rm -f -- "$HEADERS"

echo "==> unpacking the image and the source"
mkdir -p "$WORK/rootfs" "$WORK/src" "$WORK/target" "$WORK/stage"
tar -xzf "$OCI/${LAYER_DIGEST#sha256:}" -C "$WORK/rootfs"
git -C "$REPO" archive --format=tar "$SOURCE_SHA" | tar -xf - -C "$WORK/src"

cat > "$WORK/stage/build.sh" <<'INNER'
set -euo pipefail
umask 022
export DEBIAN_FRONTEND=noninteractive
APT=(apt-get -q -o APT::Sandbox::User=root)
"${APT[@]}" update
"${APT[@]}" install -y --no-install-recommends ca-certificates curl build-essential git xz-utils

cd /tmp
BASE="https://static.rust-lang.org/rustup/dist/${TARGET}"
curl --proto '=https' --tlsv1.2 -fsSLO "${BASE}/rustup-init"
RUSTUP_SHA=$(curl --proto '=https' --tlsv1.2 -fsSL "${BASE}/rustup-init.sha256" | cut -d' ' -f1)
echo "${RUSTUP_SHA}  rustup-init" | sha256sum -c -
chmod +x rustup-init
./rustup-init -y --default-toolchain none --profile minimal

cd /src
rustup toolchain install
rustup show
rustup target add "$TARGET"
cargo build --locked --release -p agent-desktop --target "$TARGET"
cargo build --locked --release -p agent-desktop-linux --bins --target "$TARGET"

DIR="/target/${TARGET}/release"
: > /stage/glibc.txt
: > /stage/sizes.txt
for FILE in agent-desktop agent-desktop-linux-clipboard-owner; do
  scripts/check-elf-glibc-floor.sh "${DIR}/${FILE}" "$GLIBC_FLOOR" | tee -a /stage/glibc.txt
  SIZE=$(stat -c%s "${DIR}/${FILE}")
  echo "${FILE}: ${SIZE} bytes" | tee -a /stage/sizes.txt
  if [ "$SIZE" -gt "$SIZE_LIMIT" ]; then
    echo "FAIL: shipped executable ${FILE} exceeds 15MB limit (${SIZE} bytes)"
    exit 1
  fi
done
"${DIR}/agent-desktop" version | tee /stage/version.txt

BINARY_SHA=$(sha256sum "${DIR}/agent-desktop" | awk '{print $1}')
HELPER_SHA=$(sha256sum "${DIR}/agent-desktop-linux-clipboard-owner" | awk '{print $1}')
install -m 0644 THIRD_PARTY_NOTICES.md "${DIR}/THIRD_PARTY_NOTICES.md"
tar -czf "/stage/${ARCHIVE}" -C "${DIR}" agent-desktop agent-desktop-linux-clipboard-owner THIRD_PARTY_NOTICES.md
if [ "$BINARY_SHA" != "$(sha256sum "${DIR}/agent-desktop" | awk '{print $1}')" ] || \
   [ "$HELPER_SHA" != "$(sha256sum "${DIR}/agent-desktop-linux-clipboard-owner" | awk '{print $1}')" ]; then
  echo "Release executable changed while packaging" >&2
  exit 1
fi
scripts/check-linux-archive-contents.sh cli "/stage/${ARCHIVE}"
(cd "$DIR" && sha256sum agent-desktop agent-desktop-linux-clipboard-owner THIRD_PARTY_NOTICES.md) > /stage/members.sha256
{
  echo "rustup-init: ${RUSTUP_SHA}"
  rustc -V
  cargo -V
  echo "image glibc: $(ldd --version | head -n 1)"
} > /stage/toolchain.txt
INNER

echo "==> building $SOURCE_SHA in ubuntu:${IMAGE_TAG} under bubblewrap"
bwrap \
  --bind "$WORK/rootfs" / \
  --dev /dev \
  --proc /proc \
  --tmpfs /tmp \
  --ro-bind /etc/resolv.conf /etc/resolv.conf \
  --bind "$WORK/src" /src \
  --bind "$WORK/target" /target \
  --bind "$WORK/stage" /stage \
  --unshare-user --uid 0 --gid 0 \
  --unshare-pid --unshare-ipc --unshare-uts \
  --die-with-parent \
  --clearenv \
  --setenv HOME /root \
  --setenv PATH /root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
  --setenv LANG C.UTF-8 \
  --setenv CARGO_TARGET_DIR /target \
  --setenv CARGO_TERM_COLOR never \
  --setenv TARGET "$TARGET" \
  --setenv ARCHIVE "$ARCHIVE" \
  --setenv GLIBC_FLOOR "$GLIBC_FLOOR" \
  --setenv SIZE_LIMIT "$SIZE_LIMIT" \
  --chdir /src \
  /bin/bash /stage/build.sh

echo "==> checking the archive on the host"
STAGED="$WORK/stage/$ARCHIVE"
"$WORK/src/scripts/check-linux-archive-contents.sh" cli "$STAGED"
MODES=$(tar -tvzf "$STAGED" | awk '{ print $1, $2, $NF }' | sort -k3)
EXPECTED_MODES=$(printf '%s\n' \
  "-rwxr-xr-x root/root agent-desktop" \
  "-rwxr-xr-x root/root agent-desktop-linux-clipboard-owner" \
  "-rw-r--r-- root/root THIRD_PARTY_NOTICES.md" | sort -k3)
[ "$MODES" = "$EXPECTED_MODES" ] || die "archive modes are '$MODES', expected '$EXPECTED_MODES'"
EXTRACT="$WORK/extract"
mkdir -p "$EXTRACT"
tar -xzf "$STAGED" -C "$EXTRACT"
(cd "$EXTRACT" && sha256sum -c --quiet "$WORK/stage/members.sha256") || die "archive members do not match the built files"

ARCHIVE_SHA=$(sha256sum "$STAGED" | awk '{ print $1 }')
cp -- "$STAGED" "$OUT/$ARCHIVE"
printf '%s  %s\n' "$ARCHIVE_SHA" "$ARCHIVE" > "$OUT/$ARCHIVE.sha256"
{
  cat "$WORK/stage/members.sha256"
  printf '%s  %s\n' "$ARCHIVE_SHA" "$ARCHIVE"
} > "$OUT/SHA256SUMS"
{
  echo "version: $VERSION"
  echo "archive: $ARCHIVE"
  echo "archive sha256: $ARCHIVE_SHA"
  echo "source ref: $SOURCE_REF"
  echo "source sha: $SOURCE_SHA"
  echo "release script: fork/release-archive.sh at $SCRIPT_SHA"
  echo "path: B (local build, bubblewrap, no GitHub Actions)"
  echo "target: $TARGET"
  echo "image: ubuntu:${IMAGE_TAG}@${IMAGE_DIGEST}"
  echo "image linux/amd64 manifest: $PLATFORM_DIGEST"
  echo "image config: $CONFIG_DIGEST"
  echo "image layer: $LAYER_DIGEST"
  sed 's/^/toolchain: /' "$WORK/stage/toolchain.txt"
  sed 's/^/glibc floor: /' "$WORK/stage/glibc.txt" | sed "s|/target/${TARGET}/release/||"
  sed 's/^/size: /' "$WORK/stage/sizes.txt"
  sed 's/^/agent-desktop version: /' "$WORK/stage/version.txt"
  sed 's/^/member sha256: /' "$WORK/stage/members.sha256"
  echo "built at: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
} > "$OUT/BUILDINFO.txt"
(cd "$OUT" && sha256sum -c --quiet "$ARCHIVE.sha256") || die "the written .sha256 does not verify"

chmod -R u+rwX "$WORK"
rm -rf -- "$WORK"

echo "==> done; nothing was pushed, tagged or published"
echo "archive: $OUT/$ARCHIVE"
echo "sha256:  $ARCHIVE_SHA"
echo "source:  $SOURCE_SHA"
