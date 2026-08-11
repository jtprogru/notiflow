`notiflow` — Telegram notifier for CI and the terminal

## `notiflow send`

Send a message

| Flag | Value | Description |
|---|---|---|
| `--config` | `PATH` | Config file to read (default: $XDG_CONFIG_HOME/notiflow/config.toml) |
| `--profile` | `NAME` | Profile inside the config file |
| `--output` | `FORMAT` | Output format |
| `--json` | `JSON` | Shorthand for --output json |
| `-q`, `--quiet` | `QUIET` | Print nothing on success |
| `-v`, `--verbose` | `VERBOSE` | Increase log verbosity; repeat for debug-level detail |
| `--bot-token` | `TOKEN` | Bot token. Prefer the environment variable: arguments are visible in `ps` |
| `-c`, `--chat-id` | `CHAT` | Destination chat: an integer id or @channelusername. Exactly one |
| `--api-base` | `URL` | Bot API base URL, for a self-hosted Bot API server |
| `--timeout` | `SECONDS` | Whole-request timeout in seconds |
| `--connect-timeout` | `SECONDS` | Connection timeout in seconds |
| `--retries` | `N` | Retries after the first attempt |
| `--max-retry-after` | `SECONDS` | Cap applied to a 429's requested retry delay, in seconds |
| `-m`, `--message` | `TEXT` | Verbatim message text. No placeholders, no escaping. `-` reads stdin |
| `--message-file` | `PATH` | Read the verbatim message from a file |
| `--stdin` | `STDIN` | Read the verbatim message from stdin |
| `-T`, `--template` | `TEXT` | Template with {{.Field}} placeholders, used for every status |
| `--template-success` | `TEXT` | Template used when status=success |
| `--template-failure` | `TEXT` | Template used when status=failure |
| `--template-cancelled` | `TEXT` | Template used when status=cancelled |
| `--template-skipped` | `TEXT` | Template used when status=skipped |
| `-s`, `--status` | `STATUS` | Status being reported |
| `--notify-on` | `CSV` | Statuses that should produce a message; `any` means all of them |
| `--parse-mode` | `MODE` | MarkdownV2, HTML, Markdown (an alias for MarkdownV2), or none |
| `--thread-id` | `ID` | Forum topic id |
| `--silent` | `SILENT` | Deliver without a notification sound |
| `--preview` | `PREVIEW` | Show link previews (they are suppressed by default) |
| `--no-preview` | `NO_PREVIEW` | Suppress link previews. The default; accepted for symmetry with --preview |
| `--dry-run` | `DRY_RUN` | Render and validate, then stop without contacting Telegram |
| `--fail-on-error` | `FAIL_ON_ERROR` | Exit non-zero when delivery ultimately fails |

## `notiflow edit`

Replace the text of a message sent earlier

| Flag | Value | Description |
|---|---|---|
| `--message-id` | `ID` | Id of the message to rewrite, typically the message_id of an earlier send. `allow_hyphen_values` keeps a negative id reaching the validator, so it fails with INVALID_EDIT_MESSAGE_ID rather than clap's generic usage error |
| `--config` | `PATH` | Config file to read (default: $XDG_CONFIG_HOME/notiflow/config.toml) |
| `--profile` | `NAME` | Profile inside the config file |
| `--output` | `FORMAT` | Output format |
| `--json` | `JSON` | Shorthand for --output json |
| `-q`, `--quiet` | `QUIET` | Print nothing on success |
| `-v`, `--verbose` | `VERBOSE` | Increase log verbosity; repeat for debug-level detail |
| `--bot-token` | `TOKEN` | Bot token. Prefer the environment variable: arguments are visible in `ps` |
| `-c`, `--chat-id` | `CHAT` | Destination chat: an integer id or @channelusername. Exactly one |
| `--api-base` | `URL` | Bot API base URL, for a self-hosted Bot API server |
| `--timeout` | `SECONDS` | Whole-request timeout in seconds |
| `--connect-timeout` | `SECONDS` | Connection timeout in seconds |
| `--retries` | `N` | Retries after the first attempt |
| `--max-retry-after` | `SECONDS` | Cap applied to a 429's requested retry delay, in seconds |
| `-m`, `--message` | `TEXT` | Verbatim message text. No placeholders, no escaping. `-` reads stdin |
| `--message-file` | `PATH` | Read the verbatim message from a file |
| `--stdin` | `STDIN` | Read the verbatim message from stdin |
| `-T`, `--template` | `TEXT` | Template with {{.Field}} placeholders, used for every status |
| `--template-success` | `TEXT` | Template used when status=success |
| `--template-failure` | `TEXT` | Template used when status=failure |
| `--template-cancelled` | `TEXT` | Template used when status=cancelled |
| `--template-skipped` | `TEXT` | Template used when status=skipped |
| `-s`, `--status` | `STATUS` | Status being reported |
| `--notify-on` | `CSV` | Statuses that should produce a message; `any` means all of them |
| `--parse-mode` | `MODE` | MarkdownV2, HTML, Markdown (an alias for MarkdownV2), or none |
| `--thread-id` | `ID` | Forum topic id |
| `--silent` | `SILENT` | Deliver without a notification sound |
| `--preview` | `PREVIEW` | Show link previews (they are suppressed by default) |
| `--no-preview` | `NO_PREVIEW` | Suppress link previews. The default; accepted for symmetry with --preview |
| `--dry-run` | `DRY_RUN` | Render and validate, then stop without contacting Telegram |
| `--fail-on-error` | `FAIL_ON_ERROR` | Exit non-zero when delivery ultimately fails |

