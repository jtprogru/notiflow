<!-- generated: 2026-05-27, template: bootstrap.md -->

# notiflow Documentation

This folder contains documentation to help LLMs and developers quickly understand the project context.

`notiflow` is a composite GitHub Action that sends a Telegram message when a workflow job completes. The implementation is a set of POSIX-compatible bash scripts (bash 3.2+) wired together by an `action.yml` manifest. There is no compile step.

## Documentation Index

### Core

- [agent-rules.md](./agent-rules.md) — mandatory rules for AI agents working on this project (style, naming, errors, testing, formatting).
- [ARCHITECTURE.md](./ARCHITECTURE.md) — pipeline overview, layer diagram, request data flow, and the seven ADRs that shape the codebase.
- [PACKAGES.md](./PACKAGES.md) — per-module reference for every file under `scripts/`, `tests/`, `.github/workflows/`, and the Makefile.
- [DOMAIN.md](./DOMAIN.md) — typed values flowing between GitHub Actions, the script pipeline, and the Telegram API: env contract, JSON shapes, placeholders, enums.
- [CODE_STYLE.md](./CODE_STYLE.md) — bash-specific conventions: `nf::*` namespace, `NF_*` env contract, exit codes, logging, bash 3.2 compatibility rules.
- [features/telegram-notify-action/](./features/telegram-notify-action/) — authoritative spec for v1.0.0 (explore, requirements, design, task plan, implementation, review).

### Development

- [TOOLS.md](./TOOLS.md) — Makefile target reference, prerequisites, first-run steps, CI cheatsheet, install commands for `bats-core` / `shellcheck` / `shfmt` / `actionlint` / `jq`.
- [TESTING.md](./TESTING.md) — `bats-core` suite (69 tests across 7 files), Python `http.server` mock at `tests/fixtures/mock_server.py`, helper patterns (`setup_clean_env`, `mock_telegram_start`, `assert_output_eq`), CP-N property-style coverage.
- [SCRIPTS.md](./SCRIPTS.md) — superseded by [PACKAGES.md](./PACKAGES.md) §3 (one entry per `scripts/*.sh`).

### Infrastructure

- [CI.md](./CI.md) — TODO. `.github/workflows/ci.yml` (matrix on ubuntu-latest + macos-latest) and `release.yml` (moving major tag).

## Quick Facts

| Aspect            | Technology |
|-------------------|------------|
| **Type**          | GitHub composite Action |
| **Language**      | Bash 3.2+ (no associative arrays, macOS system bash compatible) |
| **Runtime deps**  | `bash`, `curl`, `jq` |
| **Test framework**| `bats-core` |
| **Mock server**   | Python 3 stdlib (`http.server`) |
| **Lint**          | `shellcheck`, `shfmt -i 2 -ci`, `actionlint` |
| **CI**            | GitHub Actions, matrix `ubuntu-latest` × `macos-latest` |
| **Release**       | Tag `v*.*.*` → `release.yml` force-updates moving `v1` tag |
| **License**       | MIT |
| **Versioning**    | SemVer |
| **External API**  | Telegram Bot API (`sendMessage`) |

## Project Structure

```
notiflow/
├── action.yml              # Composite action manifest (inputs, outputs, env wiring)
├── Makefile                # help / lint / lint-fix / test / build / install-tools
├── README.md               # User-facing docs (usage, inputs, outputs, examples)
├── CHANGELOG.md            # Keep a Changelog format
├── LICENSE                 # MIT
├── scripts/                # All runtime logic
│   ├── entrypoint.sh       # Orchestrator: deps → mask → validate → filter → render → send
│   ├── lib.sh              # Shared helpers: log, mask, set_output, json_escape, require_*
│   ├── validate.sh         # Input validation (exit codes 10..15)
│   ├── filter.sh           # nf::should_notify (notify_on CSV match)
│   ├── escape.sh           # nf::escape_md_v2 / nf::escape_html / nf::escape_none
│   ├── render.sh           # Template selection + placeholder substitution + truncation
│   └── send.sh             # HTTP POST to Telegram, retry policy, output writing
├── tests/                  # bats-core suite + helpers + fixtures
│   ├── helpers.bash        # setup_clean_env, mock_telegram_start/stop, output assertions
│   ├── entrypoint.bats     # End-to-end through the orchestrator
│   ├── lib.bats            # Logging, masking, set_output, require_*
│   ├── validate.bats       # All exit codes 10..15
│   ├── filter.bats         # notify_on edge cases (whitespace, casing, unknowns)
│   ├── escape.bats         # MarkdownV2 / HTML escaping correctness
│   ├── render.bats         # Templates, placeholders, truncation, unknown placeholders
│   ├── send.bats           # 200 / 429 / 5xx / 4xx / network paths via mock
│   └── fixtures/
│       └── mock_server.py  # Stdlib HTTP server with named JSON fixtures
├── .github/workflows/
│   ├── ci.yml              # Lint + test on push to main and PRs (ubuntu, macos)
│   └── release.yml         # On v*.*.* tag, force-update v<major> moving tag
└── .spec/                  # This documentation tree
    ├── README.md           # You are here
    ├── agent-rules.md      # Agent contract
    └── features/           # Per-feature spec-driven dev artifacts
```

## Running

All targets are wrapped by `Makefile`:

| Command              | Purpose                                                      |
|----------------------|--------------------------------------------------------------|
| `make help`          | List documented targets.                                     |
| `make install-tools` | Install `bats-core`, `shellcheck`, `shfmt`, `actionlint`, `jq` (Homebrew on macOS, apt on Linux). |
| `make lint`          | Run `shellcheck -x scripts/*.sh tests/*.bash`, `shfmt -d -i 2 -ci scripts tests`, and `actionlint`. |
| `make lint-fix`      | Auto-format shell scripts with `shfmt -w`.                   |
| `make test`          | Run the full `bats tests/` suite.                            |
| `make build`         | No-op (composite action — no compile step).                  |
| `make readme`        | TODO placeholder; not implemented in v1.                     |

