# shellcheck shell=bash
# Template selection and placeholder rendering for notiflow. Source-only.
# Bash 3.2 compatible: no associative arrays.

[ -n "${_NF_RENDER_LOADED:-}" ] && return 0
_NF_RENDER_LOADED=1

# Known placeholder keys. Order is irrelevant for substitution.
_NF_PLACEHOLDER_KEYS="Repo Workflow Job Status StatusEmoji Actor Ref RefName Branch Sha ShortSha RunId RunNumber RunUrl EventName ServerUrl"

_nf::_emoji() {
  case "$1" in
    success) printf '✅' ;;
    failure) printf '❌' ;;
    cancelled) printf '⚠️' ;;
    skipped) printf '⏭' ;;
    *) : ;;
  esac
}

# _nf::_placeholder_value <key>
# Echoes the value associated with a known placeholder key, empty if unknown.
_nf::_placeholder_value() {
  local sha="${GITHUB_SHA:-}"
  local short="${sha:0:7}"
  case "$1" in
    Repo) printf '%s' "${GITHUB_REPOSITORY:-}" ;;
    Workflow) printf '%s' "${GITHUB_WORKFLOW:-}" ;;
    Job) printf '%s' "${GITHUB_JOB:-}" ;;
    Status) printf '%s' "${NF_STATUS:-}" ;;
    StatusEmoji) _nf::_emoji "${NF_STATUS:-}" ;;
    Actor) printf '%s' "${GITHUB_ACTOR:-}" ;;
    Ref) printf '%s' "${GITHUB_REF:-}" ;;
    RefName) printf '%s' "${GITHUB_REF_NAME:-}" ;;
    Branch) printf '%s' "${GITHUB_REF_NAME:-}" ;;
    Sha) printf '%s' "$sha" ;;
    ShortSha) printf '%s' "$short" ;;
    RunId) printf '%s' "${GITHUB_RUN_ID:-}" ;;
    RunNumber) printf '%s' "${GITHUB_RUN_NUMBER:-}" ;;
    RunUrl) printf '%s/%s/actions/runs/%s' \
      "${GITHUB_SERVER_URL:-}" "${GITHUB_REPOSITORY:-}" "${GITHUB_RUN_ID:-}" ;;
    EventName) printf '%s' "${GITHUB_EVENT_NAME:-}" ;;
    ServerUrl) printf '%s' "${GITHUB_SERVER_URL:-}" ;;
    *) return 1 ;;
  esac
}

_nf::_default_template() {
  cat <<'TEMPLATE'
{{.StatusEmoji}} *{{.Workflow}}* on `{{.Repo}}`
Status: {{.Status}}
Branch: {{.Branch}} @ {{.ShortSha}}
Actor: {{.Actor}}
[Open run]({{.RunUrl}})
TEMPLATE
}

# _nf::_pick_template
# Selects the active template body based on NF_MESSAGE / NF_TEMPLATE_<STATUS> / NF_MESSAGE_TEMPLATE.
# Writes the template to stdout. The caller decides whether to render placeholders.
_nf::_pick_template() {
  local per_status
  case "${NF_STATUS:-}" in
    success) per_status="${NF_TEMPLATE_SUCCESS:-}" ;;
    failure) per_status="${NF_TEMPLATE_FAILURE:-}" ;;
    cancelled) per_status="${NF_TEMPLATE_CANCELLED:-}" ;;
    skipped) per_status="${NF_TEMPLATE_SKIPPED:-}" ;;
    *) per_status="" ;;
  esac
  if [ -n "$per_status" ]; then
    printf '%s' "$per_status"
    return
  fi
  if [ -n "${NF_MESSAGE_TEMPLATE:-}" ]; then
    printf '%s' "$NF_MESSAGE_TEMPLATE"
    return
  fi
  _nf::_default_template
}

_nf::_escape_value() {
  case "${NF_PARSE_MODE:-MarkdownV2}" in
    MarkdownV2) nf::escape_md_v2 "$1" ;;
    HTML) nf::escape_html "$1" ;;
    none | *) nf::escape_none "$1" ;;
  esac
}

nf::render() {
  # NF_MESSAGE (REQ-5.1): full passthrough, no placeholder substitution, no escape.
  if [ -n "${NF_MESSAGE:-}" ]; then
    _nf::_truncate "$NF_MESSAGE"
    return 0
  fi

  local template
  template=$(_nf::_pick_template)

  # Substitution uses bash parameter expansion rather than sed. This preserves
  # newlines inside placeholder values (e.g. a multi-line workflow name) and
  # eliminates the RHS-escape dance for / & \ that sed s/// would need.
  local key value escaped_value
  for key in $_NF_PLACEHOLDER_KEYS; do
    value=$(_nf::_placeholder_value "$key") || value=""
    escaped_value=$(_nf::_escape_value "$value")
    template=${template//"{{.$key}}"/$escaped_value}
  done

  # Warn and strip unknown placeholders that remain.
  local remaining
  remaining=$(printf '%s' "$template" | grep -oE '\{\{\.[A-Za-z][A-Za-z0-9]*\}\}' | sort -u || true)
  if [ -n "$remaining" ]; then
    local stray name
    for stray in $remaining; do
      name=${stray#'{{.'}
      name=${name%'}}'}
      nf::log warn "UNKNOWN_PLACEHOLDER:$name"
      template=${template//"$stray"/}
    done
  fi

  _nf::_truncate "$template"
}

# _nf::_utf16_units <text>
# Echoes the UTF-16 code-unit count for <text>. This is the unit Telegram uses
# for its 4096-character sendMessage limit: BMP codepoints (incl. Cyrillic) cost
# 1 unit each, supplementary-plane codepoints (most emoji) cost 2 each.
# `iconv -c` silently drops any malformed UTF-8 input.
_nf::_utf16_units() {
  local bytes
  bytes=$(printf '%s' "$1" | iconv -c -f UTF-8 -t UTF-16LE 2>/dev/null | wc -c | tr -d ' ')
  printf '%s' $((bytes / 2))
}

# _nf::_truncate <text>
# Echoes <text> unchanged if it fits in 4096 UTF-16 code units (Telegram's
# sendMessage limit); otherwise echoes the first 4093 code units plus "...",
# aligned to codepoint boundaries. A trailing unpaired high surrogate left by
# the byte-level cut is silently dropped by `iconv -c`.
_nf::_truncate() {
  local text="$1"
  local limit=4096
  local units
  units=$(_nf::_utf16_units "$text")
  if [ "$units" -le "$limit" ]; then
    printf '%s' "$text"
    return
  fi
  local max_bytes=$(((limit - 3) * 2))
  local tmp
  tmp=$(mktemp)
  printf '%s' "$text" | iconv -c -f UTF-8 -t UTF-16LE 2>/dev/null >"$tmp"
  head -c "$max_bytes" "$tmp" | iconv -c -f UTF-16LE -t UTF-8 2>/dev/null
  rm -f "$tmp"
  printf '...'
}
