<!-- generated: 2026-05-27, template: bootstrap.md -->

# Agent Rules for notiflow

Mandatory rules for any AI agent (or human) modifying this repository. Violations break CI (`make lint && make test`) and must be fixed before merge.

## Code Style

- **Bash 3.2+ only.** Target is macOS system bash. No associative arrays (`declare -A`), no `[[ ... =~ ... ]]` if avoidable, no Bash-4 string operators that don't exist in 3.2. When iterating mapped data, use parallel arrays or `case` arms (see `render.sh::_NF_PLACEHOLDER_KEYS`).
- **Strict mode in entry scripts.** `entrypoint.sh` runs `set -euo pipefail`. Library files (sourced, no shebang) must be safe under it.
- **Source-only libraries.** Files under `scripts/` other than `entrypoint.sh` are sourced. Each starts with `# shellcheck shell=bash` and a load guard `[ -n "${_NF_<NAME>_LOADED:-}" ] && return 0; _NF_<NAME>_LOADED=1`. No shebang.
- **No echo of secrets.** The first observable action of the pipeline is `nf::mask "$NF_BOT_TOKEN"`. Never log or print the token, even in debug paths.
- **Avoid GNU-only flags.** `sed`, `grep`, `awk` must work under BSD (macOS) and GNU (Linux). When a flag differs, use the portable form or pipe through a portable equivalent (see `escape.sh` for the BSD/GNU-safe `sed` order).
- **Indent: 2 spaces** (enforced by `shfmt -i 2 -ci`). Use `case ... esac` indentation as `shfmt -ci` requires.
- **Use `printf`, not `echo -e`.** Echo flag semantics vary across shells; `printf '%s\n'` is portable.

## Naming Conventions

- **Public functions:** `nf::<verb_or_noun>` — e.g. `nf::log`, `nf::validate`, `nf::should_notify`, `nf::render`, `nf::send`.
- **Private helpers:** `_nf::_<name>` — e.g. `_nf::_emoji`, `_nf::_pick_template`, `_nf::_post`. The double underscore signals not part of the public API.
- **Env vars:** `NF_*` for all runtime state (`NF_BOT_TOKEN`, `NF_CHAT_ID`, `NF_PARSE_MODE`, ...). `GITHUB_*` env vars are read-only from the runner. Test-only override: `NF_API_BASE`, `NF_MOCK_*`.
- **Internal constants:** `_NF_<NAME>` upper-snake — e.g. `_NF_MAX_ATTEMPTS`, `_NF_PLACEHOLDER_KEYS`, `_NF_<MODULE>_LOADED`.
- **Files:** `scripts/<lowercase-noun>.sh`, `tests/<same-noun>.bats`. One module per file.
- **Bats tests:** `@test "<module>: <behavior in lowercase>"`. Use `setup` to call `setup_clean_env`; use `teardown` to call `mock_telegram_stop` when the mock is started.

## Error Handling

- **Exit codes carry meaning.** Use the existing scheme (10..15 = validation, 20 = unsupported bash, 22 = missing dep, 1 = send failed with `fail_on_error=true`). Allocate a new code rather than reuse one. Document every new code in `.spec/README.md` and `validate.sh` (or relevant module).
- **Errors go to stderr via `nf::log`.** Use the GitHub Actions workflow command markers (`::error::`, `::warning::`, `::notice::`, `::debug::`) — they render correctly in the Actions UI.
- **Categorize before exiting.** Errors should print a stable token first (e.g. `MISSING_DEPENDENCY:jq`, `INVALID_CHAT_ID: '<value>'`) so tests can grep on the token, not the prose.
- **Never let send failures kill the job by default.** `fail_on_error=false` is the documented default; the action returns `ok=false` and exits 0. Only exit non-zero when `fail_on_error` is one of `true|TRUE|True|1|yes`.
- **Retry policy lives in `send.sh`.** 4 attempts total. `429` honors `parameters.retry_after`. `5xx` and network errors use 1s/2s/4s backoff. Other `4xx` is final. Do not add retries elsewhere.

## Testing

- **Framework:** `bats-core`. One `.bats` file per `scripts/*.sh` module plus `entrypoint.bats` for end-to-end.
- **Always source `helpers.bash`** at the top of every `.bats` file: `load helpers`. Call `setup_clean_env` in `setup`.
- **Mock the Telegram API.** Never hit `api.telegram.org` from tests. Use `mock_telegram_start "<responses>"` from `helpers.bash` and assert against `last_mock_body` / `count_mock_requests`.
- **Test the failure modes.** Each new exit code needs a test asserting both the code and the stderr token. Each new HTTP path in `send.sh` needs a mock fixture in `tests/fixtures/mock_server.py`.
- **Outputs assertions** go through `assert_output_eq <key> <expected>` (reads `$GITHUB_OUTPUT`).
- **Keep tests green.** Baseline is 69 passing tests. `make test` must be green before any commit.
- **No flakes.** Mock server uses ephemeral ports; do not hard-code ports. Use `BATS_TEST_TMPDIR` for any temp files.

## Dependencies

- **Runtime deps are only `bash`, `curl`, `jq`.** Do not introduce new runtime dependencies; GitHub-hosted runners include this set, but self-hosted runners are documented to need only these three.
- **Check deps explicitly.** `nf::require_command <name>` in `entrypoint.sh` is the single place that asserts a dependency. Add a call there for any new binary (and exit code 22 will apply).
- **Dev deps are pinned-by-name only.** `make install-tools` installs `bats-core`, `shellcheck`, `shfmt`, `actionlint`, `jq` (Homebrew on macOS, apt + curl on Linux). Do not pin versions in code; CI installs from package managers.
- **Python in tests.** Only the stdlib. `tests/fixtures/mock_server.py` must remain a single-file Python 3 script using `http.server` only — no pip installs.
- **No vendored binaries.** Nothing is committed to the repo beyond source, manifests, and docs.

## Formatting

- **`make lint` is the contract.** It runs:
  - `shellcheck -x scripts/*.sh tests/*.bash`
  - `shfmt -d -i 2 -ci scripts tests`
  - `actionlint` (if installed)
- **Use `make lint-fix`** to auto-format with `shfmt -w -i 2 -ci`. Never commit shfmt-dirty files.
- **shellcheck must pass clean.** If you need to suppress a finding, use `# shellcheck disable=SC<NNNN>` on the immediate line with a one-line justification comment. Wholesale disables are not acceptable.
- **`action.yml` must pass `actionlint`.** Any new input/env mapping should be validated locally before pushing.
- **Markdown.** English, concise, tables for structured data, fenced code blocks with language tags. Avoid emojis except for the documented status emojis used by the action itself (`✅` `❌` `⚠️` `⏭`).
- **Commit messages.** Plain user-authored style. Do not add AI authorship trailers or generated-by markers (`Co-Authored-By: Claude ...`, `Generated with Claude Code`, etc.).
