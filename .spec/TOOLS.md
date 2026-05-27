<!-- generated: 2026-05-27, template: development.md -->

# TOOLS

Reference of commands and project tools for `notiflow` — a composite GitHub Action implemented as bash 3.2+ scripts.

For the high-level project map and where each file lives, see `.spec/README.md` and `.spec/PACKAGES.md` (do not duplicate here).

---

## 0. Dev Environment Setup

### Prerequisites

| Tool         | Version  | Install (macOS / Linux)                                                                 |
|--------------|----------|-----------------------------------------------------------------------------------------|
| `bash`       | 3.2+     | macOS system bash works. Linux: pre-installed.                                          |
| `curl`       | any      | macOS: pre-installed. Linux: `sudo apt-get install -y curl`.                            |
| `jq`         | any      | `brew install jq` / `sudo apt-get install -y jq`.                                       |
| `bats-core`  | 1.x      | `brew install bats-core` / `sudo apt-get install -y bats`.                              |
| `shellcheck` | any      | `brew install shellcheck` / `sudo apt-get install -y shellcheck`.                       |
| `shfmt`      | 3.7+     | `brew install shfmt` / download from `github.com/mvdan/sh/releases`.                    |
| `actionlint` | any      | `brew install actionlint` / `download-actionlint.bash` script. Optional locally, required in CI. |
| `python3`    | 3.x      | macOS: pre-installed. Used only by `tests/fixtures/mock_server.py` (stdlib only).        |

Authoritative source: `Makefile:43-52` (`install-tools` target) and `.github/workflows/ci.yml:18-30`.

No `go.mod`, `package.json`, `.tool-versions`, or `Dockerfile` exists — runtime stack is entirely the host shell.

### First Run

From a fresh clone to a passing test suite:

1. **Clone** — `git clone git@github.com:jtprogru/notiflow.git && cd notiflow`.
2. **Install tools** — `make install-tools` (Homebrew on macOS, apt on Linux; see `Makefile:43-52`).
3. **Lint** — `make lint`.
4. **Test** — `make test` (runs `bats tests/`, expects 69 passing tests).
5. **(Optional) Local one-off run** — bypass GitHub entirely:
   ```bash
   NF_BOT_TOKEN=... NF_CHAT_ID=... NF_STATUS=success \
   GITHUB_REPOSITORY=jtprogru/notiflow GITHUB_RUN_ID=1 GITHUB_WORKFLOW=CI \
   bash scripts/entrypoint.sh
   ```

There is no `init` / `setup` Makefile target — `make install-tools` is the closest equivalent. No `.env.example` is needed; configuration is passed through `NF_*` env vars (full list in `.spec/README.md` § Environment contract).

---

## 1. Overview

All developer commands are exposed through the `Makefile` at the repository root. Use `make <target>` from the project root. `make help` (the default goal) prints the documented targets.

```bash
make           # equivalent to `make help`
make help      # list targets
```

The Makefile uses `SHELL := /usr/bin/env bash` (`Makefile:2`) so bashisms work even on systems where `/bin/sh` is dash.

---

## 2. Quick Reference

| Action              | Command              | Source              |
|---------------------|----------------------|---------------------|
| Show available targets | `make help`        | `Makefile:7-10`     |
| Lint everything     | `make lint`          | `Makefile:12-22`    |
| Auto-format shell   | `make lint-fix`      | `Makefile:24-26`    |
| Run full test suite | `make test`          | `Makefile:28-32`    |
| Build               | `make build` (no-op) | `Makefile:34-36`    |
| Install dev tools   | `make install-tools` | `Makefile:42-52`    |
| Regenerate README   | `make readme` (stub) | `Makefile:38-40`    |

CI invokes only `make lint` and `make test` (see `.github/workflows/ci.yml:32-36`).

---

## 3. Detailed Command Groups

### 3.1 Linting

```bash
make lint
```

What it runs (`Makefile:12-22`):
- `shellcheck -x scripts/*.sh tests/*.bash` — static analysis with `-x` so sourced libraries are followed.
- `shfmt -d -i 2 -ci scripts tests` — formatting diff (2-space indent, `case` indented). Exits non-zero on any drift.
- `actionlint` — workflow linter. **Optional locally** (the rule emits a warning and continues if missing); **mandatory in CI**.

Pre-flight (`Makefile:14-15`) checks that `shellcheck` and `shfmt` exist on `$PATH` and tells the user to run `make install-tools` if not.

Dependencies: `shellcheck`, `shfmt`, optionally `actionlint`.

### 3.2 Formatting

