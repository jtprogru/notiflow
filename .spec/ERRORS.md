<!-- generated: 2026-05-27, template: errors.md -->

# notiflow Error Reference

This document is the single source of truth for every error code, log marker, and retry rule that `notiflow` can produce. All entries come from the actual scripts under `scripts/` and from the authoritative spec at `.spec/features/telegram-notify-action/design.md` §2.7 and `requirements.md` Groups 1 and 7.

Type-level error context (which values are accepted by each `NF_*` input, what the Telegram error envelope looks like) lives in [`DOMAIN.md`](./DOMAIN.md). This file documents the *behavioural* side: what gets raised, where, and how the action recovers or fails.

## 1. Error Architecture

`notiflow` is a single-process pipeline; there is no API layer and no error wrapping in the Go/Java sense. Instead, every failure is encoded as an **exit code + a workflow-command log line on stderr**. The layers map onto the orchestrator stages in `scripts/entrypoint.sh`.

```
┌───────────────────────────────────────────────────────────────────┐
│  Caller layer (GitHub Actions runner)                              │
│  Reads the exit code; treats non-zero as a step failure.           │
│  Surfaces ::error:: / ::warning:: / ::notice:: in the run summary. │
├───────────────────────────────────────────────────────────────────┤
│  Orchestrator (scripts/entrypoint.sh)                              │
│  Runs require_bash → require_command → mask → validate → filter   │
│  → render → send. Applies the NF_FAIL_ON_ERROR policy on send fail.│
├───────────────────────────────────────────────────────────────────┤
│  Pipeline stages (lib.sh, validate.sh, filter.sh, render.sh,       │
│  send.sh)                                                          │
│  Each stage calls `nf::log <level> <msg>` then exits with a stable │
│  numeric code. No recovery between stages.                         │
├───────────────────────────────────────────────────────────────────┤
│  Infrastructure (bash, curl, jq, Telegram Bot API)                 │
│  Raw failures: missing binary (exit 22), bash too old (exit 20),   │
│  curl exit != 0 (network), Telegram HTTP responses (200/4xx/5xx/   │
│  429).                                                              │
└───────────────────────────────────────────────────────────────────┘
```

Error propagation rules:

- **Errors are created** at the deepest layer that can detect them: `validate.sh` for input contract violations, `lib.sh` for runtime preconditions, `send.sh` for transport/API failures.
- **Errors are never wrapped.** `set -euo pipefail` is enabled at `scripts/entrypoint.sh:5`; any `exit` from a sourced function terminates the whole process.
- **Only `send.sh` recovers.** It retries 429 / 5xx / network failures internally up to `_NF_MAX_ATTEMPTS=4` (`scripts/send.sh:8`). All other failures are terminal.
- **`nf::log` is the only logging primitive** (`scripts/lib.sh:11-22`). It emits GitHub Actions workflow commands so failures show up in the Job summary.
- **The send-failure policy lives in the orchestrator** (`scripts/entrypoint.sh:47-56`): on `nf::send` returning 1, exit code depends on `NF_FAIL_ON_ERROR`.

## 2. Exit Code Reference

Every numeric code that `notiflow` can return to the GitHub Actions runner.

