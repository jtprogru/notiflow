# Changelog

All notable changes are documented in this file. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/).

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
