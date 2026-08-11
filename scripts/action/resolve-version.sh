#!/usr/bin/env bash
# Decide which notiflow release the Action should install.
#
# Precedence:
#   1. the `version` input, when set
#   2. $GITHUB_ACTION_PATH/VERSION — stamped by the release job, so it is part of the
#      checked-out tree and gives the same answer for @v2, @v2.1.0 and @<sha>
#   3. $GITHUB_ACTION_REF, when it is a semver tag
#   4. `latest` via the Releases API, with a warning — a network call whose answer can
#      change between two runs of the same pinned workflow
#
# Writes `version=vX.Y.Z` to $GITHUB_OUTPUT.
set -euo pipefail

v="${INPUT_VERSION:-}"
source="input"

if [ -z "$v" ]; then
  version_file="${GITHUB_ACTION_PATH:-.}/VERSION"
  if [ -r "$version_file" ]; then
    v="$(tr -d '[:space:]' <"$version_file")"
    source="VERSION file"
  fi
fi

if [ -z "$v" ]; then
  ref="${GITHUB_ACTION_REF:-}"
  if printf '%s' "$ref" | grep -Eq '^v?[0-9]+\.[0-9]+\.[0-9]+$'; then
    v="$ref"
    source="GITHUB_ACTION_REF"
  fi
fi

if [ -z "$v" ] || [ "$v" = "latest" ]; then
  echo "::warning::falling back to the latest release; pin a version for reproducible runs"
  if ! command -v gh >/dev/null 2>&1; then
    echo "::error::gh CLI is not available and no version could be resolved" >&2
    exit 1
  fi
  v="$(GH_TOKEN="${GH_TOKEN:-${GITHUB_TOKEN:-}}" gh api repos/jtprogru/notiflow/releases/latest --jq .tag_name)"
  source="releases/latest"
fi

case "$v" in
  v*) ;;
  *) v="v${v}" ;;
esac

echo "version=${v}" >>"$GITHUB_OUTPUT"
echo "Resolved notiflow version: ${v} (via ${source})"
