# Changelog

All notable changes are documented in this file. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

Release-readiness work for `2.0.0`. Every item here is a path that had never executed: the code was ready before the pipeline that ships it was.

### Fixed

- The Homebrew job no longer assumes a GPG key exists. It imported one unconditionally and committed with `-S`, unlike `publish`, which grew a guard in `2.0.0-alpha.2`. Without the key the formula update dies and `continue-on-error` swallows it, so `brew install jtprogru/tap/notiflow` — the first install line in the README — would have kept pointing at nothing while the release reported success.
- The Action refuses to resolve a version below `v2.0.0`. `releases/latest` skips pre-releases, so until the stable tag exists it answers with `v1.6.1`, a bash release with no binary archive in it, and `install.sh` failed on a bare 404 several steps away from the input that caused it.

### Added

- An end-to-end CI job that runs the composite action itself (`uses: ./`) on Linux, macOS and Windows against a published release, with `verify_signature: true` and a second run over a warm tool cache. Version resolution, platform detection, the cache, the download, the checksum, the cosign verification and the `$GITHUB_PATH` step had no coverage at all: `action-smoke` drives `scripts/action/run.sh` with a locally built binary and skips every one of them, so that half of the Action only ever ran in other people's workflows.
- Acceptance tests for `resolve-version.sh` covering the whole precedence chain, including the two branches that decide what gets downloaded when nothing is pinned.
- The GPG public key is published at <https://jtprogru.github.io/notiflow/notiflow-signing-key.asc> and its fingerprint is in the installation docs. Both locales documented `gpg --verify` without ever saying where the key comes from, which leaves the reader with a `no public key` error and nothing to compare against.

### Changed

- `cargo publish` runs after the GitHub release rather than beside it, and skips a version already on crates.io. A crate version cannot be unpublished, so it belongs last, and a re-run of a partially failed release should not go red on the one step that did succeed.
- The Homebrew job runs on pre-release tags too, generating the formula and stopping before the push. The tap checkout, the token, the GPG import and the rendered formula are now something a release candidate proves rather than something the stable tag attempts for the first time.
- `lib.rs` states what semver covers: the CLI and the Action. The library is `pub` for the binary's and the tests' benefit, has no external consumer, and its items can change in a minor release.
- The README no longer promises GPG signatures unconditionally, matching the correction already made to the installation page.

## [2.0.0-alpha.2] — 2026-08-11

Same code as `2.0.0-alpha.1`, which never produced a GitHub Release: the release workflow built and signed every artefact, published the crate, and then aborted importing a GPG key this repository does not hold. `2.0.0-alpha.1` exists on crates.io and nowhere else.

### Fixed

- The release no longer depends on GPG. Importing and signing are conditional and log a `::warning::` when skipped; keyless cosign signatures and the SLSA provenance attestation are unconditional and unaffected. The release file list uses globs so an absent `.asc` is not a missing-file failure.
- The Homebrew job is `continue-on-error`. The tap is a separate repository, and a token problem there should not retroactively fail a release whose artefacts are already published.
- `make dist` and the Action's `install.sh` no longer assume `zip` and `unzip` exist. Neither ships with git-bash, which is the bash a Windows runner provides; both now fall back to 7-Zip and then PowerShell.
- The test mock server no longer stalls in `server_bind`. `http.server` calls `socket.getfqdn()` on the address it binds, and a runner that cannot answer a reverse lookup for `127.0.0.1` blocks there until the DNS timeout — the socket is listening, but the port file appears seconds later and a caller waiting on it concludes the server never started. Observed on `macos-latest`, where the wrapper smoke test failed while the Linux and Windows runners passed; almost certainly the same thing 1.6.2 was working around when it raised the bats startup wait from 5s to 20s. The lookup is now skipped rather than waited out.
- `make msrv` runs through `rustup run` rather than `cargo +<version>`, which only works when the rustup shim is first on `PATH`.
- `make release-prep` no longer silently fails to bump the version. It edited `Cargo.toml` with sed's `0,/re/` address — a GNU extension that BSD sed ignores — so on a BSD userland, macOS included, it printed `stamped <version>` while leaving the file untouched. It now performs the edit portably and calls `version-check` on itself before reporting success.
- `time` updated past RUSTSEC-2026-0009. The crate is never compiled — it arrives through a `ureq` feature that is off — but `Cargo.lock` records optional dependencies regardless, so `cargo audit` sees it even though `cargo deny` does not.
- The docs site moved to Astro 7 and sharp 0.35, clearing ten Dependabot alerts, and the landing page no longer titles itself `notiflow | notiflow`.

