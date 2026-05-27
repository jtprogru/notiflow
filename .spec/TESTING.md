<!-- generated: 2026-05-27, template: development.md -->

# TESTING

Testing conventions for `notiflow`. The test stack is `bats-core` (bash unit/integration runner) plus a hand-rolled Python `http.server` mock for the Telegram Bot API.

For the runner command and tool installation, see `.spec/TOOLS.md` § 3.3. For the per-module file map, see `.spec/PACKAGES.md`.

---

## 1. Test Package Layout

Bash has no notion of "internal" vs "external" test packages. The convention used here:

- One `tests/<module>.bats` file per `scripts/<module>.sh`.
- A `tests/<module>.bats` file sources the module under test **and** the `lib.sh` it depends on, then exercises the public `nf::*` functions directly.
- Shared setup, env scrubbing, mock orchestration, and output assertions live in **one** `tests/helpers.bash`, loaded via `load helpers` at the top of every `.bats` file.

Example (`tests/escape.bats:1-11`):

```bash
#!/usr/bin/env bats

load helpers

setup() {
  setup_clean_env
  # shellcheck source=../scripts/lib.sh
  source "${NF_ROOT}/scripts/lib.sh"
  # shellcheck source=../scripts/escape.sh
  source "${NF_ROOT}/scripts/escape.sh"
}
```

Files:

| File                       | Tests | Scope                                                            |
|----------------------------|-------|------------------------------------------------------------------|
| `tests/lib.bats`           |   5   | Logging, masking, `set_output`, JSON-escape, `require_command`.  |
| `tests/escape.bats`        |  10   | MarkdownV2 / HTML / none escape correctness, including all 18 specials. |
| `tests/validate.bats`      |  15   | All validation exit codes (10..15).                              |
| `tests/filter.bats`        |   4   | `nf::should_notify` (membership in `NF_NOTIFY_ON`).              |
| `tests/render.bats`        |  16   | Template priority, all 16 placeholders, truncation, escape integration. |
| `tests/send.bats`          |  13   | 200 / 429 / 5xx / 4xx / token-leak via mock server.              |
| `tests/entrypoint.bats`    |   6   | End-to-end through the orchestrator.                             |
| **Total**                  | **69** |                                                                  |

---

## 2. Test File Structure

Canonical pattern (`tests/send.bats:1-21`):

```bash
#!/usr/bin/env bats

load helpers

setup() {
  setup_clean_env
  export NF_BOT_TOKEN="testtoken123"
  export NF_CHAT_ID="42"
  export NF_PARSE_MODE="MarkdownV2"
  export NF_DISABLE_WEB_PAGE_PREVIEW="true"
  export NF_DISABLE_NOTIFICATION="false"
  # shellcheck source=../scripts/lib.sh
  source "${NF_ROOT}/scripts/lib.sh"
  # shellcheck source=../scripts/send.sh
  source "${NF_ROOT}/scripts/send.sh"
}

teardown() {
  mock_telegram_stop
}
```

Convention details:

- **Shebang** — `#!/usr/bin/env bats` on every `.bats` file.
- **Helper load** — `load helpers` (no `.bash` suffix; bats appends it).
- **`setup()`** — runs before every test. Always begins with `setup_clean_env` (scrubs `NF_*` / `INPUT_*`, seeds `GITHUB_*` to known values, creates `GITHUB_OUTPUT`).
- **Module source** — explicit `source "${NF_ROOT}/scripts/lib.sh"` then `source "${NF_ROOT}/scripts/<module>.sh"`. Each `source` is preceded by a `# shellcheck source=...` directive so static analysis follows the path.
- **`teardown()`** — only modules that start the mock define it; the call is the idempotent `mock_telegram_stop`.

### Assertion idioms

Three styles are used (all from real tests):

1. **Direct bash test** (`tests/escape.bats:13-15`):
   ```bash
   @test "escape md_v2: underscore" {
     [ "$(nf::escape_md_v2 'foo_bar')" = 'foo\_bar' ]
   }
   ```

2. **`run` + status / output** (`tests/lib.bats:17-21`):
   ```bash
   @test "lib: nf::mask prints add-mask directive" {
     run bash -c 'source "'"${NF_ROOT}"'/scripts/lib.sh"; nf::mask secret_value'
     [ "$status" -eq 0 ]
     [ "$output" = "::add-mask::secret_value" ]
   }
   ```

3. **Helper-backed output assertion** for the `GITHUB_OUTPUT` file (`tests/send.bats:22-30`):
   ```bash
   @test "send: 200 success sets outputs" {
     mock_telegram_start "200:success"
     run nf::send "hello"
     [ "$status" -eq 0 ]
     assert_output_eq ok true
     assert_output_eq message_id 42
     assert_output_eq http_status 200
     [ "$(count_mock_requests)" -eq 1 ]
   }
   ```

---

## 3. Key Patterns

### 3.1 Clean env on every test

`setup_clean_env` (`tests/helpers.bash:8-29`) is the first call in every `setup()`. It:

