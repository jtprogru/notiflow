<!-- generated: 2026-05-27, template: agents-index.md -->

# AGENTS.md — Instructions for AI Agents

This file tells AI agents (Copilot, Cursor, Windsurf, Claude, etc.) how to navigate and use the `.spec/` documentation directory in this repository.

`notiflow` is a composite GitHub Action that sends a Telegram message when a workflow job completes. It is implemented as Bash 3.2+ scripts plus an `action.yml` manifest, tested with `bats-core`, and released as a moving `v1` major tag.

## 1. What is `.spec/`

`.spec/` is the project documentation directory optimized for AI agent (LLM) consumption. It contains:

- Structured descriptions of architecture, modules, and the action contract.
- Code and testing conventions (Bash 3.2+, `bats-core`, `shellcheck`, `shfmt`, `actionlint`).
- Tooling and CI/release pipeline descriptions.
- Per-feature spec-driven pipelines under `.spec/features/<feature-slug>/` containing `explore.md`, `requirements.md`, `design.md`, `task-plan.md`, `implementation.md`, `review.md`.
- Agent rules (`agent-rules.md`) — when present, must be followed verbatim.

Purpose: give the agent full project context (requirements, ADRs, correctness properties, test plan) without reading the entire source tree.

## 2. How to Use (for agents)

- **Starting work on the project** — read `.spec/README.md` first (if present) for the documentation map. If it does not exist, fall back to listing `.spec/features/` and reading the most recent feature's `design.md` plus root `README.md` and `action.yml`.
- **Before modifying code** — read the relevant document from `.spec/` for the area you touch:
  - Touching `action.yml` or `scripts/entrypoint.sh` → read `.spec/features/telegram-notify-action/design.md` §2.2–2.3 and `requirements.md`.
  - Touching templating / escaping (`scripts/render.sh`, `scripts/escape.sh`) → read `design.md` §2.4 (ADR-2, ADR-6) and Correctness Properties CP-4, CP-5, CP-6.
  - Touching HTTP / retry (`scripts/send.sh`) → read `design.md` §2.4 (ADR-3) and CP-8, CP-9, CP-10.
  - Touching tests → read `design.md` §2.8 Testing Strategy.
- **Always follow** the rules in `.spec/agent-rules.md` if present. In its absence, follow the conventions captured in this file's §6.
- **If a document appears outdated** (e.g. design references files that no longer exist, or `README.md` inputs table disagrees with `action.yml`) — flag it and propose an update; do not silently rewrite code to match stale docs.

## 3. File Structure Convention

`.spec/` layout used in this repository:

```
.spec/
├── README.md                          # index of all documents (Quick Facts, structure, commands) — create when adding broader docs
├── agent-rules.md                     # mandatory rules for the agent (code style, naming, error handling) — create as needed
├── ARCHITECTURE.md, TESTING.md, …     # topical documents in UPPER_CASE.md
└── features/
    └── <feature-slug>/
        ├── explore.md                 # exploration notes, options considered
        ├── requirements.md            # WHEN/SHALL requirements (REQ-N.M)
        ├── design.md                  # architecture, ADRs, correctness properties (CP-N), test plan
        ├── task-plan.md               # ordered task breakdown
        ├── implementation.md          # implementation log
        ├── review.md                  # post-implementation review
        ├── pipeline.json / .kv        # spec-driven-dev pipeline state
        ├── approved/                  # human-approval artifacts per phase
        └── revisions/                 # revision history
```

Additional conventions that may appear in `.spec/`:

- `skills/` — directory for project-local agent skills (custom scripts).
- `workflows/` — directory for project-local agent workflows.
- `prompts/` — prompts for (re)generating documentation.

## 4. Document Categories

For this project the relevant categories are:

- **Core**: Architecture (`action.yml` + `scripts/entrypoint.sh` wiring), Modules (`lib`, `validate`, `filter`, `escape`, `render`, `send`), Action contract (inputs/outputs), Code Style (`shellcheck`, `shfmt -i 2 -ci`).
- **Development**: Tools (`bats-core`, `shellcheck`, `shfmt`, `actionlint`, `jq`), Testing (unit tests under `tests/*.bats`, fixtures under `tests/fixtures/`), Fixtures and mocks (local HTTP mock for Telegram).
- **Auth & Security**: Telegram bot token handling (`::add-mask::`, no logs), secrets only via `secrets.*` in workflows.
- **Infrastructure**: CI matrix (`.github/workflows/ci.yml`: `ubuntu-latest`, `macos-latest`), Release flow (`.github/workflows/release.yml`: moving `v1` tag), Retry/backoff policy.
- **Clients**: Not applicable — this project has no client apps. It is consumed by other repositories' workflows via `uses: jtprogru/notiflow@v1`.

Omit any category that does not apply when generating further docs.

## 5. How to Maintain

Rules for keeping documentation current:

