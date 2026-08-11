---
title: Конфигурация
description: Откуда берутся настройки, в каком порядке и как работают профили.
---

## Приоритет

Сверху вниз, побеждает верхнее:

1. флаги командной строки
2. переменные окружения
3. выбранный профиль в конфиге
4. значения верхнего уровня в конфиге
5. встроенные дефолты

## Переменные окружения

| Переменная | Эквивалент флага |
|---|---|
| `NOTIFLOW_BOT_TOKEN` | `--bot-token` |
| `NOTIFLOW_CHAT_ID` | `--chat-id` |
| `NOTIFLOW_API_BASE` | `--api-base` |
| `NOTIFLOW_CONFIG` | `--config` |
| `NOTIFLOW_DEBUG` | `-vv` |

Окружение — правильное место для токена. Значение, переданное через `--bot-token`, видно в
`ps` любому пользователю машины, поэтому notiflow его принимает и предупреждает.

## Конфиг-файл

Расположение по умолчанию, по порядку: `$NOTIFLOW_CONFIG`, затем
`$XDG_CONFIG_HOME/notiflow/config.toml`, затем `~/.config/notiflow/config.toml`.

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

Ключи верхнего уровня — значения по умолчанию. `[profile.<name>]` перекрывает их, когда
профиль запрошен:

```bash
notiflow send -m "выкатили" --profile work
```

Отсутствующий файл — не ошибка, если ты не назвал его через `--config` и не попросил
`--profile`; любое из двух превращает «нет файла» в `CONFIG_ERROR` (exit 17), а не в тихий
откат к дефолтам. Неизвестный ключ — тоже ошибка: опечатка вроде `chat_ids` не должна
молча ничего не делать.

### Права на файл

Если в файле лежит токен, оставь его себе:

```bash
chmod 600 ~/.config/notiflow/config.toml
```

notiflow предупреждает, когда файл читаем для группы или остальных.

## Все ключи

Все они работают и на верхнем уровне, и внутри профиля.

| Ключ | Тип | По умолчанию |
|---|---|---|
| `bot_token` | строка | — |
| `chat_id` | строка | — |
| `parse_mode` | строка | `MarkdownV2` |
| `notify_on` | строка | `success,failure,cancelled` |
| `api_base` | строка | `https://api.telegram.org` |
| `message_template` | строка | встроенный шаблон |
| `template_success` | строка | — |
| `template_failure` | строка | — |
| `template_cancelled` | строка | — |
| `template_skipped` | строка | — |
| `message_thread_id` | строка | — |
| `disable_notification` | bool | `false` |
| `disable_web_page_preview` | bool | `true` |
| `connect_timeout` | целое, секунды | `5` |
| `timeout` | целое, секунды | `15` |
| `retries` | целое | `3` |
| `max_retry_after` | целое, секунды | `60` |
| `fail_on_error` | bool | `false` |

`message`, `status` и `edit_message_id` отсутствуют намеренно: они описывают один запуск, а
не сохранённое предпочтение.

## Self-hosted Bot API

```bash
notiflow send --api-base https://bot-api.internal.example.com -m "привет"
```

Или постоянно, в профиле:

```toml
[profile.internal]
api_base = "https://bot-api.internal.example.com"
chat_id  = "-1005555555555"
```

```bash
notiflow send --profile internal -m "привет"
```

Внутри job GitHub Actions вместо этого действует allowlist — `api.telegram.org` и
loopback, — потому что там переменную мог подложить предыдущий шаг того же job, а не ты.
