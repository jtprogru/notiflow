---
title: Using the CLI
description: send, edit, render and whoami from a terminal or a shell script.
---

## send

```bash
export NOTIFLOW_BOT_TOKEN=123456789:AAHdqTcv...
export NOTIFLOW_CHAT_ID=-1001234567890

notiflow send --message "deploy finished"
notiflow send -m "deploy finished" -c @my_channel
```

The message body can come from four places:

```bash
notiflow send --message "inline text"
notiflow send --message-file release-notes.md
notiflow send --stdin < build.log
./deploy.sh | notiflow send --stdin
notiflow send -m -            # `-` is a synonym for --stdin
```

`--stdin` and `--message-file` behave like `--message`: verbatim text, no placeholder
substitution and no escaping. A trailing newline from a pipeline is stripped, because
almost nobody means to send one.

To use [placeholders](/notiflow/templates/), pass a template instead:

```bash
notiflow send --template '{{.StatusEmoji}} deployed `{{.ShortSha}}` to prod'
```

Outside a workflow the repository, branch, commit and author come from the local git
checkout, so the same template works in both places.

## Statuses and filtering

`--status` defaults to `success`. Wire it to what actually happened:

```bash
if ./deploy.sh; then status=success; else status=failure; fi
notiflow send --status "$status" --template '{{.StatusEmoji}} deploy: {{.Status}}'
```

`--notify-on` filters, exactly as in the Action:

```bash
notiflow send --status "$status" --notify-on failure,cancelled
```

A filtered run exits 0, prints `skipped: status not in notify_on`, and never opens a
connection.

## edit

Rewrite a message you sent earlier, typically to turn "deploying…" into a result:

```bash
id=$(notiflow send -m "deploying…" --json | jq -r .message_id)
./deploy.sh
notiflow edit --message-id "$id" -m "deploy finished"
```

`--silent` and `--thread-id` are dropped on an edit; Telegram rejects them there.

## render

Renders a template and prints it. No token, no network — the fast way to work out why your
escaping looks wrong:

```bash
notiflow render --template 'branch `{{.Branch}}`' --explain
# template: message_template
# parse_mode: MarkdownV2
# truncated: false
branch `feature/fix\-thing`
```

`--set` overrides one placeholder, which is how you test a template against values you do
not currently have:

```bash
notiflow render -T '{{.StatusEmoji}} {{.Workflow}} on {{.Repo}}' \
  --status failure --set Workflow=Nightly --set Repo=owner/name
```

## whoami

Checks the token against `getMe`:

```bash
notiflow whoami
# Notiflow (@notiflow_bot, id=123456789)

notiflow whoami --json | jq .username
```

Exits 1 with Telegram's own description when the token is rejected — the first thing to
run when a workflow reports 401.

## Output formats

```bash
notiflow send -m hi                  # one human-readable line
notiflow send -m hi --json           # a single JSON object
notiflow send -m hi --output github  # $GITHUB_OUTPUT entries and a step summary
```

The JSON object is stable and safe to parse:

```json
{"ok":true,"skipped":false,"message_id":4242,"http_status":200,"error":null,
 "chat_id":"-1001234567890","attempts":1,"dry_run":false}
```

`--quiet` suppresses the human line but leaves `--json` and `--output github` intact, so a
script can stay silent without losing its machine-readable result.

## Dry runs

`--dry-run` does everything except the request, and prints the exact JSON body that would
have been posted:

```bash
notiflow send -m "check me" --dry-run
```

That is the quickest way to confirm what `--parse-mode`, `--silent` and `--thread-id` are
really doing to the request.

## Timeouts and retries

```bash
notiflow send -m hi --timeout 30 --connect-timeout 10 --retries 5 --max-retry-after 120
```

Defaults: 15s total, 5s to connect, 3 retries after the first attempt, and a 60s cap on a
server-requested `retry_after`. See [Reliability](/notiflow/reliability/) for what gets
retried and what does not.
