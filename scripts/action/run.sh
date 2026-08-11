#!/usr/bin/env bash
# Translate the Action's NF_* environment into a notiflow invocation.
#
# Composite steps do not receive INPUT_* automatically, so action.yml maps every input into
# an NF_* variable and this script turns those into arguments. The bot token is the one
# value that never becomes an argument: it goes across as NOTIFLOW_BOT_TOKEN so it cannot
# be read out of `ps` by anything else on the runner.
#
# NF_DRY_RUN and NF_OUTPUT_FORMAT exist for the parity harness, which drives this same
# script so the mapping itself is covered by the corpus.
set -euo pipefail

notiflow_bin="${NF_BIN:-notiflow}"

# Lenient truthiness, matching what v1 accepted.
is_true() {
  case "${1:-}" in
    true | TRUE | True | 1 | yes | YES | y) return 0 ;;
    *) return 1 ;;
  esac
}

args=()
if [ -n "${NF_EDIT_MESSAGE_ID:-}" ]; then
  args+=(edit --message-id "$NF_EDIT_MESSAGE_ID")
else
  args+=(send)
fi

args+=(--output "${NF_OUTPUT_FORMAT:-github}")
args+=(--chat-id "${NF_CHAT_ID:-}")
args+=(--status "${NF_STATUS:-}")

[ -n "${NF_PARSE_MODE:-}" ] && args+=(--parse-mode "$NF_PARSE_MODE")
[ -n "${NF_NOTIFY_ON:-}" ] && args+=(--notify-on "$NF_NOTIFY_ON")
[ -n "${NF_MESSAGE:-}" ] && args+=(--message "$NF_MESSAGE")
[ -n "${NF_MESSAGE_TEMPLATE:-}" ] && args+=(--template "$NF_MESSAGE_TEMPLATE")
[ -n "${NF_TEMPLATE_SUCCESS:-}" ] && args+=(--template-success "$NF_TEMPLATE_SUCCESS")
[ -n "${NF_TEMPLATE_FAILURE:-}" ] && args+=(--template-failure "$NF_TEMPLATE_FAILURE")
[ -n "${NF_TEMPLATE_CANCELLED:-}" ] && args+=(--template-cancelled "$NF_TEMPLATE_CANCELLED")
[ -n "${NF_TEMPLATE_SKIPPED:-}" ] && args+=(--template-skipped "$NF_TEMPLATE_SKIPPED")
[ -n "${NF_MESSAGE_THREAD_ID:-}" ] && args+=(--thread-id "$NF_MESSAGE_THREAD_ID")

# v1 read anything other than the literal "false" as "suppress previews".
if [ "${NF_DISABLE_WEB_PAGE_PREVIEW:-true}" = "false" ]; then
  args+=(--preview)
else
  args+=(--no-preview)
fi

is_true "${NF_DISABLE_NOTIFICATION:-false}" && args+=(--silent)
is_true "${NF_FAIL_ON_ERROR:-false}" && args+=(--fail-on-error)
is_true "${NF_DRY_RUN:-false}" && args+=(--dry-run)

# The binary reads these from the environment, never from argv.
export NOTIFLOW_BOT_TOKEN="${NF_BOT_TOKEN:-}"
[ -n "${NF_API_BASE:-}" ] && export NOTIFLOW_API_BASE="$NF_API_BASE"

exec "$notiflow_bin" "${args[@]}"
