# Implementation Report: Telegram Notify Action (`notiflow`)

## Summary

Composite GitHub Action `notiflow` реализован полностью согласно task-plan-у. 8 top-level задач выполнены последовательно (T-1 → T-8). Итог: 69 bats-тестов проходят (16 unit + 16 prop + 37 integration), shellcheck + shfmt чистые, project готов к релизу `v1.0.0`.

## Commands Used

- **Test:** `make test` (→ `bats tests/`)
- **Build:** `make build` (no-op для composite action)
- **Lint:** `make lint` (shellcheck + shfmt -d + actionlint)
- **Generate:** `make readme` (заглушка, не используется в v1)

## Task Execution

- [x] **T-1** Scaffold project tooling — `.gitignore`, `Makefile` (help/lint/lint-fix/test/build/install-tools/readme), `LICENSE` (MIT). `make help` работает.
- [x] **T-2** Build common bash infrastructure — `scripts/lib.sh` (5 функций), `tests/helpers.bash`, `tests/fixtures/mock_server.py` (Python http-listener с fixtures), `tests/lib.bats` (5/5 зелёные).
- [x] **T-3** Implement escape module — `scripts/escape.sh`, `tests/escape.bats` (10/10 зелёные).
  - Note: первая реализация sed character-class была неправильной (`]` не на месте). После правки `[][_*()~``>#+=|{}.!-]` все 18 спецсимволов MarkdownV2 экранируются корректно.
- [x] **T-4** Implement validate + filter modules — `scripts/validate.sh` (6 exit-кодов 10–15), `scripts/filter.sh`, `tests/validate.bats` (15/15), `tests/filter.bats` (4/4).
  - Note: в property-тесте `filter` пришлось переименовать loop-переменную, т.к. bats `run` затеняет `$status`. Также для bats не подходит native `nf::should_notify` без `|| true`, иначе rc=1 ломает тест.
- [x] **T-5** Implement render module — `scripts/render.sh`, `tests/render.bats` (16/16).
  - Note: BSD sed (macOS) не поддерживает inline `:label;...` для multi-line replace — упростил `_nf::_sed_rhs_escape` до `tr -d '\n' | sed`, т.к. значения плейсхолдеров single-line по контракту.
  - Note: env-prefix `VAR=val [ ... ]` в bats не пробрасывается в `$(...)` substitutions — переписал на explicit `export`/`unset`.
- [x] **T-6** Implement send module with retry — `scripts/send.sh`, `tests/send.bats` (13/13).
  - Note: Уточнение по REQ-7.2/7.3 (зафиксировано в task-plan T-6): «не более 3 повторов» интерпретируется как 1 первичный запрос + до 3 retry = **до 4 запросов всего** (константа `_NF_MAX_ATTEMPTS=4`). Тесты на бесконечный 429/500 ассертят ровно 4 запроса.
- [x] **T-7** Wire up entrypoint + action.yml + workflows — `scripts/entrypoint.sh`, `action.yml` (16 inputs + 3 outputs + composite-runs), `tests/entrypoint.bats` (6/6), `.github/workflows/ci.yml` (matrix ubuntu+macos, install-tools, lint, test), `.github/workflows/release.yml` (moving major-tag через `git tag -fa v1`).
  - Note: token-leak тест переформулирован: единственное допустимое появление токена в логе — это сама `::add-mask::` директива (без неё GitHub Actions не сможет замаскировать токен в последующих строках).
- [x] **T-8** Documentation + GATE — `README.md` (Usage / Inputs / Outputs / Placeholders / 5 примеров / Versioning / Development), `CHANGELOG.md` (`v1.0.0 — 2026-05-26`). Финальный прогон: `make lint` чистый, `make test` 69/69, `make build` ok, `grep -E 'TODO|FIXME|XXX'` пусто, token не утекает через `echo`/`printf` нигде в скриптах.

## Deviations from Plan

1. **Bash 3.2 совместимость вместо Bash 4+** (отступ от Design ADR-4). Локальная разработка ведётся на macOS системном bash 3.2 (homebrew bash отсутствует), и runner-ы `macos-latest` тоже используют bash 3.2 как `/bin/bash`. Решение: переписать с использованием parallel-list (`_NF_PLACEHOLDER_KEYS="..."`) и `case`-statement вместо associative array. `nf::require_bash` теперь требует ≥ 3.2, не ≥ 4. Архитектурное упрощение, не функциональный регресс.
2. **`actionlint` опционален в `make lint`.** Локально его нет, на CI ставится через `install-tools`. `make lint` выводит warning «skipping» и продолжает, если бинарь не найден.
3. **Property-тесты для bash написаны как targeted unit tests** (явный перебор представительных входов на свойство). PBT-библиотек для bash не существует — этот компромисс был зафиксирован уже в Design §2.8 «Test Style Source: Tier 3».

## Final Verification

### Tests

```
$ bats tests/
1..69
ok 1 entrypoint: happy path 200 sets outputs and exits 0
ok 2 entrypoint: skip on filter, no HTTP, exit 0 (REQ-3.2, REQ-8.2)
ok 3 entrypoint: fail_on_error=true exits non-zero on 400 (REQ-7.4, CP-13)
ok 4 entrypoint: fail_on_error=false exits 0 on 400 (REQ-7.5, CP-13)
ok 5 entrypoint: status default from JOB_STATUS env (REQ-1.3, CP-16)
ok 6 entrypoint: token only appears in mask directive (CP-11)
... (63 more)
ok 65 validate: parse_mode 'none' accepted
ok 66 validate: notify_on defaults
ok 67 validate: invalid notify_on item exits 14
ok 68 validate: thread_id integer accepted
ok 69 validate: thread_id non-integer exits 15
```

Summary: **69 passed, 0 failed**.

### Build

```
$ make build
composite action — nothing to build
```

### Lint

```
$ make lint
shellcheck -x scripts/*.sh tests/*.bash
shfmt -d -i 2 -ci scripts tests
actionlint not found — skipping (run: make install-tools)
```

No errors. `actionlint` будет запущен в CI.

## Files Changed

Created (NEW), 24 files total:

```
.gitignore
LICENSE
Makefile
README.md
CHANGELOG.md
action.yml
.github/workflows/ci.yml
.github/workflows/release.yml
scripts/entrypoint.sh
scripts/lib.sh
scripts/validate.sh
scripts/filter.sh
scripts/escape.sh
scripts/render.sh
scripts/send.sh
tests/helpers.bash
tests/lib.bats
tests/escape.bats
tests/validate.bats
tests/filter.bats
tests/render.bats
tests/send.bats
tests/entrypoint.bats
tests/fixtures/mock_server.py
tests/fixtures/.gitkeep
```

## Notes

- Все CP (CP-1..CP-16) покрыты тестами (см. coverage matrix в task-plan.md).
- Token-маскирование подтверждено: `grep -F NF_BOT_TOKEN` в логе после полного прогона retry-сценариев находит только директиву `::add-mask::testtoken123`, не «голый» токен.
- Smoke-test job в `.github/workflows/ci.yml` опциональный (требует секретов `TEST_BOT_TOKEN`/`TEST_CHAT_ID` — добавляются вручную после первого push).
- Release-workflow готов к публикации `v1.0.0`: push тега `v1.0.0` → автоматическое обновление moving `v1` тега.
- Pre-existing issues не обнаружено (репозиторий greenfield).
