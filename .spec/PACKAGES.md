<!-- generated: 2026-05-27, template: core.md -->

# Packages

`notiflow` is a flat bash codebase with no "packages" in the traditional sense — every module is a `source`-only file under `scripts/`. This document indexes those files grouped by architectural layer.

The Go-/Python-style "domain" and "adapters" categories from the template don't map cleanly here: a single composite action has no persistence layer, no separate transport adapter, and no DI container. The grouping below preserves the template's spirit (top-down by layer) using the actual layers present.

---

## 1. Transport Layer (Composite-action manifest)

### `action.yml`

**The action's public contract** — declares inputs, outputs, branding, and the single composite step that runs `scripts/entrypoint.sh`.

| File | Description |
|------|-------------|
| `action.yml` | 15 inputs (lines 9–67); 3 outputs (`ok`, `message_id`, `http_status`, lines 69–78); composite step at line 81 maps every `inputs.<x>` to `env.NF_<X>` (lines 85–100) and executes `bash "${{ github.action_path }}/scripts/entrypoint.sh"` (line 102). |

Key details:
- Uses `runs.using: composite` (ADR-1).
- The input→env mapping is the single boundary between the GitHub Actions contract and the `NF_*` namespace used by every script.
- `status` has GitHub-context default `${{ job.status }}` (line 19) — so the action picks up the surrounding job's outcome out of the box.

---

## 2. Orchestration Layer

### `scripts/entrypoint.sh`

**The only file with a shebang.** Sources every module and drives the pipeline.

| File | Description |
|------|-------------|
| `scripts/entrypoint.sh` | `#!/usr/bin/env bash`, `set -euo pipefail` (lines 1, 5); resolves `NF_HOME` from `BASH_SOURCE` (line 7); sources `lib.sh`/`validate.sh`/`filter.sh`/`escape.sh`/`render.sh`/`send.sh` (lines 9–19); enforces bash and external tools (lines 21–23); masks token (lines 27–29); calls `nf::validate`/`nf::should_notify`/`nf::render`/`nf::send` in order; applies the `NF_FAIL_ON_ERROR` policy to translate `nf::send`'s return into an exit code (lines 47–56). |

Uses: every other `scripts/*.sh` module. Imported by: nothing — this is the entry point.

---

## 3. Domain / Library Layer

Each module here is **source-only** (no shebang), idempotent via `_NF_<NAME>_LOADED` guard, bash 3.2-compatible.

### `scripts/lib.sh`

**The common substrate.** Six helpers used by every other module.

| Function | Signature | Location |
|----------|-----------|----------|
| `nf::log` | `nf::log <level> <msg>` — emits GitHub workflow command (`::notice::`/`::warning::`/`::error::`/`::debug::`) to stderr | `lib.sh:11-22` |
| `nf::mask` | `nf::mask <value>` — prints `::add-mask::<value>` so subsequent logs hide it | `lib.sh:26-28` |
| `nf::set_output` | `nf::set_output <key> <value>` — appends `key=value\n` to `$GITHUB_OUTPUT` (or stdout if unset) | `lib.sh:32-35` |
| `nf::json_escape` | `nf::json_escape <value>` — returns `jq -Rs .` JSON-quoted string | `lib.sh:39-41` |
| `nf::require_command` | `nf::require_command <cmd>` — exit 22 with `MISSING_DEPENDENCY:<cmd>` if missing | `lib.sh:45-50` |
| `nf::require_bash` | `nf::require_bash` — assert `BASH_VERSINFO[0] >= 3`, else re-exec into Homebrew bash or exit 20 | `lib.sh:55-67` |

Key details:
- The `_NF_LIB_LOADED` guard at `lib.sh:5-6` makes double-sourcing a no-op.
- Logs are written to **stderr** so they don't pollute the rendered message which `nf::render` emits on stdout.

### `scripts/validate.sh`

**Input validator.** One public function `nf::validate` plus one private set-membership helper.

| Function | Signature | Location |
|----------|-----------|----------|
| `nf::validate` | reads `NF_*` env; exits 10–15 on failure; defaults `NF_STATUS`/`NF_PARSE_MODE`/`NF_NOTIFY_ON` in-place; `export`s the normalised values | `validate.sh:25-103` |
| `_nf::_in_set` | `_nf::_in_set <value> <space-separated allowed>` → rc=0 if member | `validate.sh:16-23` |

