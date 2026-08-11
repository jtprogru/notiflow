---
title: Getting started
description: Send your first notiflow message from a workflow and from a terminal.
sidebar:
  order: 1
---

## What you need first

A Telegram bot token and a chat id. If you have neither:

1. Message [@BotFather](https://t.me/BotFather), send `/newbot`, and keep the token it gives you. It looks like `123456789:AAHdqTcv...`.
2. Add the bot to the target chat or channel. For a channel it must be an administrator.
3. Find the chat id. The simplest way is to send one message in the chat and read it back:

```bash
curl -s "https://api.telegram.org/bot<TOKEN>/getUpdates" | grep -o '"chat":{"id":[-0-9]*'
```

Group and supergroup ids are negative and usually start with `-100`. A public channel can
also be addressed as `@channelusername`.

Check that the token works before wiring anything up:

```bash
NOTIFLOW_BOT_TOKEN=<TOKEN> notiflow whoami
# Notiflow (@notiflow_bot, id=123456789)
```

## The Action in 60 seconds

Store the token and the chat id as repository secrets, then add one step at the end of the
job you care about:

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - run: make test

      - uses: jtprogru/notiflow@v2
        if: always()
        with:
          bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
          chat_id: ${{ secrets.TELEGRAM_CHAT_ID }}
          status: ${{ job.status }}
```

`if: always()` matters: without it the step is skipped when the job fails, which is
precisely when you wanted to hear about it.

`status` has to be passed explicitly. A composite action's input defaults cannot read the
`job` context, so notiflow cannot fill it in for you.

By default you get a message on `success`, `failure` and `cancelled`, formatted with the
built-in template. To be told only about failures:

```yaml
        with:
          bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
          chat_id: ${{ secrets.TELEGRAM_CHAT_ID }}
          status: ${{ job.status }}
          notify_on: failure
```

## The CLI in 60 seconds

```bash
brew install jtprogru/tap/notiflow      # or: cargo install notiflow
export NOTIFLOW_BOT_TOKEN=123456789:AAHdqTcv...
export NOTIFLOW_CHAT_ID=-1001234567890

notiflow send --message "deploy finished"
```

Anything on stdin becomes the message, which makes notiflow a drop-in end to a pipeline:

```bash
./deploy.sh 2>&1 | tail -20 | notiflow send --stdin
```

Templates work outside Actions too — repository, branch and commit come from the local git
checkout when the `GITHUB_*` variables are absent:

```bash
notiflow send --template '{{.StatusEmoji}} deployed `{{.ShortSha}}` from {{.Branch}}'
```

To see what a template produces without sending anything:

```bash
notiflow render --template '{{.Repo}} @ {{.ShortSha}}' --explain
```

## Where to go next

- [Installation](/notiflow/install/) — every way to get the binary
- [Action reference](/notiflow/action/reference/) — every input and output
- [Templates](/notiflow/templates/) — the full placeholder list and precedence rules
- [Reliability](/notiflow/reliability/) — retries, error codes, exit codes
