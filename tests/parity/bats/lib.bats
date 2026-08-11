#!/usr/bin/env bats

load helpers

setup() {
  setup_clean_env
  # shellcheck source=../v1/lib.sh
  source "${NF_V1}/lib.sh"
}

@test "lib: nf::log writes to stderr" {
  run bash -c 'source "'"${NF_V1}"'/lib.sh"; nf::log info hello 2>/tmp/nf_log_err; cat /tmp/nf_log_err'
  [ "$status" -eq 0 ]
  [[ "$output" == *"::notice::hello"* ]]
}

@test "lib: nf::mask prints add-mask directive" {
  run bash -c 'source "'"${NF_V1}"'/lib.sh"; nf::mask secret_value'
  [ "$status" -eq 0 ]
  [ "$output" = "::add-mask::secret_value" ]
}

@test "lib: nf::set_output appends to GITHUB_OUTPUT" {
  GITHUB_OUTPUT="${BATS_TEST_TMPDIR}/out"
  : >"$GITHUB_OUTPUT"
  export GITHUB_OUTPUT
  run bash -c 'source "'"${NF_V1}"'/lib.sh"; nf::set_output ok true; nf::set_output count 7'
  [ "$status" -eq 0 ]
  grep -qx 'ok=true' "$GITHUB_OUTPUT"
  grep -qx 'count=7' "$GITHUB_OUTPUT"
}

@test "lib: nf::json_escape produces quoted JSON string" {
  result=$(nf::json_escape 'a"b\c')
  [ "$result" = '"a\"b\\c"' ]
}

@test "lib: nf::require_command exits 22 on missing tool" {
  run bash -c 'source "'"${NF_V1}"'/lib.sh"; nf::require_command definitely_not_a_real_command_xyz'
  [ "$status" -eq 22 ]
  [[ "$output" == *"MISSING_DEPENDENCY:definitely_not_a_real_command_xyz"* ]]
}

@test "lib: _nf::_bash_version_ok accepts >=3.2, rejects older" {
  _nf::_bash_version_ok 3 2
  _nf::_bash_version_ok 3 5
  _nf::_bash_version_ok 4 0
  _nf::_bash_version_ok 5 1
  ! _nf::_bash_version_ok 3 1
  ! _nf::_bash_version_ok 3 0
  ! _nf::_bash_version_ok 2 9
  ! _nf::_bash_version_ok 0 0
}

@test "lib: nf::set_output falls back to stderr (not stdout) when GITHUB_OUTPUT unset" {
  unset GITHUB_OUTPUT
  local stdout stderr
  stdout=$(nf::set_output ok true 2>/dev/null)
  [ -z "$stdout" ]
  stderr=$(nf::set_output ok true 2>&1 >/dev/null)
  [ "$stderr" = "ok=true" ]
}