- **New input or output** in `action.yml` → update the inputs/outputs tables in root `README.md` and the corresponding section in `.spec/features/<feature>/design.md` (Files Requiring Changes + ENV contract table).
- **New script** under `scripts/` → update the architecture diagram and Interfaces section in the active feature's `design.md`, and add unit tests under `tests/<script>.bats`.
- **New placeholder** in templates → update `README.md` placeholder table, `scripts/render.sh` placeholder map, and a unit test in `tests/render.bats`.
- **Behavior change** (e.g. retry limits, truncation length, exit codes) → update the affected Correctness Property in `design.md` §2.6 and the matching test under `tests/`.
- **New dependency** (binary used by scripts) → update `Makefile` `install-tools` target, `README.md` Requirements section, and `nf::require_command` calls in `scripts/lib.sh`.
- `.spec/README.md` (when present) must always reflect the current list of documents.
- Documents must contain code examples from this project's actual source (`scripts/`, `action.yml`, `tests/`) — not invented snippets.

## 6. How to Add a New Document

Steps for adding a new topical document to `.spec/`:

1. Create the file as `.spec/TOPIC_NAME.md` (UPPER_CASE).
2. Use this structure:
   1. Title.
   2. Short overview (1–3 sentences).
   3. ASCII or Mermaid architectural diagram if relevant.
   4. Details / sections.
   5. Code examples taken from the actual project (`scripts/*.sh`, `action.yml`, `tests/*.bats`).
   6. Configuration (env vars, action inputs).
   7. Testing notes (which `bats` files cover it).
   8. Key files (absolute or repo-relative paths).
3. Add a link in `.spec/README.md` under the appropriate category from §4.
4. Use real code from the repository — never abstract or invented examples.

## 7. Project Quick Facts

| Item | Value |
|------|-------|
| Type | Composite GitHub Action |
| Languages | Bash 3.2+, YAML |
| Entry point | `scripts/entrypoint.sh` (invoked from `action.yml` `runs.using: composite`) |
| Modules | `scripts/lib.sh`, `validate.sh`, `filter.sh`, `escape.sh`, `render.sh`, `send.sh` |
| Tests | `bats-core` under `tests/*.bats`, fixtures in `tests/fixtures/` |
| Lint | `shellcheck`, `shfmt -i 2 -ci`, `actionlint` |
| CI | `.github/workflows/ci.yml` (matrix: `ubuntu-latest`, `macos-latest`) |
| Release | `.github/workflows/release.yml` — moves `v1` tag on each `v1.x.y` push |
| External deps at runtime | `bash`, `curl`, `jq` |
| Default `fail_on_error` | `false` (notification is a side-channel) |
| Telegram Bot API | `https://api.telegram.org/bot<TOKEN>/sendMessage` |

## 8. Common Commands

```bash
make install-tools    # bats-core, shellcheck, shfmt, actionlint, jq
make lint             # shellcheck + shfmt -d + actionlint
make lint-fix         # shfmt -w
make test             # bats tests/
make build            # no-op (composite action — nothing to build)
```

## 9. Key Files

- `/Users/jtprogru/Work/github/jtprogru/notiflow/action.yml` — action manifest (inputs, outputs, composite runs).
- `/Users/jtprogru/Work/github/jtprogru/notiflow/scripts/entrypoint.sh` — main entry point invoked by `action.yml`.
- `/Users/jtprogru/Work/github/jtprogru/notiflow/scripts/lib.sh` — shared helpers (`nf::log`, `nf::mask`, `nf::set_output`, `nf::json_escape`, `nf::require_command`).
- `/Users/jtprogru/Work/github/jtprogru/notiflow/scripts/validate.sh` — input validation, exit codes 10–15.
- `/Users/jtprogru/Work/github/jtprogru/notiflow/scripts/filter.sh` — `nf::should_notify` (status vs `notify_on`).
- `/Users/jtprogru/Work/github/jtprogru/notiflow/scripts/escape.sh` — `nf::escape_md_v2`, `nf::escape_html`, `nf::escape_none`.
- `/Users/jtprogru/Work/github/jtprogru/notiflow/scripts/render.sh` — template selection + placeholder substitution + truncation.
- `/Users/jtprogru/Work/github/jtprogru/notiflow/scripts/send.sh` — HTTP POST to Telegram with retry (429 honors `parameters.retry_after`; 5xx uses 1s/2s/4s exponential backoff; max 3 attempts).
- `/Users/jtprogru/Work/github/jtprogru/notiflow/Makefile` — `lint`, `lint-fix`, `test`, `build`, `install-tools`.
- `/Users/jtprogru/Work/github/jtprogru/notiflow/README.md` — public docs (inputs/outputs tables, examples, behavior notes).
- `/Users/jtprogru/Work/github/jtprogru/notiflow/.github/workflows/ci.yml` — CI matrix.
- `/Users/jtprogru/Work/github/jtprogru/notiflow/.github/workflows/release.yml` — moves `v1` tag.
- `/Users/jtprogru/Work/github/jtprogru/notiflow/.spec/features/telegram-notify-action/design.md` — authoritative architecture, ADRs (1–7), Correctness Properties (CP-1..16), Testing Strategy.
- `/Users/jtprogru/Work/github/jtprogru/notiflow/.spec/features/telegram-notify-action/requirements.md` — 37 REQ items (WHEN/SHALL).