Local one-off run (bypassing GitHub):

```bash
NF_BOT_TOKEN=... NF_CHAT_ID=... NF_STATUS=success \
GITHUB_REPOSITORY=jtprogru/notiflow GITHUB_RUN_ID=1 GITHUB_WORKFLOW=CI \
bash scripts/entrypoint.sh
```

## Ports

The runtime has no server component. Only the test mock server binds a port:

| Component                       | Port               | Notes |
|---------------------------------|--------------------|-------|
| `tests/fixtures/mock_server.py` | `127.0.0.1:0` (ephemeral, written to `mock.port` file) | Started by `mock_telegram_start` in `tests/helpers.bash`. Exported as `NF_API_BASE=http://127.0.0.1:<port>` so `send.sh` hits the mock instead of `api.telegram.org`. |

## Key Interfaces / Entry Points

| Entry point                       | Role |
|-----------------------------------|------|
| `action.yml` `runs.steps[0].run`  | GitHub-side bootstrap. Maps each `inputs.*` to an `NF_*` env var and execs `bash scripts/entrypoint.sh`. |
| `scripts/entrypoint.sh`           | Single composable orchestrator. Sources the libraries, asserts deps, masks the token, validates, filters, renders, sends. |
| `nf::validate` (`validate.sh`)    | Input contract — exit codes 10..15 per failure class. |
| `nf::should_notify` (`filter.sh`) | Decides whether to send based on `NF_NOTIFY_ON`. |
| `nf::render` (`render.sh`)        | Picks the template, substitutes placeholders, escapes per `parse_mode`, truncates to 4096 bytes. |
| `nf::send` (`send.sh`)            | Builds JSON via `jq`, posts via `curl`, handles 200 / 429 / 5xx / 4xx / network, writes outputs. |
| `tests/fixtures/mock_server.py`   | Test-only HTTP mock for the Telegram Bot API. |

### Environment contract (`NF_*`)

All cross-script state flows through `NF_*` env vars. The full list is set by `action.yml` from `inputs.*`:

`NF_BOT_TOKEN`, `NF_CHAT_ID`, `NF_STATUS`, `NF_PARSE_MODE`, `NF_NOTIFY_ON`, `NF_MESSAGE`, `NF_MESSAGE_TEMPLATE`, `NF_TEMPLATE_SUCCESS`, `NF_TEMPLATE_FAILURE`, `NF_TEMPLATE_CANCELLED`, `NF_TEMPLATE_SKIPPED`, `NF_DISABLE_WEB_PAGE_PREVIEW`, `NF_DISABLE_NOTIFICATION`, `NF_MESSAGE_THREAD_ID`, `NF_FAIL_ON_ERROR`.

Test-only override: `NF_API_BASE` redirects HTTP calls from `https://api.telegram.org` to the mock.

### Exit codes

| Code | Source        | Meaning |
|------|---------------|---------|
| 0    | any           | Success, or skipped, or `fail_on_error=false` with a send failure. |
| 1    | `entrypoint`  | Send failed and `fail_on_error=true`. |
| 10   | `validate.sh` | `MISSING_REQUIRED_INPUT` (`bot_token` or `chat_id` empty). |
| 11   | `validate.sh` | `INVALID_CHAT_ID`. |
| 12   | `validate.sh` | `INVALID_STATUS`. |
| 13   | `validate.sh` | `INVALID_PARSE_MODE`. |
| 14   | `validate.sh` | `INVALID_NOTIFY_ON`. |
| 15   | `validate.sh` | `INVALID_THREAD_ID`. |
| 20   | `lib.sh`      | `UNSUPPORTED_BASH` (need >= 3.2). |
| 22   | `lib.sh`      | `MISSING_DEPENDENCY` (`curl` or `jq`). |

## Adding New Features

The project uses a spec-driven workflow. Artefacts live under `.spec/features/<feature-name>/`. The v1.0.0 baseline is at `.spec/features/telegram-notify-action/`.

To add or extend functionality, the typical path is:

1. **Spec.** Create `.spec/features/<name>/explore.md` and `requirements.md`. Use the existing REQ-X.Y numbering style.
2. **Design.** Write `design.md` with ADRs and any new components (see the 7 ADRs / 16 component points pattern already in use).
3. **New input?** Add it to `action.yml` (description, `required`, `default`) **and** map it to a new `NF_*` env var in `runs.steps[0].env`. Update the README inputs table.
4. **New script?** Add `scripts/<name>.sh` as a source-only library: shebang-less header `# shellcheck shell=bash`, an `_NF_<NAME>_LOADED` guard, functions prefixed `nf::`, private helpers prefixed `_nf::_`. Source it from `entrypoint.sh` in the correct pipeline position.
5. **New placeholder?** Append the key to `_NF_PLACEHOLDER_KEYS` in `render.sh`, add a `case` arm to `_nf::_placeholder_value`, and document it in the README placeholders table.
6. **Tests.** Add or extend a `tests/<area>.bats` file. Reuse `setup_clean_env`, `mock_telegram_start`, `read_output`, `assert_output_eq` from `helpers.bash`. Use the existing JSON fixtures in `mock_server.py` (extend if needed).
7. **Lint and test.** `make lint && make test` must pass. CI re-runs both on ubuntu-latest and macos-latest.
8. **Changelog.** Append the change to `CHANGELOG.md` under an `Unreleased` heading.
9. **Release (maintainers).** Push a `vX.Y.Z` tag; `release.yml` force-updates the moving `v<major>` tag.