### Changed

- The installation docs are explicit about which signatures are guaranteed: keyless cosign and the SLSA provenance attestation always, GPG on releases built after a signing key was configured for the workflow. `2.0.0-alpha.1` and `2.0.0-alpha.2` predate that key and carry no `.asc` files.

## [2.0.0-alpha.1] — 2026-08-10

notiflow is now a Rust binary. The GitHub Action downloads and runs it; the same binary is also a standalone CLI. The bash implementation is frozen on the `v1.x` branch and kept in this tree under `tests/parity/v1/` as the reference the parity suite compares against.

For a workflow that sends to one chat, `jtprogru/notiflow@v1` → `@v2` is a drop-in change. See the [migration guide](https://jtprogru.github.io/notiflow/action/migration/).

### Removed

- **Breaking.** Multi-chat fan-out. `chat_id` accepts exactly one chat; a comma exits `11`. `edit_message_id` accepts one integer. The `message_id` output is a scalar rather than a CSV with empty slots, `error` is the raw reason without a `chat <id>:` prefix, and `http_status` is the status of the one request. Use a job matrix or repeated steps — both give per-chat outputs and per-chat status, which the aggregated CSV could not. Tracked as [a feature request](https://github.com/jtprogru/notiflow/issues/4) rather than deleted from memory.
- The bash runtime, and with it the dependency on `bash`, `curl`, `jq`, `iconv`, `mktemp` and `python3`. Exit codes `20` (`UNSUPPORTED_BASH`) and `22` (`MISSING_DEPENDENCY`) are permanently reserved and never emitted.

### Added

- A CLI: `notiflow send | edit | render | whoami | completions`, from Homebrew, crates.io, or a release archive. Reads stdin, so it works at the end of a pipeline.
- A config file at `~/.config/notiflow/config.toml` with named profiles (`--profile work`), and a permissions warning when a file holding a token is readable beyond its owner.
- `notiflow render` — render a template locally with no token and no network, with `--explain` reporting which template won, whether truncation happened, and which placeholders were unknown.
- `notiflow whoami` — check a token against `getMe`.
- `--dry-run`, which prints the exact JSON body that would have been posted.
- Placeholders resolve outside GitHub Actions: `Repo`, `Branch`, `Ref`, `Sha`, `ShortSha` and `Actor` come from the local git checkout when the `GITHUB_*` variables are absent.
- Windows runners are supported.
- `verify_signature` input for a cosign check on the downloaded binary; releases carry keyless cosign signatures, GPG signatures and SLSA build-provenance attestations alongside `checksums.txt`.
- A `version` input, plus a `VERSION` file stamped at release time so `@v2`, `@v2.1.0` and `@<sha>` all resolve deterministically without a network call.
- Exit codes `17` (`CONFIG_ERROR`), `18` (`INVALID_ARGUMENT`) and `30` (`IO_ERROR`).
- A documentation site at <https://jtprogru.github.io/notiflow/>, English with a Russian locale. The Action's input and output tables, the CLI reference, and the exit-code and placeholder tables are generated from the code; CI fails when the committed copies drift.
- A golden parity corpus (`make parity`) that runs 52 cases through both implementations and diffs them. A v2 result that differs from v1 without a declared reason fails the build.

### Fixed

- **Placeholders are substituted in one pass.** v1 substituted key by key in a loop, so a value inserted early was rescanned by every later iteration; a workflow named `{{.Actor}}` really did expand under `parse_mode: none` and `HTML`. MarkdownV2 escaping hid it by accident.
- **Truncation cuts on markup boundaries.** v1 cut on a raw UTF-16 code-unit boundary and could sever a MarkdownV2 escape pair, an HTML entity or a tag, producing a `400` on a message whose only fault was length. v2 cuts between markup tokens and closes HTML tags left open by the cut.
- **Long messages ending mid-emoji no longer abort.** `iconv` exits non-zero on a trailing incomplete character even with `-c` — BSD and GNU alike — so v1's truncation died inside `_nf::_truncate` and, under `set -e`, took the whole send with it. Any message over 4096 UTF-16 units whose cut landed inside a surrogate pair was affected, on every platform. Found by the parity corpus, not by the plan.
- **The token is scrubbed from every stream.** `::add-mask::` covers the workflow log but not the CLI, and an HTTP error can carry a URL containing `bot<TOKEN>`. Everything notiflow prints is filtered, including any `/bot<token>/` path segment for a token it was never told about.
- **`Retry-After` is honoured.** v1 read only `parameters.retry_after` from the body; proxies and self-hosted Bot API servers send the header instead.
- **Backoff has jitter** (up to 50%), so a fleet of jobs rate-limited together stops retrying in lockstep and re-triggering the limit.
- `--bot-token` on the command line now warns: arguments are readable by every process on the machine.

### Changed

- `disable_web_page_preview` keeps its input name but is sent as `link_preview_options.is_disabled`; Telegram deprecated the flat field.
- `api_base` policy depends on the mode. Inside Actions the allowlist is unchanged (`api.telegram.org` plus loopback). From the CLI, `--api-base` accepts any `http(s)` URL — a self-hosted Bot API server is a legitimate setup and the value is the user's own explicit choice.
- `RunUrl` renders empty outside Actions instead of the half-built `//actions/runs/` string v1 produced.
- Every CI step calls a `make` target, so a red build is reproducible locally with one command.
- `.spec/` was removed. It described the bash implementation and would have started lying on day one; its durable content lives in the architecture page and in the test corpus.

## [1.6.2] — 2026-08-10

### Build

- CI and release workflows bumped to `actions/checkout@v7` (#2). Runtime is untouched: `action.yml` and `scripts/` are byte-identical to v1.6.1, so this release only re-pins the moving `v1` tag.

### Tests

- `mock_telegram_start` now captures the mock server's stderr and prints it (plus the resolved `python3` path and version, and whether the process is still alive) when startup fails. The previous `mock server failed to start` said nothing about *why*, which made the macOS CI failure after `macos-latest` moved from `macos-15-arm64` to `macos-26-arm64` undiagnosable from the logs.
- Startup wait raised from 5s to 20s, and the loop now breaks immediately if the server process is gone, so a genuinely dead mock fails fast while a slow cold-start runner is no longer a false failure.
- The port file is truncated before each start. A stale file from a previous test would have ended the wait loop instantly and pointed the client at a dead port.

## [1.6.1] — 2026-05-27

### Fixed

- **Critical:** `action.yml` shipped in v1.6.0 contained a literal `${{ steps.send.outputs.message_id }}` example inside the `edit_message_id` description. GitHub evaluates `${{ ... }}` in `description:` fields at manifest-load time (the same trap that broke v1.0.0 with `${{ job.status }}`), so every workflow pulling `jtprogru/notiflow@v1` aborted with `Unrecognized named-value: 'steps'` before any step ran. Rewrote the description without the inline expression.
- `tests/manifest.bats`: new test scans every `description:` block (folded or block scalar) and forbids any `${{` inside. The previous per-context tests (`job`/`needs`/`secrets`/`matrix`/`vars`) didn't cover `steps.*`, and a name-specific guard would have false-positived on the legitimate `${{ steps.notiflow.outputs.X }}` in `outputs.value`. This generic guard catches the whole bug class.

## [1.6.0] — 2026-05-27

### Added

- New `edit_message_id` input. When set, the action calls `editMessageText` instead of `sendMessage`, so a workflow can update the same Telegram message across steps (start → progress → done) rather than spamming new ones.
- Multi-chat edit: `edit_message_id` accepts a CSV whose length must match `chat_id`. Each `(chat_id[i], edit_message_id[i])` pair targets exactly one message. Wire `edit_message_id: ${{ steps.send.outputs.message_id }}` from a previous multi-chat send and the shapes line up automatically.
- `disable_notification` and `message_thread_id` are silently dropped from the request body in edit mode — Telegram rejects them on `editMessageText`. `parse_mode` and `disable_web_page_preview` are kept (both accepted).
- New exit code `16 INVALID_EDIT_MESSAGE_ID` for bad input (non-positive-integer item, CSV length mismatch).

### Tests

- `+10` cases: five validate (single + CSV happy paths, length mismatch, non-integer, negative integer) and five send (endpoint switches to `editMessageText`, edit JSON body shape, regression that send mode still excludes `message_id`, multi-chat pairing, edit failure surfaces error like send).

## [1.5.0] — 2026-05-27

### Added

- `chat_id` now accepts a comma-separated list of destinations and fans the message out to each. Sequential execution; each chat gets its own retry chain (timeouts, 429 cap, 5xx backoff — same policy as before, applied independently per chat). Whitespace around CSV items is tolerated; validation runs per item with the existing rules (integer, negative integer, or `@username`).
- Aggregated outputs for multi-chat:
  - `ok=true` only if **every** chat succeeded.
  - `message_id` is a CSV of message ids in input order; failed chats leave an empty slot (e.g. `42,,103`).
  - `http_status` reports the first non-200 status seen (or 200 if all succeeded).
  - `error` enumerates failed chats as `chat <id>: <reason>` joined with `; `.
- Single-chat behavior is preserved bit-for-bit: `message_id="42"`, raw `error` (no `chat 42:` prefix), `http_status=<code>` exactly as before. Existing workflows do not need changes.

### Tests

- `+9` cases: four for CSV validation (valid mix, invalid item, empty item, whitespace) and five for send (all-success multi-chat, partial failure with empty slot, per-chat retry chains, per-chat JSON body, and a regression guard ensuring single-chat error stays unprefixed).

## [1.4.0] — 2026-05-27

### Added

- New `error` output. Holds Telegram's `.description` (e.g. `Bad Request: chat not found`, `Unauthorized`) on 4xx; a synthesized `HTTP <code>: <description>` after exhausted 5xx retries; `network error (curl exit N)` after exhausted network-error retries; empty on success and on skip. Pairs naturally with `ok=false` for downstream debug/branching.
- `notify_on` accepts `any` (alias `all`) as a shortcut for `success,failure,cancelled,skipped`. Saves users from typing the full CSV when they want every status.

### Fixed

- `scripts/render.sh`: `shopt -u patsub_replacement` at load time disables the bash 5.2+ behavior where `&` in a `${var//pat/repl}` replacement expands to the matched pattern. Without this, placeholder values containing `&` or `\` (e.g. branch names, repo names) were corrupted on ubuntu-latest (bash 5.2+) — the v1.3.2 substitution refactor regressed here. Silently ignored on bash < 5.2.

### Tests

- `+7` cases: notify_on shortcut (`any`/`all`), notify_on=any with skipped reaching the wire, and four error-output scenarios (200 clears it, 400/401 surface `.description`, 5xx exhausted surfaces `HTTP <code>: <description>`).

## [1.3.2] — 2026-05-27

### Fixed

- `scripts/render.sh`: placeholder substitution is now done with bash parameter expansion (`${var//pat/repl}`) instead of `sed s///`. The old path piped each value through `tr -d '\n' | sed -e 's/[\\&/]/\\&/g'`, which silently stripped newlines from placeholder values (e.g. a multi-line workflow name) and required a fragile RHS-escape dance for `/`, `&`, and `\`. The new path is faster (no `sed` fork per placeholder, ×16 per render) and preserves arbitrary value content literally. Dead helper `_nf::_sed_rhs_escape` removed.

### Security

- `scripts/send.sh`: new `_nf::_resolve_api_base` allowlist guards the Telegram API base URL. Only `https://api.telegram.org` or a loopback URL (`http://127.0.0.1:*` / `http://localhost:*`, for the test mock) is honored; any other value of `NF_API_BASE` — including userinfo tricks (`http://127.0.0.1@evil.com`), look-alike subdomains (`https://api.telegram.org.evil.com`), and protocol downgrades (`http://api.telegram.org`) — is rejected with a `::warning::` and replaced by the default. Closes the channel through which an earlier malicious step in the same workflow job could have redirected the bot token to an attacker-controlled host.

### Added

- `tests/render.bats`: newline preservation and literal-value (sed metachar) tests.
- `tests/send.bats`: API-base allowlist parameter tests covering the accept path and four classes of exfiltration attempt.

## [1.3.1] — 2026-05-27

### Fixed

- `scripts/entrypoint.sh`: `nf::mask` for the bot token now runs **before** the `nf::require_command` checks for `curl`/`jq`/`iconv`. Defensive against forks that enable `set -x` or any future change that might log argv containing the token before the masking directive lands.
- `scripts/lib.sh`: `nf::require_bash` now actually checks bash `>= 3.2`; previously `>= 3.0` would have passed (bash 3.0/3.1 lack array features we rely on). Refactored into `_nf::_bash_version_ok` so the comparison is unit-testable without forging `BASH_VERSINFO`.
- `scripts/lib.sh`: `nf::set_output` falls back to `/dev/stderr` (not `/dev/stdout`) when `GITHUB_OUTPUT` is unset. Stops local runs from interleaving key=value lines with the rendered message body on stdout.
- `scripts/validate.sh`: legacy `parse_mode: Markdown` is now explicitly upgraded to `MarkdownV2` with a `::warning::` annotation. Previously the escaper silently treated `Markdown` as V2 but the request still sent `parse_mode=Markdown` to Telegram — escape rules and parse_mode mismatched, producing broken or rejected messages. The dead branch in `_nf::_escape_value` is removed.
- `tests/`: four new cases — static mask-order invariant, bash 3.2 boundary check, set_output stderr fallback, and Markdown→MarkdownV2 upgrade with warning annotation.

## [1.3.0] — 2026-05-27

### Fixed

- `scripts/render.sh`: message truncation now counts Telegram's actual unit — UTF-16 code units — instead of bash codepoints. Previously, supplementary-plane characters (most emoji) were undercounted by 2×, so emoji-heavy templates near the limit were silently delivered as `400 MESSAGE_TOO_LONG`. Truncation cuts at codepoint boundaries; a trailing unpaired surrogate from the byte-level cut is dropped via `iconv -c`, so no half-emoji ever leaks into the wire.

### Added

- `iconv` is now a required dependency. Hard-fails with `MISSING_DEPENDENCY:iconv` (exit 22, same path as `curl`/`jq`) if absent. GitHub-hosted runners already ship it on all three OSes; self-hosted runners need it on `PATH`.
- `_nf::_utf16_units` helper, exposed for tests, counts UTF-16 code units of arbitrary UTF-8 input (BMP=1, supplementary=2).
- `tests/render.bats`: five new cases covering Cyrillic passthrough at the boundary, Cyrillic truncation, supplementary-plane emoji counting as 2 units, codepoint-aligned emoji truncation (no half emoji / no U+FFFD), and a unit-count parameter test.

### Changed

- `README.md`: length-limit note and requirements section updated to reflect UTF-16 units and the new `iconv` dependency.

## [1.2.0] — 2026-05-27

### Added

- `scripts/send.sh`: the `429` retry path now caps `parameters.retry_after` at `NF_MAX_RETRY_AFTER` (default 60s). Telegram occasionally returns pathological values (hundreds or thousands of seconds); without a cap a single rate-limited request could stall the workflow for an hour. Override via `NF_MAX_RETRY_AFTER` env var (env-only, same pattern as `NF_CONNECT_TIMEOUT` / `NF_MAX_TIME`).
- `scripts/send.sh`: `retry_after` is validated as a non-negative integer; any other shape (string, float, missing) falls back to 1s instead of crashing `sleep`.
- `tests/send.bats`: three new cases covering the cap with a stubbed `sleep` to assert the exact value, a non-integer fallback, and an env-overridden cap.
- `tests/fixtures/mock_server.py`: two new fixtures, `rate_limit_huge` (retry_after=9999) and `rate_limit_garbage` (retry_after="soon").

## [1.1.0] — 2026-05-27

### Added

- `scripts/send.sh`: every Telegram request is now bounded by `curl --connect-timeout` and `--max-time`. Defaults are 5s for the TCP/TLS handshake and 15s for the whole request. Overridable via `NF_CONNECT_TIMEOUT` and `NF_MAX_TIME` environment variables (env-only, same pattern as `NF_API_BASE`). A hung Telegram peer can no longer stall the notify job indefinitely; timeouts surface as a network error and follow the existing 5xx backoff/retry path.
- `tests/send.bats`: four new cases covering hang → retry → success, sustained hang exhausting retries within a bounded budget, and verification that the flags and their default/overridden values reach `curl`'s argv.
- `tests/fixtures/mock_server.py`: new `hang` fixture that blocks before responding (`--hang-seconds`, default 30). Server promoted to `ThreadingHTTPServer` so a blocking request does not stall sibling requests during retry tests.

## [1.0.1] — 2026-05-27

### Fixed

- Removed `default: ${{ job.status }}` from the `status` input in `action.yml`. The `job` context is not available in composite-action `default` fields, so every workflow failed manifest validation with `Unrecognized named-value: 'job'` before any step ran.

### Changed

- **Breaking (relative to the broken 1.0.0):** `status` is now `required: true`. Callers must pass it explicitly, typically as `status: ${{ job.status }}` or `status: ${{ needs.<job>.result }}`. Missing status now exits `10 MISSING_REQUIRED_INPUT` instead of `12 INVALID_STATUS`.
- `scripts/validate.sh`: dropped the dead `JOB_STATUS` env-var fallback (GitHub does not export current-job status to the runner environment).
- `README.md`: every example now includes `status:` explicitly; inputs table reflects the new contract.

### Added

- `tests/manifest.bats`: guards `action.yml` against forbidden contexts (`job`, `steps`, `needs`, `secrets`) in `default:` fields and asserts `status` stays required with no default.

## [1.0.0] — 2026-05-26

### Added

- Composite GitHub Action that sends a Telegram message when a workflow job completes.
- Inputs: `bot_token`, `chat_id`, `status`, `parse_mode`, `notify_on`, `message`, `message_template`, `template_success`/`_failure`/`_cancelled`/`_skipped`, `disable_web_page_preview`, `disable_notification`, `message_thread_id`, `fail_on_error`.
- Outputs: `ok`, `message_id`, `http_status`.
- Built-in template with status emoji (`✅` / `❌` / `⚠️` / `⏭`), workflow, repo, branch, short SHA, actor, and run URL.
- 16 placeholders (`{{.Repo}}`, `{{.Workflow}}`, `{{.Status}}`, …) with auto-escape for the active `parse_mode`.
- Retry policy: up to 4 attempts. `429` honors `parameters.retry_after`; `5xx` and network errors use 1s/2s/4s exponential backoff; other `4xx` is final.
- `::add-mask::` for the bot token before any other observable action.
- Length truncation at 4096 bytes with a trailing `...`.
- CI matrix on `ubuntu-latest` and `macos-latest`.
- Moving major tag (`v1`) maintained by `release.yml` on every `v*.*.*` push.
