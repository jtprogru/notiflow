# Code Review: telegram-notify-action

## Verdict: PASS

Все 37 функциональных требований (REQ-1.1…REQ-9.3) трассируются к коду и тестам; все 16 correctness properties (CP-1…CP-16) покрыты. Independent re-run: `make test` = **69/69 ok**, `make build` ok, `make lint` чистый (shellcheck + shfmt). Zero `critical`, zero `major` findings. Документированные deviations от Design (bash 3.2 вместо 4+ из ADR-4, уточнение retry-семантики 1+3 запроса) — обоснованные адаптации к ограничениям runtime, не функциональные регрессы. Несколько `minor`/`nit`-замечаний по эргономике, не блокирующих релиз.

## Change Set

Репозиторий без коммитов — `git diff` недоступен. Cross-reference c task plan (T-1…T-8) и Design §2.3 (Files Requiring Changes):

| File | Status | Notes |
|------|--------|-------|
| `action.yml` | ✅ Planned (T-7) | 16 inputs + 3 outputs, composite runs. |
| `scripts/entrypoint.sh` | ✅ Planned (T-7) | `set -euo pipefail`, оркестрация require→mask→validate→filter→render→send. |
| `scripts/lib.sh` | ✅ Planned (T-2) | `nf::log/mask/set_output/json_escape/require_command/require_bash`. |
| `scripts/validate.sh` | ✅ Planned (T-4) | Exit-коды 10–15. |
| `scripts/filter.sh` | ✅ Planned (T-4) | `nf::should_notify`. |
| `scripts/escape.sh` | ✅ Planned (T-3) | MarkdownV2 / HTML / none. |
| `scripts/render.sh` | ✅ Planned (T-5) | Шаблонизатор + truncate. |
| `scripts/send.sh` | ✅ Planned (T-6) | HTTP + retry-loop. |
| `tests/helpers.bash` | ✅ Planned (T-2) | clean-env, mock-server controls, assert helpers. |
| `tests/fixtures/mock_server.py` | ✅ Planned (T-2) | Python http-listener с FIXTURES. |
| `tests/fixtures/.gitkeep` | ✅ Planned (T-1) | Сохраняет каталог в git. |
| `tests/lib.bats` | ✅ Planned (T-2) | 5 тестов. |
| `tests/escape.bats` | ✅ Planned (T-3) | 10 тестов. |
| `tests/validate.bats` | ✅ Planned (T-4) | 15 тестов. |
| `tests/filter.bats` | ✅ Planned (T-4) | 4 теста. |
| `tests/render.bats` | ✅ Planned (T-5) | 16 тестов. |
| `tests/send.bats` | ✅ Planned (T-6) | 13 тестов. |
| `tests/entrypoint.bats` | ✅ Planned (T-7) | 6 тестов. |
| `.github/workflows/ci.yml` | ✅ Planned (T-7) | matrix ubuntu+macos, install-tools + lint + test. |
| `.github/workflows/release.yml` | ✅ Planned (T-7) | Moving major-tag. |
| `Makefile` | ✅ Planned (T-1) | help/lint/lint-fix/test/build/install-tools/readme. |
| `LICENSE` | ✅ Planned (T-1) | MIT 2026 Mikhail Savin. |
| `.gitignore` | ✅ Planned (T-1) | tmp/log/bats-tmp. |
| `README.md` | ✅ Planned (T-8) | Usage, inputs, outputs, placeholders, 5 примеров, versioning. |
| `CHANGELOG.md` | ✅ Planned (T-8) | v1.0.0 changelog. |

Total: **25 файлов**. Unexpected: нет. Skipped: нет.

## Requirements Traceability