- Unsets every `NF_*` and `INPUT_*` env var the previous test may have leaked.
- Seeds a deterministic set of `GITHUB_*` vars (`GITHUB_REPOSITORY=jtprogru/notiflow`, `GITHUB_SHA=abcdef0...`, etc.) so render assertions can pin exact strings.
- Creates a per-test `GITHUB_OUTPUT` file under `BATS_TEST_TMPDIR`.
- Creates the scratch dir `$NF_TMP` (`${BATS_TEST_TMPDIR}/nf`).

### 3.2 Subshell isolation when the SUT calls `exit`

Several `nf::*` functions call `exit` on the error path (e.g., `nf::require_command` exits 22, `nf::validate` exits 10..15). Calling them directly would terminate the bats process. Pattern: wrap in `bash -c '…'` so the exit only kills the subshell.

`tests/validate.bats:9-21`:

```bash
run_validate() {
  # Invoke validate in a clean subshell so exits don't terminate bats.
  bash -c '
    set -e
    source "'"${NF_ROOT}"'/scripts/lib.sh"
    source "'"${NF_ROOT}"'/scripts/validate.sh"
    nf::validate
    # print resolved values for inspection
    printf "NF_STATUS=%s\n"     "$NF_STATUS"
    printf "NF_PARSE_MODE=%s\n" "$NF_PARSE_MODE"
    printf "NF_NOTIFY_ON=%s\n"  "$NF_NOTIFY_ON"
  '
}
```

### 3.3 Mock Telegram server (mandatory for `send`)

The mock is started per-test by `mock_telegram_start` (`tests/helpers.bash:34-60`) and stopped from `teardown()` by `mock_telegram_stop` (`tests/helpers.bash:62-68`):

```bash
mock_telegram_start "200:success"                           # single success
mock_telegram_start "429:rate_limit,200:success"            # retry then succeed
mock_telegram_start "500:server_error,500:server_error,200:success"  # backoff scenario
```

The first arg is a CSV of `"status:fixture"` pairs consumed in order. The server is implemented in `tests/fixtures/mock_server.py` (Python stdlib `http.server` only). Fixtures are named:

| Name           | HTTP | Body shape                                    |
|----------------|------|-----------------------------------------------|
| `success`      | 200  | `{"ok":true,"result":{"message_id":42}}`      |
| `success_99`   | 200  | `{"ok":true,"result":{"message_id":99}}`      |
| `rate_limit`   | 429  | `parameters.retry_after = 1`                  |
| `server_error` | 500  | `{"ok":false,"description":"internal"}`       |
| `bad_request`  | 400  | `{"ok":false,"description":"Bad Request: chat not found"}` |
| `unauthorized` | 401  | `{"ok":false,"description":"Unauthorized"}`   |
| `forbidden`    | 403  | `{"ok":false,"description":"Forbidden"}`      |
| `not_found`    | 404  | `{"ok":false,"description":"Not Found"}`      |

Defined in `tests/fixtures/mock_server.py:15-35`.

The helper binds an ephemeral port (`HTTPServer(("127.0.0.1", 0), …)` in `mock_server.py:100-101`), writes it to `$NF_TMP/mock.port`, and exports `NF_API_BASE=http://127.0.0.1:<port>` — `send.sh` reads `NF_API_BASE` to redirect requests away from `api.telegram.org`. The startup loop in `helpers.bash:48-58` polls for up to 5 s waiting for the port file.

### 3.4 Inspecting captured requests

Every request the mock receives is appended to `$NF_MOCK_REQ_LOG` with this framing (`mock_server.py:47-54`):

```
---REQUEST---
PATH /botXXX/sendMessage
METHOD POST
CT application/json
---BODY---
{"chat_id":42,"text":"hi", ...}
```

Two helpers query the log:

- `count_mock_requests` — number of `---REQUEST---` markers (`helpers.bash:94-102`).
- `last_mock_body` — body block of the most recent request (`helpers.bash:106-114`).

Bodies are then asserted against with `jq -e` (`tests/send.bats:32-42`):

```bash
@test "send: JSON body contains required fields (CP-15)" {
  mock_telegram_start "200:success"
  nf::send "hi"
  local body
  body=$(last_mock_body)
  echo "$body" | jq -e '.chat_id == 42' >/dev/null
  echo "$body" | jq -e '.text == "hi"' >/dev/null
  echo "$body" | jq -e '.parse_mode == "MarkdownV2"' >/dev/null
}
```

### 3.5 `GITHUB_OUTPUT` round-trip helpers

`scripts/lib.sh` writes to `$GITHUB_OUTPUT` via `nf::set_output`. Tests verify with two helpers from `helpers.bash:80-92`:

- `read_output <key>` — last-write-wins read of a key from `$GITHUB_OUTPUT`.
- `assert_output_eq <key> <expected>` — diffs the value, prints `expected/got` to stderr on mismatch.

### 3.6 Property-style coverage (CP-N tests)

