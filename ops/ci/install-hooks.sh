#!/bin/sh
# Install the local CI gate into .git/hooks (hooks are not versioned; this script is).
set -e
repo_root="$(git rev-parse --show-toplevel)"
cp "$repo_root/ops/ci/pre-commit" "$repo_root/.git/hooks/pre-commit"
chmod +x "$repo_root/.git/hooks/pre-commit"
echo "installed .git/hooks/pre-commit"
