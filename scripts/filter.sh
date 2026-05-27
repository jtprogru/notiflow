# shellcheck shell=bash
# notify_on filter for notiflow. Source-only.

[ -n "${_NF_FILTER_LOADED:-}" ] && return 0
_NF_FILTER_LOADED=1

# nf::should_notify <status> <notify_on_csv>
# Returns 0 if status is in the comma-separated list, 1 otherwise.
# Whitespace around list items is trimmed.
nf::should_notify() {
  local status="$1" list="$2"
  local saved_ifs="$IFS"
  IFS=','
  # shellcheck disable=SC2086  # word-splitting on commas is intentional
  set -- $list
  IFS="$saved_ifs"
  local item trimmed
  for item in "$@"; do
    trimmed=$(printf '%s' "$item" | sed -e 's/^ *//' -e 's/ *$//')
    [ "$trimmed" = "$status" ] && return 0
  done
  return 1
}
