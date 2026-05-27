#!/usr/bin/env bats

load helpers

setup() {
  setup_clean_env
  export NF_BOT_TOKEN="testtoken123"
  export NF_CHAT_ID="42"
  export NF_PARSE_MODE="MarkdownV2"
  export NF_NOTIFY_ON="success,failure,cancelled"
  export NF_DISABLE_WEB_PAGE_PREVIEW="true"
  export NF_DISABLE_NOTIFICATION="false"
  export NF_FAIL_ON_ERROR="false"
}

teardown() {
  mock_telegram_stop
}

@test "entrypoint: happy path 200 sets outputs and exits 0" {
  mock_telegram_start "200:success"
  export NF_STATUS="success"
  run "${NF_ROOT}/scripts/entrypoint.sh"
  [ "$status" -eq 0 ]
  assert_output_eq ok true
  assert_output_eq message_id 42
  assert_output_eq http_status 200
  # JSON should contain text rendered from default template.
  last_mock_body | jq -e '.text | test("CI")' >/dev/null
}

@test "entrypoint: skip on filter, no HTTP, exit 0 (REQ-3.2, REQ-8.2)" {
  mock_telegram_start "200:success"
  export NF_STATUS="success"
  export NF_NOTIFY_ON="failure,cancelled"
  run "${NF_ROOT}/scripts/entrypoint.sh"
  [ "$status" -eq 0 ]
  assert_output_eq ok false
  assert_output_eq http_status 0
  [ "$(count_mock_requests)" -eq 0 ]
}

@test "entrypoint: fail_on_error=true exits non-zero on 400 (REQ-7.4, CP-13)" {
  mock_telegram_start "400:bad_request"
  export NF_STATUS="failure" NF_FAIL_ON_ERROR="true"
  run "${NF_ROOT}/scripts/entrypoint.sh"
  [ "$status" -ne 0 ]
  assert_output_eq ok false
  assert_output_eq http_status 400
}

@test "entrypoint: fail_on_error=false exits 0 on 400 (REQ-7.5, CP-13)" {
  mock_telegram_start "400:bad_request"
  export NF_STATUS="failure" NF_FAIL_ON_ERROR="false"
  run "${NF_ROOT}/scripts/entrypoint.sh"
  [ "$status" -eq 0 ]
  assert_output_eq ok false
  assert_output_eq http_status 400
  [[ "$output" == *"::warning::"* ]]
}

@test "entrypoint: status default from JOB_STATUS env (REQ-1.3, CP-16)" {
  mock_telegram_start "200:success"
  unset NF_STATUS
  export JOB_STATUS="success"
  run "${NF_ROOT}/scripts/entrypoint.sh"
  [ "$status" -eq 0 ]
  [ "$(count_mock_requests)" -eq 1 ]
}

@test "entrypoint: token only appears in mask directive (CP-11)" {
  mock_telegram_start "500:server_error,500:server_error,500:server_error,500:server_error"
  export NF_STATUS="failure"
  run "${NF_ROOT}/scripts/entrypoint.sh"
  # The single legitimate occurrence is the ::add-mask:: directive that
  # tells GitHub Actions to redact the token in subsequent logs. Anywhere
  # else (request URLs, error bodies, warnings) is a leak.
  ! grep -F -- "$NF_BOT_TOKEN" <<<"$output" | grep -vF '::add-mask::'
}