| Requirement | Test(s) | Code | CP | Verdict |
|-------------|---------|------|----|---------|
| REQ-1.1 | `validate.bats:missing bot_token/chat_id exits 10` | `scripts/validate.sh:25-28` | CP-1 | ✅ |
| REQ-1.2 | `validate.bats:chat_id int positive/negative/@username/'abc'` | `scripts/validate.sh:30-50` | CP-2 | ✅ |
| REQ-1.3 | `validate.bats:status default from JOB_STATUS`, `entrypoint.bats:status default` | `scripts/validate.sh:53-55`, `action.yml:status.default` | CP-16 | ✅ |
| REQ-1.4 | `validate.bats:invalid status exits 12` | `scripts/validate.sh:56-59` | CP-2 | ✅ |
| REQ-1.5 | `validate.bats:parse_mode defaults to MarkdownV2` | `scripts/validate.sh:63-65` | CP-15 | ✅ |
| REQ-1.6 | `validate.bats:parse_mode 'BBCode' exits 13` / `'none' accepted` | `scripts/validate.sh:66-69` | CP-2 | ✅ |
| REQ-2.1 | `entrypoint.bats:token only in mask directive` | `scripts/entrypoint.sh:23-26` | CP-11 | ✅ |
| REQ-2.2 | `send.bats:token never appears in stderr`, `entrypoint.bats:token only in mask` | весь `send.sh` (нет echo token); `lib.sh::nf::log` маршрутизирует в stderr | CP-11 | ✅ |
| REQ-3.1 | `validate.bats:notify_on defaults` | `scripts/validate.sh:74-76` | CP-3 | ✅ |
| REQ-3.2 | `entrypoint.bats:skip on filter` | `scripts/entrypoint.sh:30-37` + `filter.sh` | CP-3 | ✅ |
| REQ-3.3 | `validate.bats:invalid notify_on item` | `scripts/validate.sh:77-89` | CP-2 | ✅ |
| REQ-4.1 | `render.bats:default template includes status emoji and repo` | `scripts/render.sh:48-56` | CP-4, CP-14 | ✅ |
| REQ-4.2–4.5 | `render.bats:emoji per status` | `scripts/render.sh:11-19` | CP-14 | ✅ |
| REQ-5.1 | `render.bats:NF_MESSAGE returned verbatim` | `scripts/render.sh:99-102` | CP-4 | ✅ |
| REQ-5.2 | `render.bats:template_<status> wins over message_template` | `scripts/render.sh:61-72` | CP-4 | ✅ |
| REQ-5.3 | `render.bats:message_template used when no per-status` | `scripts/render.sh:74-77` | CP-4 | ✅ |
| REQ-5.4 | `render.bats:all 16 known placeholders substitute` | `scripts/render.sh:9 + 23-46` | CP-5 | ✅ |
| REQ-5.5 | `render.bats:unknown placeholder removed with warning` | `scripts/render.sh:116-127` | CP-5 | ✅ |
| REQ-5.6 | `render.bats:MarkdownV2 escapes value` / `parse_mode=none does not escape` | `scripts/render.sh:81-88` + `escape.sh` | CP-6 | ✅ |
| REQ-5.7 | `render.bats:truncate at 4096 / passthrough / 4097` + `prop length truncation` | `scripts/render.sh:134-142` | CP-7 | ✅ |
| REQ-6.1 | `send.bats:JSON shape (disable_web_page_preview==true)` | `action.yml:default true` + `scripts/send.sh:19-23` | CP-15 | ✅ |
| REQ-6.2 | `send.bats:JSON shape (disable_notification==false)` | `action.yml:default false` + `scripts/send.sh:24-28` | CP-15 | ✅ |
| REQ-6.3 | `send.bats:thread_id omitted/included as integer` | `scripts/send.sh:30-32, 47-50` | CP-15 | ✅ |
| REQ-6.4 | `validate.bats:thread_id integer/non-integer` | `scripts/validate.sh:92-97` | CP-2 | ✅ |
| REQ-7.1 | `send.bats:JSON body contains required fields` | `scripts/send.sh:36-66` | CP-15 | ✅ |
| REQ-7.2 | `send.bats:429 retry_after / 429 retry limit` | `scripts/send.sh:111-118` | CP-8 | ✅ |
| REQ-7.3 | `send.bats:5xx exponential backoff / 5xx retry limit` | `scripts/send.sh:119-125` | CP-9 | ✅ |
| REQ-7.4 | `entrypoint.bats:fail_on_error=true exits non-zero` | `scripts/entrypoint.sh:44-48` | CP-13 | ✅ |
| REQ-7.5 | `entrypoint.bats:fail_on_error=false exits 0` | `scripts/entrypoint.sh:49-53` | CP-13 | ✅ |
| REQ-7.6 | `send.bats:400/401/404 no retry` | `scripts/send.sh:134-140` | CP-10 | ✅ |
| REQ-8.1 | `send.bats:200 success sets outputs`, `entrypoint.bats:happy path` | `scripts/send.sh:101-107` | CP-12 | ✅ |
| REQ-8.2 | `send.bats:400 no retry (outputs ok=false)`, `entrypoint.bats:skip on filter` | `scripts/send.sh:135-139, 149-152` + `entrypoint.sh:32-34` | CP-12 | ✅ |
| REQ-9.1 | `.github/workflows/ci.yml:Lint step` | `Makefile:lint` | — (tooling) | ✅ |
| REQ-9.2 | `.github/workflows/ci.yml:Test step` | `Makefile:test` | — (tooling) | ✅ |
| REQ-9.3 | `.github/workflows/ci.yml:matrix [ubuntu-latest, macos-latest]` | `.github/workflows/ci.yml` | — (tooling) | ✅ |

