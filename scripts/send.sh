# shellcheck shell=bash
# HTTP send + retry logic for Telegram Bot API. Source-only.

[ -n "${_NF_SEND_LOADED:-}" ] && return 0
_NF_SEND_LOADED=1

# Number of attempts: 1 primary + up to 3 retries = 4 total.
_NF_MAX_ATTEMPTS=4
# Exponential backoff base: attempt 1 → 1s, attempt 2 → 2s, attempt 3 → 4s.

# _nf::_build_json <chat_id> <text>
# Constructs the JSON request body for sendMessage. Uses jq so we don't have to
# worry about quoting. Output is a single-line JSON object.
_nf::_build_json() {
  local chat_id="$1"
  local text="$2"
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
    --arg chat_id "$chat_id" \
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

# _nf::_resolve_api_base <candidate>
# Returns a safe Telegram API base URL. The default is api.telegram.org; a
# loopback URL is also accepted so the bats mock server can intercept. Any
# other value (including userinfo-tricked URLs like http://127.0.0.1@evil.com
# or look-alikes like https://api.telegram.org.evil.com) is rejected with a
# warning and replaced by the default — env vars set by prior workflow steps
# cannot redirect the bot token to an attacker-controlled host.
_nf::_resolve_api_base() {
  local base="$1"
  if [[ "$base" =~ ^https://api\.telegram\.org(/.*)?$ ]]; then
    printf '%s' "$base"
    return
  fi
  if [[ "$base" =~ ^https?://(127\.0\.0\.1|localhost)(:[0-9]+)?(/.*)?$ ]]; then
    printf '%s' "$base"
    return
  fi
  nf::log warn "ignoring NF_API_BASE='$base' (host not allowed); using default"
  printf 'https://api.telegram.org'
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
  local base
  base=$(_nf::_resolve_api_base "${NF_API_BASE:-https://api.telegram.org}")
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

# _nf::_send_one <chat_id> <text>
# Per-chat retry loop. Returns 0 on success, 1 on terminal failure. Writes
# per-chat results into globals consumed by nf::send:
#   _NF_RESULT_MID         — Telegram message_id on success, empty on failure
#   _NF_RESULT_HTTP_STATUS — last HTTP status code observed for this chat
#   _NF_RESULT_ERROR       — empty on success, populated on failure
# Globals (rather than parsed stdout) keep the retry/jq output of the loop
# from being captured into a variable by the caller.
_nf::_send_one() {
  local chat_id="$1"
  local text="$2"
  local json
  json=$(_nf::_build_json "$chat_id" "$text")

  local attempt=1
  local http_status=0
  local body=""
  local raw
  local last_error=""

  while [ "$attempt" -le "$_NF_MAX_ATTEMPTS" ]; do
    local curl_exit
    raw=$(_nf::_post "$json") || curl_exit=$?
    curl_exit=${curl_exit:-0}

    if [ "$curl_exit" -ne 0 ]; then
      nf::log warn "curl failed (exit=$curl_exit) chat=$chat_id attempt=$attempt"
      http_status=0
      body=""
      last_error="network error (curl exit $curl_exit)"
    else
      http_status=$(printf '%s' "$raw" | tail -n1)
      body=$(printf '%s' "$raw" | sed '$d')
    fi
    unset curl_exit

    case "$http_status" in
      200)
        if printf '%s' "$body" | jq -e '.ok == true' >/dev/null 2>&1; then
          _NF_RESULT_MID=$(printf '%s' "$body" | jq -r '.result.message_id')
          _NF_RESULT_HTTP_STATUS=200
          _NF_RESULT_ERROR=""
          return 0
        fi
        last_error=$(printf '%s' "$body" | jq -r '.description // "200 ok=false"' 2>/dev/null)
        nf::log warn "Telegram returned 200 but ok=false chat=$chat_id attempt=$attempt"
        ;;
      429)
        local retry_after_raw retry_after cap
        retry_after_raw=$(printf '%s' "$body" | jq -r '.parameters.retry_after // 1' 2>/dev/null)
        case "${retry_after_raw:-}" in
          '' | *[!0-9]*) retry_after=1 ;;
          *) retry_after="$retry_after_raw" ;;
        esac
        cap="${NF_MAX_RETRY_AFTER:-60}"
        if [ "$retry_after" -gt "$cap" ]; then
          nf::log warn "429 retry_after=${retry_after}s capped at ${cap}s (chat=$chat_id)"
          retry_after="$cap"
        fi
        last_error=$(printf '%s' "$body" | jq -r '.description // "rate limited"' 2>/dev/null)
        if [ "$attempt" -lt "$_NF_MAX_ATTEMPTS" ]; then
          nf::log warn "429 rate-limited; sleeping ${retry_after}s chat=$chat_id attempt=$attempt"
          sleep "$retry_after"
        fi
        ;;
      5*)
        local desc
        desc=$(printf '%s' "$body" | jq -r '.description // empty' 2>/dev/null)
        if [ -n "$desc" ]; then
          last_error="HTTP $http_status: $desc"
        else
          last_error="HTTP $http_status"
        fi
        if [ "$attempt" -lt "$_NF_MAX_ATTEMPTS" ]; then
          local delay=$((1 << (attempt - 1)))
          nf::log warn "5xx ($http_status); backoff ${delay}s chat=$chat_id attempt=$attempt"
          sleep "$delay"
        fi
        ;;
      0)
        if [ "$attempt" -lt "$_NF_MAX_ATTEMPTS" ]; then
          local delay=$((1 << (attempt - 1)))
          nf::log warn "network error; backoff ${delay}s chat=$chat_id attempt=$attempt"
          sleep "$delay"
        fi
        ;;
      4*)
        local desc4
        desc4=$(printf '%s' "$body" | jq -r '.description // empty' 2>/dev/null)
        if [ -n "$desc4" ]; then
          last_error="$desc4"
        else
          last_error="HTTP $http_status"
        fi
        nf::log error "Telegram returned $http_status (no retry) chat=$chat_id: $last_error"
        _NF_RESULT_MID=""
        _NF_RESULT_HTTP_STATUS="$http_status"
        _NF_RESULT_ERROR="$last_error"
        return 1
        ;;
      *)
        last_error="HTTP $http_status (unexpected)"
        nf::log warn "unexpected http_status=$http_status chat=$chat_id attempt=$attempt"
        ;;
    esac

    attempt=$((attempt + 1))
  done

  nf::log warn "send failed after $_NF_MAX_ATTEMPTS attempts chat=$chat_id (last http_status=$http_status)"
  _NF_RESULT_MID=""
  _NF_RESULT_HTTP_STATUS="$http_status"
  _NF_RESULT_ERROR="${last_error:-send failed}"
  return 1
}

