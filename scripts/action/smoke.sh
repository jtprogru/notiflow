#!/usr/bin/env bash
# End-to-end smoke test of the Action wrapper against a local mock Telegram.
#
# Deliberately free of bats: this runs on Windows runners too, where installing bats-core
# is more trouble than the coverage is worth. The bats suite in tests/action/ is the
# detailed one; this is the portable "does the whole wrapper work here" check.
#
# Inputs (env):
#   NF_BIN — path to the notiflow binary (default: notiflow from PATH)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export NF_BIN="${NF_BIN:-notiflow}"

tmp="$(mktemp -d)"
port_file="${tmp}/port"
log_file="${tmp}/requests.log"
err_file="${tmp}/mock.err"
: >"$port_file"
: >"$err_file"

python3 "${ROOT}/tests/mock_server.py" \
  --responses "200:success" \
  --log "$log_file" \
  --port-file "$port_file" \
  --pid-file "${tmp}/pid" 2>"$err_file" &
mock_pid=$!

cleanup() {
  kill "$mock_pid" 2>/dev/null || true
  wait "$mock_pid" 2>/dev/null || true
  rm -rf "$tmp"
}
trap cleanup EXIT

# Say why, not just that. "mock server never bound a port" on its own is undiagnosable from
# a CI log, which is the same hole v1.6.2 closed in the bats helpers.
mock_diagnostics() {
  {
    echo "--- mock server diagnostics ---"
    echo "python3: $(command -v python3 || echo '<not on PATH>')"
    python3 -V 2>&1 || true
    if kill -0 "$mock_pid" 2>/dev/null; then
      echo "process ${mock_pid}: alive"
    else
      echo "process ${mock_pid}: gone"
    fi
    echo "port file: $(wc -c <"$port_file" | tr -d ' ') byte(s)"
    echo "--- mock stderr ---"
    cat "$err_file" 2>/dev/null
    echo "--- end ---"
  } >&2
}

# 30s: interpreter startup on a cold runner is slower than on a warm laptop, and a timeout
# here fails the job for the wrong reason.
waited=0
while [ "$waited" -lt 300 ]; do
  [ -s "$port_file" ] && break
  kill -0 "$mock_pid" 2>/dev/null || break
  sleep 0.1
  waited=$((waited + 1))
done

if [ ! -s "$port_file" ]; then
  echo "mock server never bound a port after $((waited / 10))s" >&2
  mock_diagnostics
  exit 1
fi

export GITHUB_OUTPUT="${tmp}/gh_output"
export GITHUB_STEP_SUMMARY="${tmp}/gh_summary"
: >"$GITHUB_OUTPUT"
: >"$GITHUB_STEP_SUMMARY"

mock_port="$(cat "$port_file")"
export NF_API_BASE="http://127.0.0.1:${mock_port}"
export NF_BOT_TOKEN="123456789:AAHdqTcvCH1vGWJxfSeofSAs0K5PALDsaw"
export NF_CHAT_ID="-1001234567890"
export NF_STATUS="success"
export NF_MESSAGE="smoke test from ${RUNNER_OS:-local}"

"${ROOT}/scripts/action/run.sh"

fail=0
check() {
  local key="$1" expected="$2" actual
  actual="$(awk -F= -v k="$key" '$1==k {v=substr($0, length(k)+2)} END {print v}' "$GITHUB_OUTPUT")"
  if [ "$actual" != "$expected" ]; then
    echo "output[$key]: expected '$expected', got '$actual'" >&2
    fail=1
  fi
}

check ok true
check message_id 42
check http_status 200
check error ""

if ! grep -q "| result | sent |" "$GITHUB_STEP_SUMMARY"; then
  echo "step summary is missing the result row" >&2
  fail=1
fi

if grep -q -F -- "$NF_BOT_TOKEN" "$GITHUB_OUTPUT" "$GITHUB_STEP_SUMMARY"; then
  echo "the bot token leaked into an output file" >&2
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  echo "--- \$GITHUB_OUTPUT ---" >&2
  cat "$GITHUB_OUTPUT" >&2
  exit 1
fi

echo "action smoke test passed on ${RUNNER_OS:-local}"
