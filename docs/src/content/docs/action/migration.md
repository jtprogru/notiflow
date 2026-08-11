---
title: Migrating from v1
description: What changed between the bash Action and the Rust one, and what you have to do about it.
---

For a workflow that sends to one chat, `jtprogru/notiflow@v1` → `jtprogru/notiflow@v2` is a
drop-in change. Every input keeps its name and meaning, every output keeps its name, and
exit codes 10–16 still mean what they meant.

There is exactly one breaking change, plus a set of fixes that change behaviour only in
cases that were already broken.

## Breaking: multi-chat fan-out is gone

v1.5.0 let `chat_id` hold a comma-separated list, with a parallel list in
`edit_message_id`. v2 sends to exactly one chat.

| | v1 | v2 |
|---|---|---|
| `chat_id` | one value or a CSV | one value; a comma exits 11 |
| `edit_message_id` | integer or CSV, length had to match `chat_id` | one integer |
| `message_id` output | CSV in input order, empty slot for a failed chat (`42,,103`) | a single id |
| `error` output | `chat <id>: <reason>` joined with `; ` | the raw reason |
| `http_status` output | 200, or the first non-200 | the status of the one request |
| exit 16 | CSV lengths did not match | the value is not a positive integer |

### Why

The aggregated outputs were lossy in the way that mattered most. `message_id=42,,103` told
you something failed but not what, `http_status` collapsed several results into one number,
and the step showed a single green or red tick for what were really N independent
deliveries. Rebuilding that fidelity inside one step means reinventing what the job matrix
already does properly.

### What to do instead

A matrix, when the chats are known up front:

```yaml
jobs:
  notify:
    needs: [build]
    if: always()
    strategy:
      fail-fast: false
      matrix:
        chat: ['-1001111111111', '-1002222222222']
    runs-on: ubuntu-latest
    steps:
      - uses: jtprogru/notiflow@v2
        with:
          bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
          chat_id: ${{ matrix.chat }}
          status: ${{ needs.build.result }}
```

Or repeated steps, when they are not:

```yaml
      - uses: jtprogru/notiflow@v2
        id: team
        with:
          bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
          chat_id: '-1001111111111'
          status: ${{ job.status }}

      - uses: jtprogru/notiflow@v2
        id: oncall
        if: always()
        with:
          bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
          chat_id: '-1002222222222'
          status: ${{ job.status }}
```

Both give per-chat outputs and per-chat status in the Actions UI.

If you were pairing `edit_message_id` with a CSV `chat_id`, each matrix leg now carries its
own `message_id` — which is what editing needed anyway, since the ids were never
interchangeable between chats.

Multi-chat fan-out is tracked as a feature request rather than deleted from memory. If you
depended on it, that issue is the place to say so.

## Fixes that change observable behaviour

Every one of these is recorded in the parity corpus with the v1 and v2 outputs side by
side, so the difference is documented rather than discovered.

### Placeholders are substituted once

v1 substituted key by key in a loop, so a value inserted early was itself scanned for
placeholders by every later iteration. A workflow named `{{.Actor}}` really did expand to
the actor's name under `parse_mode: none` and `HTML`. Under `MarkdownV2` escaping hid it by
accident. v2 reads the template exactly once.

### Truncation cuts on markup boundaries

Messages over Telegram's 4096-unit limit are cut. v1 cut on a raw UTF-16 code-unit
boundary, which could sever a MarkdownV2 escape pair (`\` + `.`), an HTML entity
(`&amp;`) or a tag — Telegram answered 400 on a message whose only sin was being long.
v2 cuts between markup tokens, and in HTML mode also closes tags left open by the cut.

The same code path had a second failure on macOS: BSD `iconv` exits non-zero on a trailing
incomplete character even with `-c`, so a long message ending mid-emoji aborted the send
entirely. v2 has no `iconv`.

### The token is scrubbed from every stream

`::add-mask::` covers the workflow log, but the CLI has no equivalent and an HTTP error can
carry a URL containing `bot<TOKEN>`. v2 filters everything it prints, and rewrites any
surviving `/bot<token>/` path segment even for a token it was never told about.

### `--bot-token` on the command line warns

Arguments are visible to every process on the machine. The flag still works; it prints a
warning, and the docs point at `NOTIFLOW_BOT_TOKEN` or a `0600` config file.

### Backoff has jitter, and `Retry-After` is honoured

v1 slept exactly 1, 2 and 4 seconds, so a fleet of jobs rate-limited together retried in
lockstep and re-triggered the limit. v2 adds up to 50% jitter. It also reads the
`Retry-After` response header when the body has no `parameters.retry_after`, which is what
proxies and self-hosted Bot API servers send.

### `disable_web_page_preview` moved on the wire

The input keeps its name; the request now carries `link_preview_options.is_disabled`,
because Telegram deprecated the flat field.

### `api_base` policy depends on where you are

Inside Actions the allowlist is unchanged: `api.telegram.org` plus loopback, anything else
ignored with a warning. From the CLI, `--api-base` accepts any `http(s)` URL, because a
self-hosted Bot API server is a legitimate setup and the value is your own explicit choice.

## Things that did not change

- Every input name and default, apart from the `chat_id` and `edit_message_id` semantics above.
- Every output name.
- Template precedence: `message` > `template_<status>` > `message_template` > built-in default.
- `message` is still verbatim: no substitution, no escaping.
- The placeholder set, the default template, and the escaping rules for each parse mode.
- Exit codes 10–16. Codes 20 (`UNSUPPORTED_BASH`) and 22 (`MISSING_DEPENDENCY`) belonged to
  the bash runtime; v2 never emits them and nothing else will ever claim those numbers.
- `notify_on` semantics, including the `any` and `all` shorthands.

## New in v2

- A CLI: `notiflow send | edit | render | whoami | completions`, installable from Homebrew,
  crates.io or a release archive.
- Windows runners are supported.
- `verify_signature` for a cosign check on the downloaded binary.
- Exit codes 17 (`CONFIG_ERROR`), 18 (`INVALID_ARGUMENT`) and 30 (`IO_ERROR`) for failure
  modes the CLI can hit and the Action mostly cannot.

## Staying on v1

The `v1` tag keeps pointing at the bash implementation on the `v1.x` branch, which receives
security fixes only. The bash sources also live in the v2 tree under `tests/parity/v1/`,
where they serve as the reference implementation the parity suite compares against on every
commit.
