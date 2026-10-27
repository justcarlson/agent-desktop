# Releasing

Update the Cargo workspace version, lockfile, npm source version, and changelog in one pull request.
After it merges to `main`, changes to `Cargo.toml` trigger the existing Release workflow.
An unchanged published version skips publication. Manual dispatch remains available for retries.

The workflow builds and checks the native CLI and FFI archives, bundles the CLI executables into npm,
publishes npm through trusted publishing (OIDC), and publishes GitHub archives, checksums, and provenance
with the built-in `GITHUB_TOKEN`. No extra release token or package-update commit is required.

Homebrew and Scoop package definitions belong in `Homebrew/homebrew-core` and `ScoopInstaller/Main`.
Their maintainers and update automation manage those catalogs after the initial submissions are accepted.
Homebrew builds from source; Scoop downloads the Windows release archives. Catalog availability can lag
behind npm and GitHub publication. Do not advertise default-catalog installation before acceptance.

## Moving the repository

After a GitHub organization transfer, update repository URLs in package metadata and documentation,
and coordinate the Homebrew/Scoop definition changes. Configure npm's trusted publisher for the new
organization, repository, and `release.yml`, with direct publishing allowed. Keep the npm package name
`agent-desktop`. Do not recreate the old repository, which would remove GitHub's redirects.
