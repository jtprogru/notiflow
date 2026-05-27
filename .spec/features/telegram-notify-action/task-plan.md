# Telegram Notify Action (`notiflow`) — Task Plan

**Status:** Draft
**Date:** 2026-05-26
**Work Type:** Pure feature (greenfield — no prior implementation)

---

**Test Style Source:** Tier 3
- No adjacent tests found (greenfield repository). Поиск: `find . -name "*_test.*" -o -name "*.bats"` → пусто.
- Стиль определяется из design §2.8: `bats-core` юнит-тесты, один `.bats`-файл на модуль, общие хелперы в `tests/helpers.bash` (loaded via `load helpers`), mock Telegram API через локальный Python http-listener в `setup_file`/`teardown_file`. PBT для bash недоступен — используем targeted unit tests с явным перебором представительных входов.
- Имена тестов: `test_<feature>_<scenario>` / `prop_<feature>_<property>`. Тэги — в комментариях вида `# tag: Feature/<name>` или `# tag: Property/<N>`.

**Commands:**

| Action   | Command       | Source                       |
|----------|---------------|------------------------------|
| Test     | `make test`   | Design §2.8 / Makefile (NEW) |
| Build    | `make build`  | Design §2.8 / Makefile (NEW) |
| Lint     | `make lint`   | Design §2.8 / Makefile (NEW) |
| Generate | `make readme` | Design §2.8 / Makefile (NEW) |

> `make build` — no-op для composite action. `make readme` — опциональная регенерация секции inputs/outputs README из `action.yml`.

---

## Coverage Matrix

| Requirement | Task(s) | Correctness Property |
|-------------|---------|----------------------|
| REQ-1.1 | T-4 | CP-1 (absence) |
| REQ-1.2 | T-4 | CP-2 (absence) |
| REQ-1.3 | T-4, T-7 | CP-16 (equivalence) |
| REQ-1.4 | T-4 | CP-2 (absence) |
| REQ-1.5 | T-4 | CP-15 (equivalence) |
| REQ-1.6 | T-4 | CP-2 (absence) |
| REQ-2.1 | T-2, T-7 | CP-11 (absence) |
| REQ-2.2 | T-2, T-6 | CP-11 (absence) |
| REQ-3.1 | T-4 | CP-3 (exclusion) |
| REQ-3.2 | T-4 | CP-3 (exclusion) |
| REQ-3.3 | T-4 | CP-2 (absence) |
| REQ-4.1 | T-5 | CP-4 (propagation), CP-14 (equivalence) |
| REQ-4.2 | T-5 | CP-14 (equivalence) |
| REQ-4.3 | T-5 | CP-14 (equivalence) |
| REQ-4.4 | T-5 | CP-14 (equivalence) |
| REQ-4.5 | T-5 | CP-14 (equivalence) |
| REQ-5.1 | T-5 | CP-4 (propagation) |
| REQ-5.2 | T-5 | CP-4 (propagation) |
| REQ-5.3 | T-5 | CP-4 (propagation) |
| REQ-5.4 | T-5 | CP-5 (equivalence) |
| REQ-5.5 | T-5 | CP-5 (equivalence) |
| REQ-5.6 | T-3, T-5 | CP-6 (absence) |
| REQ-5.7 | T-5 | CP-7 (absence) |
| REQ-6.1 | T-6 | CP-15 (equivalence) |
| REQ-6.2 | T-6 | CP-15 (equivalence) |
| REQ-6.3 | T-6 | CP-15 (equivalence) |
| REQ-6.4 | T-4 | CP-2 (absence) |
| REQ-7.1 | T-6 | CP-15 (equivalence) |
| REQ-7.2 | T-6 | CP-8 (absence) |
| REQ-7.3 | T-6 | CP-9 (absence) |
| REQ-7.4 | T-7 | CP-13 (equivalence) |
| REQ-7.5 | T-7 | CP-13 (equivalence) |
| REQ-7.6 | T-6 | CP-10 (exclusion) |
| REQ-8.1 | T-6 | CP-12 (equivalence) |
| REQ-8.2 | T-4, T-6 | CP-12 (equivalence) |
| REQ-9.1 | T-1, T-7 | — (tooling) |
| REQ-9.2 | T-1, T-7 | — (tooling) |
| REQ-9.3 | T-7 | — (tooling) |

Каждое требование покрыто как минимум одной задачей. Каждое CP-свойство (CP-1…CP-16) привязано к ≥1 задаче.

---

## Task Order (Pure Feature)

```
GREEN (test stubs)  →  CODE (implementation)  →  GREEN (full tests)  →  GATE
```

Внутри каждой имплементационной задачи (T-3..T-7) тесты пишутся **до** кода соответствующего модуля. Сборка идёт снизу вверх: общая инфраструктура → чистые функции → модули с зависимостями → склейка.

---

## T-1 — Scaffold project tooling

***_Requirements: 9.1, 9.2_***
***_Complexity: mechanical_***

GOAL: подготовить базовую файловую структуру, Makefile с реальными целями `test`/`lint`/`build`/`install-tools`, `.gitignore`, и установку инструментов разработки.