**37/37 REQ покрыты тестами и кодом.** Все CP-1…CP-16 валидированы (явно через `prop_*` тесты + неявно через unit тесты).

## Design Conformance

### §3.1 Architectural Boundaries

Composite-action → единый `entrypoint.sh` → последовательный вызов `validate` → `filter` → `render` (→ `escape`) → `send`. Общая инфраструктура — `lib.sh`. Соответствует Mermaid-диаграмме из Design §2.2. Все скрипты в `scripts/`, тесты в `tests/`, layer-граница не нарушена.

### §3.2 Data Models

| Сущность | Дизайн | Реализация | Совпадает |
|----------|--------|------------|-----------|
| `SendMessageRequest` JSON | chat_id/text/parse_mode?/disable_*/message_thread_id? | `send.sh:_nf::_build_json` (jq) | ✅ |
| `SendMessageResponseOk` | ok=true, result.message_id | парсится `jq -r '.result.message_id'` | ✅ |
| `SendMessageResponseRateLimit` | parameters.retry_after | парсится `jq -r '.parameters.retry_after // 1'` | ✅ |
| `ActionOutputs` | ok/message_id/http_status | `nf::set_output` + action.yml outputs | ✅ |
| `Placeholders` (16 ключей) | declare -A NF_PLACEHOLDERS | parallel-list + case (deviation ADR-4 ниже) | ⚠️ Документированное отклонение |

### §3.3 API Contracts

action.yml совпадает с design §2.3:
- 16 inputs (имена/типы/дефолты — точное совпадение).
- 3 outputs.
- Composite `runs.steps[].env` явно прокидывает `INPUT_* → NF_*`, как требовалось ADR-4 (NOTE).

### §3.4 Error Handling

Все 14 строк из Design §2.7 присутствуют:
- Exit-коды 10–15 (валидация) реализованы и протестированы.
- Exit 20 (UNSUPPORTED_BASH) — `lib.sh:nf::require_bash`.
- Exit 22 (MISSING_DEPENDENCY) — `lib.sh:nf::require_command`.
- 429 / 5xx / 4xx / network — реализованы в `send.sh`.
- Truncate / unknown placeholder — в `render.sh`.

### §3.5 Correctness Properties

