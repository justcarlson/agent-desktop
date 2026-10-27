#!/usr/bin/env bash
set -euo pipefail

VERSION=$(node scripts/release-version.js)
git add -- Formula/agent-desktop.rb bucket/agent-desktop.json npm/package.json
if git diff --cached --quiet -- Formula/agent-desktop.rb bucket/agent-desktop.json npm/package.json; then
  exit 0
fi

git config user.name "$(gh api "users/${GITHUB_ACTOR}" --jq '.name // .login')"
git config user.email "${GITHUB_ACTOR_ID}+${GITHUB_ACTOR}@users.noreply.github.com"
git commit -m "chore: update packages to ${VERSION}" --only -- \
  Formula/agent-desktop.rb bucket/agent-desktop.json npm/package.json
gh auth setup-git
git push origin HEAD:refs/heads/main