```bash
make lint-fix
```

Runs `shfmt -w -i 2 -ci scripts tests` (`Makefile:24-26`) — writes the formatted output back to disk. Use this before committing to silence the `shfmt -d` diff in `make lint`.

Dependencies: `shfmt`.

### 3.3 Testing

```bash
make test
```

Runs `bats tests/` (`Makefile:28-32`) — the full bats-core suite (7 files, 69 tests).

Pre-flight (`Makefile:30-31`) checks for `bats` and `jq` (the latter is consumed by send-path tests for assertions against the captured request body).

Dependencies: `bats-core`, `jq`, `python3` (transitively — the mock server in `tests/fixtures/mock_server.py` is launched by `mock_telegram_start` from `tests/helpers.bash:34-60`).

See `.spec/TESTING.md` for everything about how the tests are structured.

### 3.4 Build

```bash
make build
```

No-op (`Makefile:34-36`): a composite action has no compile step. Kept so any meta-tooling that runs `make build` succeeds.

### 3.5 README regeneration

```bash
make readme
```

Placeholder (`Makefile:38-40`): prints `TODO: not implemented in v1`. Reserved for a future generator that re-derives the README inputs/outputs table from `action.yml`.

### 3.6 Tool bootstrap

```bash
make install-tools
```

Branches by OS (`Makefile:42-52`):
- macOS: `brew install bats-core shellcheck shfmt actionlint jq`.
- Linux (`apt-get` present): installs `bats`, `shellcheck`, `jq` from apt; downloads `shfmt` v3.7.0 binary into `/usr/local/bin/`; installs `actionlint` via its upstream `download-actionlint.bash` script.
- Other: prints a manual-install message and exits 1.

---

## 4. Code Generation

**None.** This project has no code-generation step:
- No proto / OpenAPI definitions.
- No mock generators (`mock_server.py` is hand-written stdlib HTTP, not generated; see `tests/fixtures/mock_server.py:1-122`).
- No `go:generate` directives or stringer-style tools.
- `make readme` is reserved but not implemented in v1 (`Makefile:38-40`).

If a future iteration adds OpenAPI-driven docs for the placeholders table, this section is the place to record the command and source-of-truth file.

---

## 5. CI/CD Cheatsheet

The CI pipeline (`.github/workflows/ci.yml`) is two steps wide. To reproduce it locally:

```bash
# Match what CI's `Install tools` step does (Makefile:42-52)
make install-tools

# 1:1 with .github/workflows/ci.yml:32-33
make lint

# 1:1 with .github/workflows/ci.yml:35-36
make test
```

The CI matrix runs on both `ubuntu-latest` and `macos-latest` (`.github/workflows/ci.yml:10-14`) with `fail-fast: false`, so a local Linux developer should also spot-check on macOS (bash 3.2, BSD sed) before merging — `lib.sh` and `render.sh` contain explicit BSD-portable workarounds.

Release flow (`.github/workflows/release.yml`): pushing a `vMAJOR.MINOR.PATCH` tag triggers `update-major-tag`, which force-pushes the moving major tag (e.g. `v1`) to the current commit (`release.yml:8-28`). Nothing to simulate locally; verify by inspecting the workflow file.

---

## 6. Tool Installation

| Tool          | macOS                              | Linux (apt-based)                                                                                  |
|---------------|------------------------------------|----------------------------------------------------------------------------------------------------|
| `bats-core`   | `brew install bats-core`           | `sudo apt-get install -y bats`                                                                     |
| `shellcheck`  | `brew install shellcheck`          | `sudo apt-get install -y shellcheck`                                                               |
| `shfmt`       | `brew install shfmt`               | `curl -sSfL -o /tmp/shfmt https://github.com/mvdan/sh/releases/download/v3.7.0/shfmt_v3.7.0_linux_amd64 && sudo install -m 0755 /tmp/shfmt /usr/local/bin/shfmt` |
| `actionlint`  | `brew install actionlint`          | `bash <(curl -sSfL https://raw.githubusercontent.com/rhysd/actionlint/main/scripts/download-actionlint.bash) && sudo install -m 0755 ./actionlint /usr/local/bin/actionlint` |
| `jq`          | `brew install jq`                  | `sudo apt-get install -y jq`                                                                       |
| `python3`     | pre-installed                      | pre-installed on Ubuntu; `sudo apt-get install -y python3` otherwise                               |

The single command `make install-tools` performs all of the above for the detected OS (`Makefile:42-52`, mirrored by `.github/workflows/ci.yml:18-30`).
