#!/usr/bin/env bash
#
# Fails when a vendored Wayland protocol file no longer matches the SHA-256
# that crates/linux/protocols/PROVENANCE.md records for it, when a file in
# that folder has no provenance row, or when THIRD_PARTY_NOTICES.md names a
# different upstream revision. The generated bindings come from these files,
# so an unrecorded edit would ship code nobody reviewed against upstream.
#
# Before it checks the tree, the script runs the same check on a scratch copy
# with one byte changed and one unlisted file added, and fails unless both are
# caught, so a check that stops detecting fails the gate.

set -euo pipefail

cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

sha256_of() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | cut -d' ' -f1
    else
        shasum -a 256 "$1" | cut -d' ' -f1
    fi
}

# Prints one problem per line for the protocol folder $1 and notice file $2.
# shellcheck disable=SC2016
problems_in() {
    local dir=$1 notices=$2 provenance="$1/PROVENANCE.md"
    local listed=" " line file revision digest actual path name
    while IFS= read -r line; do
        file=$(printf '%s\n' "$line" | cut -d'|' -f2 | tr -d ' `')
        revision=$(printf '%s\n' "$line" | cut -d'|' -f4 | sed -E 's/.*\(`?([0-9a-f]{40})`?\).*/\1/')
        digest=$(printf '%s\n' "$line" | cut -d'|' -f6 | tr -d ' `')
        listed="$listed$file "
        path="$dir/$file"
        if [ ! -f "$path" ]; then
            echo "$path: listed in PROVENANCE.md but missing"
            continue
        fi
        actual=$(sha256_of "$path")
        if [ "$actual" != "$digest" ]; then
            echo "$path: SHA-256 is $actual, PROVENANCE.md records $digest"
        fi
        if ! grep -q "$revision" "$notices"; then
            echo "$notices: does not name upstream revision $revision of $file"
        fi
    done < <(grep -E '^\| `[^`]+` \|' "$provenance")
    for path in "$dir"/*; do
        name=$(basename "$path")
        [ "$name" = "PROVENANCE.md" ] && continue
        case "$listed" in
            *" $name "*) ;;
            *) echo "$path: vendored without a PROVENANCE.md row" ;;
        esac
    done
}

self_test() {
    local scratch
    scratch=$(mktemp -d)
    trap 'rm -rf "$scratch"' RETURN
    cp -R crates/linux/protocols "$scratch/protocols"
    cp THIRD_PARTY_NOTICES.md "$scratch/NOTICES.md"
    if [ -n "$(problems_in "$scratch/protocols" "$scratch/NOTICES.md")" ]; then
        echo "FAIL: self-test copy of the tree is not clean"
        return 1
    fi
    printf ' ' >>"$scratch/protocols/hyprland-toplevel-mapping-v1.xml"
    printf 'x' >"$scratch/protocols/unlisted.xml"
    local found
    found=$(problems_in "$scratch/protocols" "$scratch/NOTICES.md")
    case "$found" in
        *"hyprland-toplevel-mapping-v1.xml: SHA-256"*) ;;
        *) echo "FAIL: self-test edit was not detected"; return 1 ;;
    esac
    case "$found" in
        *"unlisted.xml: vendored without"*) ;;
        *) echo "FAIL: self-test unlisted file was not detected"; return 1 ;;
    esac
}

self_test
found=$(problems_in crates/linux/protocols THIRD_PARTY_NOTICES.md)
if [ -n "$found" ]; then
    printf 'FAIL: %s\n' "$found"
    exit 1
fi
echo "OK: vendored protocols match PROVENANCE.md"
