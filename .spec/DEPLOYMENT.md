<!-- generated: 2026-05-27, template: deployment.md -->

# notiflow Deployment

## 1. Overview

`notiflow` is distributed as a **composite GitHub Action** published on GitHub (and surfaced on the GitHub Marketplace). "Deployment" here means cutting a SemVer git tag — there is no server, no container image, no PaaS, no runtime infrastructure. Consumers reference the action directly from their workflow files via `uses: jtprogru/notiflow@<ref>`.

Release pipeline:

```
local commit on main
        │
        ▼
git tag -a vX.Y.Z -m "..."
        │
        ▼
git push origin vX.Y.Z
        │
        ▼ (push event on tag matching v*.*.*)
.github/workflows/release.yml
        │
        ├── checkout @ tag (fetch-depth: 0)
        └── git tag -fa v<major> -m "Update v<major> to vX.Y.Z"
            git push --force origin refs/tags/v<major>
        │
        ▼
consumers see new v<major> automatically;
v<X.Y.Z> available for pinning
```

## 2. Environments

`notiflow` has no runtime environments of its own. The only "environment" is the GitHub Actions runner on which a consuming workflow executes.

| Environment | Where it runs | Purpose | Trigger | Auto-deploy |
|-------------|---------------|---------|---------|-------------|
| Local dev | Workstation, `bash scripts/entrypoint.sh` with `NF_*` env vars | Manual smoke / debugging | — | — |
| CI (this repo) | `ubuntu-latest` + `macos-latest` GitHub runners | Lint + `bats` test matrix | push to `main`, pull_request | Yes (`.github/workflows/ci.yml:3-7`) |
| Release (this repo) | `ubuntu-latest` GitHub runner | Force-update the moving `v<major>` tag | push of tag matching `v*.*.*` | Yes (`.github/workflows/release.yml:3-5`) |
| Consumer workflow | Any GitHub-hosted or self-hosted runner with `bash`, `curl`, `jq` | Invokes the action via `uses:` | Whatever the consumer schedules | Per consumer |

Reference for self-hosted requirements: `README.md:175-178` ("Requirements").

## 3. Docker

**N/A.** `notiflow` is a pure composite action. No `Dockerfile`, no `docker-compose.yml`, no image registry. Execution model is documented in `action.yml:80-102` (`runs.using: composite`, single step `bash "${{ github.action_path }}/scripts/entrypoint.sh"`).

The decision to avoid Docker is recorded in ADR-1 (`.spec/features/telegram-notify-action/design.md:151-157`): "Composite action вместо Docker / JS" — minimal cold start, multi-OS support, no image build/publish step.

## 4. CI/CD Pipeline

### 4.1 `.github/workflows/ci.yml`

- **File**: `.github/workflows/ci.yml`
- **Trigger** (`ci.yml:3-6`): push to `main`, every pull_request
- **Runner matrix** (`ci.yml:10-14`): `ubuntu-latest` and `macos-latest`, `fail-fast: false`
- **Steps**:
  1. `actions/checkout@v4` (`ci.yml:16`)
  2. Tool install on macOS (`ci.yml:18-20`): `brew install bats-core shellcheck shfmt actionlint jq`
  3. Tool install on Linux (`ci.yml:22-30`): `apt-get install -y bats shellcheck jq`, then download `shfmt v3.7.0` and `actionlint` and `install` them to `/usr/local/bin`
  4. `make lint` (`ci.yml:32-33`) — runs `shellcheck` + `shfmt -d` + `actionlint`
  5. `make test` (`ci.yml:35-36`) — runs the `bats` suite under `tests/`
- **Secrets required**: none. The action itself never needs a secret to lint or test; the Telegram API is mocked by `tests/fixtures/mock_server.py` (see `.spec/README.md:107-110`).

### 4.2 `.github/workflows/release.yml`

- **File**: `.github/workflows/release.yml`
- **Trigger** (`release.yml:3-5`): push of any tag matching `v*.*.*`
- **Runner** (`release.yml:9`): `ubuntu-latest`
- **Permissions** (`release.yml:10-11`): `contents: write` (required to force-push the moving major tag)
- **Steps**:
  1. `actions/checkout@v4` with `fetch-depth: 0` (`release.yml:13-15`) — full history is needed because the next step performs tag operations
  2. "Update moving major tag" (`release.yml:17-28`):
     - Reads the pushed tag from `GITHUB_REF_NAME` (e.g. `v1.2.3`)
     - Computes `major="${tag%%.*}"` → `v1`
     - Configures git identity to `github-actions[bot]`
     - `git tag -fa "${major}" -m "Update ${major} to ${tag}"`
     - `git push --force origin "refs/tags/${major}"`
