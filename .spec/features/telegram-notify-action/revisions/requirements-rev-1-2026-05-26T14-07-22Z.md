# Telegram Notify Action (`notiflow`) — Requirements

**Status:** Draft
**Author:** spec-driven-dev agent
**Date:** 2026-05-26

## Overview

`notiflow` — это composite GitHub Action, который отправляет в Telegram уведомление о завершении job воркфлоу. Action принимает токен бота и идентификатор чата, определяет итоговый статус (`success` / `failure` / `cancelled`), формирует сообщение по стандартному либо пользовательскому шаблону и отправляет его через Telegram Bot API `sendMessage`. Кастомизация включает выбор `parse_mode`, per-status шаблоны, фильтр статусов отправки, тихий режим, thread/topic для форум-чатов, отключение превью ссылок и набор плейсхолдеров с контекстом GitHub Actions.

## Glossary

| Термин | Определение | Code Artifact |
|--------|-------------|---------------|
| `notiflow action` | Сам composite action — корневой `action.yml` и его шаги | `action.yml` |
| `Render Script` | Bash-скрипт, формирующий итоговый текст сообщения из шаблона и контекста | `scripts/render.sh` |
| `Send Script` | Bash-скрипт, выполняющий HTTP POST к Telegram Bot API с retry-логикой | `scripts/send.sh` |
| `Status` | Итоговый результат job-а: `success`, `failure`, `cancelled` | `scripts/render.sh` |
| `Message Template` | Строка с плейсхолдерами `{{.Field}}`, рендерится Render Script | `scripts/render.sh` |
| `Per-status Template` | Шаблон, применяемый только при конкретном статусе (`template_success` и т.п.) | `action.yml` |
| `Placeholder` | Подстановка вида `{{.Field}}`, заменяется на значение из контекста GitHub | `scripts/render.sh` |
| `Notify Filter` | Список статусов из `notify_on`, при которых разрешена отправка | `scripts/render.sh` |
| `Bot Token` | Секрет Telegram-бота, используемый для аутентификации `sendMessage` | input `bot_token` |
| `Chat ID` | Идентификатор Telegram-чата получателя (включая отрицательные ID для групп) | input `chat_id` |
| `Thread ID` | `message_thread_id` для форум-чатов Telegram | input `message_thread_id` |

## User Stories

- Как **разработчик**, я хочу одной строкой в workflow подключить уведомления в Telegram, чтобы узнавать о падениях CI без захода в GitHub UI.
- Как **тимлид**, я хочу настроить кастомный шаблон сообщения с упоминанием автора коммита и ссылкой на run, чтобы виновник падения сразу видел свой ник.
- Как **DevOps-инженер**, я хочу отправлять уведомления только при `failure` и `cancelled`, чтобы не зашумлять чат успешными прогонами.
- Как **владелец репозитория**, я хочу, чтобы action никогда не печатал `bot_token` в логе job-а, даже при ошибке.
- Как **админ форум-чата**, я хочу указывать `message_thread_id`, чтобы уведомления попадали в нужную тему.
- Как **оператор CI**, я хочу, чтобы при временной ошибке Telegram API action повторил запрос, а не падал на первой 429.

## Requirements

### Группа 1 — Inputs и их валидация

**REQ-1.1** WHEN action запускается, the system SHALL прочитать входы `bot_token` и `chat_id` из конфигурации шага и завершиться с ошибкой и кодом `MISSING_REQUIRED_INPUT`, если хотя бы один из них пуст.

**REQ-1.2** WHEN значение `chat_id` не является целым числом и не начинается с `@` (для публичных каналов), the system SHALL завершиться с ошибкой и кодом `INVALID_CHAT_ID`.

**REQ-1.3** WHEN вход `status` не задан, the system SHALL использовать значение `${{ job.status }}` runtime-контекста GitHub Actions как значение по умолчанию.

**REQ-1.4** WHEN значение `status` не входит в множество `{success, failure, cancelled, skipped}`, the system SHALL завершиться с ошибкой и кодом `INVALID_STATUS`.

**REQ-1.5** WHEN вход `parse_mode` не задан, the system SHALL использовать значение `MarkdownV2` по умолчанию.

**REQ-1.6** WHEN значение `parse_mode` не входит в множество `{MarkdownV2, HTML, Markdown, none}`, the system SHALL завершиться с ошибкой и кодом `INVALID_PARSE_MODE`.

### Группа 2 — Маскирование секретов

**REQ-2.1** WHEN action стартует, the system SHALL зарегистрировать значение `bot_token` через `::add-mask::` до выполнения любого шага, который может его использовать.

**REQ-2.2** WHEN action логирует HTTP-запросы или ошибки, the system SHALL никогда не печатать значение `bot_token` в открытом виде в stdout/stderr.

### Группа 3 — Определение и фильтрация статуса

