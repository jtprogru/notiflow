# shellcheck shell=bash
# Shared bats helpers for notiflow tests.

# Repository root, derived from this file's location.
NF_ROOT="${BATS_TEST_DIRNAME%/tests/parity/bats}"
# Frozen v1 bash implementation, kept as the parity reference.
NF_V1="${BATS_TEST_DIRNAME%/bats}/v1"
NF_TMP="${BATS_TEST_TMPDIR:-/tmp}/nf"

setup_clean_env() {
  local var
  for var in $(env | awk -F= '/^(NF_|INPUT_)/ {print $1}'); do
    unset "$var"
  done

  export GITHUB_REPOSITORY="jtprogru/notiflow"
  export GITHUB_WORKFLOW="CI"
  export GITHUB_JOB="test"
  export GITHUB_RUN_ID="1"
  export GITHUB_RUN_NUMBER="1"
  export GITHUB_SHA="abcdef0123456789abcdef0123456789abcdef01"
  export GITHUB_ACTOR="tester"
  export GITHUB_REF="refs/heads/main"
  export GITHUB_REF_NAME="main"
  export GITHUB_EVENT_NAME="push"
  export GITHUB_SERVER_URL="https://github.com"
  export GITHUB_OUTPUT="${BATS_TEST_TMPDIR}/gh_output"
  : >"$GITHUB_OUTPUT"

  mkdir -p "$NF_TMP"
}

# mock_telegram_start [<responses_csv>]
# responses_csv: comma-separated "status:fixture_name" pairs, e.g. "200:success,429:rate_limit,200:success".
# Starts the mock server on a free port and exports NF_API_BASE / NF_MOCK_PID / NF_MOCK_REQ_LOG.
mock_telegram_start() {
  local responses="${1:-200:success}"
  local port_file="${NF_TMP}/mock.port"
  local pid_file="${NF_TMP}/mock.pid"
  local err_file="${NF_TMP}/mock.err"
  export NF_MOCK_REQ_LOG="${NF_TMP}/requests.log"
  : >"$NF_MOCK_REQ_LOG"
  # Truncate: a stale port file from a previous test would end the wait loop
  # below immediately and point the client at a dead port.
  : >"$port_file"
  : >"$err_file"

  python3 "${NF_ROOT}/tests/mock_server.py" \
    --responses "$responses" \
    --log "$NF_MOCK_REQ_LOG" \
    --port-file "$port_file" \
    --pid-file "$pid_file" 2>"$err_file" &
  export NF_MOCK_PID=$!

  # 20s, not the former 5s: interpreter startup on a cold CI runner is slower
  # than on a warm laptop, and a timeout here fails the test for the wrong reason.
  local i=0 max=200 alive=yes
  while [ ! -s "$port_file" ] && [ $i -lt $max ]; do
    # Server died before binding — no point waiting out the full timeout.
    kill -0 "$NF_MOCK_PID" 2>/dev/null || {
      alive=no
      break
    }
    sleep 0.1
    i=$((i + 1))
  done
  [ -s "$port_file" ] || {
    {
      echo "mock server failed to start after $((i / 10))s (process alive: ${alive})"
      echo "python3: $(command -v python3 || echo '<not on PATH>') -> $(python3 -V 2>&1 || true)"
      echo "--- mock stderr ---"
      cat "$err_file" 2>/dev/null
      echo "--- end mock stderr ---"
    } >&2
    return 1
  }
  local port
  port=$(cat "$port_file")
  export NF_API_BASE="http://127.0.0.1:${port}"
}

mock_telegram_stop() {
  if [ -n "${NF_MOCK_PID:-}" ]; then
    kill "$NF_MOCK_PID" 2>/dev/null || true
    wait "$NF_MOCK_PID" 2>/dev/null || true
    unset NF_MOCK_PID
  fi
}

assert_no_token_in_log() {
  local token="$1" log_file="$2"
  if grep -F -- "$token" "$log_file" >/dev/null 2>&1; then
    echo "token leaked into $log_file" >&2
    return 1
  fi
}

# read_output <key>
# Returns the value of <key> from the $GITHUB_OUTPUT file (last occurrence wins).
read_output() {
  local key="$1"
  awk -F= -v k="$key" '$1==k {v=substr($0, length(k)+2)} END {print v}' "$GITHUB_OUTPUT"
}

assert_output_eq() {
  local key="$1" expected="$2" actual
  actual=$(read_output "$key")
  if [ "$actual" != "$expected" ]; then
    echo "output[$key]: expected '$expected', got '$actual'" >&2
    return 1
  fi
}

count_mock_requests() {
  if [ ! -f "${NF_MOCK_REQ_LOG:-}" ]; then
    echo 0
    return
  fi
  local n
  n=$(grep -c '^---REQUEST---$' "$NF_MOCK_REQ_LOG" 2>/dev/null) || n=0
  echo "$n"
}

# last_mock_body
# Prints the body of the most recent request captured by the mock.
last_mock_body() {
  awk '/^---REQUEST---$/ {found=NR} END {start=found} {a[NR]=$0} END {
    seen=0
    for (i=start+1; i<=NR; i++) {
      if (a[i] ~ /^---BODY---$/) {seen=1; continue}
      if (seen) print a[i]
    }
  }' "$NF_MOCK_REQ_LOG"
}
