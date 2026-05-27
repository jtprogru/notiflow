#!/usr/bin/env bats

load helpers

setup() {
  setup_clean_env
  export NF_BOT_TOKEN="testtoken123"
  export NF_CHAT_ID="42"
  export NF_PARSE_MODE="MarkdownV2"
  export NF_DISABLE_WEB_PAGE_PREVIEW="true"
  export NF_DISABLE_NOTIFICATION="false"
  # shellcheck source=../scripts/lib.sh
  source "${NF_ROOT}/scripts/lib.sh"
  # shellcheck source=../scripts/send.sh
  source "${NF_ROOT}/scripts/send.sh"
}

teardown() {
  mock_telegram_stop
}

@test "send: 200 success sets outputs" {
  mock_telegram_start "200:success"
  run nf::send "hello"
  [ "$status" -eq 0 ]
  assert_output_eq ok true
  assert_output_eq message_id 42
  assert_output_eq http_status 200
  [ "$(count_mock_requests)" -eq 1 ]
}

@test "send: JSON body contains required fields (CP-15)" {
  mock_telegram_start "200:success"
  nf::send "hi"
  local body
  body=$(last_mock_body)
  echo "$body" | jq -e '.chat_id == 42' >/dev/null
  echo "$body" | jq -e '.text == "hi"' >/dev/null
  echo "$body" | jq -e '.parse_mode == "MarkdownV2"' >/dev/null
  echo "$body" | jq -e '.disable_web_page_preview == true' >/dev/null
  echo "$body" | jq -e '.disable_notification == false' >/dev/null
}

@test "send: parse_mode omitted when none" {
  export NF_PARSE_MODE="none"
  mock_telegram_start "200:success"
  nf::send "hi"
  ! last_mock_body | jq -e 'has("parse_mode")' >/dev/null
}

@test "send: thread_id omitted when empty" {
  mock_telegram_start "200:success"
  nf::send "hi"
  ! last_mock_body | jq -e 'has("message_thread_id")' >/dev/null
}

@test "send: thread_id included as integer when set" {
  export NF_MESSAGE_THREAD_ID="7"
  mock_telegram_start "200:success"
  nf::send "hi"
  last_mock_body | jq -e '.message_thread_id == 7' >/dev/null
}

@test "send: 429 with retry_after retries and succeeds (REQ-7.2, CP-8)" {
  mock_telegram_start "429:rate_limit,200:success"
  local start end
  start=$(date +%s)
  run nf::send "hi"
  end=$(date +%s)
  [ "$status" -eq 0 ]
  [ "$(count_mock_requests)" -eq 2 ]
  # 429 fixture asks retry_after=1 → at least 1 second elapsed.
  [ $((end - start)) -ge 1 ]
}

@test "send: 429 retry limit = 4 attempts (REQ-7.2)" {
  mock_telegram_start "429:rate_limit,429:rate_limit,429:rate_limit,429:rate_limit,429:rate_limit"
  run nf::send "hi"
  [ "$status" -eq 1 ]
  [ "$(count_mock_requests)" -eq 4 ]
  assert_output_eq ok false
}

@test "send: 5xx exponential backoff then success (REQ-7.3, CP-9)" {
  mock_telegram_start "500:server_error,500:server_error,200:success"
  local start end
  start=$(date +%s)
  run nf::send "hi"
  end=$(date +%s)
  [ "$status" -eq 0 ]
  [ "$(count_mock_requests)" -eq 3 ]
  # Backoff after attempts 1 and 2 → 1 + 2 = 3 seconds minimum.
  [ $((end - start)) -ge 3 ]
}

@test "send: 5xx retry limit = 4 attempts (REQ-7.3)" {
  mock_telegram_start "500:server_error,500:server_error,500:server_error,500:server_error,500:server_error"
  run nf::send "hi"
  [ "$status" -eq 1 ]
  [ "$(count_mock_requests)" -eq 4 ]
  assert_output_eq ok false
}

@test "send: 400 no retry (REQ-7.6, CP-10)" {
  mock_telegram_start "400:bad_request"
  run nf::send "hi"
  [ "$status" -eq 1 ]
  [ "$(count_mock_requests)" -eq 1 ]
  assert_output_eq ok false
  assert_output_eq http_status 400
}

@test "send: 401 no retry (CP-10)" {
  mock_telegram_start "401:unauthorized"
  run nf::send "hi"
  [ "$status" -eq 1 ]
  [ "$(count_mock_requests)" -eq 1 ]
  assert_output_eq http_status 401
}

@test "send: 404 no retry (CP-10)" {
  mock_telegram_start "404:not_found"
  run nf::send "hi"
  [ "$status" -eq 1 ]
  [ "$(count_mock_requests)" -eq 1 ]
}

@test "send: token never appears in stderr (CP-11)" {
  local logf="${BATS_TEST_TMPDIR}/stderr.log"
  mock_telegram_start "500:server_error,400:bad_request"
  nf::send "hi" 2>"$logf" || true # we expect failure (400), only checking log
  ! grep -F -- "$NF_BOT_TOKEN" "$logf"
}