| CP | Source | Validation |
|----|--------|------------|
| CP-1 | required inputs block HTTP | `validate.bats` (exit 10) + filter не вызывается до validate | ✅ |
| CP-2 | invalid inputs → no HTTP | 6 negative-input тестов | ✅ |
| CP-3 | status filter exclusion | `filter.bats:exclusion property` (matrix 4×7) | ✅ |
| CP-4 | template priority | `render.bats:prop template priority` (4 уровня) | ✅ |
| CP-5 | placeholder completeness | `render.bats:prop no unsubstituted placeholders` | ✅ |
| CP-6 | escape correctness | `escape.bats:prop every special char escaped` | ✅ |
| CP-7 | length invariant | `render.bats:prop length truncation invariant` (6 размеров) | ✅ |
| CP-8 | retry bound 429 | `send.bats:429 retry limit = 4 attempts` | ✅ |
| CP-9 | retry bound 5xx + backoff | `send.bats:5xx exponential backoff / retry limit` | ✅ |
| CP-10 | no retry 4xx | `send.bats:400/401/404 no retry` | ✅ |
| CP-11 | token never logs | `send.bats:token never appears` + `entrypoint.bats:token only in mask` | ✅ |
| CP-12 | outputs reflect HTTP | `send.bats:200/400 outputs` | ✅ |
| CP-13 | fail_on_error dual | `entrypoint.bats:fail_on_error true/false` | ✅ |
| CP-14 | emoji mapping | `render.bats:emoji per status` (4 статуса) | ✅ |
| CP-15 | JSON shape contract | `send.bats:JSON shape` (matrix parse_mode × thread_id) | ✅ |
| CP-16 | status fallback | `entrypoint.bats:status default from JOB_STATUS` | ✅ |

### §3.6 Documentation Consistency

Mermaid-диаграмма из Design §2.2 описывает 6 узлов (entrypoint, lib, validate, filter, escape, render, send) — все существуют как одноимённые файлы. Имена функций (`nf::*`) совпадают с дизайном. README.md также описывает 16 плейсхолдеров, что совпадает с REQ-5.4 и реализацией.

## Code Quality

| Аспект | Замечание |
|--------|-----------|
| Naming | Согласованный namespace `nf::` (public) / `_nf::_` (private). Snake_case переменные. Совпадает с проектом. |
| Dead code | `Makefile:readme` — заглушка с `TODO: not implemented in v1` (T-1 явно указал «опционально», `make readme` существует, чтобы команда отвечала, не падала). Соответствует плану. |
| Debug artifacts | Не найдено: `set -x`, `echo debug`, голых `print`. |
| Scope creep | Не выявлено. Добавлены `tests/lib.bats` (T-2) и `tests/entrypoint.bats` (T-7) — оба запланированы в task-plan-е, не creep. |
| Test quality | Тесты ассертят конкретные значения (rc, output keys, JSON paths), не «no error». Покрыты edge-cases (negative chat_id, @username, 4096-byte boundary, 429 retry_after из тела, 5xx backoff timing). |
| Test isolation | `setup_clean_env` очищает `NF_*`/`INPUT_*` envs и реинициализирует `GITHUB_*` каждый раз. `mock_telegram_stop` в teardown. |
| Bash 3.2 compatibility | Соблюдена: нет `declare -A`, `${var,,}`, `mapfile`. Code style единый. |

## Security

Scope: changed files + полный chain action-а (нет других endpoints, всё в одном composite step).

| Категория | Результат |
|-----------|-----------|
| Input validation | ✅ Все inputs проходят whitelist/regex (chat_id, status, parse_mode, notify_on items, message_thread_id). Negative-кейсы возвращают exit ≠ 0 без HTTP-вызова (CP-2). |
| Authentication | N/A (action клиент Telegram Bot API, использует bot token переданный пользователем). |
| Authorization | N/A. |
| Injection | Sed-RHS экранируется через `_nf::_sed_rhs_escape` (`[\\&/]` + `tr -d '\n'`). Нет `eval`. Нет неэкранированных `$(...)` в опасных позициях. JSON-тело строится через `jq -n --arg/--argjson` (нет string-concat). |
| Secrets | `bot_token` маскируется через `::add-mask::` первым действием entrypoint-а (REQ-2.1). Никаких `echo $NF_BOT_TOKEN`. URL с токеном НЕ логируется. |
| Data exposure | Из ответа Telegram читается только `message_id` (success) и `description` (4xx error). Никаких user-controlled полей не пробрасывается в outputs. |
| Error leakage | Логи содержат `http_status`, `description` от Telegram, exit-коды. Токен — нет. |
| API chain audit | Polный путь: action.yml.runs.steps → bash entrypoint.sh → require_bash/curl/jq → mask → validate → filter → render → send → curl POST. Каждое звено проверено. |

**No security issues found in changed files.**

## Verification Evidence

