# Exploration: Telegram Notify Action (`notiflow`)

## Намерение (Intent)

Создать переиспользуемый GitHub Action `jtprogru/notiflow`, который отправляет уведомление в Telegram о завершении воркфлоу. Action должен:

- принимать `bot_token` и `chat_id` (плюс опциональные параметры) как inputs;
- определять итоговый статус job/workflow (`success` / `failure` / `cancelled`) и формировать соответствующее сообщение;
- предоставлять явные «ручки» кастомизации формата (шаблон, parse_mode, эмодзи статусов, тихий режим, thread/topic, превью ссылок и т.п.).

Это «greenfield»: репозиторий пустой (кроме `.git` и `.serena/`). Никаких legacy-ограничений нет, можно выбрать архитектуру с нуля.

## Investigation

Исследована структура каталога `/Users/jtprogru/Work/github/jtprogru/notiflow/`:

- `.git/` — remote `git@github.com:jtprogru/notiflow.git`, ветка `main`, коммитов нет.
- `.serena/project.yml` — Serena сконфигурирована, `languages: []` (явный язык не зафиксирован).
- Никаких `action.yml`, `Makefile`, `Taskfile.yml`, `package.json`, `go.mod`, `Dockerfile`, `.github/` ещё не существует.

Изученная предметная область — Telegram Bot API `sendMessage` (HTTPS POST, JSON body), и GitHub Actions runtime:

- Action может быть **composite**, **Docker**, или **JavaScript/TypeScript** (см. Options Considered).
- Контекст для шаблона полностью доступен через `${{ github.* }}` / `${{ job.* }}` / переменные окружения (`GITHUB_REPOSITORY`, `GITHUB_RUN_ID`, `GITHUB_SHA`, `GITHUB_WORKFLOW`, `GITHUB_ACTOR`, `GITHUB_REF_NAME`, `GITHUB_EVENT_NAME` и т.д.).
- Статус job на момент `post`-шага или внутри `if: always()` шага доступен через `${{ job.status }}` (`success` / `failure` / `cancelled`). Статус всего воркфлоу штатно из работающего job недоступен — общепринятая практика: Action работает на уровне *job* и принимает явный input `status` (или читает `job.status`).
- Telegram `sendMessage` ограничивает длину текста 4096 символами; `parse_mode` поддерживает `MarkdownV2` (требует экранирования `_*[]()~``>#+-=|{}.!`), `HTML`, `Markdown` (legacy). Для форум-чатов нужен `message_thread_id`.

## Build Tooling

- **Orchestrator:** `Makefile` (будет создан на фазе Implementation).
- **Test:** `make test` — запускает `bats` (Bash Automated Testing System) для unit-тестов формирующего скрипта + `act` или smoke-workflow в `.github/workflows/` для интеграционного теста.
- **Build:** `make build` — no-op для composite action (artifact = сам `action.yml` + скрипты).
- **Lint:** `make lint` — `actionlint` (для `action.yml` и тестовых workflow) + `shellcheck` (для bash-скриптов) + `shfmt -d` (форматирование).
- **Generate:** `make readme` — генерация секции `Inputs/Outputs` README из `action.yml` (например, через `npx action-docs` или собственный awk-скрипт). Опционально.
- **Source:** `Makefile` (создаётся в Implementation).

Все инструменты — стандартные: `bats`, `shellcheck`, `shfmt`, `actionlint` ставятся через Homebrew/`go install`/`apt`. CI устанавливает их в job-шагах.

## Options Considered

### Option A: Composite action (Bash + `curl`)

- **Описание:** `action.yml` с `runs.using: composite`, шаги — несколько `run:` блоков на bash, которые формируют JSON через `jq` и POSTят его в `https://api.telegram.org/bot<TOKEN>/sendMessage` через `curl`.
- **Pros:**
  - Нулевая холодная стоимость: нет pull-а образа, нет `npm install`.
  - Прозрачность: пользователь action-а видит весь код в репозитории.
  - Минимум зависимостей: `bash`, `curl`, `jq` — есть на всех ubuntu/macos раннерах.
  - Простой релизный цикл: тегнул — готово; нет `dist/` бандла, нет образа в GHCR.
