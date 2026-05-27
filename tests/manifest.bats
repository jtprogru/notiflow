#!/usr/bin/env bats
#
# Manifest sanity checks for action.yml.
#
# Catches the class of regressions where a composite action's input.default
# references a context that GitHub Actions does not expose at manifest-load
# time. The runner aborts before any step runs with:
#   Unrecognized named-value: 'job' (or 'steps' / 'needs' / 'secrets').
# See: https://docs.github.com/en/actions/learn-github-actions/contexts#context-availability

load helpers

setup() {
  ACTION_YML="${NF_ROOT}/action.yml"
}

@test "manifest: action.yml exists" {
  [ -f "$ACTION_YML" ]
}

@test "manifest: input defaults do not reference job context" {
  # Any line that looks like `default: ${{ job.* }}` is illegal in composite
  # action manifests, regardless of which input it lives under.
  run grep -nE '^[[:space:]]*default:[[:space:]]*\$\{\{[[:space:]]*job\.' "$ACTION_YML"
  if [ "$status" -eq 0 ]; then
    echo "forbidden 'job' context in default:" >&2
    echo "$output" >&2
    return 1
  fi
}

@test "manifest: input defaults do not reference steps context" {
  run grep -nE '^[[:space:]]*default:[[:space:]]*\$\{\{[[:space:]]*steps\.' "$ACTION_YML"
  if [ "$status" -eq 0 ]; then
    echo "forbidden 'steps' context in default:" >&2
    echo "$output" >&2
    return 1
  fi
}

@test "manifest: input defaults do not reference needs context" {
  run grep -nE '^[[:space:]]*default:[[:space:]]*\$\{\{[[:space:]]*needs\.' "$ACTION_YML"
  if [ "$status" -eq 0 ]; then
    echo "forbidden 'needs' context in default:" >&2
    echo "$output" >&2
    return 1
  fi
}

@test "manifest: input defaults do not reference secrets context" {
  run grep -nE '^[[:space:]]*default:[[:space:]]*\$\{\{[[:space:]]*secrets\.' "$ACTION_YML"
  if [ "$status" -eq 0 ]; then
    echo "forbidden 'secrets' context in default:" >&2
    echo "$output" >&2
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
