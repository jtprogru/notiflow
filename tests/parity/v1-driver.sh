#!/usr/bin/env bash
# Drive the frozen v1 bash implementation for one parity case.
#
# The parity harness sets NF_* / GITHUB_* in the environment and asks for one of:
#   render   — print the rendered message body
#   request  — print the JSON that v1 would POST
#   validate — validate only; the exit code is the answer
#
# Nothing here contacts the network: `request` stops at the JSON, which is the last point
# where v1 and v2 can be compared without a live Telegram.
#
# shellcheck source-path=SCRIPTDIR
set -euo pipefail

V1="$(cd "$(dirname "${BASH_SOURCE[0]}")/v1" && pwd)"
# shellcheck source=v1/lib.sh
source "${V1}/lib.sh"
# shellcheck source=v1/validate.sh
source "${V1}/validate.sh"
# shellcheck source=v1/filter.sh
source "${V1}/filter.sh"
# shellcheck source=v1/escape.sh
source "${V1}/escape.sh"
# shellcheck source=v1/render.sh
source "${V1}/render.sh"
# shellcheck source=v1/send.sh
source "${V1}/send.sh"

mode="${1:?usage: v1-driver.sh render|request|validate}"

case "$mode" in
  validate)
    # nf::validate exits with the code under test; log noise goes to stderr already.
    nf::validate
    ;;
  render)
    nf::validate >/dev/null
    nf::render
    ;;
  request)
    nf::validate >/dev/null
    text="$(nf::render)"
    _nf::_build_json "$NF_CHAT_ID" "$text" "${NF_EDIT_MESSAGE_ID:-}"
    ;;
  *)
    echo "unknown mode '$mode'" >&2
    exit 64
    ;;
esac
