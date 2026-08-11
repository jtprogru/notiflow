# shellcheck shell=bash
# Helpers for the Action wrapper acceptance tests.
#
# These drive scripts/action/run.sh with the same NF_* environment action.yml builds, so
# what is under test is the whole wrapper — the env mapping included — rather than the
# binary alone.

NF_ROOT="${BATS_TEST_DIRNAME%/tests/action}"
NF_RUN="${NF_ROOT}/scripts/action/run.sh"
NF_TMP="${BATS_TEST_TMPDIR:-/tmp}/nf-action"

# A token shaped like a real one, so leak checks have something realistic to look for.
NF_TEST_TOKEN="123456789:AAHdqTcvCH1vGWJxfSeofSAs0K5PALDsaw"

setup_clean_env() {
  local var
  for var in $(env | awk -F= '/^(NF_|INPUT_|NOTIFLOW_)/ {print $1}'); do
    unset "$var"
  done

  export GITHUB_ACTIONS="true"
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
  export GITHUB_STEP_SUMMARY="${BATS_TEST_TMPDIR}/gh_summary"
  : >"$GITHUB_OUTPUT"
  : >"$GITHUB_STEP_SUMMARY"

  export NF_BIN="${NF_ROOT}/target/debug/notiflow"
  [ -x "$NF_BIN" ] || {
    echo "notiflow binary not built; run 'make build' first" >&2
    return 1
  }

  export NF_BOT_TOKEN="$NF_TEST_TOKEN"
  export NF_CHAT_ID="-1001234567890"
  export NF_STATUS="success"

  mkdir -p "$NF_TMP"
}

# mock_telegram_start [<responses_csv>]
# Starts the shared mock and exports NF_API_BASE / NF_MOCK_PID / NF_MOCK_REQ_LOG.
mock_telegram_start() {
  local responses="${1:-200:success}"
  local port_file="${NF_TMP}/mock.port"
  local pid_file="${NF_TMP}/mock.pid"
  local err_file="${NF_TMP}/mock.err"
  export NF_MOCK_REQ_LOG="${NF_TMP}/requests.log"
  : >"$NF_MOCK_REQ_LOG"
  : >"$port_file"
  : >"$err_file"

  python3 "${NF_ROOT}/tests/mock_server.py" \
    --responses "$responses" \
    --log "$NF_MOCK_REQ_LOG" \
    --port-file "$port_file" \
    --pid-file "$pid_file" 2>"$err_file" &
  export NF_MOCK_PID=$!

  local i=0 max=200 alive=yes
  while [ ! -s "$port_file" ] && [ $i -lt $max ]; do
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
      cat "$err_file" 2>/dev/null
    } >&2
    return 1
  }
  export NF_API_BASE="http://127.0.0.1:$(cat "$port_file")"
}

mock_telegram_stop() {
  if [ -n "${NF_MOCK_PID:-}" ]; then
    kill "$NF_MOCK_PID" 2>/dev/null || true
    wait "$NF_MOCK_PID" 2>/dev/null || true
    unset NF_MOCK_PID
  fi
}

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
  [ -f "${NF_MOCK_REQ_LOG:-}" ] || {
    echo 0
    return
  }
  local n
  n=$(grep -c '^---REQUEST---$' "$NF_MOCK_REQ_LOG" 2>/dev/null) || n=0
  echo "$n"
}

last_mock_body() {
  awk '/^---REQUEST---$/ {found=NR} {a[NR]=$0} END {
    seen=0
    for (i=found+1; i<=NR; i++) {
      if (a[i] ~ /^---BODY---$/) {seen=1; continue}
      if (seen) print a[i]
    }
  }' "$NF_MOCK_REQ_LOG"
}

last_mock_path() {
  awk '/^---REQUEST---$/ {found=NR} {a[NR]=$0} END {
    for (i=found+1; i<=NR; i++) if (a[i] ~ /^PATH /) {print substr(a[i], 6); exit}
  }' "$NF_MOCK_REQ_LOG"
}
