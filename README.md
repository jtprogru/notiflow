# notiflow

[![CI](https://github.com/jtprogru/notiflow/actions/workflows/ci.yml/badge.svg)](https://github.com/jtprogru/notiflow/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

`notiflow` is a composite GitHub Action that sends a Telegram message when a workflow job completes. It supports custom message templates, per-status overrides, MarkdownV2/HTML/Markdown/plain rendering, retry on rate limits, forum-chat threads, and silent delivery.

## Usage

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: ./run-tests.sh

  notify:
    needs: build
    if: always()
    runs-on: ubuntu-latest
    steps:
      - uses: jtprogru/notiflow@v1
        with:
          bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
          chat_id:   ${{ secrets.TELEGRAM_CHAT_ID }}
          status:    ${{ needs.build.result }}
```

## Inputs

| Name | Required | Default | Description |
|------|----------|---------|-------------|
| `bot_token` | yes | — | Telegram bot token (store as a secret). |
| `chat_id` | yes | — | Target chat ID (integer, possibly negative) or `@channel_username`. Accepts a comma-separated list to fan out to multiple chats; each item is validated independently. Whitespace around items is tolerated. |
| `status` | yes | — | Job status. Must be passed explicitly (typically `${{ job.status }}` or `${{ needs.<job>.result }}`). Allowed: `success`, `failure`, `cancelled`, `skipped`. |
| `parse_mode` | no | `MarkdownV2` | `MarkdownV2`, `HTML`, `Markdown`, or `none`. |
| `notify_on` | no | `success,failure,cancelled` | Comma-separated statuses that trigger a notification. Accepts `any` or `all` as a shortcut for all four statuses. |
| `message` | no | _empty_ | Verbatim message. Bypasses templates and placeholder substitution. |
| `message_template` | no | _empty_ | Default template (used when no per-status template matches). |
| `template_success` | no | _empty_ | Template used when `status=success`. |
| `template_failure` | no | _empty_ | Template used when `status=failure`. |
| `template_cancelled` | no | _empty_ | Template used when `status=cancelled`. |
| `template_skipped` | no | _empty_ | Template used when `status=skipped`. |
| `disable_web_page_preview` | no | `true` | Suppress link previews. |
| `disable_notification` | no | `false` | Send silently. |
| `message_thread_id` | no | _empty_ | Forum-chat thread (topic) ID. Integer. |
| `fail_on_error` | no | `false` | If true, the action exits non-zero when Telegram delivery ultimately fails. |

## Outputs

| Name | Description |
|------|-------------|
| `ok` | `true` if the message was delivered to **every** chat in `chat_id`, `false` otherwise. |
| `message_id` | Telegram `message_id` on single-chat success. For CSV `chat_id`, a CSV of message_ids in input order with an empty slot for any failed chat (e.g. `42,,103`). Empty entirely on single-chat failure or skip. |
| `http_status` | Last HTTP status code observed. `200` if all chats succeeded; otherwise the first non-200 status seen. `0` for skip / network error. |
| `error` | Error reason on failure — Telegram's `.description` when available, otherwise `HTTP <code>` / `network error (curl exit N)`. Single-chat: raw reason. Multi-chat: each failed chat formatted as `chat <id>: <reason>`, joined with `; `. Empty on success and on skip. |

## Template priority

Highest wins:

1. `message` — verbatim text. No placeholder substitution, no escaping (you own the output).
2. `template_<status>` — matches the current status.
3. `message_template` — common template.
4. Built-in default template (status emoji, workflow, repo, branch, short SHA, actor, run URL).

## Placeholders

Available inside templates as `{{.Field}}`. Values are escaped according to `parse_mode` before substitution.

| Placeholder | Value |
|-------------|-------|
| `{{.Repo}}` | `owner/repo` |
| `{{.Workflow}}` | Workflow name |
| `{{.Job}}` | Job name |
| `{{.Status}}` | `success` / `failure` / `cancelled` / `skipped` |
| `{{.StatusEmoji}}` | `✅` / `❌` / `⚠️` / `⏭` |
| `{{.Actor}}` | GitHub actor (login) |
| `{{.Ref}}` | Full ref (`refs/heads/main`) |
| `{{.RefName}}` | Short ref (`main`) |
| `{{.Branch}}` | Alias for `RefName` |
| `{{.Sha}}` | Full commit SHA |
| `{{.ShortSha}}` | First 7 characters of SHA |
| `{{.RunId}}` | Run ID |
| `{{.RunNumber}}` | Run number |
| `{{.RunUrl}}` | Link to the run page |
| `{{.EventName}}` | Triggering event |
| `{{.ServerUrl}}` | `https://github.com` (or GHES URL) |

Unknown placeholders are removed and produce a `::warning::UNKNOWN_PLACEHOLDER:<name>` annotation.

## Examples

### Custom message template

```yaml
- uses: jtprogru/notiflow@v1
  with:
    bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
    chat_id:   ${{ secrets.TELEGRAM_CHAT_ID }}
    status:    ${{ job.status }}
    message_template: |
      {{.StatusEmoji}} *{{.Workflow}}* on `{{.Repo}}`@`{{.Branch}}`
      by *{{.Actor}}* — [open run]({{.RunUrl}})
```

### Per-status templates

```yaml
- uses: jtprogru/notiflow@v1
  with:
    bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
    chat_id:   ${{ secrets.TELEGRAM_CHAT_ID }}
    status:    ${{ job.status }}
    template_success: "✅ {{.Repo}} build {{.RunNumber}} is green"
    template_failure: |
      ❌ {{.Repo}} build {{.RunNumber}} *FAILED*
      Actor: {{.Actor}}
      Run: {{.RunUrl}}
```

### Notify only on failure or cancellation

```yaml
- uses: jtprogru/notiflow@v1
  with:
    bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
    chat_id:   ${{ secrets.TELEGRAM_CHAT_ID }}
    status:    ${{ needs.build.result }}
    notify_on: 'failure,cancelled'
```

### Fan-out to multiple chats

```yaml
- uses: jtprogru/notiflow@v1
  with:
    bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
    chat_id:   '${{ secrets.TELEGRAM_CHAT_ID }},@release_channel,-1001234567890'
    status:    ${{ job.status }}
```

`message_id` becomes a CSV in input order (e.g. `42,99,103`). On partial failure the failed slot is empty (`42,,103`) and `error` enumerates the failed chats (`chat @release_channel: Forbidden; chat -1001234567890: chat not found`). `ok=false` and `http_status` reports the first non-200 status.

### Forum chat thread

```yaml
- uses: jtprogru/notiflow@v1
  with:
    bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
    chat_id:   ${{ secrets.TELEGRAM_CHAT_ID }}
    status:    ${{ job.status }}
    message_thread_id: '123'
```

### HTML parse_mode

```yaml
- uses: jtprogru/notiflow@v1
  with:
    bot_token:  ${{ secrets.TELEGRAM_BOT_TOKEN }}
    chat_id:    ${{ secrets.TELEGRAM_CHAT_ID }}
    status:     ${{ job.status }}
    parse_mode: 'HTML'
    message_template: |
      <b>{{.Workflow}}</b> @ <code>{{.Repo}}</code>
      Status: {{.Status}}
      Actor: <i>{{.Actor}}</i>
      <a href="{{.RunUrl}}">Open run</a>
```

### Verbatim message (no escaping, no templating)

```yaml
- uses: jtprogru/notiflow@v1
  with:
    bot_token:  ${{ secrets.TELEGRAM_BOT_TOKEN }}
    chat_id:    ${{ secrets.TELEGRAM_CHAT_ID }}
    status:     ${{ job.status }}
    parse_mode: 'none'
    message:    'Plain text alert — no placeholders, no escaping'
```

## Behavior notes

- **Token masking.** The first action `notiflow` takes is `::add-mask::<bot_token>`, so the token never leaks in subsequent log lines.
- **Default `fail_on_error: false`.** Telegram delivery is a side-channel — when it fails (network, rate-limit exhausted, 4xx) the action still exits 0 with `ok=false`. Set `fail_on_error: true` to surface failures.
- **Retry policy.** Up to 4 attempts. `429` honors `parameters.retry_after` from the response body. `5xx` and network errors use 1s/2s/4s exponential backoff. `4xx` other than `429` is final — no retry.
- **Length limit.** Telegram caps messages at 4096 UTF-16 code units. BMP codepoints (including Cyrillic) count as 1 unit each; supplementary-plane codepoints (most emoji) count as 2 each. Longer renders are truncated to 4093 units + `...`, aligned to codepoint boundaries.
- **Verbatim `message`.** Bypasses both templating and escaping. With `parse_mode=MarkdownV2`/`HTML` you are responsible for valid markup.

## Requirements

- A Linux, macOS, or Windows GitHub-hosted runner (defaults: bash, curl, jq, iconv are available).
- For self-hosted runners: bash 3.2+, `curl`, `jq`, `iconv` on `PATH`.

## Development

```bash
make install-tools    # bats-core, shellcheck, shfmt, actionlint, jq
make lint             # shellcheck + shfmt -d + actionlint
make lint-fix         # shfmt -w
make test             # bats tests/
```

## Versioning

[Semantic Versioning](https://semver.org/). The `v1` tag is a moving major tag that always points at the latest `v1.x.y` release. Pin to `v1` for automatic patch/minor updates, or to a fixed tag (`v1.2.3`) for reproducible builds.

## License

[MIT](LICENSE) © 2026 Mikhail Savin.
