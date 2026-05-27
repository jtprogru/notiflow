# shellcheck shell=bash
# Input validation for notiflow. Source-only.
# Compatible with bash 3.2+. Reads/writes NF_* environment variables.

[ -n "${_NF_VALIDATE_LOADED:-}" ] && return 0
_NF_VALIDATE_LOADED=1

# Exit codes:
#   10 MISSING_REQUIRED_INPUT
#   11 INVALID_CHAT_ID
#   12 INVALID_STATUS
#   13 INVALID_PARSE_MODE
#   14 INVALID_NOTIFY_ON
#   15 INVALID_THREAD_ID

_nf::_in_set() {
  # _nf::_in_set <value> <space-separated allowed values>
  local value="$1" allowed="$2" item
  for item in $allowed; do
    [ "$item" = "$value" ] && return 0
  done
  return 1
}

nf::validate() {
  # Required inputs.
  if [ -z "${NF_BOT_TOKEN:-}" ] || [ -z "${NF_CHAT_ID:-}" ] || [ -z "${NF_STATUS:-}" ]; then
    nf::log error "MISSING_REQUIRED_INPUT: bot_token, chat_id and status are required"
    exit 10
  fi

  # chat_id: integer (with optional leading minus) OR @username (4..32 of [A-Za-z0-9_]).
  case "$NF_CHAT_ID" in
    @*)
      local uname="${NF_CHAT_ID#@}"
      if ! printf '%s' "$uname" | grep -Eq '^[A-Za-z0-9_]{4,32}$'; then
        nf::log error "INVALID_CHAT_ID: '$NF_CHAT_ID'"
        exit 11
      fi
      ;;
    -*)
      if ! printf '%s' "${NF_CHAT_ID#-}" | grep -Eq '^[0-9]+$'; then
        nf::log error "INVALID_CHAT_ID: '$NF_CHAT_ID'"
        exit 11
      fi
      ;;
    *)
      if ! printf '%s' "$NF_CHAT_ID" | grep -Eq '^[0-9]+$'; then
        nf::log error "INVALID_CHAT_ID: '$NF_CHAT_ID'"
        exit 11
      fi
      ;;
  esac

  # status: must be one of the allowed values. Caller passes it from the workflow
  # (typically ${{ job.status }} or ${{ needs.<job>.result }}) — composite action
  # `default` fields cannot reference the `job` context, so no fallback here.
  if ! _nf::_in_set "$NF_STATUS" "success failure cancelled skipped"; then
    nf::log error "INVALID_STATUS: '$NF_STATUS' (allowed: success|failure|cancelled|skipped)"
    exit 12
  fi
  export NF_STATUS

  # parse_mode: default MarkdownV2.
  if [ -z "${NF_PARSE_MODE:-}" ]; then
    NF_PARSE_MODE="MarkdownV2"
  fi
  if ! _nf::_in_set "$NF_PARSE_MODE" "MarkdownV2 HTML Markdown none"; then
    nf::log error "INVALID_PARSE_MODE: '$NF_PARSE_MODE'"
    exit 13
  fi
  # Legacy `Markdown` is silently upgraded to MarkdownV2: notiflow escapes
  # placeholder values per MarkdownV2 rules, and sending parse_mode=Markdown
  # to Telegram with V2 escaping produces broken or rejected messages.
  if [ "$NF_PARSE_MODE" = "Markdown" ]; then
    nf::log warn "parse_mode=Markdown upgraded to MarkdownV2 (notiflow escapes per V2 rules)"
    NF_PARSE_MODE="MarkdownV2"
  fi
  export NF_PARSE_MODE

  # notify_on: default success,failure,cancelled.
  if [ -z "${NF_NOTIFY_ON:-}" ]; then
    NF_NOTIFY_ON="success,failure,cancelled"
  fi
  # `any` / `all` shortcut: expand to the full status set so users don't have
  # to type the CSV. Either keyword behaves identically.
  case "$NF_NOTIFY_ON" in
    any | all)
      NF_NOTIFY_ON="success,failure,cancelled,skipped"
      ;;
  esac
  local saved_ifs="$IFS"
  IFS=','
  # shellcheck disable=SC2086  # word-splitting on commas is intentional
  set -- $NF_NOTIFY_ON
  IFS="$saved_ifs"
  local item trimmed
  for item in "$@"; do
    trimmed=$(printf '%s' "$item" | sed -e 's/^ *//' -e 's/ *$//')
    if ! _nf::_in_set "$trimmed" "success failure cancelled skipped"; then
      nf::log error "INVALID_NOTIFY_ON: '$trimmed' not in {success,failure,cancelled,skipped}"
      exit 14
    fi
  done
  export NF_NOTIFY_ON

  # message_thread_id: optional; must be a non-negative integer if set.
  if [ -n "${NF_MESSAGE_THREAD_ID:-}" ]; then
    if ! printf '%s' "$NF_MESSAGE_THREAD_ID" | grep -Eq '^[0-9]+$'; then
      nf::log error "INVALID_THREAD_ID: '$NF_MESSAGE_THREAD_ID'"
      exit 15
    fi
  fi

  return 0
}
