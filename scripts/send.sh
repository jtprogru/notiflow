# shellcheck shell=bash
# HTTP send + retry logic for Telegram Bot API. Source-only.

[ -n "${_NF_SEND_LOADED:-}" ] && return 0
_NF_SEND_LOADED=1

# Number of attempts: 1 primary + up to 3 retries = 4 total.
_NF_MAX_ATTEMPTS=4
# Exponential backoff base: attempt 1 → 1s, attempt 2 → 2s, attempt 3 → 4s.

# _nf::_build_json <text>
# Constructs the JSON request body for sendMessage. Uses jq so we don't have to
# worry about quoting. Output is a single-line JSON object.
_nf::_build_json() {
  local text="$1"
  local thread_arg=""
  local dwpp dnotif

  case "${NF_DISABLE_WEB_PAGE_PREVIEW:-true}" in
    true) dwpp=true ;;
    false) dwpp=false ;;
    *) dwpp=true ;;
  esac
  case "${NF_DISABLE_NOTIFICATION:-false}" in
    true) dnotif=true ;;
    false) dnotif=false ;;
    *) dnotif=false ;;
  esac

  if [ -n "${NF_MESSAGE_THREAD_ID:-}" ]; then
    thread_arg=1
  fi

  local pm="${NF_PARSE_MODE:-MarkdownV2}"

  jq -n \
    --arg chat_id "$NF_CHAT_ID" \
    --arg text "$text" \
    --arg parse_mode "$pm" \
    --argjson dwpp "$dwpp" \
    --argjson dnotif "$dnotif" \
    --arg thread_id "${NF_MESSAGE_THREAD_ID:-}" \
    --argjson include_thread "${thread_arg:-0}" \
    '
    ({
       chat_id: ( ($chat_id | tonumber?) // $chat_id ),
       text: $text,
       disable_web_page_preview: $dwpp,
       disable_notification: $dnotif
     }
     + (if $parse_mode == "none" or $parse_mode == "" then {} else {parse_mode: $parse_mode} end)
     + (if $include_thread == 1 then {message_thread_id: ($thread_id | tonumber)} else {} end))
    '
}

# _nf::_post <json>
# Performs one HTTP POST. Prints "<http_code>\n<body>".
# Timeouts are bounded so a hung peer cannot stall the workflow:
#   NF_CONNECT_TIMEOUT — TCP/TLS handshake budget (seconds, default 5)
#   NF_MAX_TIME        — whole-request budget (seconds, default 15)
# Exceeding either causes curl to exit non-zero; the caller treats this as a
# network error and applies the same backoff/retry path as a 5xx.
_nf::_post() {
  local json="$1"
  local base="${NF_API_BASE:-https://api.telegram.org}"
  local url="${base}/bot${NF_BOT_TOKEN}/sendMessage"
  local connect_timeout="${NF_CONNECT_TIMEOUT:-5}"
  local max_time="${NF_MAX_TIME:-15}"
  curl -sS -o - -w '\n%{http_code}' \
    --connect-timeout "$connect_timeout" \
    --max-time "$max_time" \
    -X POST \
    -H 'Content-Type: application/json' \
    --data-binary @- \
    "$url" <<<"$json"
}

# nf::send <text>
# Sends the message. Sets outputs ok/message_id/http_status. Returns 0 on
# success, 1 on failure. The caller (entrypoint) maps return code to exit
# code based on fail_on_error.
nf::send() {
  local text="$1"
  local json
  json=$(_nf::_build_json "$text")

  local attempt=1
  local http_status=0
  local body=""
  local raw

  while [ "$attempt" -le "$_NF_MAX_ATTEMPTS" ]; do
    local curl_exit
    raw=$(_nf::_post "$json") || curl_exit=$?
    curl_exit=${curl_exit:-0}

    if [ "$curl_exit" -ne 0 ]; then
      nf::log warn "curl failed (exit=$curl_exit) attempt=$attempt"
      http_status=0
      body=""
    else
      # Last line of $raw is the http code; everything before is the body.
      http_status=$(printf '%s' "$raw" | tail -n1)
      body=$(printf '%s' "$raw" | sed '$d')
    fi
    unset curl_exit

    case "$http_status" in
      200)
        if printf '%s' "$body" | jq -e '.ok == true' >/dev/null 2>&1; then
          local mid
          mid=$(printf '%s' "$body" | jq -r '.result.message_id')
          nf::set_output ok true
          nf::set_output message_id "$mid"
          nf::set_output http_status 200
          return 0
        fi
        nf::log warn "Telegram returned 200 but ok=false attempt=$attempt"
        ;;
      429)
        # Telegram occasionally returns pathological retry_after values
        # (hundreds or thousands of seconds). Without a cap a single
        # rate-limited request could stall the workflow for an hour.
        # Validate the value, then bound it by NF_MAX_RETRY_AFTER (default 60).
        local retry_after_raw retry_after cap
        retry_after_raw=$(printf '%s' "$body" | jq -r '.parameters.retry_after // 1' 2>/dev/null)
        case "${retry_after_raw:-}" in
          '' | *[!0-9]*) retry_after=1 ;;
          *) retry_after="$retry_after_raw" ;;
        esac
        cap="${NF_MAX_RETRY_AFTER:-60}"
        if [ "$retry_after" -gt "$cap" ]; then
          nf::log warn "429 retry_after=${retry_after}s capped at ${cap}s"
          retry_after="$cap"
        fi
        if [ "$attempt" -lt "$_NF_MAX_ATTEMPTS" ]; then
          nf::log warn "429 rate-limited; sleeping ${retry_after}s (attempt=$attempt)"
          sleep "$retry_after"
        fi
        ;;
      5*)
        if [ "$attempt" -lt "$_NF_MAX_ATTEMPTS" ]; then
          local delay=$((1 << (attempt - 1)))
          nf::log warn "5xx ($http_status); backoff ${delay}s (attempt=$attempt)"
          sleep "$delay"
        fi
        ;;
      0)
        # network error — backoff like 5xx
        if [ "$attempt" -lt "$_NF_MAX_ATTEMPTS" ]; then
          local delay=$((1 << (attempt - 1)))
          nf::log warn "network error; backoff ${delay}s (attempt=$attempt)"
          sleep "$delay"
        fi
        ;;
      4*)
        nf::log error "Telegram returned $http_status (no retry): $(printf '%s' "$body" | jq -r '.description // "."' 2>/dev/null)"
        nf::set_output ok false
        nf::set_output message_id ""
        nf::set_output http_status "$http_status"
        return 1
        ;;
      *)
        nf::log warn "unexpected http_status=$http_status attempt=$attempt"
        ;;
    esac

    attempt=$((attempt + 1))
  done

  nf::log warn "send failed after $_NF_MAX_ATTEMPTS attempts (last http_status=$http_status)"
  nf::set_output ok false
  nf::set_output message_id ""
  nf::set_output http_status "$http_status"
  return 1
}