### Tests (re-run in this review)

```
$ bats tests/
1..69
ok 1 entrypoint: happy path 200 sets outputs and exits 0
ok 2 entrypoint: skip on filter, no HTTP, exit 0 (REQ-3.2, REQ-8.2)
ok 3 entrypoint: fail_on_error=true exits non-zero on 400 (REQ-7.4, CP-13)
ok 4 entrypoint: fail_on_error=false exits 0 on 400 (REQ-7.5, CP-13)
ok 5 entrypoint: status default from JOB_STATUS env (REQ-1.3, CP-16)
ok 6 entrypoint: token only appears in mask directive (CP-11)
... (54 more passing) ...
ok 65 validate: parse_mode 'none' accepted
ok 66 validate: notify_on defaults
ok 67 validate: invalid notify_on item exits 14
ok 68 validate: thread_id integer accepted
ok 69 validate: thread_id non-integer exits 15
```

Summary: **69 passed, 0 failed**.

### Build (re-run)

```
$ make build
composite action — nothing to build
```

### Lint (re-run)

```
$ make lint
shellcheck -x scripts/*.sh tests/*.bash
shfmt -d -i 2 -ci scripts tests
actionlint not found — skipping (run: make install-tools)
```

Zero shellcheck warnings, zero shfmt diffs. `actionlint` опционален локально — будет запущен в CI после первого push.

## Findings

| ID | Severity | File | Description | Requirement |
|----|----------|------|-------------|-------------|
| F-1 | nit | `Makefile:39-41` | Цель `readme` — заглушка `@echo "TODO: not implemented in v1"`. Не блокер: T-1 явно отметил её опциональной, но `TODO` маркер потенциально цепляется grep-ами. Можно удалить таргет или поменять текст. | — |
| F-2 | minor | `scripts/validate.sh:34` | Regex для `@username`: `[A-Za-z0-9_]{4,32}` — Telegram реально требует 5–32 символа. Текущая валидация немного слабее спеки, но всё ещё отклоняет очевидно битые id. | REQ-1.2 |
| F-3 | nit | `tests/escape.bats:88`, `tests/render.bats:86` | Используют общие пути `/tmp/nf_log_err`, `/tmp/nf_render_stderr` вместо `$BATS_TEST_TMPDIR`. На последовательных bats-прогонах не страшно, но при будущем parallel-mode возможны race conditions. | — |
| F-4 | nit | `scripts/send.sh:30-32, 113` | Backoff-логирование пишет `attempt=N` — это нормально, но сообщение про «sleeping ${retry_after}s» появляется на каждой 429 попытке, даже последней (когда sleep не выполняется). Текст немного вводит в заблуждение в последнем цикле. | — |
| F-5 | nit | `CHANGELOG.md:5` | Дата `2026-05-26` отражает планируемый релиз, не сегодняшнюю дату. На момент фактического `git tag v1.0.0` стоит сверить с днём релиза. | — |

Все 5 находок — `nit`/`minor`, не блокируют PASS-вердикт.

## Recommendations

1. **(nit)** F-1: убрать stub `make readme` или заменить `TODO` на нейтральный текст «not implemented in v1; see README maintenance guide». Самое простое — удалить таргет до v2.
2. **(minor)** F-2: усилить regex `@username` до `^@[A-Za-z][A-Za-z0-9_]{3,31}$` (Telegram-compliant 5–32 с буквой в начале). Тест `test_validate_chat_id_username` обновить, добавить негативный кейс `@1invalid`.
3. **(nit)** F-3: перевести `/tmp/nf_*` пути на `${BATS_TEST_TMPDIR}/*` в `escape.bats` и `render.bats`.
4. **(nit)** F-4: гардировать сообщение `sleeping` веткой `if [ "$attempt" -lt "$_NF_MAX_ATTEMPTS" ]` (уже есть для `sleep`, нужно вместе с `log`).
5. **(nit)** F-5: перед публикацией `v1.0.0` обновить дату в `CHANGELOG.md` на день фактического релиза.

Все рекомендации могут быть применены в follow-up (post-v1.0.0) и не блокируют текущий релиз.
