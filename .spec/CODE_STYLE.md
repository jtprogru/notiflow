<!-- generated: 2026-05-27, template: core.md -->

# Code Style

Bash-specific conventions for `notiflow`. The template's struct-tag / conversion-pattern sections from typed languages do not apply; the equivalent here is the **`NF_*` env-var contract** + **`nf::*` namespace discipline**. Concurrency is N/A — the action runs in a single bash process.

## 1. Layer Structure

```
  ┌────────────────────────────────────────────────────────────────┐
  │ Transport Layer        action.yml                              │
  │   - declarative manifest only (no logic)                       │
  │   - maps inputs.<x> → env.NF_<X>  (action.yml:85-100)          │
  └──────────────────────────┬─────────────────────────────────────┘
                             │ exec bash script (single child step)
  ┌──────────────────────────▼─────────────────────────────────────┐
  │ Orchestration Layer    scripts/entrypoint.sh                   │
  │   - the ONLY file with shebang + `set -euo pipefail`           │
  │   - sources every library module and runs the pipeline         │
  │   - owns the exit-code policy (NF_FAIL_ON_ERROR mapping)       │
  └──────────────────────────┬─────────────────────────────────────┘
                             │ source (no subprocess)
  ┌──────────────────────────▼─────────────────────────────────────┐
  │ Library Layer          scripts/{lib,validate,filter,           │
  │                                 escape,render,send}.sh         │
  │   - source-only (no shebang)                                   │
  │   - idempotent via `_NF_<NAME>_LOADED` guard                   │
  │   - communicate via NF_* env vars and stdout/stderr            │
  │   - exit codes are reserved (10-15 validate, 20/22 runtime)    │
  └────────────────────────────────────────────────────────────────┘
```

**Rule**: business logic lives in the Library Layer; `entrypoint.sh` only sequences calls and decides exit codes. Tests target `nf::*` functions directly via `load helpers` in bats — never by spawning `entrypoint.sh` unless the test is in `tests/entrypoint.bats`.

## 2. File Headers and Idempotent Sourcing

Every script in `scripts/` except `entrypoint.sh` follows this exact header pattern (example from `scripts/lib.sh:1-6`):

```bash
# shellcheck shell=bash
# <short purpose summary>. Source-only — no shebang.
# Compatible with bash 3.2+.

[ -n "${_NF_LIB_LOADED:-}" ] && return 0
_NF_LIB_LOADED=1
```

Why:
- `# shellcheck shell=bash` tells shellcheck the dialect since there is no shebang.
- The guard variable name is the file name in upper snake case plus `_LOADED`. Sourcing twice is a no-op.
- No shebang → the file cannot be executed directly; only `source`d.

`entrypoint.sh` is the only file that begins with `#!/usr/bin/env bash` and `set -euo pipefail` (`entrypoint.sh:1`, `entrypoint.sh:5`).

## 3. Naming Conventions

### 3.1 Functions

| Visibility | Pattern | Example | Defined at |
|------------|---------|---------|------------|
| Public | `nf::<verb_or_noun>` | `nf::log`, `nf::validate`, `nf::should_notify`, `nf::render`, `nf::send`, `nf::escape_md_v2`, `nf::set_output`, `nf::mask`, `nf::json_escape`, `nf::require_command`, `nf::require_bash` | across all files |
| Private (file-local) | `_nf::_<verb_or_noun>` | `_nf::_in_set` (`validate.sh:16`), `_nf::_emoji` (`render.sh:11`), `_nf::_placeholder_value`, `_nf::_pick_template`, `_nf::_escape_value`, `_nf::_sed_rhs_escape`, `_nf::_truncate`, `_nf::_default_template`, `_nf::_build_json` (`send.sh:14`), `_nf::_post` (`send.sh:58`) | across all files |

`::` is the conventional bash "namespace" separator (legal in function names since bash 3). The leading underscore on `_nf::_*` is the second-line signal of "do not call from another module."

### 3.2 Environment Variables

