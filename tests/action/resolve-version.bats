#!/usr/bin/env bats
# Acceptance tests for scripts/action/resolve-version.sh.
#
# The precedence chain decides which release the Action downloads, so getting it wrong is
# not a cosmetic bug: it installs the wrong binary, or nothing at all. The `latest` branch
# is not exercised here — it needs the Releases API — but everything that guards it is.

load helpers

setup() {
  NF_ROOT="${BATS_TEST_DIRNAME%/tests/action}"
  RESOLVE="${NF_ROOT}/scripts/action/resolve-version.sh"

  export GITHUB_OUTPUT="${BATS_TEST_TMPDIR}/gh_output"
  : >"$GITHUB_OUTPUT"

  # A stand-in for the Action's own tree, so the VERSION-file branch has something to read.
  export GITHUB_ACTION_PATH="${BATS_TEST_TMPDIR}/action"
  mkdir -p "$GITHUB_ACTION_PATH"

  unset INPUT_VERSION GITHUB_ACTION_REF || true
}

resolved() {
  read_output version
}

# Shadow the gh CLI with one that answers the releases/latest query with $1, so the
# fallback branch is testable without a network call or a token.
fake_gh() {
  local bin="${BATS_TEST_TMPDIR}/bin"
  mkdir -p "$bin"
  cat >"${bin}/gh" <<EOF
#!/usr/bin/env bash
echo "$1"
EOF
  chmod +x "${bin}/gh"
  export PATH="${bin}:${PATH}"
}

@test "resolve-version: the version input wins over everything else" {
  echo "v2.9.9" >"${GITHUB_ACTION_PATH}/VERSION"
  export INPUT_VERSION="v2.0.0"
  export GITHUB_ACTION_REF="v2.5.0"

  run bash "$RESOLVE"
  [ "$status" -eq 0 ]
  [ "$(resolved)" = "v2.0.0" ]
}

@test "resolve-version: the stamped VERSION file is used when no input is given" {
  echo "v2.3.4" >"${GITHUB_ACTION_PATH}/VERSION"

  run bash "$RESOLVE"
  [ "$status" -eq 0 ]
  [ "$(resolved)" = "v2.3.4" ]
  [[ "$output" == *"VERSION file"* ]]
}

@test "resolve-version: a semver GITHUB_ACTION_REF is used when there is no VERSION file" {
  export GITHUB_ACTION_REF="v2.1.0"

  run bash "$RESOLVE"
  [ "$status" -eq 0 ]
  [ "$(resolved)" = "v2.1.0" ]
  [[ "$output" == *"GITHUB_ACTION_REF"* ]]
}

@test "resolve-version: a branch-shaped GITHUB_ACTION_REF falls through to the latest lookup" {
  # @main must not resolve to `main` and hand install.sh a URL built from a branch name.
  export GITHUB_ACTION_REF="main"
  fake_gh "v2.4.0"

  run bash "$RESOLVE"
  [ "$status" -eq 0 ]
  [ "$(resolved)" = "v2.4.0" ]
  [[ "$output" == *"pin a version for reproducible runs"* ]]
}

@test "resolve-version: a latest lookup that lands on v1 is rejected" {
  # releases/latest skips pre-releases, so until v2.0.0 ships it answers with a v1 tag —
  # a release with no binary archive in it.
  fake_gh "v1.6.1"

  run bash "$RESOLVE"
  [ "$status" -ne 0 ]
  [[ "$output" == *"v2.0.0 onward"* ]]
}

@test "resolve-version: a bare version is normalised with a leading v" {
  export INPUT_VERSION="2.0.0"

  run bash "$RESOLVE"
  [ "$status" -eq 0 ]
  [ "$(resolved)" = "v2.0.0" ]
}

@test "resolve-version: a v1 release is rejected because it ships no binary" {
  # v1 was bash. Its releases have no archive to download, so resolving one can only end in
  # a 404 inside install.sh, several steps away from the input that caused it.
  export INPUT_VERSION="v1.6.1"

  run bash "$RESOLVE"
  [ "$status" -ne 0 ]
  [[ "$output" == *"v2.0.0 onward"* ]]
  [ ! -s "$GITHUB_OUTPUT" ]
}

@test "resolve-version: a pre-release version resolves unchanged" {
  export INPUT_VERSION="v2.0.0-rc.1"

  run bash "$RESOLVE"
  [ "$status" -eq 0 ]
  [ "$(resolved)" = "v2.0.0-rc.1" ]
}