Since bash has no PBT library, "property" tests are implemented as targeted unit tests that loop over representative inputs. They are named `prop <module>: <name> (CP-N)` and reference the coverage point ID from the spec.

Example — full membership matrix (`tests/filter.bats:30-52`):

```bash
@test "filter: exclusion property (CP-3)" {
  local st lst expected actual
  for st in success failure cancelled skipped; do
    for lst in success failure cancelled skipped \
      "success,failure" \
      "failure,cancelled,skipped" \
      "success,failure,cancelled,skipped"; do
      case ",$lst," in
        *",$st,"*) expected=0 ;;
        *) expected=1 ;;
      esac
      bash -c '... nf::should_notify "'"$st"'" "'"$lst"'" ...' \
        && actual=0 || actual=$?
      [ "$actual" -eq "$expected" ] || return 1
    done
  done
}
```

Other CP-style tests: `tests/escape.bats:62-71` (CP-6), `tests/render.bats:129-181` (CP-4, CP-5, CP-7), `tests/send.bats` (CP-8..11, CP-15).

### 3.7 `bats run` shadows `$status`

A gotcha worth recording: inside a `@test` body, `run` overwrites `$status`. When a test needs to compare the SUT's exit code against an iterator variable, the iterator must **not** be named `status`. This is why `tests/filter.bats:30-52` uses `st` (status) and `lst` (notify list), and why the property test `tests/render.bats:129-148` uses explicit `export` / `unset` instead of `VAR=val cmd` env-prefix syntax (which doesn't propagate into `$(…)` substitutions inside `@test` bodies).

### 3.8 Token-leak assertions

`tests/send.bats:128-133` and `tests/entrypoint.bats:71-79` enforce that the bot token never appears in any log output except the legitimate `::add-mask::` directive that tells GitHub Actions to redact it. Pattern:

```bash
@test "send: token never appears in stderr (CP-11)" {
  local logf="${BATS_TEST_TMPDIR}/stderr.log"
  mock_telegram_start "500:server_error,400:bad_request"
  nf::send "hi" 2>"$logf" || true
  ! grep -F -- "$NF_BOT_TOKEN" "$logf"
}
```

There is also a generalized helper `assert_no_token_in_log` (`helpers.bash:70-76`).

### 3.9 Builders / golden files / snapshots

**Not used.** No golden-file fixtures, no snapshot library, no test-data builders. All expected values are literal strings inline in the test or computed from `printf` / `seq`.

---

## 4. Mock Generation

**No automated mock generator.** The Telegram Bot API mock is the single Python file `tests/fixtures/mock_server.py` (122 lines), maintained by hand. Adding a new fixture is a one-line addition to the `FIXTURES` dict (`mock_server.py:15-35`):

```python
FIXTURES = {
    "success": (200, {"ok": True, "result": {"message_id": 42}}),
    ...
    "new_case": (HTTP_STATUS, {"ok": ..., "description": "..."}),
}
```

No `go:generate`, no `mockgen`, no testify equivalents — bash has no DI surface, and the only external boundary is HTTP, which is replaced wholesale by the mock.

---

## 5. Integration vs Unit Separation

There is **no formal split** between unit and integration tests (no build tags, no separate directory, no naming convention). Tests are organized by SUT module, not by tier. In practice:

- **Pure-bash tests** (no I/O beyond the test's own tmp dir): `lib.bats`, `escape.bats`, `validate.bats`, `filter.bats`, `render.bats`.
- **Tests that boot the mock HTTP server**: `send.bats`, `entrypoint.bats`.
- **End-to-end through the orchestrator**: `entrypoint.bats:23` invokes `bash "${NF_ROOT}/scripts/entrypoint.sh"` against the mock — the closest thing the project has to an integration test.

All of them are run by the same `bats tests/` invocation. No test container framework is used (the mock is a daemon thread in a Python process started by `mock_telegram_start`).

---

## 6. Testing Private Functions

Bash has no access modifiers. Convention (defined in `.spec/CODE_STYLE.md`):

- **Public functions:** `nf::<name>` (e.g., `nf::send`, `nf::render`).
- **Private helpers:** `_nf::_<name>` (e.g., `_nf::_placeholder_value`, `_nf::_sed_rhs_escape`).

Tests source the module and call private helpers directly when needed — there is no `export_test.go` equivalent because everything in a sourced script is visible to the caller. Most tests target the public surface; private helpers are exercised transitively.

---

## 7. Commands

```bash
# Full test suite (69 tests across 7 files)
make test
# Equivalent to:
bats tests/

# Run a single file
bats tests/send.bats

# Run a single test by name
bats tests/send.bats -f "429 with retry_after"

# Run with verbose TAP output
bats --verbose-run tests/

# Lint (precondition before commits)
make lint
```

Tests do not run race detectors or coverage tooling — bash has no equivalent. Coverage is asserted at the requirements level by the CP-N tests and the coverage matrix in `.spec/features/telegram-notify-action/task-plan.md`.