| Code | Symbol | Stage | Trigger | Raised at | How to recover |
|------|--------|-------|---------|-----------|----------------|
| `0` | (success) | any | Message sent, OR filtered-out by `notify_on`, OR send failed with `fail_on_error=false` (soft-failure) | `scripts/entrypoint.sh:38`, `scripts/entrypoint.sh:44`, `scripts/entrypoint.sh:55` | No action needed. |
| `1` | `SEND_FAILED` | orchestrator | `nf::send` exhausted retries and `NF_FAIL_ON_ERROR` is `true`/`TRUE`/`True`/`1`/`yes` | `scripts/entrypoint.sh:47-51` | Inspect the preceding `::warning::` / `::error::` lines for `http_status`; if 4xx, fix the payload (chat_id, parse_mode escaping); if 5xx/429/network, retry the workflow. |
| `10` | `MISSING_REQUIRED_INPUT` | validate | `NF_BOT_TOKEN` or `NF_CHAT_ID` is empty | `scripts/validate.sh:27-30` | Set both `with.bot_token` and `with.chat_id` in the step. Use a GitHub Actions secret for the token. |
| `11` | `INVALID_CHAT_ID` | validate | `NF_CHAT_ID` is not `^-?[0-9]+$` and not `^@[A-Za-z0-9_]{4,32}$` | `scripts/validate.sh:33-53` | Pass a numeric chat id (positive for users, negative for groups/channels) or `@public_channel_username`. |
| `12` | `INVALID_STATUS` | validate | `NF_STATUS` (after defaulting to `JOB_STATUS`) is not in `{success, failure, cancelled, skipped}` | `scripts/validate.sh:59-62` | Omit `with.status` (the default `${{ job.status }}` is always valid) or pass one of the four allowed values. |
| `13` | `INVALID_PARSE_MODE` | validate | `NF_PARSE_MODE` is not in `{MarkdownV2, HTML, Markdown, none}` | `scripts/validate.sh:69-72` | Use one of the four allowed modes; omit the input to get the `MarkdownV2` default. |
| `14` | `INVALID_NOTIFY_ON` | validate | Any comma-separated token in `NF_NOTIFY_ON` is not in `{success, failure, cancelled, skipped}` after whitespace trimming | `scripts/validate.sh:79-91` | Fix the CSV list — for example, `success,failure,cancelled`. Whitespace around commas is allowed. |
| `15` | `INVALID_THREAD_ID` | validate | `NF_MESSAGE_THREAD_ID` is set but does not match `^[0-9]+$` | `scripts/validate.sh:95-100` | Pass a non-negative integer (Telegram forum topic id), or omit the input to send to the general topic. |
| `20` | `UNSUPPORTED_BASH` | runtime | `BASH_VERSINFO[0] < 3` and no fallback bash at `/opt/homebrew/bin/bash` or `/usr/local/bin/bash` | `scripts/lib.sh:55-67` | Upgrade bash (`brew install bash` on macOS). On GitHub-hosted runners this should never trigger. |
| `22` | `MISSING_DEPENDENCY:<name>` | runtime | `command -v <name>` returned non-zero; `<name>` is `curl` or `jq` (called from `scripts/entrypoint.sh:22-23`) | `scripts/lib.sh:45-50` | Install the named binary on the runner. Both are preinstalled on `ubuntu-latest` and `macos-latest`. |

Notes:

- **Code `21` is intentionally unused.** The original design (`design.md` §2.7, row "Отсутствует curl") reserved `21` for `MISSING_DEPENDENCY:curl` and `22` for `MISSING_DEPENDENCY:jq`. The implementation collapsed both into the single code `22` via the generic `nf::require_command` helper, with the dependency name carried in the log message. This is a deliberate simplification; do not introduce `21` unless you also split the helper.
- **Group 1 of `requirements.md`** (REQ-1.1..1.6) maps 1:1 onto codes 10..13. Group 6 (`message_thread_id`) is REQ-6.4 → code 15. Group 3 (`notify_on`) is REQ-3.3 → code 14.

## 3. Telegram API Error Matrix

`nf::send` (`scripts/send.sh:73-154`) classifies every Telegram response by HTTP status. The retry budget is `_NF_MAX_ATTEMPTS=4` total attempts (one initial + up to three retries), per REQ-7.2 and REQ-7.3.

| HTTP status | Class | Retry? | Wait between retries | Final outcome on exhaustion | Implementation |
|-------------|-------|--------|----------------------|------------------------------|----------------|
| `200` with `ok==true` | Success | n/a | n/a | Sets outputs `ok=true`, `message_id=<id>`, `http_status=200`; returns 0 | `scripts/send.sh:100-108` |
| `200` with `ok==false` | Anomaly | Yes — loop continues | none (immediate next attempt) | Falls through to "after attempts" branch → `ok=false`, last `http_status=200` | `scripts/send.sh:109` |
| `429` Too Many Requests | Rate-limited | Yes, up to 3 retries | `parameters.retry_after` seconds from the JSON body, default `1` if missing | `ok=false`, `http_status=429`; orchestrator applies `fail_on_error` policy | `scripts/send.sh:111-118` (REQ-7.2) |
| `5xx` Server error | Transient | Yes, up to 3 retries | Exponential: `1s`, `2s`, `4s` (`1 << (attempt - 1)`) | `ok=false`, `http_status=<last 5xx>`; orchestrator applies `fail_on_error` policy | `scripts/send.sh:119-125` (REQ-7.3) |
| `0` (curl exit != 0) | Network error | Yes, up to 3 retries | Same exponential schedule as 5xx | `ok=false`, `http_status=0`; orchestrator applies `fail_on_error` policy | `scripts/send.sh:84-96`, `scripts/send.sh:126-133` |
| `4xx` (any other) | Client error | **No** — terminal | n/a | Logs `::error::` with `description` from JSON body; sets `ok=false`, `http_status=<4xx>`; returns 1 immediately | `scripts/send.sh:134-140` (REQ-7.6) |
| Other (`1xx`, `3xx`, unknown) | Unexpected | Yes, loop continues | none (immediate next attempt) | Same as anomaly | `scripts/send.sh:141-143` |