| Scope | Pattern | Example |
|-------|---------|---------|
| Public input contract (set from `action.yml`) | `NF_<UPPER_SNAKE>` | `NF_BOT_TOKEN`, `NF_CHAT_ID`, `NF_STATUS`, `NF_PARSE_MODE`, … |
| Internal constants (defined in scripts) | `_NF_<UPPER_SNAKE>` | `_NF_LIB_LOADED`, `_NF_PLACEHOLDER_KEYS` (`render.sh:9`), `_NF_MAX_ATTEMPTS` (`send.sh:8`) |
| GitHub Actions context (consumed read-only) | `GITHUB_*` | `GITHUB_REPOSITORY`, `GITHUB_RUN_ID`, `GITHUB_SHA`, … |
| Test-only override | `NF_API_BASE` | set by `tests/helpers.bash:59` |

The leading underscore on `_NF_*` marks "internal — not part of the action's public contract."

### 3.3 Files

| Type | Location | Pattern | Example |
|------|----------|---------|---------|
| Orchestrator | `scripts/` | `entrypoint.sh` (singular) | `scripts/entrypoint.sh` |
| Library module | `scripts/` | `<purpose>.sh` (singular noun) | `scripts/lib.sh`, `scripts/validate.sh` |
| Test suite | `tests/` | `<module>.bats` matching `scripts/<module>.sh` | `tests/render.bats` ↔ `scripts/render.sh` |
| Test helpers | `tests/` | `helpers.bash` | `tests/helpers.bash` |
| Fixture | `tests/fixtures/` | `<purpose>.<ext>` | `tests/fixtures/mock_server.py` |

## 4. Function Signature Conventions

Three rules, all observable in `scripts/lib.sh`:

1. **Positional arguments only.** No flags. Document the order in the comment immediately above the function. Example (`lib.sh:8-11`):
   ```bash
   # nf::log <level> <msg>
   # Writes to stderr with a GitHub Actions workflow command marker.
   # Levels: info|notice|warn|warning|error|debug
   nf::log() {
     local level="$1"
     shift
     ...
   }
   ```
2. **Public functions return 0 on success and a small positive integer on failure.** Example: `nf::should_notify` (`filter.sh:10`) returns 0 if the status is in the list, 1 otherwise. Functions that exit the script use the exit-code table (§7), not `return`.
3. **No global mutation beyond documented env vars.** `nf::validate` is the only function that intentionally mutates env (it fills in defaults for `NF_STATUS`/`NF_PARSE_MODE`/`NF_NOTIFY_ON` and `export`s them, `validate.sh:63/73/92`). Other functions read env but never write it.

## 5. Argument Quoting