- **Secrets required**: only the built-in `secrets.GITHUB_TOKEN` (`release.yml:19`), passed as `GH_TOKEN`. No PAT, no third-party token.

## 5. Release Process (Step-by-Step)

The process below is what a maintainer runs to publish a new release.

1. **Land all changes on `main`.** Verify CI is green for the most recent commit (the badge in `README.md:3` links to the CI workflow).
2. **Update `CHANGELOG.md`.** Move the "Unreleased" entries under a new `## [X.Y.Z] — YYYY-MM-DD` heading. Keep the Keep-a-Changelog structure already established in `CHANGELOG.md:1-19`.
3. **Commit and push to `main`.**
   ```bash
   git add CHANGELOG.md
   git commit -m "chore(release): vX.Y.Z"
   git push origin main
   ```
4. **Cut the SemVer tag.**
   ```bash
   git tag -a v1.0.0 -m "v1.0.0"
   git push origin v1.0.0
   ```
   Tag names MUST match the pattern `v*.*.*` (three numeric segments) or `release.yml` will not run — see `release.yml:5`.
5. **Wait for `release.yml` to complete.** It computes the major prefix (`v1.0.0` → `v1`) and force-updates the moving major tag (`release.yml:21-28`). Verify the run on the Actions tab.
6. **(Optional) Publish a GitHub Release.** From the GitHub UI, click "Draft a new release", pick `vX.Y.Z`, paste the matching `CHANGELOG.md` section as release notes. This step is what surfaces the new version in the Marketplace listing.
7. **Verify consumer pin.** From any sandbox repo, `uses: jtprogru/notiflow@v<major>` should now resolve to the new commit; `uses: jtprogru/notiflow@vX.Y.Z` should resolve to the immutable tag.

## 6. Consumer Pinning

There are three ways consumers can reference the action. Choose per consumer policy.

| Ref form | Example | Stability | When to use |
|----------|---------|-----------|-------------|
| Moving major tag | `uses: jtprogru/notiflow@v1` | Receives every non-breaking `v1.x.y` automatically | Default. Matches the project policy in `README.md:189-191`. |
| Immutable SemVer tag | `uses: jtprogru/notiflow@v1.0.0` | Pinned to one release; never moves | Reproducible builds, regulated environments |
| Commit SHA | `uses: jtprogru/notiflow@<40-char-sha>` | Maximally immutable; survives tag rewrites | Supply-chain hardening (e.g. when org policy bans tags) |

The `v<major>` moving-tag contract is recorded in ADR-7 (`.spec/features/telegram-notify-action/design.md:214-222`): SemVer with `v1` as a moving major tag, breaking changes bump to `v2`, `v1` remains operational.

Usage example from `README.md:23-27`:

```yaml
- uses: jtprogru/notiflow@v1
  with:
    bot_token: ${{ secrets.TELEGRAM_BOT_TOKEN }}
    chat_id:   ${{ secrets.TELEGRAM_CHAT_ID }}
    status:    ${{ needs.build.result }}
```

## 7. Rollout Strategy

- **Type**: instantaneous tag-swap. There is no fleet, no replicas, no canary. The moving major tag is force-updated atomically by `release.yml:27-28`.
- **Zero-downtime**: yes by construction — consumers resolve the ref each time their workflow runs; there is no long-lived process to drain.
- **Staggered rollout**: not applicable at the action level. Consumers control their own rollout cadence by choosing between `@v1` (auto-uptake) and `@v1.2.3` (manual uptake).

## 8. Health Checks

**N/A** for the published action — there is no running service. The closest equivalents are:

| Check | Where | Verifies |
|-------|-------|----------|
| `make lint` | `ci.yml:32-33` | `shellcheck`, `shfmt -d -i 2 -ci`, `actionlint` all pass |
| `make test` | `ci.yml:35-36` | The `bats` suite under `tests/` (described in `.spec/README.md:62-72`) |
| Runtime output `ok` | `action.yml:69-72` | Set to `true` after a successful Telegram delivery, else `false` |
| Runtime output `http_status` | `action.yml:76-78` | Last HTTP status from Telegram (`0` for skip / network error) |
| Exit code | `entrypoint.sh` via `.spec/README.md:132-146` | `0` = success or `fail_on_error=false`; `1` = send failure with `fail_on_error=true`; `10..15` = validate failures; `20`/`22` = environment failures |

Consumers can gate downstream steps on `steps.<id>.outputs.ok == 'true'` to assert delivery health.

## 9. Rollback Procedure

A bad release shows up as failing CI in consumer workflows that pinned `@v1`. Roll back by re-pointing the moving major tag at the previous good immutable tag.

