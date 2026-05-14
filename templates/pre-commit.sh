#!/usr/bin/env bash
# Installed by `code-ratchet install-hook`.
# Runs the ratchet on staged changes. Blocks the commit on regression.
set -euo pipefail

if ! command -v code-ratchet >/dev/null 2>&1; then
    echo "code-ratchet not on PATH — skipping ratchet check (hook installed but binary missing)"
    exit 0
fi

repo_root="$(git rev-parse --show-toplevel)"
exit_code=0
code-ratchet --repo "$repo_root" check || exit_code=$?

if [ "$exit_code" -ne 0 ]; then
    echo
    echo "commit blocked by code-ratchet. read .ratchet/feedback.md for details."
    echo "to bypass (NOT RECOMMENDED): git commit --no-verify"
fi

exit "$exit_code"
