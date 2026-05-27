#!/usr/bin/env bats

load helpers

setup() {
  setup_clean_env
}

run_validate() {
  # Invoke validate in a clean subshell so exits don't terminate bats.
  bash -c '
    set -e
    source "'"${NF_ROOT}"'/scripts/lib.sh"
    source "'"${NF_ROOT}"'/scripts/validate.sh"
    nf::validate
    # print resolved values for inspection
    printf "NF_STATUS=%s\n"     "$NF_STATUS"
    printf "NF_PARSE_MODE=%s\n" "$NF_PARSE_MODE"
    printf "NF_NOTIFY_ON=%s\n"  "$NF_NOTIFY_ON"
  '
}

@test "validate: missing bot_token exits 10" {
  export NF_BOT_TOKEN="" NF_CHAT_ID="123" NF_STATUS="success"
  run run_validate
  [ "$status" -eq 10 ]
  [[ "$output" == *"MISSING_REQUIRED_INPUT"* ]]
}

@test "validate: missing chat_id exits 10" {
  export NF_BOT_TOKEN="t" NF_CHAT_ID="" NF_STATUS="success"
  run run_validate
  [ "$status" -eq 10 ]
}

@test "validate: chat_id positive integer accepted" {
  export NF_BOT_TOKEN="t" NF_CHAT_ID="12345" NF_STATUS="success"
  run run_validate
  [ "$status" -eq 0 ]
}

@test "validate: chat_id negative integer accepted" {
  export NF_BOT_TOKEN="t" NF_CHAT_ID="-100123456789" NF_STATUS="success"
  run run_validate
  [ "$status" -eq 0 ]
}

@test "validate: chat_id @username accepted" {
  export NF_BOT_TOKEN="t" NF_CHAT_ID="@my_channel" NF_STATUS="success"
  run run_validate
  [ "$status" -eq 0 ]
}

@test "validate: chat_id 'abc' rejected with exit 11" {
  export NF_BOT_TOKEN="t" NF_CHAT_ID="abc" NF_STATUS="success"
  run run_validate
  [ "$status" -eq 11 ]
  [[ "$output" == *"INVALID_CHAT_ID"* ]]
}

@test "validate: missing status exits 10" {
  export NF_BOT_TOKEN="t" NF_CHAT_ID="1" NF_STATUS=""
  run run_validate
  [ "$status" -eq 10 ]
  [[ "$output" == *"MISSING_REQUIRED_INPUT"* ]]
}

@test "validate: JOB_STATUS env is no longer a fallback" {
  # status must be passed explicitly — legacy JOB_STATUS fallback was removed
  # when default: ${{ job.status }} was dropped from action.yml.
  export NF_BOT_TOKEN="t" NF_CHAT_ID="1" NF_STATUS="" JOB_STATUS="success"
  run run_validate
  [ "$status" -eq 10 ]
  [[ "$output" == *"MISSING_REQUIRED_INPUT"* ]]
}

@test "validate: invalid status exits 12" {
  export NF_BOT_TOKEN="t" NF_CHAT_ID="1" NF_STATUS="wat"
  run run_validate
  [ "$status" -eq 12 ]
  [[ "$output" == *"INVALID_STATUS"* ]]
}

@test "validate: parse_mode defaults to MarkdownV2" {
  export NF_BOT_TOKEN="t" NF_CHAT_ID="1" NF_STATUS="success" NF_PARSE_MODE=""
  run run_validate
  [ "$status" -eq 0 ]
  [[ "$output" == *"NF_PARSE_MODE=MarkdownV2"* ]]
}

@test "validate: parse_mode 'BBCode' exits 13" {
  export NF_BOT_TOKEN="t" NF_CHAT_ID="1" NF_STATUS="success" NF_PARSE_MODE="BBCode"
  run run_validate
  [ "$status" -eq 13 ]
  [[ "$output" == *"INVALID_PARSE_MODE"* ]]
}

@test "validate: parse_mode 'Markdown' upgraded to MarkdownV2" {
  # Legacy `Markdown` is silently rewritten so escape rules and Telegram's
  # parse_mode stay consistent (notiflow's escaper is V2-only).
  export NF_BOT_TOKEN="t" NF_CHAT_ID="1" NF_STATUS="success" NF_PARSE_MODE="Markdown"
  run run_validate
  [ "$status" -eq 0 ]
  [[ "$output" == *"NF_PARSE_MODE=MarkdownV2"* ]]
  [[ "$output" == *"::warning::"* ]]
  [[ "$output" == *"Markdown upgraded to MarkdownV2"* ]]
}

@test "validate: parse_mode 'none' accepted" {
  export NF_BOT_TOKEN="t" NF_CHAT_ID="1" NF_STATUS="success" NF_PARSE_MODE="none"
  run run_validate
  [ "$status" -eq 0 ]
}

@test "validate: notify_on defaults" {
  export NF_BOT_TOKEN="t" NF_CHAT_ID="1" NF_STATUS="success" NF_NOTIFY_ON=""
  run run_validate
  [ "$status" -eq 0 ]
  [[ "$output" == *"NF_NOTIFY_ON=success,failure,cancelled"* ]]
}

@test "validate: notify_on 'any' expands to all four statuses" {
  export NF_BOT_TOKEN="t" NF_CHAT_ID="1" NF_STATUS="success" NF_PARSE_MODE="none"
  export NF_NOTIFY_ON="any"
  run run_validate
  [ "$status" -eq 0 ]
  [[ "$output" == *"NF_NOTIFY_ON=success,failure,cancelled,skipped"* ]]
}

@test "validate: notify_on 'all' expands to all four statuses" {
  export NF_BOT_TOKEN="t" NF_CHAT_ID="1" NF_STATUS="success" NF_PARSE_MODE="none"
  export NF_NOTIFY_ON="all"
  run run_validate
  [ "$status" -eq 0 ]
  [[ "$output" == *"NF_NOTIFY_ON=success,failure,cancelled,skipped"* ]]
}

@test "validate: invalid notify_on item exits 14" {
  export NF_BOT_TOKEN="t" NF_CHAT_ID="1" NF_STATUS="success" NF_NOTIFY_ON="failure,wat"
  run run_validate
  [ "$status" -eq 14 ]
  [[ "$output" == *"INVALID_NOTIFY_ON"* ]]
}

@test "validate: thread_id integer accepted" {
  export NF_BOT_TOKEN="t" NF_CHAT_ID="1" NF_STATUS="success" NF_MESSAGE_THREAD_ID="123"
  run run_validate
  [ "$status" -eq 0 ]
}

@test "validate: thread_id non-integer exits 15" {
  export NF_BOT_TOKEN="t" NF_CHAT_ID="1" NF_STATUS="success" NF_MESSAGE_THREAD_ID="abc"
  run run_validate
  [ "$status" -eq 15 ]
  [[ "$output" == *"INVALID_THREAD_ID"* ]]
}
