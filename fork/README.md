# Fork release tools

This folder exists only on the fork. Upstream never receives it.

## Path B: local release archive

Path A builds the fork's release archive with GitHub Actions. Path B builds it
on a workstation, for the time when Actions is off on the fork. Both make the
same Linux x86_64 archive as `.github/workflows/release.yml`.

`release-archive.sh` does these steps:

1. Refuses a checkout with any uncommitted change, staged change or untracked
   file. It builds a `git archive` export of one commit, never the working tree.
2. Downloads the pinned `ubuntu:22.04` image from the workflow by digest, and
   checks every downloaded part against its digest.
3. Runs the build in that image under bubblewrap, rootless. This needs no
   Docker group and no root. Inside the image it installs rustup, checks rustup
   against its published checksum, installs the toolchain from
   `rust-toolchain.toml` and runs the same `cargo build --locked --release`
   commands as the workflow.
4. Checks both programs against the glibc 2.35 floor and the 15 MiB size limit,
   and runs `agent-desktop version` inside the image only.
5. Packs the archive like the workflow does, then runs
   `scripts/check-linux-archive-contents.sh cli` on it, in the image and on the
   host. It also checks the member modes.

```bash
bash fork/release-archive.sh --check 1.0.2-hypr.0-rc1
bash fork/release-archive.sh 1.0.2-hypr.0-rc1
bash fork/release-archive.sh --ref v1.0.2-hypr.1 1.0.2-hypr.1
```

`--check` validates the arguments and the checkout, prints the plan and stops.
`--ref` defaults to `hypr/base`. If the ref is not a local name, the script
tries `origin/<ref>`. Run `git fetch origin` first. The build needs network
access, `bwrap`, `curl`, `python3` and 10 GiB free on the build disk. All work
happens under `~/.cache/ad-release/`, not in the checkout and not in `/tmp`.

## What it makes

In `~/.cache/ad-release/v<version>/` (change it with `--out`):

| File | Contents |
|---|---|
| `agent-desktop-v<version>-x86_64-unknown-linux-gnu.tar.gz` | `agent-desktop` (0755), `agent-desktop-linux-clipboard-owner` (0755), `THIRD_PARTY_NOTICES.md` (0644) |
| `<archive>.sha256` | The archive checksum, in `sha256sum -c` format |
| `SHA256SUMS` | Checksums of the three members and the archive |
| `BUILDINFO.txt` | Source ref and SHA, image digests, rustup and toolchain versions, glibc floor result, sizes, `agent-desktop version` output |

The script never overwrites these files. Remove the folder to build the same
version again.

Path B builds are not bit-for-bit reproducible. The checksum of the published
archive is the pin, so an installer checks file checksums. `agent-desktop
version` prints the `Cargo.toml` version, not the fork tag.

## Publishing is a separate step

The script never pushes, tags, publishes or uploads anything. To publish,
a maintainer runs these steps by hand, after review:

1. Push the tag `v<version>` on the source SHA from `BUILDINFO.txt`.
2. Run `sha256sum -c` on the `.sha256` file again.
3. Create a GitHub release on the fork with the four files above, marked
   pre-release for an `-rc` version.

Never publish to npm from the fork.

## Tests

```bash
bash fork/release-archive-test.sh
```

The test runs the script in `--check` mode against a throwaway repository.
It covers the argument rules, the dirty-tree refusal and the overwrite
refusal, and it checks that the script holds no publish command.