After the retry loop drains, the post-loop branch at `scripts/send.sh:149-153` logs `send failed after $_NF_MAX_ATTEMPTS attempts` as a warning, writes the failure outputs, and returns 1 to the orchestrator. The orchestrator then applies the `fail_on_error` policy described in §2 (exit code `0` or `1`).

For the JSON shape of the Telegram error envelope (`{ "ok": false, "error_code": ..., "description": ..., "parameters": { "retry_after": ... } }`), see [`DOMAIN.md`](./DOMAIN.md) §2 (Telegram Bot API response).

## 4. Workflow-Command Log Markers

Every log line in `notiflow` is produced by `nf::log <level> <message>` (`scripts/lib.sh:11-22`) and is written to **stderr** with a GitHub Actions workflow-command prefix. The level name passed by the caller is normalised inside the function.

| Caller level | Emitted marker | Where it surfaces in the GitHub Actions UI |
|--------------|----------------|--------------------------------------------|
| `debug` | `::debug::<msg>` | Hidden by default. Visible only when the workflow runs with `ACTIONS_STEP_DEBUG=true` or the user enables debug logging on the run. |
| `info`, `notice` | `::notice::<msg>` | Shown in the step log; aggregated into the "Annotations" tab on the run summary as a blue notice. Not surfaced on the PR diff. |
| `warn`, `warning` | `::warning::<msg>` | Yellow annotation in the step log and the run summary. Does **not** fail the step. |
| `error` | `::error::<msg>` | Red annotation. Does **not** by itself fail the step (only a non-zero exit code does), but appears prominently in the summary and on the run page. |
| anything else | plain `<msg>` (no prefix) | Falls through to the default `*)` arm; printed verbatim to stderr. Used as a safety net only. |

Where each level is used today:

- `::error::` — only at *fatal* sites: every `exit` in `validate.sh` (lines 28, 37, 43, 49, 60, 70, 88, 97), `nf::require_command` (`scripts/lib.sh:47`), `nf::require_bash` (`scripts/lib.sh:65`), the 4xx terminal branch in `nf::send` (`scripts/send.sh:135`), and the orchestrator's `SEND_FAILED (fail_on_error=true)` line (`scripts/entrypoint.sh:50`).
- `::warning::` — every retryable failure in `nf::send` (`scripts/send.sh:89, 109, 115, 122, 130, 142`), the post-retry give-up line (`scripts/send.sh:149`), and the orchestrator's soft-failure line `SEND_FAILED (fail_on_error=false) — exiting 0 to avoid masking job result` (`scripts/entrypoint.sh:53`). Also `UNKNOWN_PLACEHOLDER:<name>` from `nf::render` (`scripts/render.sh:124`).
- `::notice::` — the filter-skip message `notiflow: skipping (status=<S> not in notify_on=<L>)` (`scripts/entrypoint.sh:34`).
- `::debug::` — not currently emitted by `notiflow`; reserved for future diagnostics.

In addition to log markers, `notiflow` emits one **value-mask** workflow command:

- `::add-mask::<bot_token>` — written by `nf::mask` (`scripts/lib.sh:26-28`) from `scripts/entrypoint.sh:28`, **before** validation, so the token cannot leak in any subsequent log line. This is REQ-2.1.

## 5. Output Contract on Failure

Even on failure, `notiflow` writes three step outputs to `$GITHUB_OUTPUT` via `nf::set_output` (`scripts/lib.sh:32-35`). Downstream steps can branch on these without parsing logs.

