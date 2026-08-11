---
title: Using the Action
description: Wiring notiflow into a workflow, from the minimal case to per-status templates.
---

## The minimal step

```yaml
- uses: jtprogru/notiflow@v2
  if: always()
  with:
    bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
    chat_id: ${{ secrets.TELEGRAM_CHAT_ID }}
    status: ${{ job.status }}
```

Three things about that snippet are load-bearing:

`if: always()` — steps do not run after a failed step unless you say so, and the failure
is exactly the case you wanted a message about. Use `if: always()` for "tell me either
way" and `if: failure()` for "only when it breaks".

`status` — must be passed. Composite-action input defaults cannot read the `job` context,
so there is no way for notiflow to default it to `job.status` on your behalf.

`bot_token` — comes from a secret. notiflow masks it in the step log the moment it starts,
but a token pasted into the workflow file is already in your git history.

## Reporting on another job

`job.status` is the status of the job the step lives in. To notify about a job that ran
earlier, add a reporting job and read `needs`:

```yaml
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - run: make ci

  notify:
    needs: [build]
    if: always()
    runs-on: ubuntu-latest
    steps:
      - uses: jtprogru/notiflow@v2
        with:
          bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
          chat_id: ${{ secrets.TELEGRAM_CHAT_ID }}
          status: ${{ needs.build.result }}
```

## Choosing when to speak

`notify_on` is a comma-separated list of statuses that should produce a message. The
default is `success,failure,cancelled` — everything except `skipped`.

```yaml
    notify_on: failure,cancelled     # only bad news
    notify_on: any                   # everything, including skipped
```

When the reported status is not in the list, notiflow exits 0 without contacting Telegram,
and sets `ok=false` with an empty `message_id`.

## Custom text

Three ways, in order of precedence.

`message` is verbatim: no placeholders, no escaping, exactly what you wrote.

```yaml
    message: "Nightly build finished"
```

`message_template` is rendered with [placeholders](/notiflow/templates/):

```yaml
    message_template: |
      {{.StatusEmoji}} *{{.Workflow}}* — {{.Status}}
      `{{.Repo}}` @ `{{.ShortSha}}`
      [run]({{.RunUrl}})
```

`template_success`, `template_failure`, `template_cancelled` and `template_skipped`
override `message_template` for one status:

```yaml
    message_template: "{{.StatusEmoji}} {{.Workflow}}: {{.Status}}"
    template_failure: |
      ❌ *{{.Workflow}}* broke on `{{.Branch}}`
      {{.Actor}} pushed `{{.ShortSha}}`
      [logs]({{.RunUrl}})
```

## Using the outputs

```yaml
- uses: jtprogru/notiflow@v2
  id: notify
  with:
    bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
    chat_id: ${{ secrets.TELEGRAM_CHAT_ID }}
    status: ${{ job.status }}

- if: steps.notify.outputs.ok != 'true'
  run: echo "notification failed: ${{ steps.notify.outputs.error }}"
```

## Failing the job when the message fails

By default a delivery failure does not fail the job: notiflow's opinion about Telegram
should not overwrite the result your build actually produced. If the notification is part
of the contract, opt in:

```yaml
    fail_on_error: true
```

## Several chats

v2 sends to exactly one chat per step. Use a matrix when you want several:

```yaml
jobs:
  notify:
    strategy:
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

Each chat gets its own step in the UI, its own outputs and its own pass or fail, which the
comma-separated form in v1.5 could not give you. See the
[migration guide](/notiflow/action/migration/) for the reasoning.
