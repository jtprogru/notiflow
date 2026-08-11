---
title: Использование Action
description: Как встроить notiflow в workflow — от минимума до шаблонов на каждый статус.
---

## Минимальный шаг

```yaml
- uses: jtprogru/notiflow@v2
  if: always()
  with:
    bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
    chat_id: ${{ secrets.TELEGRAM_CHAT_ID }}
    status: ${{ job.status }}
```

Здесь три несущие детали.

`if: always()` — шаги после упавшего шага не выполняются, если явно не сказать обратное, а
падение — ровно тот случай, ради которого сообщение и нужно. `if: always()` — «расскажи в
любом случае», `if: failure()` — «только когда сломалось».

`status` — обязателен. Дефолты входов composite action не видят контекст `job`, так что
подставить `job.status` за тебя невозможно.

`bot_token` — из secret. notiflow маскирует его в логе шага сразу на старте, но токен,
вписанный в файл workflow, уже лежит в истории git.

## Отчёт про другой job

`job.status` — это статус того job, в котором стоит шаг. Чтобы отчитаться про job, который
отработал раньше, заведи отдельный job и читай `needs`:

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

## Когда говорить

`notify_on` — список статусов через запятую, на которые нужно сообщение. По умолчанию
`success,failure,cancelled`, то есть всё кроме `skipped`.

```yaml
    notify_on: failure,cancelled     # только плохие новости
    notify_on: any                   # всё, включая skipped
```

Если статус не в списке, notiflow выходит с 0, не обращаясь к Telegram, и ставит
`ok=false` с пустым `message_id`.

## Свой текст

Три способа, по приоритету.

`message` — дословно: без плейсхолдеров и без экранирования, ровно как написано.

```yaml
    message: "Ночная сборка закончилась"
```

`message_template` — рендерится с [плейсхолдерами](/notiflow/ru/templates/):

```yaml
    message_template: |
      {{.StatusEmoji}} *{{.Workflow}}* — {{.Status}}
      `{{.Repo}}` @ `{{.ShortSha}}`
      [запуск]({{.RunUrl}})
```

`template_success`, `template_failure`, `template_cancelled` и `template_skipped`
перекрывают `message_template` для одного статуса:

```yaml
    message_template: "{{.StatusEmoji}} {{.Workflow}}: {{.Status}}"
    template_failure: |
      ❌ *{{.Workflow}}* сломался на `{{.Branch}}`
      {{.Actor}} запушил `{{.ShortSha}}`
      [логи]({{.RunUrl}})
```

## Выходы

```yaml
- uses: jtprogru/notiflow@v2
  id: notify
  with:
    bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
    chat_id: ${{ secrets.TELEGRAM_CHAT_ID }}
    status: ${{ job.status }}

- if: steps.notify.outputs.ok != 'true'
  run: echo "уведомление не ушло: ${{ steps.notify.outputs.error }}"
```

## Ронять job при сбое отправки

По умолчанию сбой доставки не роняет job: мнение notiflow о Telegram не должно затирать
результат, который реально дала сборка. Если уведомление — часть контракта, включи явно:

```yaml
    fail_on_error: true
```

## Несколько чатов

v2 отправляет ровно в один чат за шаг. Для нескольких — матрица:

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

У каждого чата свой шаг в UI, свои выходы и свой успех или провал — чего CSV-форма из v1.5
дать не могла. Обоснование — в [гайде по миграции](/notiflow/ru/action/migration/).
