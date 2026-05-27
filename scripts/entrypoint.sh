#!/usr/bin/env bash
# notiflow composite-action entry point.
# Orchestrates: require-deps → mask → validate → filter → render → send.
# shellcheck source-path=SCRIPTDIR
set -euo pipefail

NF_HOME="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib.sh
source "${NF_HOME}/lib.sh"
# shellcheck source=validate.sh
source "${NF_HOME}/validate.sh"
# shellcheck source=filter.sh disable=SC1091
source "${NF_HOME}/filter.sh"
# shellcheck source=escape.sh disable=SC1091
source "${NF_HOME}/escape.sh"
# shellcheck source=render.sh disable=SC1091
source "${NF_HOME}/render.sh"
# shellcheck source=send.sh disable=SC1091
source "${NF_HOME}/send.sh"

nf::require_bash "$@"

# Mask the bot token before any other observable action — including the
# require_command checks below. If a fork enables `set -x` or a future change
# logs an argv that contains the token, it stays masked.
if [ -n "${NF_BOT_TOKEN:-}" ]; then
  nf::mask "$NF_BOT_TOKEN"
fi

nf::require_command curl
nf::require_command jq
nf::require_command iconv

nf::validate

if ! nf::should_notify "$NF_STATUS" "$NF_NOTIFY_ON"; then
  nf::log info "notiflow: skipping (status=$NF_STATUS not in notify_on=$NF_NOTIFY_ON)"
  nf::set_output ok false
  nf::set_output message_id ""
  nf::set_output http_status 0
  nf::set_output error ""
  exit 0
fi

text="$(nf::render)"

if nf::send "$text"; then
  exit 0
fi

case "${NF_FAIL_ON_ERROR:-false}" in
  true | TRUE | True | 1 | yes)
    nf::log error "SEND_FAILED (fail_on_error=true)"
    exit 1
    ;;
  *)
    nf::log warn "SEND_FAILED (fail_on_error=false) — exiting 0 to avoid masking job result"
    exit 0
    ;;
esac