1. **Identify the issue.** Inspect the failed consumer run, the action's stderr (`nf::log error` lines), and any `::warning::UNKNOWN_PLACEHOLDER:<name>` annotations.
2. **Pick the last good tag.** From the maintainer's clone:
   ```bash
   git fetch --tags
   git tag --list 'v*.*.*' --sort=-version:refname | head
   ```
3. **Force-update the moving major tag back to the prior version.** Same shape as `release.yml:25-28`, run locally:
   ```bash
   git tag -fa v1 -m "Rollback v1 to v1.0.0" v1.0.0
   git push --force origin refs/tags/v1
   ```
   Consumers pinned to `@v1` immediately get the previous release on their next run.
4. **(If the bad tag is also a problem)** Optionally delete the bad immutable tag, but prefer to **publish a hotfix `vX.Y.(Z+1)`** so the history stays auditable. Deleting an immutable tag breaks any consumer that pinned to it directly.
   ```bash
   # only if absolutely necessary
   git push origin :refs/tags/v1.0.1
   git tag -d v1.0.1
   ```
5. **Verify.** Re-run the impacted consumer workflow; confirm `ok=true` and `http_status=200` (or the expected non-200 for the failure under test).
6. **Post-mortem.** Document the failure mode in `CHANGELOG.md` under the next release, and add a regression test to the relevant `tests/<area>.bats` file. The seven ADRs in `.spec/features/telegram-notify-action/design.md:149-222` are the right place to record any architectural lesson learned.

## 10. Secrets Management

### 10.1 In this repository

| Secret | Purpose | Where set | Required by |
|--------|---------|-----------|-------------|
| `GITHUB_TOKEN` | Force-push the moving major tag | Auto-injected by GitHub for every workflow run | `release.yml:19` |

No third-party secrets, no PATs, no registry credentials. `ci.yml` does not reference any secret.

### 10.2 For consumers

The action requires the consumer to pass two secret-bearing inputs (`action.yml:9-15`):

| Input | Purpose | Where set | Notes |
|-------|---------|-----------|-------|
| `bot_token` | Telegram bot token used for `sendMessage` | Repository or organisation secret in the consumer's repo | First action `notiflow` takes is `::add-mask::<bot_token>` so it never leaks to logs (`README.md:169`, ADR / CP-11 in `design.md:388-394`) |
| `chat_id` | Target chat ID or `@channel_username` | Repository or organisation secret in the consumer's repo | Validated by `nf::validate` (`design.md:107-109`); invalid values exit `11` |

Rotation policy: not documented at the project level — owned by each consumer.

Do not include real tokens, chat IDs, or run URLs in this repo.

## 11. Marketplace Publishing Checklist

GitHub Marketplace listing requirements that this repo already satisfies:

- [x] `action.yml` at repo root (`action.yml:1`)
- [x] `name` is unique on the Marketplace (`action.yml:1` — `"notiflow — Telegram CI Notifier"`)
- [x] `description` present and non-empty (`action.yml:2`)
- [x] `author` set (`action.yml:3`)
- [x] `branding.icon` from the allowed Feather set (`action.yml:6` — `send`)
- [x] `branding.color` from the allowed palette (`action.yml:7` — `blue`)
- [x] Public repository named for the action (`jtprogru/notiflow`)
- [x] `README.md` with Usage / Inputs / Outputs / Examples (`README.md:8-165`)
- [x] `LICENSE` at repo root (`LICENSE`, MIT, referenced in `README.md:193-195`)
- [x] `CHANGELOG.md` in Keep-a-Changelog format (`CHANGELOG.md:1-19`)
- [x] CI badge in README (`README.md:3`)
- [x] SemVer + moving major-tag policy (`README.md:189-191`, ADR-7)

To publish or re-publish on the Marketplace: open the GitHub Release UI for a `vX.Y.Z` tag, tick "Publish this Action to the GitHub Marketplace", pick the primary category, then publish.

## 12. Infrastructure Requirements

**N/A.** No CPU / memory / storage / replica budget — the action runs inside the consumer's runner. The only runtime dependencies are `bash`, `curl`, and `jq` (`README.md:175-178`, also `.spec/README.md:36`).

## 13. Monitoring & Alerts

**N/A.** No metrics, no log aggregator, no on-call rotation. Observability surfaces are:

- The GitHub Actions UI for this repo's `ci.yml` and `release.yml` runs
- The consumer-side run log of any workflow that uses the action
- Action outputs `ok`, `message_id`, `http_status` (`action.yml:69-78`) — consumers can fan these out into their own monitoring if desired
- `::notice::` / `::warning::` / `::error::` annotations emitted by `nf::log` (described in `design.md:99-103`)
