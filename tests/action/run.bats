#!/usr/bin/env bats
#
# Acceptance tests for the Action wrapper.
#
# These are the descendants of v1's entrypoint.bats: same contract, same mock server, but
# driving scripts/action/run.sh and the Rust binary instead of the bash pipeline. They are
# what proves the NF_* environment action.yml builds still lands on the right arguments.

load helpers

setup() {
  setup_clean_env
}

teardown() {
  mock_telegram_stop
}

@test "action: a successful send writes the four outputs" {
  mock_telegram_start "200:success"
  run "$NF_RUN"
  [ "$status" -eq 0 ]
  assert_output_eq ok true
  assert_output_eq message_id 42
  assert_output_eq http_status 200
  assert_output_eq error ""
  [ "$(count_mock_requests)" -eq 1 ]
}

@test "action: the request targets sendMessage with the mapped fields" {
  mock_telegram_start "200:success"
  export NF_MESSAGE="hello from the wrapper"
  export NF_DISABLE_NOTIFICATION="true"
  export NF_MESSAGE_THREAD_ID="7"
  run "$NF_RUN"
  [ "$status" -eq 0 ]
  [[ "$(last_mock_path)" == */sendMessage ]]
  body=$(last_mock_body)
  echo "$body" | jq -e '.text == "hello from the wrapper"'
  echo "$body" | jq -e '.disable_notification == true'
  echo "$body" | jq -e '.message_thread_id == 7'
  echo "$body" | jq -e '.link_preview_options.is_disabled == true'
}

@test "action: disable_web_page_preview=false enables previews" {
  mock_telegram_start "200:success"
  export NF_DISABLE_WEB_PAGE_PREVIEW="false"
  run "$NF_RUN"
  [ "$status" -eq 0 ]
  last_mock_body | jq -e '.link_preview_options.is_disabled == false'
}

@test "action: a status outside notify_on skips without a request" {
  mock_telegram_start "200:success"
  export NF_STATUS="skipped"
  run "$NF_RUN"
  [ "$status" -eq 0 ]
  assert_output_eq ok false
  assert_output_eq message_id ""
  assert_output_eq http_status 0
  [ "$(count_mock_requests)" -eq 0 ]
}

@test "action: notify_on=any covers every status" {
  mock_telegram_start "200:success"
  export NF_STATUS="skipped"
  export NF_NOTIFY_ON="any"
  run "$NF_RUN"
  [ "$status" -eq 0 ]
  assert_output_eq ok true
  [ "$(count_mock_requests)" -eq 1 ]
}

@test "action: edit_message_id switches to editMessageText" {
  mock_telegram_start "200:success"
  export NF_EDIT_MESSAGE_ID="42"
  export NF_MESSAGE="corrected"
  run "$NF_RUN"
  [ "$status" -eq 0 ]
  [[ "$(last_mock_path)" == */editMessageText ]]
  last_mock_body | jq -e '.message_id == 42'
}

@test "action: a 4xx is reported without retrying" {
  mock_telegram_start "400:bad_request"
  run "$NF_RUN"
  [ "$status" -eq 0 ]
  assert_output_eq ok false
  assert_output_eq http_status 400
  assert_output_eq error "Bad Request: chat not found"
  [ "$(count_mock_requests)" -eq 1 ]
}

@test "action: fail_on_error=true turns a failure into a non-zero exit" {
  mock_telegram_start "400:bad_request"
  export NF_FAIL_ON_ERROR="true"
  run "$NF_RUN"
  [ "$status" -eq 1 ]
  assert_output_eq ok false
}

@test "action: fail_on_error=false leaves the job result intact" {
  mock_telegram_start "400:bad_request"
  export NF_FAIL_ON_ERROR="false"
  run "$NF_RUN"
  [ "$status" -eq 0 ]
}

@test "action: a rate limit is retried" {
  mock_telegram_start "429:rate_limit,200:success"
  export NF_MAX_RETRY_AFTER="1"
  run "$NF_RUN"
  [ "$status" -eq 0 ]
  assert_output_eq ok true
  [ "$(count_mock_requests)" -eq 2 ]
}

@test "action: an invalid chat_id exits 11 before any request" {
  mock_telegram_start "200:success"
  export NF_CHAT_ID="not-a-chat"
  run "$NF_RUN"
  [ "$status" -eq 11 ]
  [ "$(count_mock_requests)" -eq 0 ]
}

@test "action: a comma-separated chat_id is rejected, not sent as one chat" {
  mock_telegram_start "200:success"
  export NF_CHAT_ID="-1001234567890,-1009876543210"
  run "$NF_RUN"
  [ "$status" -eq 11 ]
  [ "$(count_mock_requests)" -eq 0 ]
}

@test "action: a missing bot_token exits 10" {
  export NF_BOT_TOKEN=""
  run "$NF_RUN"
  [ "$status" -eq 10 ]
}

@test "action: the token is masked and never printed" {
  mock_telegram_start "200:success"
  run "$NF_RUN"
  [ "$status" -eq 0 ]
  [[ "$output" == *"::add-mask::${NF_TEST_TOKEN}"* ]]
  # One mask line is expected; the token must appear nowhere else.
  occurrences=$(grep -o -F -- "$NF_TEST_TOKEN" <<<"$output" | wc -l | tr -d ' ')
  [ "$occurrences" -eq 1 ]
  ! grep -q -F -- "$NF_TEST_TOKEN" "$GITHUB_OUTPUT"
  ! grep -q -F -- "$NF_TEST_TOKEN" "$GITHUB_STEP_SUMMARY"
}

@test "action: a foreign api_base is ignored inside a workflow" {
  mock_telegram_start "200:success"
  export NF_API_BASE="https://api.telegram.org.evil.example"
  export NF_DRY_RUN="true"
  run "$NF_RUN"
  [ "$status" -eq 0 ]
  [[ "$output" == *"host not allowed"* ]]
  [ "$(count_mock_requests)" -eq 0 ]
}

@test "action: the step summary records the outcome" {
  mock_telegram_start "200:success"
  run "$NF_RUN"
  [ "$status" -eq 0 ]
  grep -q "| result | sent |" "$GITHUB_STEP_SUMMARY"
  grep -q "| message_id | \`42\` |" "$GITHUB_STEP_SUMMARY"
}