- **Cons:**
  - Шаблонизация в bash громоздкая: подстановка `${{ github.* }}` идёт на уровне action.yml, а пользовательский шаблон с плейсхолдерами (`{{repo}}`, `{{status}}`) придётся реализовывать через `envsubst` или sed.
  - Экранирование MarkdownV2 в bash требует аккуратной функции (`sed -E 's/([_*\[\]()~`>#+\-=|{}.!])/\\&/g'`).
  - Юнит-тесты пишутся на `bats` — менее выразительно, чем Go/TS test-фреймворки.
- **Сложность:** Низкая.

### Option B: Docker action (Go binary)

- **Описание:** `action.yml` с `runs.using: docker`, образ собирается из `Dockerfile` (multi-stage: `golang:1.x` → `gcr.io/distroless/static`). Go-программа парсит inputs, рендерит `text/template`, шлёт HTTP-запрос через `net/http`.
- **Pros:**
  - Полноценная типизация, нормальное тестирование (`go test`, table-driven).
  - Мощный шаблонизатор `text/template` с функциями, условиями, циклами.
  - Удобно расширять (другие транспорты — Slack, Mattermost — общим ядром).
  - Пользователь skills `golang-pro` / `golang-cli` — стек знакомый.
- **Cons:**
  - Холодный старт docker-action заметный (5–30 с на pull образа на первом запуске в job).
  - Docker actions работают только на **Linux** раннерах — отрезаем `macos-*` и `windows-*` (для уведомления это редко критично, но ограничение есть).
  - Требуется публикация образа (GHCR) или сборка из исходников при каждом запуске.
  - Релизный цикл сложнее: tag + образ + проверка SHA-пиннинга.
- **Сложность:** Средняя.

### Option C: JavaScript/TypeScript action (`@actions/core`)

- **Описание:** TS-проект, бандл через `@vercel/ncc` в `dist/index.js`, `runs.using: node20`. Использует `@actions/core` для inputs/outputs и `node:fetch` для HTTP.
- **Pros:**
  - Самый быстрый старт (node20 встроен в раннер).
  - Богатый SDK (`@actions/core`, `@actions/github` — готовый `context` без парсинга env).
  - Кроссплатформенный (Linux / macOS / Windows).
- **Cons:**
  - Требуется коммитить собранный `dist/` в репозиторий (стандартная практика, но это шум в диффах).
  - Нужен `node_modules` для разработки, npm-релизный цикл.
  - Пользователь не упоминал интереса к Node/TS, стек менее «свой» в сравнении с Go.
- **Сложность:** Средняя.

## Constraints & Risks

- **Безопасность токена:** `bot_token` должен передаваться как input из `secrets.*`, не логироваться (использовать `::add-mask::` на самом первом шаге).
- **Идемпотентность сообщений:** retry-логика на сетевых ошибках (429, 5xx) с экспоненциальной задержкой; Telegram возвращает `retry_after` в JSON ошибки 429 — нужно уважать.
- **Длина сообщения:** > 4096 символов — обрезка с маркером `…` (или ошибка по флагу).
- **Экранирование `MarkdownV2`:** одна из самых частых причин 400 от Telegram. Нужен корректный escape пользовательского текста, но **не** placeholder-разметки.
- **Failure-mode action-а:** если action упал внутри `if: always()`-шага, он не должен «затмить» причину провала job. Делаем `continue-on-error: true` рекомендацией в README, а сами завершаемся `exit 0` при ошибке отправки + warning в лог (опционально — флаг `fail_on_error: false` по умолчанию).
- **Совместимость раннеров:** composite работает везде; пользователь может захотеть запустить на `windows-latest` — тогда нужен либо PowerShell-фоллбэк, либо явное ограничение в README.
- **Telegram Bot API rate limits:** 30 сообщений/сек на бота — для CI не проблема, но retry нужен.
- **Dependencies:** `curl`, `jq`, `bash` (для composite). `jq` есть на ubuntu-latest, macos-latest, windows-latest GitHub runners — проверено.

## Recommended Direction

**Option A — composite action на Bash + `curl` + `jq`.**

Обоснование:
- Action узкоспециализирован (POST одного JSON), сложного домена нет.
- Composite даёт минимальный overhead для пользователей и максимально простой релизный цикл (push тег — готово).
- Все три раннер-ОС поддерживаются «из коробки».
- Прозрачность кода для аудита (важно — токены).
- При появлении необходимости в Slack/Mattermost можно либо сделать отдельные actions, либо поднять Option B как `v2` (риск отложен).

## Scope Boundaries

### Must-have (v1)

1. Inputs: `bot_token` (required), `chat_id` (required), `status` (default: `${{ job.status }}`).
2. Авто-сообщения для статусов `success` / `failure` / `cancelled` со стандартным шаблоном, включающим: repo, workflow name, branch/ref, commit SHA (короткий), actor, run URL.
3. Кастомные сообщения: `message` (полная замена шаблона) и/или `message_template` (Go-template-подобный синтаксис `{{.Repo}}`, `{{.Status}}`, …).
4. `parse_mode`: `MarkdownV2` (default), `HTML`, `Markdown`, `none`. Для MarkdownV2 — автоэскейп **значений** плейсхолдеров, не шаблона.
5. `disable_web_page_preview` (bool, default `true`), `disable_notification` (bool, default `false`).
6. `message_thread_id` (для форум-чатов, опционально).
7. Per-status шаблоны: `template_success`, `template_failure`, `template_cancelled` — перекрывают общий `message_template`.
8. Условная отправка: `notify_on` — список из `success,failure,cancelled` (default — все).
9. Retry на 429 (уважение `retry_after`) и 5xx (экспонента, до 3 попыток).
10. Маскирование `bot_token` в логах.
11. Outputs: `message_id`, `ok` (boolean), `http_status`.

### Deferred (v2)

- Отправка медиа (`sendPhoto`, `sendDocument`) — отдельные actions / суб-команды.
- Inline-клавиатура с кнопками (например, ссылка на run, на PR).
- Множественные `chat_id` (рассылка).
- Slack/Mattermost/Discord транспорты — отдельные actions.
- Поддержка `proxy` / кастомного API endpoint (для self-hosted Telegram Bot API).
- Локализация стандартных сообщений (RU / EN).

### Needs spike

- **Windows-раннер совместимость:** нужно проверить, что `bash` (Git Bash) + `curl` + `jq` работают на `windows-latest` так, как в Linux. Возможно, потребуется отдельный shell-блок для `pwsh`. Решается smoke-тестом в matrix-CI.
- **Поведение при failure внутри post-шага в `if: always()`:** убедиться, что наш `exit 0` не маскирует исходный exit code job-а. Проверяется на тестовом workflow.

## Assumptions & Open Questions

### Assumptions

- [ASSUMPTION: пользователь работает в основном на ubuntu/macos GitHub-раннерах; Windows-совместимость желательна, но не блокер v1.]
- [ASSUMPTION: пользователю важнее простота релиза (composite, без бандлов и образов), чем встроенная типизация Go-кода.]
- [ASSUMPTION: «уведомление о завершении Workflow» в требовании = уведомление о завершении конкретного job, т.к. action из работающего job-а не знает статус *всего* workflow; стандартная схема — добавить отдельный job `notify` с `needs:` и `if: always()`, который анализирует `needs.*.result`.]
- [ASSUMPTION: парсинг шаблона делаем на стороне action-а (наш собственный мини-шаблонизатор `{{.Field}}`), не полагаемся на `${{ }}` GitHub Actions для пользовательского шаблона — иначе пользователь ограничен синтаксисом GHA expressions.]
- [ASSUMPTION: дефолтный `parse_mode` — `MarkdownV2` (современный стандарт Telegram), но в README покажем, как переключить на `HTML`.]
- [ASSUMPTION: action принимает либо `message` (готовый текст, отправляется как есть), либо `message_template` / `template_<status>` (рендерится с подстановкой), но не обе одновременно — приоритет у `message`.]

### Open Questions

- Нужен ли `proxy_url` уже в v1 (для пользователей с заблокированным Telegram API)? Сейчас вынесено в Deferred.
- Какой выбрать механизм условной отправки: input `notify_on: "failure,cancelled"` (наш собственный фильтр) или предлагать пользователю использовать `if:` на уровне step (стандартный GHA подход)? Сейчас планируем **оба**: дефолт = «всё», но пользователь может ограничить через `notify_on` без вынесения `if:` в workflow.
- Стандартный шаблон по умолчанию — на RU или EN? Сейчас планируем EN с эмодзи, чтобы action был универсальным.