1. Создать файл `.gitignore` со строками: `*.log`, `tmp/`, `.bats-tmp/`, `*.bak`, `node_modules/`, `coverage/`.
2. Создать файл `Makefile` с целями:
   - `.DEFAULT_GOAL := help`
   - `help` — печать списка таргетов через `awk` из `## ` комментариев.
   - `lint` — `actionlint && shellcheck scripts/*.sh tests/*.bash && shfmt -d scripts/ tests/`.
   - `lint-fix` — `shfmt -w scripts/ tests/`.
   - `test` — `bats tests/`.
   - `build` — `@echo "composite action — nothing to build"` (no-op).
   - `install-tools` — установка `bats`, `shellcheck`, `shfmt`, `actionlint` через `brew` (macOS) или `apt` (Linux), с детектом OS через `uname`.
   - `readme` — пометить как `@echo "TODO: not implemented in v1"` (заглушка, чтобы команда существовала).
3. Создать пустые каталоги `scripts/`, `tests/`, `tests/fixtures/`, `.github/workflows/` командами `mkdir -p` (но коммитим через `.gitkeep` файлы в каталогах, где иначе git их не сохранит, — `tests/fixtures/.gitkeep`).
4. Создать файл `LICENSE` с MIT-текстом, copyright `2026 Mikhail Savin`.

IMPORTANT: цели `lint` и `test` ДОЛЖНЫ при отсутствии инструментов завершаться с понятным сообщением «install with `make install-tools`», а не с криптичным `command not found`.

---

## T-2 — Build common bash infrastructure (`lib.sh`) and bats helpers

***_Requirements: 2.1, 2.2_***
***_Complexity: standard_***
***_Preservation: CP-11_***

GOAL: реализовать общую библиотеку shell-функций, используемую всеми модулями, и общие bats-хелперы (включая mock Telegram API).

1. Создать файл `scripts/lib.sh` с функциями (без shebang — файл предназначен для `source`):
   - `nf::log <level> <msg>` — `level ∈ {info|warn|error|debug}`; пишет в stderr; маппит на GH workflow-команды (`::warning::`, `::error::`, `::notice::`, `::debug::`).
   - `nf::mask <value>` — печатает `::add-mask::<value>` в stdout.
   - `nf::set_output <key> <value>` — `printf '%s=%s\n' "$key" "$value" >> "${GITHUB_OUTPUT:-/dev/stdout}"`.
   - `nf::json_escape <value>` — `printf '%s' "$1" | jq -Rs .` (возвращает корректно квотированную JSON-строку с обёрткой `"..."`).
   - `nf::require_command <cmd>` — `command -v "$cmd" >/dev/null || { nf::log error "MISSING_DEPENDENCY:$cmd"; exit 22; }`.
   - `nf::require_bash_4` — проверяет `${BASH_VERSINFO[0]:-0} -ge 4`; если меньше, пробует `exec /opt/homebrew/bin/bash "$0" "$@"` или `exec /usr/local/bin/bash "$0" "$@"`; на провал — `exit 20`.
   - В начале файла: `[[ -n "${_NF_LIB_LOADED:-}" ]] && return 0; _NF_LIB_LOADED=1` (idempotent source).
2. Создать файл `tests/helpers.bash` с функциями:
   - `setup_clean_env` — `unset $(env | grep -E '^(NF_|INPUT_)' | cut -d= -f1)`; затем экспорт минимально необходимых `GITHUB_*` фейков (`GITHUB_REPOSITORY=jtprogru/notiflow`, `GITHUB_WORKFLOW=CI`, `GITHUB_RUN_ID=1`, `GITHUB_SHA=abcdef0123456789`, `GITHUB_ACTOR=tester`, `GITHUB_REF=refs/heads/main`, `GITHUB_REF_NAME=main`, `GITHUB_RUN_NUMBER=1`, `GITHUB_EVENT_NAME=push`, `GITHUB_SERVER_URL=https://github.com`, `GITHUB_JOB=test`).
   - `mock_telegram_start <fixture_path>` — запускает `python3 -m http.server 0 --bind 127.0.0.1` на свободном порту, сохраняет PID и URL в файлы `tests/.tmp/mock.pid`, `tests/.tmp/mock.url`. Для сценариев с конкретными ответами — генерирует CGI-handler. **NOTE:** используем `python3 -c "..."` со скриптом из `tests/fixtures/mock_server.py`.
   - `mock_telegram_stop` — kill PID, чистка `tests/.tmp/`.
   - `assert_no_token_in_log <log_file>` — `! grep -F "$NF_BOT_TOKEN" "$log_file"`.
   - `assert_output_contains <key> <expected>` — парсит `$GITHUB_OUTPUT` файл и сравнивает.
3. Создать файл `tests/fixtures/mock_server.py` — небольшой Python-скрипт (≤80 строк), который слушает на заданном порту и отдаёт ответы по очереди из `RESPONSES` (массив `[(status, body), ...]`), либо отвечает 200/OK по умолчанию. Логирует входящие запросы (метод, путь, тело) в `tests/.tmp/requests.log` для assert-проверок.
4. Создать файл `tests/lib.bats` с 3 тестами:
   - `test_lib_log_writes_to_stderr` — проверка, что `nf::log info hi` пишет в stderr, не в stdout.
   - `test_lib_mask_emits_add_mask` — проверка вывода `::add-mask::secret`.
   - `test_lib_set_output_appends` — `GITHUB_OUTPUT=$BATS_TEST_TMPDIR/out`; вызов; проверка содержимого файла.

