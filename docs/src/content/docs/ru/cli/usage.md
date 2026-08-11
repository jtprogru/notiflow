---
title: Использование CLI
description: send, edit, render и whoami из терминала или шелл-скрипта.
---

## send

```bash
export NOTIFLOW_BOT_TOKEN=123456789:AAHdqTcv...
export NOTIFLOW_CHAT_ID=-1001234567890

notiflow send --message "деплой закончился"
notiflow send -m "деплой закончился" -c @my_channel
```

Тело сообщения берётся из четырёх мест:

```bash
notiflow send --message "текст прямо здесь"
notiflow send --message-file release-notes.md
notiflow send --stdin < build.log
./deploy.sh | notiflow send --stdin
notiflow send -m -            # `-` — синоним --stdin
```

`--stdin` и `--message-file` ведут себя как `--message`: дословный текст, без подстановки
плейсхолдеров и без экранирования. Хвостовой перевод строки из пайпа срезается — его почти
никогда не имеют в виду.

Чтобы работали [плейсхолдеры](/notiflow/ru/templates/), передавай шаблон:

```bash
notiflow send --template '{{.StatusEmoji}} выкатил `{{.ShortSha}}` в прод'
```

Вне workflow репозиторий, ветка, коммит и автор берутся из локального git, поэтому один и
тот же шаблон работает в обоих местах.

## Статусы и фильтрация

`--status` по умолчанию `success`. Подключи его к тому, что реально произошло:

```bash
if ./deploy.sh; then status=success; else status=failure; fi
notiflow send --status "$status" --template '{{.StatusEmoji}} деплой: {{.Status}}'
```

`--notify-on` фильтрует ровно как в Action:

```bash
notiflow send --status "$status" --notify-on failure,cancelled
```

Отфильтрованный запуск выходит с 0, печатает `skipped: status not in notify_on` и не
открывает соединение.

## edit

Переписать отправленное раньше сообщение — обычно чтобы превратить «деплою…» в результат:

```bash
id=$(notiflow send -m "деплою…" --json | jq -r .message_id)
./deploy.sh
notiflow edit --message-id "$id" -m "деплой закончился"
```

`--silent` и `--thread-id` при редактировании отбрасываются — Telegram их там не принимает.

## render

Рендерит шаблон и печатает. Ни токена, ни сети — быстрый способ понять, почему
экранирование выглядит не так:

```bash
notiflow render --template 'ветка `{{.Branch}}`' --explain
# template: message_template
# parse_mode: MarkdownV2
# truncated: false
ветка `feature/fix\-thing`
```

`--set` перекрывает один плейсхолдер — так проверяют шаблон на значениях, которых сейчас
нет:

```bash
notiflow render -T '{{.StatusEmoji}} {{.Workflow}} on {{.Repo}}' \
  --status failure --set Workflow=Nightly --set Repo=owner/name
```

## whoami

Проверяет токен через `getMe`:

```bash
notiflow whoami
# Notiflow (@notiflow_bot, id=123456789)

notiflow whoami --json | jq .username
```

Выходит с 1 и описанием от самого Telegram, если токен отклонён, — первое, что стоит
запустить, когда workflow сообщает про 401.

## Форматы вывода

```bash
notiflow send -m hi                  # одна человекочитаемая строка
notiflow send -m hi --json           # один JSON-объект
notiflow send -m hi --output github  # записи $GITHUB_OUTPUT и step summary
```

JSON стабилен и его безопасно парсить:

```json
{"ok":true,"skipped":false,"message_id":4242,"http_status":200,"error":null,
 "chat_id":"-1001234567890","attempts":1,"dry_run":false}
```

`--quiet` убирает человекочитаемую строку, но не трогает `--json` и `--output github` —
скрипт может молчать, не теряя машиночитаемый результат.

## Сухой прогон

`--dry-run` делает всё, кроме запроса, и печатает точное JSON-тело, которое ушло бы:

```bash
notiflow send -m "проверь меня" --dry-run
```

Самый быстрый способ убедиться, что `--parse-mode`, `--silent` и `--thread-id` делают с
запросом именно то, что ты думаешь.

## Таймауты и ретраи

```bash
notiflow send -m hi --timeout 30 --connect-timeout 10 --retries 5 --max-retry-after 120
```

По умолчанию: 15 с на весь запрос, 5 с на соединение, 3 ретрая после первой попытки и
потолок 60 с на запрошенный сервером `retry_after`. Что ретраится, а что нет —
в [Надёжности](/notiflow/ru/reliability/).
