# Releasing

Merge the Cargo workspace version, lockfile, and changelog update, then run the
Release workflow from `main`. One run publishes npm, native CLI and FFI archives,
checksums, and provenance. It then verifies the published CLI archives and commits
the Homebrew formula, Scoop manifest, and npm source version directly to `main`.
There is no package-update pull request.

## Publishing permission

Configure the repository Actions secret `RELEASE_TOKEN` with a dedicated
fine-grained GitHub token restricted to this repository and **Contents: read and
write**. Its owner must be allowed to push directly to `main`. With this
repository's current protection, that requires an administrator; organization
rules can impose additional restrictions. Do not use a broadly scoped personal
CLI token or disable branch protection for ordinary contributors.

The workflow rejects a missing secret before creating a tag or publishing. Git
rejects a metadata push if `main` has advanced concurrently. Fix the permission
or race and rerun Release: an already published version skips rebuilding and
publishing, while the metadata update runs again. Unchanged metadata produces
no commit. Renew the credential before it expires.

## Moving the repository

After a GitHub organization transfer, update repository URLs in package metadata,
documentation, and installation instructions. Configure npm's trusted publisher
for the new organization, repository, and `release.yml`, with direct publishing
allowed. Keep the npm package name `agent-desktop`.

Fine-grained tokens are scoped to a resource owner: replace `RELEASE_TOKEN` for
the new organization and confirm its branch rules permit the release identity
to publish metadata. Test the workflow before announcing the new installation
URLs. Do not recreate a repository at the previous location, which would remove
GitHub's redirects.
