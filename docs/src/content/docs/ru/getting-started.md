---
title: Быстрый старт
description: Первое сообщение notiflow — из workflow и из терминала.
sidebar:
  order: 1
---

## Что нужно заранее

Токен бота и chat id. Если нет ни того, ни другого:

1. Напиши [@BotFather](https://t.me/BotFather), отправь `/newbot` и сохрани выданный токен. Выглядит как `123456789:AAHdqTcv...`.
2. Добавь бота в нужный чат или канал. Для канала он должен быть администратором.
3. Узнай chat id. Проще всего — отправить одно сообщение в чат и прочитать его обратно:

```bash
curl -s "https://api.telegram.org/bot<TOKEN>/getUpdates" | grep -o '"chat":{"id":[-0-9]*'
```

Id групп и супергрупп отрицательные и обычно начинаются с `-100`. Публичный канал можно
адресовать как `@channelusername`.

Проверь токен до того, как что-то настраивать:

```bash
NOTIFLOW_BOT_TOKEN=<TOKEN> notiflow whoami
# Notiflow (@notiflow_bot, id=123456789)
```

## Action за 60 секунд

Положи токен и chat id в secrets репозитория и добавь один шаг в конец нужного job:

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

`if: always()` важен: без него шаг пропускается, когда job упал, — то есть ровно тогда,
когда сообщение и нужно.

`status` приходится передавать явно. Дефолты входов composite action не видят контекст
`job`, поэтому notiflow не может подставить его сам.

По умолчанию сообщение приходит на `success`, `failure` и `cancelled`, со встроенным
шаблоном. Чтобы получать только про падения:

```yaml
        with:
          bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
          chat_id: ${{ secrets.TELEGRAM_CHAT_ID }}
          status: ${{ job.status }}
          notify_on: failure
```

## CLI за 60 секунд

```bash
brew install jtprogru/tap/notiflow      # или: cargo install notiflow
export NOTIFLOW_BOT_TOKEN=123456789:AAHdqTcv...
export NOTIFLOW_CHAT_ID=-1001234567890

notiflow send --message "деплой закончился"
```

Всё, что пришло на stdin, становится телом сообщения — notiflow нормально живёт в конце
пайплайна:

```bash
./deploy.sh 2>&1 | tail -20 | notiflow send --stdin
```

Шаблоны работают и вне Actions: репозиторий, ветка и коммит берутся из локального git,
когда переменных `GITHUB_*` нет:

```bash
notiflow send --template '{{.StatusEmoji}} выкатил `{{.ShortSha}}` из {{.Branch}}'
```

Посмотреть, что даёт шаблон, ничего не отправляя:

```bash
notiflow render --template '{{.Repo}} @ {{.ShortSha}}' --explain
```

## Дальше

- [Установка](/notiflow/ru/install/) — все способы получить бинарь
- [Справочник Action](/notiflow/ru/action/reference/) — все входы и выходы
- [Шаблоны](/notiflow/ru/templates/) — полный список плейсхолдеров и приоритеты
- [Надёжность](/notiflow/ru/reliability/) — ретраи, ошибки, exit-коды
