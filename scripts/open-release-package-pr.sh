#!/usr/bin/env bash
set -euo pipefail

VERSION=$(node scripts/release-version.js)
BRANCH="release/packages-v${VERSION}"
git add -- Formula/agent-desktop.rb bucket/agent-desktop.json npm/package.json
if git diff --cached --quiet; then
  exit 0
fi

git switch -c "$BRANCH"
git config user.name "$(gh api "users/${GITHUB_ACTOR}" --jq '.name // .login')"
git config user.email "${GITHUB_ACTOR_ID}+${GITHUB_ACTOR}@users.noreply.github.com"
git commit -m "chore: update packages to ${VERSION}"
gh auth setup-git
REMOTE_SHA=$(git ls-remote --heads origin "refs/heads/$BRANCH" | cut -f1)
git push --force-with-lease="refs/heads/$BRANCH:$REMOTE_SHA" origin "HEAD:refs/heads/$BRANCH"

PR=$(gh pr list --head "$BRANCH" --base main --state open --json number --jq '.[0].number // empty')
if [ -z "$PR" ]; then
  BODY=$(mktemp)
  cat > "$BODY" <<'EOF'
Update native package definitions and npm source metadata from the published release.
All CLI archive hashes and the checksum manifest's release-workflow attestation were verified.
Repository and native installation checks run on this branch. Merge after review and green checks.
EOF
  gh pr create --base main --head "$BRANCH" \
    --title "chore: update packages to ${VERSION}" --body-file "$BODY"
fi
for workflow in ci.yml supply-chain.yml codeql.yml homebrew.yml scoop.yml; do
  gh workflow run "$workflow" --ref "$BRANCH"
done
