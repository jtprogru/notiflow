#!/usr/bin/env bats
#
# Manifest sanity checks for action.yml.
#
# Catches the class of regressions where the manifest references a GitHub
# Actions context that is not exposed to composite-action manifests at load
# time. The runner aborts before any step runs with messages like:
#
#   Unrecognized named-value: 'job'.
#   Unrecognized named-value: 'needs'.
#
# Composite-action manifests evaluate ${{ ... }} expressions in `default:`,
# `description:`, `env:`, and elsewhere — so a literal example like
# `${{ job.status }}` in a description string blows up the same way the
# original `default: ${{ job.status }}` did.
#
# See: https://docs.github.com/en/actions/learn-github-actions/contexts#context-availability

load helpers

setup() {
  ACTION_YML="${NF_ROOT}/action.yml"
}

# nf_forbidden_grep <context_name>
# Greps action.yml for any ${{ <context>. ... }} expression. Returns the
# matching lines so the failing test prints something useful.
nf_forbidden_grep() {
  local ctx="$1"
  grep -nE '\$\{\{[[:space:]]*'"$ctx"'\.' "$ACTION_YML" || true
}

@test "manifest: action.yml exists" {
  [ -f "$ACTION_YML" ]
}

@test "manifest: no \${{ job.* }} anywhere in action.yml" {
  local hits
  hits=$(nf_forbidden_grep job)
  if [ -n "$hits" ]; then
    echo "forbidden 'job' context (composite-action manifests do not expose it):" >&2
    echo "$hits" >&2
    return 1
  fi
}

@test "manifest: no \${{ needs.* }} anywhere in action.yml" {
  local hits
  hits=$(nf_forbidden_grep needs)
  if [ -n "$hits" ]; then
    echo "forbidden 'needs' context:" >&2
    echo "$hits" >&2
    return 1
  fi
}

@test "manifest: no \${{ secrets.* }} anywhere in action.yml" {
  local hits
  hits=$(nf_forbidden_grep secrets)
  if [ -n "$hits" ]; then
    echo "forbidden 'secrets' context (must come in through an input):" >&2
    echo "$hits" >&2
    return 1
  fi
}

@test "manifest: no \${{ matrix.* }} anywhere in action.yml" {
  local hits
  hits=$(nf_forbidden_grep matrix)
  if [ -n "$hits" ]; then
    echo "forbidden 'matrix' context:" >&2
    echo "$hits" >&2
    return 1
  fi
}

@test "manifest: no \${{ vars.* }} anywhere in action.yml" {
  local hits
  hits=$(nf_forbidden_grep vars)
  if [ -n "$hits" ]; then
    echo "forbidden 'vars' context (must come in through an input):" >&2
    echo "$hits" >&2
    return 1
  fi
}

@test "manifest: status input is required and has no default" {
  # Parse the `status:` input block (until the next top-level input key) and
  # verify it declares required: true and no default: field.
  run awk '
    /^[[:space:]]{2}[a-z_]+:[[:space:]]*$/ {
      in_status = ($0 ~ /^[[:space:]]{2}status:[[:space:]]*$/) ? 1 : 0
      next
    }
    in_status { print }
  ' "$ACTION_YML"
  [ "$status" -eq 0 ]
  [[ "$output" == *"required: true"* ]]
  if [[ "$output" == *"default:"* ]]; then
    echo "status: must not declare a default" >&2
    echo "$output" >&2
    return 1
  fi
}