| Rule | Example | Why |
|------|---------|-----|
| Always quote `"$variable"` expansions on the RHS of test/case/assignment. | `[ -z "${NF_BOT_TOKEN:-}" ]` (`entrypoint.sh:27`) | Prevent word-splitting on values containing spaces. |
| Use `${VAR:-}` instead of `$VAR` for possibly-unset vars under `set -u`. | `${NF_STATUS:-}` everywhere it is referenced | `set -u` aborts the script on a bare reference to an unset var. |
| Use `printf '%s' "$x"` instead of `echo "$x"` when `$x` may start with `-` or contain `\`. | `_nf::_sed_rhs_escape` at `render.sh:95` | `echo` is non-portable and interprets backslash escapes on some platforms. |
| Intentional word-splitting is annotated with a `shellcheck disable=SC2086` and a comment. | `filter.sh:14-16`, `validate.sh:81-83` | Makes the intent explicit and silences the linter. |

## 6. Sed and Regex Conventions

Two recurring patterns:

1. **MarkdownV2 character class** (`escape.sh:16`): `[][_*()~``>#+=|{}.!-]`. `]` must come right after `[`, and `-` must be at the end (or the start). This order is portable across BSD and GNU `sed`.
2. **RHS escaping for `sed s///`** (`render.sh:94-96`): escape `\`, `&`, and `/` in any value before splicing it into a `sed` replacement. Newlines are stripped first because they would split the substitution.

`grep -E` and `sed -E` (extended regex) are preferred over basic regex; `grep -F` is used for fixed-string searches like `assert_no_token_in_log` (`tests/helpers.bash:72`).

## 7. Exit Codes

`notiflow` reserves a fixed exit-code table. Every script writes one of these codes via `exit <N>` — no `exit 1` for validation errors.

| Code | Constant | Where | Cause |
|------|----------|-------|-------|
| 0 | success | normal completion | message delivered, filter-skipped, or `fail_on_error=false` after send failure |
| 1 | `SEND_FAILED` | `entrypoint.sh:50` | send exhausted retries and `fail_on_error=true` |
| 10 | `MISSING_REQUIRED_INPUT` | `validate.sh:28-30` | `NF_BOT_TOKEN` or `NF_CHAT_ID` empty |
| 11 | `INVALID_CHAT_ID` | `validate.sh:33-53` | not `@username`, not negative int, not non-negative int |
| 12 | `INVALID_STATUS` | `validate.sh:59-61` | `NF_STATUS` ∉ `{success, failure, cancelled, skipped}` |
| 13 | `INVALID_PARSE_MODE` | `validate.sh:69-71` | `NF_PARSE_MODE` ∉ `{MarkdownV2, HTML, Markdown, none}` |
| 14 | `INVALID_NOTIFY_ON` | `validate.sh:87-89` | any CSV item ∉ status set |
| 15 | `INVALID_THREAD_ID` | `validate.sh:96-98` | `NF_MESSAGE_THREAD_ID` set and not `^[0-9]+$` |
| 20 | `UNSUPPORTED_BASH` | `lib.sh:65-66` | bash < 3 and no Homebrew bash to re-exec |
| 22 | `MISSING_DEPENDENCY:<cmd>` | `lib.sh:47-49` | required command not on `PATH` |

Codes 16–19 and 21 are intentionally unallocated to leave room for future validation and dependency-check categories without overlap.

## 8. Error Propagation

Bash has no exception mechanism; the codebase uses three propagation patterns:

1. **`set -euo pipefail` in the orchestrator** (`entrypoint.sh:5`). Any uncaught error in a sourced function tears down the script. Library modules do not set this themselves — they rely on the orchestrator's setting.
2. **Explicit `exit <code>` for terminal validation errors** (`validate.sh:30/37/49/61/71/88/97`). Validation never returns a non-zero status; it either accepts the input or terminates the script.
3. **`return 1` for recoverable failures** (`send.sh:138`, `send.sh:153`, `filter.sh:22`). The caller decides what to do. `entrypoint.sh:43-56` is the single place that maps `nf::send`'s return into the final exit code via `NF_FAIL_ON_ERROR`.

When a function emits an error log it uses the constant name + a short value, e.g. `nf::log error "INVALID_CHAT_ID: '$NF_CHAT_ID'"` (`validate.sh:37`). The constant is grep-able; the value gives the user something to fix.

## 9. Logging Conventions

| Layer | Level | Channel | Format | Example |
|-------|-------|---------|--------|---------|
| Library | `info` / `notice` | stderr | `::notice::<msg>` | `nf::log info "notiflow: skipping (status=$NF_STATUS not in notify_on=$NF_NOTIFY_ON)"` (`entrypoint.sh:34`) |
| Library | `warn` | stderr | `::warning::<msg>` | `nf::log warn "5xx ($http_status); backoff ${delay}s (attempt=$attempt)"` (`send.sh:122`) |
| Library | `error` | stderr | `::error::<msg>` | `nf::log error "MISSING_REQUIRED_INPUT: bot_token and chat_id are required"` (`validate.sh:29`) |
| Library | `debug` | stderr | `::debug::<msg>` | suppressed unless `ACTIONS_STEP_DEBUG=true` (GitHub Actions convention) |

Rules:

1. **Logs go to stderr. Stdout is reserved for rendered output** (`nf::render`'s text return value). Mixing them would corrupt the message that `nf::send` POSTs.
2. **Never log `NF_BOT_TOKEN` in plaintext**. The token is masked at `entrypoint.sh:27-29` *before* any other side effect. Subsequent `::warning::`/`::error::` lines containing the token will be auto-redacted by the runner. Property 11 in `.spec/features/telegram-notify-action/design.md` §2.6 makes this a verified invariant; `tests/helpers.bash:70-76` provides `assert_no_token_in_log` to enforce it.
3. **Use the `nf::log <level>` wrapper, never bare `echo` or `printf` to stderr.** Bare `echo` skips the GitHub workflow-command prefix and the message will not appear as a styled notice/warning/error in the run log.
4. **Correlation IDs are GitHub-native** — `$GITHUB_RUN_ID`/`$GITHUB_JOB`/attempt counter (`send.sh:89/116/123/131`). The action does not generate its own request IDs.

## 10. Bash Compatibility Rules (3.2 floor)

macOS ships with bash 3.2 and the codebase commits to that floor. The following patterns are **disallowed**:

| Forbidden | Why | Alternative used here |
|-----------|-----|----------------------|
| `declare -A` (associative arrays) | bash 4+ only | space-separated key list (`render.sh:9`) + `case` dispatcher (`render.sh:23-46`) |
| `mapfile` / `readarray` | bash 4+ only | `while IFS= read -r line; do …; done` |
| `${var,,}` / `${var^^}` (case conversion) | bash 4+ only | `case "$var" in lowercase|UPPER) … esac` |
| `local -n` (namerefs) | bash 4.3+ only | return value via stdout, capture with `$(…)` |
| `&>` / `|&` redirection shorthand | bash 4+ only | `>file 2>&1` / `2>&1 |` |

Allowed but checked at runtime:
- Re-exec into a newer bash when found (`lib.sh:55-67`) — used only as a courtesy on macOS so the codebase can stay 3.2-clean and still get tested on systems where Homebrew bash is `PATH`-first.

## 11. Import / Source Ordering

`entrypoint.sh:9-19` defines the canonical order:

```bash
source "${NF_HOME}/lib.sh"        # foundational helpers, no other deps
source "${NF_HOME}/validate.sh"   # depends on lib.sh
source "${NF_HOME}/filter.sh"     # depends on lib.sh
source "${NF_HOME}/escape.sh"     # depends on nothing
source "${NF_HOME}/render.sh"     # depends on lib.sh + escape.sh
source "${NF_HOME}/send.sh"       # depends on lib.sh
```

Rule: order by dependency. `lib.sh` first, leaf modules second, modules that consume them last. The `_NF_<NAME>_LOADED` guards make the order tolerant of mistakes but the comments at each `source` line keep intent visible. Each source has a `# shellcheck source=<path>` directive so shellcheck can follow the include.