| Output | On `200/ok=true` | On filtered-out (skip) | On 4xx terminal | On retries exhausted | On validation failure |
|--------|-------------------|------------------------|------------------|----------------------|------------------------|
| `ok` | `true` | `false` | `false` | `false` | (not set — process exits before send) |
| `message_id` | `<id>` from `result.message_id` | `""` | `""` | `""` | (not set) |
| `http_status` | `200` | `0` | `<4xx>` | last seen status (`5xx`, `429`, or `0`) | (not set) |

References: `scripts/send.sh:102-107` (success), `scripts/entrypoint.sh:35-37` (skip), `scripts/send.sh:136-138` (4xx), `scripts/send.sh:150-152` (exhaustion). The validation path never sets outputs because the process exits at `validate.sh` before reaching `nf::send`.

This matches REQ-8.1 (success outputs) and REQ-8.2 (failure outputs).

## 6. Retry Policy Summary

A consolidated view of which errors are retried, with cross-references to the per-error rows above.

| Error category | Retryable | Strategy | Source |
|----------------|-----------|----------|--------|
| Validation (exit 10–15) | No | Fail fast on first detection | `scripts/validate.sh` |
| Runtime preconditions (exit 20, 22) | No | Fail fast at orchestrator startup | `scripts/lib.sh:45-67` |
| Telegram `4xx` (except 429) | No | Log `::error::`, return 1, apply `fail_on_error` | `scripts/send.sh:134-140` |
| Telegram `429` | Yes (max 3 retries) | Sleep `parameters.retry_after` (default 1s) | `scripts/send.sh:111-118` |
| Telegram `5xx` | Yes (max 3 retries) | Exponential backoff `1s/2s/4s` | `scripts/send.sh:119-125` |
| Network error (`curl` exit ≠ 0) | Yes (max 3 retries) | Exponential backoff `1s/2s/4s` | `scripts/send.sh:84-96`, `scripts/send.sh:126-133` |
| `200` with `ok=false` (anomaly) | Yes — loop continues | No sleep | `scripts/send.sh:109` |

The retry budget is fixed at `_NF_MAX_ATTEMPTS=4` (one initial attempt plus up to three retries) and is not configurable as of v1.0.0.

## 7. Soft-Failure Policy (`fail_on_error`)

The orchestrator's final responsibility is translating "send returned 1" into an exit code, governed by the `NF_FAIL_ON_ERROR` env var (action input `fail_on_error`).

```
scripts/entrypoint.sh:47-56
case "${NF_FAIL_ON_ERROR:-false}" in
  true | TRUE | True | 1 | yes)
    nf::log error "SEND_FAILED (fail_on_error=true)"
    exit 1
    ;;
  *)
    nf::log warn "SEND_FAILED (fail_on_error=false) — exiting 0 to avoid masking job result"
    exit 0
    ;;
esac
```

- **`fail_on_error=true`** (REQ-7.4): exit `1`, log `::error::SEND_FAILED (fail_on_error=true)`. Use this when the notification itself is mission-critical.
- **`fail_on_error=false`** (REQ-7.5, default): exit `0`, log `::warning::SEND_FAILED (fail_on_error=false) — exiting 0 to avoid masking job result`. This is the safe default — a broken notification must not flip a green job to red, but the warning is still visible in the run summary.

Note that the soft-failure policy applies **only** to send-time failures. Validation and runtime-precondition failures are always hard (codes 10–15, 20, 22) regardless of `fail_on_error` — they indicate a broken caller configuration that no amount of retrying can fix.

## 8. Cross-References

- [`DOMAIN.md`](./DOMAIN.md) §2 — Telegram API response envelope shape (the JSON that `send.sh` parses with `jq`).
- [`DOMAIN.md`](./DOMAIN.md) §3 (Business Errors) — points back to this file as the canonical catalog.
- [`CODE_STYLE.md`](./CODE_STYLE.md) — `nf::*` naming, exit-code conventions, logging rules.
- [`PACKAGES.md`](./PACKAGES.md) §3 — `validate.sh` per-function reference with the same exit-code table.
- [`features/telegram-notify-action/requirements.md`](./features/telegram-notify-action/requirements.md) Groups 1, 3, 6, 7, 8 — the requirements rows behind every code in §2 and the retry rules in §3.
- [`features/telegram-notify-action/design.md`](./features/telegram-notify-action/design.md) §2.7 — the original Error Handling table from which §2 and §3 here are derived.