## `notiflow render`

Render a template locally and print it — no network, no token needed

| Flag | Value | Description |
|---|---|---|
| `--config` | `PATH` | Config file to read (default: $XDG_CONFIG_HOME/notiflow/config.toml) |
| `--profile` | `NAME` | Profile inside the config file |
| `--output` | `FORMAT` | Output format |
| `--json` | `JSON` | Shorthand for --output json |
| `-q`, `--quiet` | `QUIET` | Print nothing on success |
| `-v`, `--verbose` | `VERBOSE` | Increase log verbosity; repeat for debug-level detail |
| `-m`, `--message` | `TEXT` | Verbatim message text. No placeholders, no escaping. `-` reads stdin |
| `--message-file` | `PATH` | Read the verbatim message from a file |
| `--stdin` | `STDIN` | Read the verbatim message from stdin |
| `-T`, `--template` | `TEXT` | Template with {{.Field}} placeholders, used for every status |
| `--template-success` | `TEXT` | Template used when status=success |
| `--template-failure` | `TEXT` | Template used when status=failure |
| `--template-cancelled` | `TEXT` | Template used when status=cancelled |
| `--template-skipped` | `TEXT` | Template used when status=skipped |
| `-s`, `--status` | `STATUS` | Status being reported |
| `--notify-on` | `CSV` | Statuses that should produce a message; `any` means all of them |
| `--parse-mode` | `MODE` | MarkdownV2, HTML, Markdown (an alias for MarkdownV2), or none |
| `--thread-id` | `ID` | Forum topic id |
| `--silent` | `SILENT` | Deliver without a notification sound |
| `--preview` | `PREVIEW` | Show link previews (they are suppressed by default) |
| `--no-preview` | `NO_PREVIEW` | Suppress link previews. The default; accepted for symmetry with --preview |
| `--set` | `KEY=VALUE` | Override one placeholder, e.g. --set Repo=owner/name. Repeatable |
| `--explain` | `EXPLAIN` | Also print which template was chosen and which placeholders were unknown |

## `notiflow whoami`

Check the bot token with getMe

| Flag | Value | Description |
|---|---|---|
| `--config` | `PATH` | Config file to read (default: $XDG_CONFIG_HOME/notiflow/config.toml) |
| `--profile` | `NAME` | Profile inside the config file |
| `--output` | `FORMAT` | Output format |
| `--json` | `JSON` | Shorthand for --output json |
| `-q`, `--quiet` | `QUIET` | Print nothing on success |
| `-v`, `--verbose` | `VERBOSE` | Increase log verbosity; repeat for debug-level detail |
| `--bot-token` | `TOKEN` | Bot token. Prefer the environment variable: arguments are visible in `ps` |
| `-c`, `--chat-id` | `CHAT` | Destination chat: an integer id or @channelusername. Exactly one |
| `--api-base` | `URL` | Bot API base URL, for a self-hosted Bot API server |
| `--timeout` | `SECONDS` | Whole-request timeout in seconds |
| `--connect-timeout` | `SECONDS` | Connection timeout in seconds |
| `--retries` | `N` | Retries after the first attempt |
| `--max-retry-after` | `SECONDS` | Cap applied to a 429's requested retry delay, in seconds |

## `notiflow completions`

Print a shell completion script

| Flag | Value | Description |
|---|---|---|
| `<shell>` | `SHELL` | Shell to generate for |