## 12. Test File Organization

| Pattern | Where | Example |
|---------|-------|---------|
| One `.bats` file per `scripts/*.sh` module | `tests/` | `tests/render.bats` ↔ `scripts/render.sh` |
| Helpers source via `load helpers` at the top of every `.bats` file | `tests/` | `load helpers` |
| `setup()` calls `setup_clean_env` to wipe `NF_*`/`INPUT_*` and seed `GITHUB_*` | `tests/helpers.bash:8-29` | every bats suite |
| Mock-using tests call `mock_telegram_start` in `setup()` and `mock_telegram_stop` in `teardown()` | `tests/helpers.bash:34-68` | `tests/send.bats`, `tests/entrypoint.bats` |
| Output assertions use `assert_output_eq <key> <expected>` | `tests/helpers.bash:85-92` | `assert_output_eq ok true` |

The Python mock at `tests/fixtures/mock_server.py` is the **only** non-bash code in the repo and is permitted because (a) the test fixture is not shipped with the action and (b) the mock needs to programmatically replay a sequence of HTTP responses with bodies — a task that is non-trivial in bash alone.

## 13. Quick Reference

| Aspect | Transport (`action.yml`) | Orchestrator (`entrypoint.sh`) | Library (`scripts/*.sh`) |
|--------|--------------------------|--------------------------------|--------------------------|
| Shebang | n/a (YAML) | `#!/usr/bin/env bash` | absent (source-only) |
| `set -euo pipefail` | n/a | yes | no (relies on orchestrator's setting) |
| Idempotency guard | n/a | n/a (entry point) | `_NF_<NAME>_LOADED` at top |
| Public symbols | `inputs.*`, `outputs.*` | none (calls `nf::*` only) | `nf::*` functions |
| Private symbols | n/a | n/a | `_nf::_*` functions, `_NF_*` vars |
| Allowed to `exit` | n/a | yes (final exit code) | yes (validation errors only); recoverable failures use `return` |
| Allowed to mutate env | yes (`env:` block) | no | `nf::validate` only (defaults) |
| Allowed to call HTTP | no | no | `scripts/send.sh` only |
| Stdout meaning | n/a | n/a | reserved for `nf::render` output |
| Stderr meaning | n/a | passes through | `nf::log` workflow commands |

## 14. Concurrency

Not applicable. The action runs in a single bash process, performs at most four sequential HTTP requests (`_NF_MAX_ATTEMPTS=4` in `send.sh:8`), and writes `$GITHUB_OUTPUT` only after the request loop completes. There are no goroutines, threads, background processes, or shared mutable state.
