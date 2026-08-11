---
title: Configuration
description: Where settings come from, in what order, and how profiles work.
---

## Precedence

Highest wins:

1. command-line flags
2. environment variables
3. the selected config-file profile
4. the config file's top-level defaults
5. built-in defaults

## Environment variables

| Variable | Equivalent flag |
|---|---|
| `NOTIFLOW_BOT_TOKEN` | `--bot-token` |
| `NOTIFLOW_CHAT_ID` | `--chat-id` |
| `NOTIFLOW_API_BASE` | `--api-base` |
| `NOTIFLOW_CONFIG` | `--config` |
| `NOTIFLOW_DEBUG` | `-vv` |

The environment is the right place for the token. A value passed as `--bot-token` is
visible in `ps` to every user on the machine, so notiflow accepts it and warns.

## The config file

Default location, in order: `$NOTIFLOW_CONFIG`, then
`$XDG_CONFIG_HOME/notiflow/config.toml`, then `~/.config/notiflow/config.toml`.

```toml
# ~/.config/notiflow/config.toml
bot_token = "123456789:AAHdqTcv..."
chat_id   = "-1001234567890"
parse_mode = "MarkdownV2"
notify_on = "success,failure,cancelled"
timeout = 15
retries = 3

[profile.work]
chat_id = "-1009876543210"
message_template = "{{.StatusEmoji}} {{.Repo}} — {{.Status}}"

[profile.selfhosted]
api_base = "https://bot-api.internal.example.com"
chat_id  = "-1005555555555"
```

Top-level keys are the defaults. `[profile.<name>]` overrides them when you ask for it:

```bash
notiflow send -m "shipped" --profile work
```

A missing file is fine unless you named one with `--config` or asked for a `--profile`;
either of those turns "not there" into a `CONFIG_ERROR` (exit 17) rather than a silent
fallback. An unknown key is also an error — a typo like `chat_ids` should not quietly do
nothing.

### File permissions

If the file holds a token, keep it to yourself:

```bash
chmod 600 ~/.config/notiflow/config.toml
```

notiflow warns when the file is readable by group or others.

## Every key

All of these work at the top level and inside a profile.

| Key | Type | Default |
|---|---|---|
| `bot_token` | string | — |
| `chat_id` | string | — |
| `parse_mode` | string | `MarkdownV2` |
| `notify_on` | string | `success,failure,cancelled` |
| `api_base` | string | `https://api.telegram.org` |
| `message_template` | string | built-in default template |
| `template_success` | string | — |
| `template_failure` | string | — |
| `template_cancelled` | string | — |
| `template_skipped` | string | — |
| `message_thread_id` | string | — |
| `disable_notification` | bool | `false` |
| `disable_web_page_preview` | bool | `true` |
| `connect_timeout` | integer, seconds | `5` |
| `timeout` | integer, seconds | `15` |
| `retries` | integer | `3` |
| `max_retry_after` | integer, seconds | `60` |
| `fail_on_error` | bool | `false` |

`message`, `status` and `edit_message_id` are deliberately absent: they describe one
invocation, not a stored preference.

## A self-hosted Bot API server

```bash
notiflow send --api-base https://bot-api.internal.example.com -m "hello"
```

From the CLI any `http(s)` URL is accepted. Inside a GitHub Actions job the allowlist
applies instead — `api.telegram.org` and loopback only — because there the variable could
have been planted by an earlier step in the same job rather than chosen by you.