# nf::send <text>
# Sends <text> to each chat in NF_CHAT_ID (single value or CSV) sequentially.
# Aggregates per-chat results into outputs:
#   ok          — true only if every chat succeeded
#   message_id  — CSV of message_ids in input order; empty slot for failed
#                 chats (e.g. "42,,103")
#   http_status — 200 if all succeeded, else the first non-200 status seen
#   error       — empty on success; on failure, "chat <id>: <reason>" joined
#                 with "; " for every failed chat
# Returns 0 only if every chat succeeded; 1 on any failure. The caller
# (entrypoint) maps return code to exit code based on fail_on_error.
nf::send() {
  local text="$1"

  # validate.sh already ensured every CSV item is a well-formed chat_id.
  local saved_ifs="$IFS"
  IFS=','
  # shellcheck disable=SC2086  # word-splitting on commas is intentional
  set -- $NF_CHAT_ID
  IFS="$saved_ifs"

  local chat_count=$#
  local all_ok=1
  local mids=""
  local errors=""
  local first_failure_status=""
  local first=1
  local chat trimmed

  for chat in "$@"; do
    trimmed=$(printf '%s' "$chat" | sed -e 's/^ *//' -e 's/ *$//')
    if [ "$first" -eq 1 ]; then
      first=0
    else
      mids="${mids},"
    fi
    if _nf::_send_one "$trimmed" "$text"; then
      mids="${mids}${_NF_RESULT_MID}"
    else
      all_ok=0
      # mids gets an empty slot (the separator already appended above).
      # Single-chat keeps the raw error format; multi-chat prefixes with the
      # chat id so users can tell which destination failed.
      if [ "$chat_count" -gt 1 ]; then
        errors="${errors:+${errors}; }chat ${trimmed}: ${_NF_RESULT_ERROR}"
      else
        errors="${_NF_RESULT_ERROR}"
      fi
      [ -z "$first_failure_status" ] && first_failure_status="$_NF_RESULT_HTTP_STATUS"
    fi
  done

  if [ "$all_ok" -eq 1 ]; then
    nf::set_output ok true
    nf::set_output message_id "$mids"
    nf::set_output http_status 200
    nf::set_output error ""
    return 0
  fi

  nf::set_output ok false
  nf::set_output message_id "$mids"
  nf::set_output http_status "$first_failure_status"
  nf::set_output error "$errors"
  return 1
}