**REQ-3.1** WHEN вход `notify_on` не задан, the system SHALL использовать значение `success,failure,cancelled` по умолчанию.

**REQ-3.2** WHEN текущее значение `status` не входит в список `notify_on`, the system SHALL завершить выполнение успешно (exit 0) без отправки сообщения и записать в лог причину пропуска.

**REQ-3.3** WHEN значение `notify_on` содержит элемент, не входящий в `{success, failure, cancelled, skipped}`, the system SHALL завершиться с ошибкой и кодом `INVALID_NOTIFY_ON`.

### Группа 4 — Стандартное сообщение

**REQ-4.1** WHEN ни `message`, ни `message_template`, ни `template_<status>` не заданы, the system SHALL сформировать сообщение по встроенному шаблону, включающему: эмодзи статуса, название workflow, репозиторий (`owner/repo`), branch/ref, короткий SHA коммита (7 символов), автора (actor) и URL run-а.

**REQ-4.2** WHEN статус — `success`, the system SHALL использовать в стандартном сообщении эмодзи-префикс `✅`.

**REQ-4.3** WHEN статус — `failure`, the system SHALL использовать в стандартном сообщении эмодзи-префикс `❌`.

**REQ-4.4** WHEN статус — `cancelled`, the system SHALL использовать в стандартном сообщении эмодзи-префикс `⚠️`.

**REQ-4.5** WHEN статус — `skipped`, the system SHALL использовать в стандартном сообщении эмодзи-префикс `⏭`.

### Группа 5 — Кастомные шаблоны

**REQ-5.1** WHEN задан вход `message`, the system SHALL отправить его значение как тело сообщения без шаблонной подстановки и проигнорировать `message_template` / `template_<status>`.

**REQ-5.2** WHEN задан вход `template_<status>` для текущего статуса (например, `template_failure` при `status=failure`), the system SHALL использовать его как шаблон с приоритетом над `message_template`.

**REQ-5.3** WHEN задан только `message_template`, the system SHALL использовать его как шаблон для всех статусов, не перекрытых per-status входами.

**REQ-5.4** WHEN шаблон содержит плейсхолдер `{{.Field}}`, the system SHALL заменить его на значение поля из набора: `Repo`, `Workflow`, `Job`, `Status`, `StatusEmoji`, `Actor`, `Ref`, `RefName`, `Sha`, `ShortSha`, `RunId`, `RunNumber`, `RunUrl`, `EventName`, `ServerUrl`, `Branch`.

**REQ-5.5** WHEN шаблон содержит плейсхолдер с неизвестным именем поля, the system SHALL заменить его на пустую строку и записать предупреждение `UNKNOWN_PLACEHOLDER:<name>` в лог.

**REQ-5.6** WHEN значение плейсхолдера подставляется и `parse_mode` равен `MarkdownV2` или `HTML`, the system SHALL экранировать значение по правилам выбранного режима до подстановки в шаблон.

**REQ-5.7** WHEN итоговая длина сообщения превышает 4096 символов, the system SHALL обрезать сообщение до 4093 символов и добавить суффикс `...`.

### Группа 6 — Опции отправки

**REQ-6.1** WHEN вход `disable_web_page_preview` не задан, the system SHALL использовать значение `true` по умолчанию.

**REQ-6.2** WHEN вход `disable_notification` не задан, the system SHALL использовать значение `false` по умолчанию.

**REQ-6.3** WHEN вход `message_thread_id` задан и его значение — целое число, the system SHALL включить поле `message_thread_id` в JSON-тело запроса к Telegram API.

**REQ-6.4** WHEN вход `message_thread_id` задан, но не является целым числом, the system SHALL завершиться с ошибкой и кодом `INVALID_THREAD_ID`.

### Группа 7 — HTTP-вызов и retry

**REQ-7.1** WHEN action готов к отправке, the system SHALL выполнить HTTPS POST на `https://api.telegram.org/bot<bot_token>/sendMessage` с `Content-Type: application/json` и JSON-телом, содержащим минимум `chat_id`, `text` и (если `parse_mode != none`) `parse_mode`.

**REQ-7.2** WHEN ответ Telegram имеет HTTP-статус 429, the system SHALL подождать количество секунд, указанное в поле `parameters.retry_after` тела ответа (или 1 секунду при отсутствии поля), и повторить запрос — не более 3 повторов суммарно.

**REQ-7.3** WHEN ответ Telegram имеет HTTP-статус из диапазона `5xx`, the system SHALL повторить запрос с экспоненциальной задержкой (`1s`, `2s`, `4s`) — не более 3 повторов суммарно.

**REQ-7.4** WHEN после исчерпания повторов запрос всё ещё неуспешен и вход `fail_on_error` равен `true`, the system SHALL завершиться с ненулевым кодом и кодом ошибки `SEND_FAILED`.