CRITICAL: `nf::json_escape` ОБЯЗАТЕЛЬНО возвращает JSON-строку с обёрткой `"..."`, потому что `send.sh` будет подставлять её через `--argjson`. Без обёртки — будет сломанный JSON.

DO NOT добавлять в `lib.sh` любую логику, специфичную для Telegram (это для `send.sh`).

---

## T-3 — Implement escape module (MarkdownV2 / HTML / none)

***_Requirements: 5.6_***
***_Complexity: standard_***
***_Preservation: CP-6_***

GOAL: реализовать чистые функции экранирования значений плейсхолдеров для каждого `parse_mode`.

1. Создать файл `tests/escape.bats` (тесты ДО реализации):
   - `test_escape_md_v2_underscore` — вход `foo_bar` → `foo\_bar`.
   - `test_escape_md_v2_all_specials` — вход с всеми 18 спецсимволами `_*[]()~``>#+-=|{}.!` → каждый префиксирован `\`.
   - `test_escape_md_v2_passthrough` — `Hello world` → `Hello world` (без изменений).
   - `test_escape_md_v2_backslash` — вход `foo\bar` → `foo\\bar` (экранирование `\` тоже требуется по MarkdownV2-спеке — обновить, если требование уточнится).
   - `test_escape_html_basic` — `<a&b>` → `&lt;a&amp;b&gt;`.
   - `test_escape_html_passthrough` — `Hello` → `Hello`.
   - `test_escape_none_noop` — `<_*foo*_>` → `<_*foo*_>` (вход == выход).
   - `prop_escape_md_v2_all_specials` (CP-6) — генератор: каждая 1-символьная строка из множества спецсимволов; ассерт: вывод равен `\<char>`.
2. Создать файл `scripts/escape.sh` (source-only):
   - Header `[[ -n "${_NF_ESCAPE_LOADED:-}" ]] && return 0; _NF_ESCAPE_LOADED=1`.
   - `nf::escape_md_v2 <text>` — реализация через `printf '%s' "$1" | sed -E 's/([_*\[\]()~`>#+\-=|{}.!\\])/\\\\&/g'`.
   - `nf::escape_html <text>` — последовательный `sed`: сначала `s/&/\&amp;/g`, потом `s/</\&lt;/g` и `s/>/\&gt;/g`.
   - `nf::escape_none <text>` — `printf '%s' "$1"`.
3. Запустить `make test` (ожидание: ВСЕ тесты `escape.bats` зелёные).

IMPORTANT: порядок sed-команд в `nf::escape_html` критичен — `&` должен экранироваться первым, иначе будут двойные `&amp;amp;`.

NOTE: тесты пишутся первыми (GREEN → CODE → GREEN), но в pure-feature этот порядок означает: stubs/тесты падают (модуль отсутствует), потом пишем код, потом тесты зелёные.

---

## T-4 — Implement validate + filter modules

***_Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 3.1, 3.2, 3.3, 6.4, 8.2_***
***_Complexity: standard_***
***_Preservation: CP-1, CP-2, CP-3, CP-15, CP-16_***

GOAL: реализовать валидацию всех inputs (с кодами ошибок 10–15) и логику фильтрации по `notify_on`.

1. Создать файл `tests/validate.bats` (тесты ДО кода):
   - `test_validate_missing_token` (REQ-1.1) — `NF_BOT_TOKEN=""`, `NF_CHAT_ID="123"` → exit 10, stderr содержит `MISSING_REQUIRED_INPUT`.
   - `test_validate_missing_chat_id` (REQ-1.1) — наоборот → exit 10.
   - `test_validate_chat_id_int_positive` (REQ-1.2) — `chat_id="12345"` → exit 0.
   - `test_validate_chat_id_int_negative` (REQ-1.2) — `chat_id="-100123456789"` → exit 0.
   - `test_validate_chat_id_username` (REQ-1.2) — `chat_id="@my_channel"` → exit 0.
   - `test_validate_chat_id_invalid` (REQ-1.2) — `chat_id="abc"` → exit 11.
   - `test_validate_status_default_from_job` (REQ-1.3) — `INPUT_STATUS=""`, `JOB_STATUS="success"` → `NF_STATUS=success` после validate (проверка через outputs или env).
   - `test_validate_status_allowed_values` (REQ-1.4) — перебор `{success,failure,cancelled,skipped}` → exit 0; `wat` → exit 12.
   - `test_validate_parse_mode_default` (REQ-1.5) — `INPUT_PARSE_MODE=""` → `NF_PARSE_MODE=MarkdownV2`.
   - `test_validate_parse_mode_invalid` (REQ-1.6) — `BBCode` → exit 13.
   - `test_validate_parse_mode_none` (REQ-1.6) — `none` → exit 0.
   - `test_validate_notify_on_default` (REQ-3.1) — пустое → `success,failure,cancelled`.
   - `test_validate_notify_on_invalid_item` (REQ-3.3) — `failure,wat` → exit 14.
   - `test_validate_thread_id_int` (REQ-6.4) — `123` → exit 0.
   - `test_validate_thread_id_invalid` (REQ-6.4) — `abc` → exit 15.
   - `prop_validate_invalid_no_http` (CP-2) — запустить scenario list: каждый невалидный input по очереди; ассерт exit != 0 и mock-сервер НЕ получил запросов.
2. Создать файл `tests/filter.bats`:
   - `test_filter_status_in_list` (REQ-3.2) — `nf::should_notify failure "success,failure"` → rc=0.
   - `test_filter_status_not_in_list` (REQ-3.2) — `nf::should_notify success "failure"` → rc=1.
   - `test_filter_handles_whitespace` — `nf::should_notify success " success , failure "` → rc=0.
   - `prop_status_filter_exclusion` (CP-3) — для каждого `status ∈ {success,failure,cancelled,skipped}` × `notify_on ⊆ powerset(...)`; ассерт rc=0 iff status ∈ notify_on.
3. Создать файл `scripts/validate.sh` (source-only, с `_NF_VALIDATE_LOADED` guard):
   - Source `lib.sh`.
   - `nf::validate` — линейная проверка: required → chat_id regex → status default + check → parse_mode default + check → notify_on default + per-item check → thread_id (если непустой) regex. На каждой ошибке: `nf::log error "<CODE>: <detail>"` + `exit <N>`.
   - Регексы:
     - `chat_id`: `^(-?[0-9]+|@[A-Za-z0-9_]{4,32})$`.
     - `thread_id`: `^[0-9]+$`.
   - Дефолтизация: `NF_STATUS="${NF_STATUS:-${JOB_STATUS:-}}"` (внутри action.yml `INPUT_STATUS` дефолтится в `${{ job.status }}`, но если пуст — берём из env-переменной `JOB_STATUS`, которую тесты могут подсунуть).
4. Создать файл `scripts/filter.sh` (source-only):
   - `nf::should_notify <status> <notify_on_csv>`:
     - `IFS=',' read -ra items <<< "$2"`.
     - Для каждого item: `item="${item// /}"` (trim spaces); если `item == status` → return 0.
     - Иначе → return 1.
5. Запустить `make test` — все `validate.bats` и `filter.bats` тесты зелёные.

CRITICAL: regex для `chat_id` ДОЛЖЕН использовать `=~` в bash, не внешний `grep` — это быстрее и не зависит от GNU/BSD различий.

DO NOT использовать `set -e` внутри функций — оно мешает обработке rc внутри `nf::should_notify`. Источник `set -euo pipefail` — только в `entrypoint.sh`.

---

## T-5 — Implement render module (templates + placeholders)

***_Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7_***
***_Complexity: complex_***
***_Preservation: CP-4, CP-5, CP-6, CP-7, CP-14_***

GOAL: реализовать выбор шаблона по приоритету, подстановку плейсхолдеров с автоэскейпом, и обрезку по 4096.

1. Создать файл `tests/render.bats` (тесты ДО кода):
   - `test_render_uses_message_when_set` (REQ-5.1) — `NF_MESSAGE="hello"`, `NF_TEMPLATE_FAILURE="X"`, `NF_STATUS=failure` → выход `hello`.
   - `test_render_template_status_priority` (REQ-5.2) — `NF_STATUS=failure`, заданы `NF_TEMPLATE_FAILURE="A"` и `NF_MESSAGE_TEMPLATE="B"` → выход `A`.
   - `test_render_message_template_fallback` (REQ-5.3) — `NF_STATUS=success`, задан только `NF_MESSAGE_TEMPLATE="ok"` → выход `ok`.
   - `test_render_default_template_per_status` (REQ-4.1, REQ-4.2..4.5) — `parse_mode=none`, по 1 ассерту на каждый статус: вывод начинается на `✅`/`❌`/`⚠️`/`⏭` и содержит `jtprogru/notiflow`, `CI`, `abcdef0`, `tester`.
   - `test_render_placeholder_repo` (REQ-5.4) — шаблон `Repo={{.Repo}}`, `parse_mode=none` → выход `Repo=jtprogru/notiflow`.
   - `test_render_placeholder_all_known` (REQ-5.4) — шаблон со всеми 16 плейсхолдерами → каждый подставлен корректным значением из fake-окружения helpers.
   - `test_render_unknown_placeholder` (REQ-5.5) — шаблон `X={{.Wat}}` → выход `X=`; stderr содержит `UNKNOWN_PLACEHOLDER:Wat`.
   - `test_render_escapes_md_v2_value` (REQ-5.6) — `parse_mode=MarkdownV2`, `GITHUB_REPOSITORY=foo_bar/baz`, шаблон `{{.Repo}}` → выход `foo\_bar/baz`.
   - `test_render_no_escape_when_parse_mode_none` (REQ-5.6 negative) — `parse_mode=none`, `GITHUB_REPOSITORY=foo_bar/baz`, шаблон `{{.Repo}}` → выход `foo_bar/baz`.
   - `test_render_truncate_exact_4096` (REQ-5.7) — `NF_MESSAGE=` строка из 5000 `a` → выход длиной ровно 4096, последние 3 символа `...`.
   - `test_render_truncate_at_boundary` (REQ-5.7) — длина входа ровно 4096 → выход без `...`; длина 4097 → выход 4096 с `...`.
   - `prop_template_priority` (CP-4) — все 8 комбинаций (message, template_status, message_template) bool-bool-bool; ассерт победителя.
   - `prop_placeholders_complete` (CP-5) — шаблон с 16 known + 3 unknown; ассерт: после рендера нет `{{.}}` паттернов в выходе.
   - `prop_emoji_mapping` (CP-14) — для каждого статуса в default-шаблоне проверить эмодзи-префикс.
   - `prop_truncate_length` (CP-7) — длины {0, 4095, 4096, 4097, 8192}; ассерт `len ≤ 4096`, `len == 4096 ∧ суффикс="..."` если вход > 4096.
2. Создать файл `scripts/render.sh` (source-only, guard):
   - Source `lib.sh`, `escape.sh`.
   - Функция `_nf::_emoji <status>` → echo `✅`/`❌`/`⚠️`/`⏭` или пусто.
   - Функция `_nf::_default_template <status>` → возвращает строку (HEREDOC):
     ```
     {{.StatusEmoji}} *{{.Workflow}}* on `{{.Repo}}`
     Status: {{.Status}}
     Branch: {{.Branch}} @ {{.ShortSha}}
     Actor: {{.Actor}}
     [Open run]({{.RunUrl}})
     ```
   - Функция `_nf::_pick_template <status>`:
     - `[[ -n "$NF_MESSAGE" ]] && { printf '%s' "$NF_MESSAGE"; return; }`.
     - По статусу — взять `NF_TEMPLATE_<UPPER>`; если непустое, вернуть.
     - Иначе если `NF_MESSAGE_TEMPLATE` непустое — вернуть его.
     - Иначе — `_nf::_default_template <status>`.
   - Функция `_nf::_build_placeholders` — заполняет `declare -gA NF_PLACEHOLDERS` 16-ю парами (см. design §2.5).
   - Функция `_nf::_escape_value <value>` — диспатч по `$NF_PARSE_MODE` к `nf::escape_md_v2`/`nf::escape_html`/`nf::escape_none`.
   - Функция `nf::render`:
     - `template="$(_nf::_pick_template "$NF_STATUS")"`.
     - Если `NF_MESSAGE` задан — вернуть его без подстановок (REQ-5.1 — финальный текст уже готов, эскейп не применяется).
     - Иначе `_nf::_build_placeholders`; для каждого ключа из `NF_PLACEHOLDERS`: эскейпнуть значение, заменить `{{.<key>}}` в шаблоне через `sed` (с использованием heredoc-pipe для значения, чтобы избежать инжекта).
     - После всех подстановок — найти оставшиеся `{{\.[A-Za-z]+}}` через `grep -oE`, на каждый — `nf::log warn "UNKNOWN_PLACEHOLDER:<name>"` и удалить (replace на пусто).
     - Truncate: `if (( ${#result} > 4096 )); then result="${result:0:4093}..."; fi`.
     - `printf '%s' "$result"`.
3. Запустить `make test` — все `render.bats` тесты зелёные.

CRITICAL: для подстановки значений плейсхолдеров через `sed` НЕОБХОДИМО квотировать значение от интерпретации как RHS sed-выражения. IMPORTANT: используй паттерн «значение в файл/stdin, sed читает через `r`-команду» ИЛИ замени экранирование `[&/\\]` в RHS:
```bash
escaped_rhs="$(printf '%s' "$value" | sed -e 's/[&/\\]/\\&/g')"
result="$(printf '%s' "$template" | sed "s/{{.$key}}/$escaped_rhs/g")"
```
Это критично — без этого `&` или `/` в значении автора/ветки сломают подстановку.

NOTE: `NF_MESSAGE` (REQ-5.1) обходит весь цикл шаблонизации — значит, и эскейп. Пользователь, задавший готовый текст с `parse_mode=MarkdownV2`, отвечает за корректное экранирование сам. Это явно зафиксировать в README.

DO NOT использовать `bash`-замену `${template//{{.X}}/value}` — она не справляется с динамическим именем ключа `X` без `eval`, а `eval` — дыра в безопасности.

---

## T-6 — Implement send module with retry logic

***_Requirements: 2.2, 6.1, 6.2, 6.3, 7.1, 7.2, 7.3, 7.6, 8.1, 8.2_***
***_Complexity: complex_***
***_Preservation: CP-8, CP-9, CP-10, CP-11, CP-12, CP-15_***

GOAL: реализовать HTTP POST к Telegram Bot API с собственной retry-логикой (уважение `retry_after` из JSON, экспонента для 5xx, no-retry для 4xx \ {429}), выставление outputs.

1. Создать файл `tests/send.bats` (тесты ДО кода):
   - `test_send_success_200_outputs` (REQ-7.1, REQ-8.1) — mock возвращает `{"ok":true,"result":{"message_id":42}}`; ассерт outputs `ok=true`, `message_id=42`, `http_status=200`.
   - `test_send_json_shape_minimal` (REQ-7.1, CP-15) — ассерт: `requests.log` содержит JSON с ключами `chat_id`, `text`, `parse_mode`, `disable_web_page_preview`, `disable_notification`.
   - `test_send_json_omit_parse_mode_when_none` (REQ-1.5 negative, CP-15) — `NF_PARSE_MODE=none` → JSON НЕ содержит ключ `parse_mode`.
   - `test_send_json_omit_thread_id_when_empty` (REQ-6.3, CP-15) — пустой `NF_MESSAGE_THREAD_ID` → JSON НЕ содержит ключ `message_thread_id`.
   - `test_send_json_includes_thread_id_when_set` (REQ-6.3, CP-15) — `NF_MESSAGE_THREAD_ID=7` → JSON содержит `"message_thread_id": 7` (число, не строка).
   - `test_send_disable_web_page_preview_default_true` (REQ-6.1) — пустой input → JSON `"disable_web_page_preview": true`.
   - `test_send_disable_notification_default_false` (REQ-6.2) — пустой input → JSON `"disable_notification": false`.
   - `test_send_429_retry_with_retry_after` (REQ-7.2, CP-8) — mock последовательность `[429 retry_after=1, 200]`; ассерт: 2 запроса, общее время ≥ 1с, ok=true.
   - `test_send_429_retry_limit_3` (REQ-7.2, CP-8) — mock всегда 429; ассерт: ровно 3 запроса, итог ok=false.
   - `test_send_5xx_exponential_backoff` (REQ-7.3, CP-9) — mock `[500, 500, 200]`; ассерт: 3 запроса, паузы ≥1с и ≥2с (с допуском 0.5с).
   - `test_send_5xx_retry_limit_3` (REQ-7.3, CP-9) — mock всегда 500; ассерт: 3 запроса, ok=false.
   - `test_send_400_no_retry` (REQ-7.6, CP-10) — mock `400` с `{"ok":false,"description":"bad chat"}`; ассерт: 1 запрос; outputs `ok=false`, `http_status=400`.
   - `test_send_401_no_retry` (CP-10) — mock 401; 1 запрос.
   - `test_send_token_masked_in_url_log` (REQ-2.2, CP-11) — в stderr нет открытого `NF_BOT_TOKEN`; даже при debug-логе curl-команды.
   - `prop_retry_limit_429` (CP-8) — mock бесконечный 429; ассерт ровно 3 запроса.
   - `prop_retry_limit_5xx_backoff` (CP-9) — mock бесконечный 500; ассерт 3 запроса + интервалы 1с/2с/4с (±0.5).
   - `prop_no_retry_4xx` (CP-10) — for each in {400, 401, 403, 404} → ровно 1 запрос.
   - `prop_json_shape` (CP-15) — matrix `parse_mode ∈ {MarkdownV2, none}` × `thread_id ∈ {empty, "7"}`; ассерт через `jq -e` на актуальный JSON.
2. Создать файл `scripts/send.sh` (source-only, guard):
   - Source `lib.sh`.
   - Константа `_NF_API_BASE="${NF_API_BASE:-https://api.telegram.org}"` (env-override полезен для тестов с mock-сервером).
   - Функция `_nf::_build_json <text>`:
     - Формирует JSON через `jq -n` с `--arg`/`--argjson`. Например:
       ```bash
       jq -n \
         --arg chat_id "$NF_CHAT_ID" \
         --arg text "$1" \
         --arg parse_mode "$NF_PARSE_MODE" \
         --argjson dwpp "$NF_DISABLE_WEB_PAGE_PREVIEW" \
         --argjson dnotif "$NF_DISABLE_NOTIFICATION" \
         --arg thread_id "$NF_MESSAGE_THREAD_ID" \
         '{
            chat_id: ($chat_id | tonumber? // $chat_id),
            text: $text,
            disable_web_page_preview: $dwpp,
            disable_notification: $dnotif
          }
          + (if $parse_mode == "none" or $parse_mode == "" then {} else {parse_mode: $parse_mode} end)
          + (if $thread_id == "" then {} else {message_thread_id: ($thread_id | tonumber)} end)'
       ```
   - Функция `_nf::_do_request <json>` — выполняет один POST, возвращает через stdout двухстрочный вывод: `<http_status>\n<body>`. Использует `curl -sS -o - -w '\n%{http_code}' -X POST -H 'Content-Type: application/json' --data-binary @-`. URL формируется как `${_NF_API_BASE}/bot${NF_BOT_TOKEN}/sendMessage`. Токен — НЕ логировать.
   - Функция `nf::send <text>`:
     - `json="$(_nf::_build_json "$1")"`.
     - Loop с `attempt ∈ {1,2,3}`:
       1. Выполнить request, разобрать `http_status` и `body`.
       2. Если `http=200` и `(echo "$body" | jq -er '.ok')` true → выставить outputs (`ok=true`, `message_id=$(echo "$body"|jq -r '.result.message_id')`, `http_status=200`) и return 0.
       3. Если `http=429`: `retry_after=$(echo "$body"|jq -r '.parameters.retry_after // 1')`; if attempt < 3: `sleep "$retry_after"`; continue.
       4. Если `http >= 500`: if attempt < 3: `sleep $((1 << (attempt-1)))` (1, 2, 4 — но т.к. attempt максимум 3, паузы 1с и 2с; для unit-теста с 3 повторами интервалы [1, 2]); continue. **CORRECTION:** REQ-7.3 говорит «экспонента 1s, 2s, 4s до 3 попыток». 3 попытки = 2 интервала. Интервалы: 1с после 1-й попытки, 2с после 2-й. Если хотим точно 1/2/4 — нужно 4 попытки. **DECISION:** держим REQ-7.3 в букве — 3 попытки, интервалы 1с/2с. Документировать.
       5. Если `http в [400..499] \ {429}` → break loop (no retry).
       6. Иначе (сетевая ошибка, rc curl != 0) — трактовать как 5xx-эквивалент: backoff + retry.
     - После loop: outputs (`ok=false`, `message_id=""`, `http_status=<последний>`); return 1.
   - **CORRECTION к шагу выше:** проверить с REQ-7.3 ещё раз — буквально «экспонента (`1s`, `2s`, `4s`) — не более 3 повторов суммарно». То есть «повторов» = retry attempts, итого до 4 запросов. Уточняем: первичный запрос + до 3 ретраев = до 4 запросов. Тесты `test_send_5xx_exponential_backoff` обновить: 4 запроса, интервалы [1, 2, 4]. Аналогично для 429.
3. Запустить `make test` — все `send.bats` тесты зелёные.

IMPORTANT (уточнение REQ-7.2/7.3): «3 повтора» в REQ означает 3 retry-попыток после исходной, итого до 4 запросов. Реализация: `for attempt in 1 2 3 4`, при провале на attempt < 4 — sleep по правилу. Тесты строятся именно на этом: при бесконечной 429/500 mock получает 4 запроса. При смешанном `[500, 500, 200]` — 3 запроса успешны. Дополнить design-уточнение в файле `design.md` как пометку (НО: не редактировать design сейчас — фиксируем в implementation report).

CRITICAL: при формировании URL `https://api.telegram.org/bot${TOKEN}/sendMessage` — НЕ логируй полную команду curl. Если включаешь `--verbose` для debug-режима, перенаправляй в `/dev/null` или фильтруй токен через `nf::mask` перед записью.

DO NOT использовать `curl --retry` — не удовлетворяет REQ-7.2.

---

## T-7 — Wire up entrypoint, action.yml, CI matrix, release workflow

***_Requirements: 1.3, 2.1, 7.4, 7.5, 9.1, 9.2, 9.3_***
***_Complexity: standard_***
***_Preservation: CP-11, CP-13, CP-16_***

GOAL: собрать оркестрацию (entrypoint), описать манифест action-а, написать CI и release workflows. На этой задаче подключаются все ранее реализованные модули.

1. Создать файл `scripts/entrypoint.sh` (executable, `#!/usr/bin/env bash`):
   - `set -euo pipefail`.
   - Source `lib.sh`, `validate.sh`, `filter.sh`, `escape.sh`, `render.sh`, `send.sh`.
   - `nf::require_bash_4`; `nf::require_command curl`; `nf::require_command jq`.
   - Сразу после старта: `nf::mask "$NF_BOT_TOKEN"` (REQ-2.1).
   - `nf::validate` (REQ-1.1..1.6, 3.3, 6.4 — exit-коды).
   - `if ! nf::should_notify "$NF_STATUS" "$NF_NOTIFY_ON"; then nf::log info "skipping (...)"; nf::set_output ok false; nf::set_output message_id ""; nf::set_output http_status 0; exit 0; fi`.
   - `text="$(nf::render)"`.
   - `if nf::send "$text"; then exit 0; fi`.
   - Если `nf::send` вернул 1: `if [[ "${NF_FAIL_ON_ERROR}" == "true" ]]; then nf::log error "SEND_FAILED"; exit 1; fi`; иначе `nf::log warn "SEND_FAILED (fail_on_error=false)"; exit 0`.
2. Создать файл `action.yml`:
   - `name: 'notiflow — Telegram CI Notifier'`.
   - `description: 'Send a Telegram message when a workflow job completes.'`.
   - `author: 'jtprogru'`.
   - `branding: { icon: 'send', color: 'blue' }`.
   - `inputs:` (все 15 inputs из design §2.3 с `description`, `required`, `default`). Для `status` — `default: ${{ job.status }}`. Для `parse_mode` — `default: MarkdownV2`. Для `notify_on` — `default: 'success,failure,cancelled'`. Для `disable_web_page_preview` — `default: 'true'`. Для `disable_notification` — `default: 'false'`. Для `fail_on_error` — `default: 'false'`. Остальные — пустые дефолты.
   - `outputs:` `ok`, `message_id`, `http_status` с `value: ${{ steps.notiflow.outputs.<key> }}`.
   - `runs:`
     - `using: composite`.
     - `steps:`
       - id `notiflow`, `shell: bash`, `run: |\n  bash "${{ github.action_path }}/scripts/entrypoint.sh"`.
       - `env:` — экспорт всех `INPUT_*` → `NF_*` явно (например, `NF_BOT_TOKEN: ${{ inputs.bot_token }}`, …, `NF_FAIL_ON_ERROR: ${{ inputs.fail_on_error }}`).
3. Создать файл `tests/entrypoint.bats`:
   - `test_entrypoint_fail_on_error_true_exits_nonzero` (REQ-7.4, CP-13) — mock 400, `NF_FAIL_ON_ERROR=true` → exit != 0.
   - `test_entrypoint_fail_on_error_false_exits_zero` (REQ-7.5, CP-13) — mock 400, `NF_FAIL_ON_ERROR=false` → exit 0; stderr содержит `::warning::`.
   - `test_entrypoint_skip_outputs_correct` (REQ-3.2, REQ-8.2) — status=success, notify_on=failure → outputs `ok=false`, `http_status=0`, exit 0; mock НЕ получает запросов.
   - `test_entrypoint_status_default_from_job` (REQ-1.3, CP-16) — `INPUT_STATUS` unset, `JOB_STATUS=success` (env) → render использует success; mock получает запрос.
   - `test_entrypoint_full_happy_path` — все inputs корректные, mock 200 → outputs `ok=true`, exit 0; verify JSON-shape в `requests.log`.
4. Создать файл `.github/workflows/ci.yml`:
   - `on: [push, pull_request]`.
   - Job `test`:
     - `strategy: matrix: os: [ubuntu-latest, macos-latest]`.
     - `runs-on: ${{ matrix.os }}`.
     - Steps: `actions/checkout@v4` → `make install-tools` → `make lint` → `make test`.
   - Job `smoke-test` (опциональный):
     - `needs: test`.
     - `if: github.event_name == 'push' && github.ref == 'refs/heads/main'` (требует секрета; пропускается на PR от форков).
     - Использует сам action: `uses: ./` с тестовыми `bot_token: ${{ secrets.TEST_BOT_TOKEN }}`, `chat_id: ${{ secrets.TEST_CHAT_ID }}`; `continue-on-error: true`.
5. Создать файл `.github/workflows/release.yml`:
   - `on: { push: { tags: ['v*.*.*'] } }`.
   - Job `update-major-tag`:
     - `runs-on: ubuntu-latest`.
     - Steps: `actions/checkout@v4` с `fetch-depth: 0` → bash-step, который извлекает мажор (`v1` из `v1.2.3`) и делает `git tag -fa v1 -m "..."`, `git push origin v1 --force-with-lease`.
6. Запустить локально `make lint && make test` — все 5 файлов тестов зелёные (escape + validate + filter + render + send + entrypoint = 6 файлов; lib также есть → 7 в сумме). После — push на временную ветку и проверить, что CI на ubuntu+macos зелёный.

CRITICAL: в `action.yml` поле `runs.steps[].env` ОБЯЗАТЕЛЬНО задаёт каждую `NF_*` переменную явно — НЕ полагайся на автоматический экспорт `INPUT_*`. Это нужно, чтобы `entrypoint.sh` мог читать только `NF_*` env-переменные (упрощает тестирование вне action-контекста).

IMPORTANT: путь к скрипту — `${{ github.action_path }}/scripts/entrypoint.sh`. На windows-latest этот путь будет с обратными слешами, но при `shell: bash` они корректно интерпретируются Git Bash.

NOTE: smoke-test job требует, чтобы у репозитория `jtprogru/notiflow` были настроены секреты `TEST_BOT_TOKEN` и `TEST_CHAT_ID`. Если их нет — job завершится с `secrets not set`; это допустимо, т.к. `continue-on-error: true`.

---

## T-8 — Documentation + final checkpoint (GATE)

***_Requirements: 9.1, 9.2, 9.3_***
***_Complexity: mechanical_***
***_Preservation: CP-1..CP-16_***

GOAL: написать README, провести полную проверку, зафиксировать готовность к релизу.

1. Создать файл `README.md` со структурой:
   - Заголовок + бэйджи (CI status, license).
   - Раздел `Usage` — минимальный пример workflow:
     ```yaml
     jobs:
       build: { runs-on: ubuntu-latest, steps: [...] }
       notify:
         needs: build
         if: always()
         runs-on: ubuntu-latest
         steps:
           - uses: jtprogru/notiflow@v1
             with:
               bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
               chat_id: ${{ secrets.TELEGRAM_CHAT_ID }}
               status: ${{ needs.build.result }}
     ```
   - Таблица `Inputs` (16 строк: name | description | required | default).
   - Таблица `Outputs` (3 строки).
   - Раздел `Placeholders` — список 16 полей из `{{.Field}}`.
   - Раздел `Examples`:
     - кастомный шаблон с `message_template`;
     - per-status шаблон (`template_failure`);
     - фильтр `notify_on: failure,cancelled`;
     - форум-чат с `message_thread_id`;
     - режим `parse_mode: HTML`.
   - Раздел `Requirements` — bash 4+, curl, jq (есть на всех GitHub-раннерах).
   - Раздел `Versioning` — описание `v1` moving tag, ссылка на CHANGELOG.
   - Раздел `Development` — `make install-tools`, `make lint`, `make test`.
   - Раздел `License` — MIT.
2. Создать файл `CHANGELOG.md` со строкой `## v1.0.0 — 2026-05-26` и пунктами всех v1 фич.
3. Финальный GATE:
   - Запустить `make lint` — exit 0.
   - Запустить `make test` — все 6 тестовых файлов зелёные, ассертить: запущено ≥ (16 unit + 16 prop) = 32 теста.
   - Запустить `grep -rE "TODO|FIXME|XXX" scripts/ action.yml` — empty (нет недоделок).
   - Сверить coverage matrix из task-plan.md: каждый REQ-X.Y из requirements.md имеет соответствующий зелёный тест (по тэгу).
   - Проверить, что `NF_BOT_TOKEN` не появляется в `git grep -F "NF_BOT_TOKEN"` ничего кроме определений в скриптах (т.е. нет случайных echo/log с токеном).
   - Зафиксировать в implementation report список «что реализовано / что отложено».

CRITICAL: GATE НЕ проходит, если хотя бы один тест красный, lint выдал ошибки, или TODO-маркеры остались в коде.

DO NOT публиковать релиз `v1.0.0` до approve фазы Review.