Exit-code table (declared at `validate.sh:8-14`):

| Code | Constant | When |
|------|----------|------|
| 10 | `MISSING_REQUIRED_INPUT` | `NF_BOT_TOKEN` or `NF_CHAT_ID` empty (`validate.sh:27-30`) |
| 11 | `INVALID_CHAT_ID` | not `@username` (4–32 of `[A-Za-z0-9_]`), not negative integer, not non-negative integer (`validate.sh:33-53`) |
| 12 | `INVALID_STATUS` | `NF_STATUS` (or fallback `JOB_STATUS`) ∉ `{success, failure, cancelled, skipped}` (`validate.sh:56-62`) |
| 13 | `INVALID_PARSE_MODE` | `NF_PARSE_MODE` ∉ `{MarkdownV2, HTML, Markdown, none}` (`validate.sh:66-72`) |
| 14 | `INVALID_NOTIFY_ON` | any CSV item ∉ status set (`validate.sh:79-91`) |
| 15 | `INVALID_THREAD_ID` | `NF_MESSAGE_THREAD_ID` set and not `^[0-9]+$` (`validate.sh:95-99`) |

Uses: `nf::log` from `lib.sh`.

### `scripts/filter.sh`

**Status filter.** One public function.

| Function | Signature | Location |
|----------|-----------|----------|
| `nf::should_notify` | `nf::should_notify <status> <notify_on_csv>` → rc=0 if `<status>` is in CSV (whitespace-trimmed), else rc=1 | `filter.sh:10-23` |

Key details:
- Implements REQ-3.1/3.2.
- Splits on commas via `IFS=','; set -- $list` (`filter.sh:13-16`) so the function is dependency-free.
- Bash 3.2-safe: no arrays, only positional parameters.

### `scripts/escape.sh`

**Pure escape functions.** Three public, no private state.

| Function | Signature | Location |
|----------|-----------|----------|
| `nf::escape_md_v2` | `nf::escape_md_v2 <text>` — escapes `\` first, then `_*[]()~``>#+-=\|{}.!` via single-pass sed | `escape.sh:11-17` |
| `nf::escape_html` | `nf::escape_html <text>` — escapes `&` first, then `<` then `>` | `escape.sh:21-26` |
| `nf::escape_none` | `nf::escape_none <text>` — identity, prints input unchanged | `escape.sh:30-32` |

Key details:
- All functions read from `$1` (no stdin) and write to stdout via `printf '%s' | sed`.
- MarkdownV2 escapes backslash **before** other specials so the result is idempotent under double-application within the same value.
- The character class is `[][_*()~``>#+=|{}.!-]` (`escape.sh:16`): `]` immediately after `[` and `-` at the end are required by POSIX `sed` bracket-expression rules.

### `scripts/render.sh`

**Template selector and placeholder substituter.** One public function, multiple private helpers.

| Function | Signature | Location |
|----------|-----------|----------|
| `nf::render` | reads `NF_*` and `GITHUB_*` env; prints rendered, escaped, ≤4096-byte text to stdout | `render.sh:98-130` |
| `_nf::_emoji` | `_nf::_emoji <status>` → emoji prefix per REQ-4.2/4.3/4.4/4.5 | `render.sh:11-19` |
| `_nf::_placeholder_value` | `_nf::_placeholder_value <key>` → looked-up GitHub-context value, rc=1 for unknown | `render.sh:23-46` |
| `_nf::_default_template` | heredoc-emits the built-in default template | `render.sh:48-56` |
| `_nf::_pick_template` | applies the 4-level priority (`NF_TEMPLATE_<STATUS>` → `NF_MESSAGE_TEMPLATE` → default) | `render.sh:61-79` |
| `_nf::_escape_value` | dispatches to `nf::escape_md_v2`/`_html`/`_none` per `NF_PARSE_MODE` | `render.sh:81-88` |
| `_nf::_sed_rhs_escape` | escapes `\&/` so a value is safe on the RHS of `sed s///` | `render.sh:94-96` |
| `_nf::_truncate` | clips text to ≤4096 bytes, appending `...` if over | `render.sh:134-142` |

Known placeholders (`render.sh:9`): `Repo`, `Workflow`, `Job`, `Status`, `StatusEmoji`, `Actor`, `Ref`, `RefName`, `Branch`, `Sha`, `ShortSha`, `RunId`, `RunNumber`, `RunUrl`, `EventName`, `ServerUrl`.