**REQ-7.5** WHEN после исчерпания повторов запрос всё ещё неуспешен и вход `fail_on_error` равен `false` (значение по умолчанию), the system SHALL записать ошибку в лог как `::warning::`, выставить output `ok=false` и завершиться с exit 0.

**REQ-7.6** WHEN Telegram возвращает HTTP 400 с телом, содержащим `ok:false`, the system SHALL не выполнять retry и обработать ошибку согласно REQ-7.4 / REQ-7.5.

### Группа 8 — Outputs

**REQ-8.1** WHEN сообщение успешно отправлено, the system SHALL установить output `ok=true`, output `message_id=<id из ответа>` и output `http_status=200`.

**REQ-8.2** WHEN отправка не удалась (включая пропуск по фильтру `notify_on`), the system SHALL установить output `ok=false`, `message_id=` (пустая строка) и `http_status=<последний код ответа или 0>`.

### Группа 9 — Verification (CI / build tooling)

**REQ-9.1** WHEN разработчик запускает `make lint`, the system SHALL прогнать `actionlint`, `shellcheck` (для всех скриптов в `scripts/`) и `shfmt -d` и завершиться с ненулевым кодом при любых нарушениях.

**REQ-9.2** WHEN разработчик запускает `make test`, the system SHALL прогнать `bats` тесты в `tests/` и завершиться с ненулевым кодом при падении любого теста.

**REQ-9.3** WHEN в репозитории создан pull request, the system SHALL автоматически запустить `make lint` и `make test` в CI workflow `.github/workflows/ci.yml` на матрице `ubuntu-latest` + `macos-latest`.

## Topological Order

```
REQ-1.* (input validation) → REQ-2.* (masking) → REQ-3.* (notify_on filter)
                                                       │
                                                       ▼
                                       REQ-4.* (defaults) ──┐
                                       REQ-5.* (templates) ─┼─► REQ-7.* (send + retry) → REQ-8.* (outputs)
                                       REQ-6.* (send opts) ─┘
```

Reason: валидация входов и маскирование секрета должны произойти до любых действий, способных привести к утечке токена в лог. Фильтр `notify_on` решает, выполнять ли остальные шаги вообще. Рендеринг (шаблоны / опции) должен завершиться до HTTP-вызова. Outputs выставляются по результату отправки.

REQ-9.* (verification) — независимы, выполняются на этапе разработки/CI.

## Conflict Priority

**REQ-5.1 vs REQ-5.2 / REQ-5.3:** `message` (готовый текст) конфликтует с любыми шаблонами.
Resolution: `message` имеет наивысший приоритет; при его наличии шаблоны игнорируются (это явно проговорено в REQ-5.1).

**REQ-5.2 vs REQ-5.3:** per-status шаблон конфликтует с общим `message_template`.
Resolution: `template_<status>` имеет приоритет над `message_template` (REQ-5.2).

**REQ-7.4 vs REQ-7.5:** «упасть при ошибке отправки» конфликтует с «не падать».
Resolution: поведение управляется входом `fail_on_error`; default — `false` (не падать, REQ-7.5), чтобы не маскировать причину провала исходного job-а.

## Open Design Questions

| Вопрос | Почему важно | Затрагиваемые требования |
|--------|--------------|--------------------------|
| Как именно реализовать рендеринг плейсхолдеров `{{.Field}}` в bash — через `sed`/`awk`/`envsubst` или собственную функцию? | Влияет на корректность экранирования и производительность; envsubst требует переменных окружения и не поддерживает синтаксис `{{.X}}` без обработки. | REQ-5.4, REQ-5.5, REQ-5.6 |
| Где хранить функцию escape для MarkdownV2 / HTML — inline в `render.sh` или вынести в `scripts/escape.sh`? | Влияет на тестируемость функции escape отдельно от рендеринга. | REQ-5.6 |
| Использовать ли `curl --retry` встроенный, или реализовывать retry самим с парсингом `retry_after`? | `curl --retry` не умеет читать `Retry-After` из JSON-тела ответа Telegram, только из заголовка; требование REQ-7.2 требует чтения именно из тела. | REQ-7.2, REQ-7.3 |
| Какой shell использовать в шагах composite-action — `bash` явно или дефолтный `sh`? | Влияет на портативность; на windows-latest дефолтный shell — PowerShell. | REQ-9.3 (matrix) |

## Verification Commands

| Action   | Command                                          | Source                       |
|----------|--------------------------------------------------|------------------------------|
| Test     | `make test`                                      | Makefile (будет создан)      |
| Build    | `make build` (no-op для composite action)        | Makefile (будет создан)      |
| Lint     | `make lint`                                      | Makefile (будет создан)      |
| Generate | `make readme` (опциональная регенерация README)  | Makefile (будет создан)      |
