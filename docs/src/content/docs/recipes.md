---
title: Recipes
description: Worked examples for the situations people actually hit.
---

## Edit one message instead of sending three

Announce the deploy, then rewrite the same message with the result — the chat keeps one
line instead of a running commentary.

```yaml
jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: jtprogru/notiflow@v2
        id: announce
        with:
          bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
          chat_id: ${{ secrets.TELEGRAM_CHAT_ID }}
          status: success
          message_template: '⏳ deploying `{{.ShortSha}}` to production…'

      - run: ./deploy.sh

      - uses: jtprogru/notiflow@v2
        if: always()
        with:
          bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
          chat_id: ${{ secrets.TELEGRAM_CHAT_ID }}
          status: ${{ job.status }}
          edit_message_id: ${{ steps.announce.outputs.message_id }}
          notify_on: any
          template_success: '✅ `{{.ShortSha}}` is live'
          template_failure: '❌ deploy of `{{.ShortSha}}` failed — [logs]({{.RunUrl}})'
```

`notify_on: any` on the second step matters: with the default set, a cancelled job would
skip the edit and leave "deploying…" on screen forever.

The same shape from a shell script:

```bash
id=$(notiflow send -m "⏳ deploying…" --json | jq -r .message_id)
if ./deploy.sh; then
  notiflow edit --message-id "$id" -m "✅ deployed"
else
  notiflow edit --message-id "$id" -m "❌ deploy failed"
fi
```

## Several chats

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

`fail-fast: false` keeps the second chat from being cancelled because the first one failed.

## Different chats for different outcomes

```yaml
      - uses: jtprogru/notiflow@v2
        if: success()
        with:
          bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
          chat_id: ${{ secrets.TEAM_CHAT_ID }}
          status: success

      - uses: jtprogru/notiflow@v2
        if: failure()
        with:
          bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
          chat_id: ${{ secrets.ONCALL_CHAT_ID }}
          status: failure
          fail_on_error: true
```

`fail_on_error: true` only on the on-call path: if the page did not arrive, that is worth
failing the job over.

## Posting into a forum topic

```yaml
    message_thread_id: '42'
```

The topic id is in the URL when you open the topic in Telegram Web. It is dropped
automatically when editing, because Telegram rejects it there.

## Reporting on the whole workflow

One job that waits for everything and reports the aggregate:

```yaml
jobs:
  lint: { runs-on: ubuntu-latest, steps: [{ run: make lint }] }
  test: { runs-on: ubuntu-latest, steps: [{ run: make test }] }

  notify:
    needs: [lint, test]
    if: always()
    runs-on: ubuntu-latest
    steps:
      - uses: jtprogru/notiflow@v2
        with:
          bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
          chat_id: ${{ secrets.TELEGRAM_CHAT_ID }}
          status: ${{ (contains(needs.*.result, 'failure') && 'failure') || (contains(needs.*.result, 'cancelled') && 'cancelled') || 'success' }}
```

## Sending a log tail

```bash
./build.sh > build.log 2>&1 || {
  tail -40 build.log | notiflow send --status failure --parse-mode none --stdin
  exit 1
}
```

`--parse-mode none` because compiler output is full of `_`, `[` and `*`, and none of it is
meant as markup.

## A self-hosted Bot API server

```bash
notiflow send --api-base https://bot-api.internal.example.com -m "hello"
```

Or permanently, in a profile:

```toml
[profile.internal]
api_base = "https://bot-api.internal.example.com"
chat_id  = "-1005555555555"
```

```bash
notiflow send --profile internal -m "hello"
```

Inside a GitHub Actions job the allowlist applies instead — see the
[Action reference](/notiflow/action/reference/).

## Verifying the download

```yaml
      - uses: sigstore/cosign-installer@v3
      - uses: jtprogru/notiflow@v2
        with:
          verify_signature: true
          bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
          chat_id: ${{ secrets.TELEGRAM_CHAT_ID }}
          status: ${{ job.status }}
```

## Notifying from a container without glibc

The `musl` archives are fully static, so they run on `alpine` and even `scratch`:

```dockerfile
FROM alpine
ADD https://github.com/jtprogru/notiflow/releases/download/v2.0.1/notiflow-x86_64-unknown-linux-musl.tar.gz /tmp/
RUN tar -xzf /tmp/notiflow-x86_64-unknown-linux-musl.tar.gz -C /usr/local/bin
```

## Local git as the placeholder source

Outside a workflow, `{{.Repo}}`, `{{.Branch}}`, `{{.Sha}}` and `{{.ShortSha}}` come from
the checkout you are standing in, so a git hook can reuse a workflow's template unchanged:

```bash
# .git/hooks/post-merge
notiflow send --template 'pulled `{{.ShortSha}}` on {{.Branch}}' --quiet
```