Uses: `nf::escape_md_v2`/`nf::escape_html`/`nf::escape_none` from `escape.sh`; `nf::log` from `lib.sh`.

### `scripts/send.sh`

**HTTP send with retry.** One public function, two private helpers.

| Function | Signature | Location |
|----------|-----------|----------|
| `nf::send` | `nf::send <text>` — builds JSON, POSTs with retry loop, sets outputs, returns 0/1 | `send.sh:73-154` |
| `_nf::_build_json` | builds the `sendMessage` request body via `jq -n --arg/--argjson` | `send.sh:14-54` |
| `_nf::_post` | one `curl -sS -X POST` invocation; prints `<body>\n<http_code>` | `send.sh:58-67` |

Key details:
- Attempt cap: `_NF_MAX_ATTEMPTS=4` (`send.sh:8`).
- Backoff for 5xx and network errors: `1 << (attempt-1)` → 1s, 2s, 4s (`send.sh:121`, `send.sh:129`).
- 429 backoff: `body.parameters.retry_after // 1` (`send.sh:113`).
- 4xx (any non-429) is terminal: sets outputs and returns 1 with **no** retry (`send.sh:134-140`).
- Base URL is overridable via `NF_API_BASE` (`send.sh:60`), used by tests to point at `tests/fixtures/mock_server.py`.
- JSON shape: always includes `chat_id`, `text`, `disable_web_page_preview`, `disable_notification`; conditionally `parse_mode` (omitted when `NF_PARSE_MODE` is `none` or empty) and `message_thread_id` (omitted when `NF_MESSAGE_THREAD_ID` is empty).

Uses: `nf::set_output`, `nf::log` from `lib.sh`.

---

## 4. Test / Verification Layer

### `tests/`

bats-core test suites, one per `scripts/` module.

| File | Tests | Notes |
|------|-------|-------|
| `tests/lib.bats` | `nf::log`/`nf::mask`/`nf::set_output`/`nf::json_escape` | No mock server needed. |
| `tests/validate.bats` | All six exit codes 10–15. | No mock. |
| `tests/filter.bats` | `nf::should_notify` truth table. | No mock. |
| `tests/escape.bats` | MarkdownV2 18-symbol set, HTML, `escape_none`. | No mock. |
| `tests/render.bats` | Priority, placeholders, unknown placeholder, truncate. | No mock. |
| `tests/send.bats` | 200, 429 retry, 5xx backoff, 400 no-retry. | Uses `mock_telegram_start`. |
| `tests/entrypoint.bats` | End-to-end + `fail_on_error` semantics + skip-by-filter outputs. | Uses mock. |
| `tests/helpers.bash` | `setup_clean_env`, `mock_telegram_start`/`_stop`, `read_output`/`assert_output_eq`, `count_mock_requests`, `last_mock_body`. | Sourced via `load helpers` in every `.bats` file. |

### `tests/fixtures/mock_server.py`

Programmable Telegram mock. CLI: `--responses <csv>` (e.g. `429:rate_limit,200:success`), `--log <path>`, `--port-file <path>`, `--pid-file <path>`. Captures each request body with a `---REQUEST---`/`---BODY---` marker scheme that `count_mock_requests` and `last_mock_body` in `tests/helpers.bash` parse.

---

## 5. CI / Release

### `.github/workflows/`

| File | Description |
|------|-------------|
| `.github/workflows/ci.yml` | Matrix lint+test on `ubuntu-latest` and `macos-latest`. |
| `.github/workflows/release.yml` | On tag push `v*`, moves the major-version tag (`v1`) to the new commit (ADR-7 moving-tag scheme). |

### `Makefile`

| Target | Description |
|--------|-------------|
| `make help` | Default target — lists every documented target. |
| `make lint` | `shellcheck -x scripts/*.sh tests/*.bash`, `shfmt -d -i 2 -ci scripts tests`, optional `actionlint`. |
| `make lint-fix` | `shfmt -w -i 2 -ci scripts tests`. |
| `make test` | `bats tests`. |
| `make build` | No-op (composite action — nothing to build). |
| `make readme` | Placeholder (not implemented in v1). |
| `make install-tools` | Installs `bats-core`/`shellcheck`/`shfmt`/`actionlint`/`jq` via `brew` on Darwin or `apt-get` on Linux. |
