# Changelog

All notable changes are documented in this file. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/).

## [1.6.0] — 2026-05-27

### Added

- New `edit_message_id` input. When set, the action calls `editMessageText` instead of `sendMessage`, so a workflow can update the same Telegram message across steps (start → progress → done) rather than spamming new ones.
- Multi-chat edit: `edit_message_id` accepts a CSV whose length must match `chat_id`. Each `(chat_id[i], edit_message_id[i])` pair targets exactly one message. Wire `edit_message_id: ${{ steps.send.outputs.message_id }}` from a previous multi-chat send and the shapes line up automatically.
- `disable_notification` and `message_thread_id` are silently dropped from the request body in edit mode — Telegram rejects them on `editMessageText`. `parse_mode` and `disable_web_page_preview` are kept (both accepted).
- New exit code `16 INVALID_EDIT_MESSAGE_ID` for bad input (non-positive-integer item, CSV length mismatch).

### Tests

- `+10` cases: five validate (single + CSV happy paths, length mismatch, non-integer, negative integer) and five send (endpoint switches to `editMessageText`, edit JSON body shape, regression that send mode still excludes `message_id`, multi-chat pairing, edit failure surfaces error like send).

## [1.5.0] — 2026-05-27

### Added

- `chat_id` now accepts a comma-separated list of destinations and fans the message out to each. Sequential execution; each chat gets its own retry chain (timeouts, 429 cap, 5xx backoff — same policy as before, applied independently per chat). Whitespace around CSV items is tolerated; validation runs per item with the existing rules (integer, negative integer, or `@username`).
- Aggregated outputs for multi-chat:
  - `ok=true` only if **every** chat succeeded.
  - `message_id` is a CSV of message ids in input order; failed chats leave an empty slot (e.g. `42,,103`).
  - `http_status` reports the first non-200 status seen (or 200 if all succeeded).
  - `error` enumerates failed chats as `chat <id>: <reason>` joined with `; `.
- Single-chat behavior is preserved bit-for-bit: `message_id="42"`, raw `error` (no `chat 42:` prefix), `http_status=<code>` exactly as before. Existing workflows do not need changes.

### Tests

- `+9` cases: four for CSV validation (valid mix, invalid item, empty item, whitespace) and five for send (all-success multi-chat, partial failure with empty slot, per-chat retry chains, per-chat JSON body, and a regression guard ensuring single-chat error stays unprefixed).

## [1.4.0] — 2026-05-27

### Added

- New `error` output. Holds Telegram's `.description` (e.g. `Bad Request: chat not found`, `Unauthorized`) on 4xx; a synthesized `HTTP <code>: <description>` after exhausted 5xx retries; `network error (curl exit N)` after exhausted network-error retries; empty on success and on skip. Pairs naturally with `ok=false` for downstream debug/branching.
- `notify_on` accepts `any` (alias `all`) as a shortcut for `success,failure,cancelled,skipped`. Saves users from typing the full CSV when they want every status.

### Fixed

