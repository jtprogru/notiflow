# shellcheck shell=bash
# notiflow common helpers — source-only library (no shebang).
# Compatible with bash 3.2+ (macOS system bash).

[ -n "${_NF_LIB_LOADED:-}" ] && return 0
_NF_LIB_LOADED=1

# nf::log <level> <msg>
# Writes to stderr with a GitHub Actions workflow command marker.
# Levels: info|notice|warn|warning|error|debug
nf::log() {
  local level="$1"
  shift
  local msg="$*"
  case "$level" in
    debug) printf '::debug::%s\n' "$msg" >&2 ;;
    info | notice) printf '::notice::%s\n' "$msg" >&2 ;;
    warn | warning) printf '::warning::%s\n' "$msg" >&2 ;;
    error) printf '::error::%s\n' "$msg" >&2 ;;
    *) printf '%s\n' "$msg" >&2 ;;
  esac
}

# nf::mask <value>
# Hides the value from subsequent log output.
nf::mask() {
  printf '::add-mask::%s\n' "$1"
}

# nf::set_output <key> <value>
# Appends key=value to $GITHUB_OUTPUT (or stderr if unset, for local runs —
# never stdout, which is reserved for the rendered message body).
nf::set_output() {
  local key="$1" value="$2"
  printf '%s=%s\n' "$key" "$value" >>"${GITHUB_OUTPUT:-/dev/stderr}"
}

# nf::json_escape <value>
# Returns a JSON-quoted string (including surrounding ""). Safe for jq --argjson.
nf::json_escape() {
  printf '%s' "$1" | jq -Rs .
}

# nf::require_command <cmd>
# Exits 22 if the command is not on PATH.
nf::require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    nf::log error "MISSING_DEPENDENCY:$1"
    exit 22
  fi
}

# _nf::_bash_version_ok <major> <minor>
# Returns 0 if the given bash version is >= 3.2 (the minimum we support).
# Split out so tests can exercise the comparison without forging BASH_VERSINFO.
_nf::_bash_version_ok() {
  local major="$1" minor="$2"
  [ "$major" -gt 3 ] && return 0
  [ "$major" -eq 3 ] && [ "$minor" -ge 2 ] && return 0
  return 1
}

# nf::require_bash
# Verifies bash >= 3.2 (the minimum we support). Re-exec into a newer bash if
# the current one is older (rare on macOS where system bash is 3.2 exactly).
# Exits 20 if no suitable bash is found.
nf::require_bash() {
  if _nf::_bash_version_ok "${BASH_VERSINFO[0]:-0}" "${BASH_VERSINFO[1]:-0}"; then
    return 0
  fi
  for candidate in /opt/homebrew/bin/bash /usr/local/bin/bash; do
    if [ -x "$candidate" ]; then
      exec "$candidate" "$0" "$@"
    fi
  done
  nf::log error "UNSUPPORTED_BASH:require>=3.2"
  exit 20
}
