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

@test "send: --max-time triggers retry then success on hang" {
  export NF_CONNECT_TIMEOUT=1
  export NF_MAX_TIME=1
  mock_telegram_start "200:hang,200:success"
  local start end
  start=$(date +%s)
  run nf::send "hi"
  end=$(date +%s)
  [ "$status" -eq 0 ]
  [ "$(count_mock_requests)" -eq 2 ]
  assert_output_eq ok true
  # First attempt times out in ~1s, 1s backoff, second succeeds. Budget < 10s.
  [ $((end - start)) -lt 10 ]
}

@test "send: hung peer exhausts retries within bounded time" {
  export NF_CONNECT_TIMEOUT=1
  export NF_MAX_TIME=1
  mock_telegram_start "200:hang,200:hang,200:hang,200:hang"
  local start end
  start=$(date +%s)
  run nf::send "hi"
  end=$(date +%s)
  [ "$status" -eq 1 ]
  [ "$(count_mock_requests)" -eq 4 ]
  assert_output_eq ok false
  assert_output_eq http_status 0
  # 4 attempts × ~1s max-time + (1+2+4)s backoff ≈ 11s; budget < 20s.
  [ $((end - start)) -lt 20 ]
  # And it really did wait, not bail instantly.
  [ $((end - start)) -ge 4 ]
}

@test "send: curl receives --connect-timeout and --max-time flags" {
  # Stub curl in a temp PATH dir; record its argv and emit a synthetic 200.
  local stub_dir="${BATS_TEST_TMPDIR}/stub"
  local argv_log="${BATS_TEST_TMPDIR}/curl.argv"
  mkdir -p "$stub_dir"
  cat >"${stub_dir}/curl" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$@" >"${argv_log}"
printf '{"ok":true,"result":{"message_id":42}}\n200\n'
EOF
  chmod +x "${stub_dir}/curl"
  PATH="${stub_dir}:${PATH}" nf::send "hi"
  grep -Fxq -- '--connect-timeout' "$argv_log"
  grep -Fxq -- '--max-time' "$argv_log"
  # Default values from send.sh.
  awk 'p {print; exit} /^--connect-timeout$/ {p=1}' "$argv_log" | grep -Fxq '5'
  awk 'p {print; exit} /^--max-time$/ {p=1}' "$argv_log" | grep -Fxq '15'
}

@test "send: 429 huge retry_after is capped by NF_MAX_RETRY_AFTER" {
  export NF_MAX_RETRY_AFTER=1
  mock_telegram_start "429:rate_limit_huge,200:success"
  local start end
  start=$(date +%s)
  run nf::send "hi"
  end=$(date +%s)
  [ "$status" -eq 0 ]
  [ "$(count_mock_requests)" -eq 2 ]
  # Without the cap the test would sleep 9999s. Cap=1 → total < 10s.
  [ $((end - start)) -lt 10 ]
}

@test "send: 429 non-integer retry_after falls back to 1s" {
  mock_telegram_start "429:rate_limit_garbage,200:success"
  local start end
  start=$(date +%s)
  run nf::send "hi"
  end=$(date +%s)
  [ "$status" -eq 0 ]
  [ "$(count_mock_requests)" -eq 2 ]
  # Fallback is 1s; total should land in [1, 10).
  [ $((end - start)) -ge 1 ]
  [ $((end - start)) -lt 10 ]
}

@test "send: 429 cap default is 60s when NF_MAX_RETRY_AFTER unset" {
  # Stub sleep to capture its argument without actually waiting.
  local stub_dir="${BATS_TEST_TMPDIR}/stub"
  local sleep_log="${BATS_TEST_TMPDIR}/sleep.arg"
  mkdir -p "$stub_dir"
  cat >"${stub_dir}/sleep" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$1" >>"${sleep_log}"
EOF
  chmod +x "${stub_dir}/sleep"
  mock_telegram_start "429:rate_limit_huge,200:success"
  PATH="${stub_dir}:${PATH}" nf::send "hi"
  # First (and only) sleep should be the capped value, 60.
  [ "$(head -n1 "$sleep_log")" = "60" ]
}

@test "send: NF_API_BASE allowlist accepts api.telegram.org and loopback" {
  [ "$(_nf::_resolve_api_base 'https://api.telegram.org')" = "https://api.telegram.org" ]
  [ "$(_nf::_resolve_api_base 'https://api.telegram.org/bot123/sendMessage')" = "https://api.telegram.org/bot123/sendMessage" ]
  [ "$(_nf::_resolve_api_base 'http://127.0.0.1:8080')" = "http://127.0.0.1:8080" ]
  [ "$(_nf::_resolve_api_base 'http://localhost:8080')" = "http://localhost:8080" ]
  [ "$(_nf::_resolve_api_base 'http://127.0.0.1:8080/path')" = "http://127.0.0.1:8080/path" ]
}

@test "send: NF_API_BASE allowlist rejects exfiltration attempts" {
  # Arbitrary host
  [ "$(_nf::_resolve_api_base 'https://evil.com' 2>/dev/null)" = "https://api.telegram.org" ]
  # Userinfo trick — curl would resolve evil.com, not 127.0.0.1
  [ "$(_nf::_resolve_api_base 'http://127.0.0.1:8080@evil.com' 2>/dev/null)" = "https://api.telegram.org" ]
  # Look-alike subdomain
  [ "$(_nf::_resolve_api_base 'https://api.telegram.org.evil.com' 2>/dev/null)" = "https://api.telegram.org" ]
  # http (not https) for api.telegram.org → rejected too, avoid downgrade
  [ "$(_nf::_resolve_api_base 'http://api.telegram.org' 2>/dev/null)" = "https://api.telegram.org" ]
  # Each rejection logs a warning.
  local err
  err=$(_nf::_resolve_api_base 'https://evil.com' 2>&1 >/dev/null)
  [[ "$err" == *"warning"* ]]
  [[ "$err" == *"evil.com"* ]]
}

@test "send: NF_CONNECT_TIMEOUT and NF_MAX_TIME override defaults" {
  export NF_CONNECT_TIMEOUT=3
  export NF_MAX_TIME=7
  local stub_dir="${BATS_TEST_TMPDIR}/stub"
  local argv_log="${BATS_TEST_TMPDIR}/curl.argv"
  mkdir -p "$stub_dir"
  cat >"${stub_dir}/curl" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$@" >"${argv_log}"
printf '{"ok":true,"result":{"message_id":42}}\n200\n'
EOF
  chmod +x "${stub_dir}/curl"
  PATH="${stub_dir}:${PATH}" nf::send "hi"
  awk 'p {print; exit} /^--connect-timeout$/ {p=1}' "$argv_log" | grep -Fxq '3'
  awk 'p {print; exit} /^--max-time$/ {p=1}' "$argv_log" | grep -Fxq '7'
}