- `scripts/render.sh`: `shopt -u patsub_replacement` at load time disables the bash 5.2+ behavior where `&` in a `${var//pat/repl}` replacement expands to the matched pattern. Without this, placeholder values containing `&` or `\` (e.g. branch names, repo names) were corrupted on ubuntu-latest (bash 5.2+) — the v1.3.2 substitution refactor regressed here. Silently ignored on bash < 5.2.

### Tests

- `+7` cases: notify_on shortcut (`any`/`all`), notify_on=any with skipped reaching the wire, and four error-output scenarios (200 clears it, 400/401 surface `.description`, 5xx exhausted surfaces `HTTP <code>: <description>`).

## [1.3.2] — 2026-05-27

### Fixed

- `scripts/render.sh`: placeholder substitution is now done with bash parameter expansion (`${var//pat/repl}`) instead of `sed s///`. The old path piped each value through `tr -d '\n' | sed -e 's/[\\&/]/\\&/g'`, which silently stripped newlines from placeholder values (e.g. a multi-line workflow name) and required a fragile RHS-escape dance for `/`, `&`, and `\`. The new path is faster (no `sed` fork per placeholder, ×16 per render) and preserves arbitrary value content literally. Dead helper `_nf::_sed_rhs_escape` removed.

### Security

- `scripts/send.sh`: new `_nf::_resolve_api_base` allowlist guards the Telegram API base URL. Only `https://api.telegram.org` or a loopback URL (`http://127.0.0.1:*` / `http://localhost:*`, for the test mock) is honored; any other value of `NF_API_BASE` — including userinfo tricks (`http://127.0.0.1@evil.com`), look-alike subdomains (`https://api.telegram.org.evil.com`), and protocol downgrades (`http://api.telegram.org`) — is rejected with a `::warning::` and replaced by the default. Closes the channel through which an earlier malicious step in the same workflow job could have redirected the bot token to an attacker-controlled host.

### Added

- `tests/render.bats`: newline preservation and literal-value (sed metachar) tests.
- `tests/send.bats`: API-base allowlist parameter tests covering the accept path and four classes of exfiltration attempt.

## [1.3.1] — 2026-05-27

### Fixed

- `scripts/entrypoint.sh`: `nf::mask` for the bot token now runs **before** the `nf::require_command` checks for `curl`/`jq`/`iconv`. Defensive against forks that enable `set -x` or any future change that might log argv containing the token before the masking directive lands.
- `scripts/lib.sh`: `nf::require_bash` now actually checks bash `>= 3.2`; previously `>= 3.0` would have passed (bash 3.0/3.1 lack array features we rely on). Refactored into `_nf::_bash_version_ok` so the comparison is unit-testable without forging `BASH_VERSINFO`.
- `scripts/lib.sh`: `nf::set_output` falls back to `/dev/stderr` (not `/dev/stdout`) when `GITHUB_OUTPUT` is unset. Stops local runs from interleaving key=value lines with the rendered message body on stdout.
- `scripts/validate.sh`: legacy `parse_mode: Markdown` is now explicitly upgraded to `MarkdownV2` with a `::warning::` annotation. Previously the escaper silently treated `Markdown` as V2 but the request still sent `parse_mode=Markdown` to Telegram — escape rules and parse_mode mismatched, producing broken or rejected messages. The dead branch in `_nf::_escape_value` is removed.
- `tests/`: four new cases — static mask-order invariant, bash 3.2 boundary check, set_output stderr fallback, and Markdown→MarkdownV2 upgrade with warning annotation.

## [1.3.0] — 2026-05-27

### Fixed

- `scripts/render.sh`: message truncation now counts Telegram's actual unit — UTF-16 code units — instead of bash codepoints. Previously, supplementary-plane characters (most emoji) were undercounted by 2×, so emoji-heavy templates near the limit were silently delivered as `400 MESSAGE_TOO_LONG`. Truncation cuts at codepoint boundaries; a trailing unpaired surrogate from the byte-level cut is dropped via `iconv -c`, so no half-emoji ever leaks into the wire.

### Added

- `iconv` is now a required dependency. Hard-fails with `MISSING_DEPENDENCY:iconv` (exit 22, same path as `curl`/`jq`) if absent. GitHub-hosted runners already ship it on all three OSes; self-hosted runners need it on `PATH`.
- `_nf::_utf16_units` helper, exposed for tests, counts UTF-16 code units of arbitrary UTF-8 input (BMP=1, supplementary=2).
- `tests/render.bats`: five new cases covering Cyrillic passthrough at the boundary, Cyrillic truncation, supplementary-plane emoji counting as 2 units, codepoint-aligned emoji truncation (no half emoji / no U+FFFD), and a unit-count parameter test.

### Changed

- `README.md`: length-limit note and requirements section updated to reflect UTF-16 units and the new `iconv` dependency.

## [1.2.0] — 2026-05-27

### Added

- `scripts/send.sh`: the `429` retry path now caps `parameters.retry_after` at `NF_MAX_RETRY_AFTER` (default 60s). Telegram occasionally returns pathological values (hundreds or thousands of seconds); without a cap a single rate-limited request could stall the workflow for an hour. Override via `NF_MAX_RETRY_AFTER` env var (env-only, same pattern as `NF_CONNECT_TIMEOUT` / `NF_MAX_TIME`).
- `scripts/send.sh`: `retry_after` is validated as a non-negative integer; any other shape (string, float, missing) falls back to 1s instead of crashing `sleep`.
- `tests/send.bats`: three new cases covering the cap with a stubbed `sleep` to assert the exact value, a non-integer fallback, and an env-overridden cap.
- `tests/fixtures/mock_server.py`: two new fixtures, `rate_limit_huge` (retry_after=9999) and `rate_limit_garbage` (retry_after="soon").

## [1.1.0] — 2026-05-27

### Added

- `scripts/send.sh`: every Telegram request is now bounded by `curl --connect-timeout` and `--max-time`. Defaults are 5s for the TCP/TLS handshake and 15s for the whole request. Overridable via `NF_CONNECT_TIMEOUT` and `NF_MAX_TIME` environment variables (env-only, same pattern as `NF_API_BASE`). A hung Telegram peer can no longer stall the notify job indefinitely; timeouts surface as a network error and follow the existing 5xx backoff/retry path.
- `tests/send.bats`: four new cases covering hang → retry → success, sustained hang exhausting retries within a bounded budget, and verification that the flags and their default/overridden values reach `curl`'s argv.
- `tests/fixtures/mock_server.py`: new `hang` fixture that blocks before responding (`--hang-seconds`, default 30). Server promoted to `ThreadingHTTPServer` so a blocking request does not stall sibling requests during retry tests.

## [1.0.1] — 2026-05-27

### Fixed

- Removed `default: ${{ job.status }}` from the `status` input in `action.yml`. The `job` context is not available in composite-action `default` fields, so every workflow failed manifest validation with `Unrecognized named-value: 'job'` before any step ran.

### Changed

- **Breaking (relative to the broken 1.0.0):** `status` is now `required: true`. Callers must pass it explicitly, typically as `status: ${{ job.status }}` or `status: ${{ needs.<job>.result }}`. Missing status now exits `10 MISSING_REQUIRED_INPUT` instead of `12 INVALID_STATUS`.
- `scripts/validate.sh`: dropped the dead `JOB_STATUS` env-var fallback (GitHub does not export current-job status to the runner environment).
- `README.md`: every example now includes `status:` explicitly; inputs table reflects the new contract.

### Added

- `tests/manifest.bats`: guards `action.yml` against forbidden contexts (`job`, `steps`, `needs`, `secrets`) in `default:` fields and asserts `status` stays required with no default.

## [1.0.0] — 2026-05-26

### Added

- Composite GitHub Action that sends a Telegram message when a workflow job completes.
- Inputs: `bot_token`, `chat_id`, `status`, `parse_mode`, `notify_on`, `message`, `message_template`, `template_success`/`_failure`/`_cancelled`/`_skipped`, `disable_web_page_preview`, `disable_notification`, `message_thread_id`, `fail_on_error`.
- Outputs: `ok`, `message_id`, `http_status`.
- Built-in template with status emoji (`✅` / `❌` / `⚠️` / `⏭`), workflow, repo, branch, short SHA, actor, and run URL.
- 16 placeholders (`{{.Repo}}`, `{{.Workflow}}`, `{{.Status}}`, …) with auto-escape for the active `parse_mode`.
- Retry policy: up to 4 attempts. `429` honors `parameters.retry_after`; `5xx` and network errors use 1s/2s/4s exponential backoff; other `4xx` is final.
- `::add-mask::` for the bot token before any other observable action.
- Length truncation at 4096 bytes with a trailing `...`.
- CI matrix on `ubuntu-latest` and `macos-latest`.
- Moving major tag (`v1`) maintained by `release.yml` on every `v*.*.*` push.
