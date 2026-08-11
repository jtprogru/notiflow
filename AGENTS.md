# AGENTS.md

Working notes for automated contributors. Humans should start at
[the docs site](https://jtprogru.github.io/notiflow/); this file covers what is specific to
changing the repository rather than using it.

## What this is

notiflow is a Rust binary that sends a Telegram message. The same binary backs two products:

- **the GitHub Action** — a composite action that downloads the released binary and runs it;
- **the CLI** — `notiflow`, distributed via Homebrew, crates.io and release archives.

v2 replaced a bash implementation. That predecessor is still in the tree, frozen, as the
reference the parity suite compares against.

## Map

```
Cargo.toml                 crate metadata, MSRV pin
VERSION                    the version the Action installs; must match Cargo.toml and the tag
action.yml                 the composite Action manifest
Makefile                   every task; CI calls these targets and nothing else

src/
  main.rs                  parse args, map errors onto exit codes, nothing more
  lib.rs                   the pipeline: render → truncate → request → retry → report
  cli.rs                   clap definitions and the five commands
  config.rs                settings layering, validation, the config file, api_base policy
  model.rs                 Status, ParseMode, ChatId, MessageId, ThreadId, NotifyOn
  context.rs               placeholder values, from GITHUB_* or from local git
  escape.rs                MarkdownV2 / HTML / none
  template.rs              template selection and single-pass substitution
  truncate.rs              UTF-16 counting and markup-safe cutting
  telegram/                request building, transport, error classification, retries
  actions.rs               ::add-mask::, annotations, $GITHUB_OUTPUT, step summary
  output.rs                human / json / github report formats
  redact.rs                the filter every printed byte passes through
  error.rs                 the error taxonomy and its exit codes

scripts/action/            the Action wrapper: resolve-version, detect-platform, install, run, smoke
scripts/gen-action-tables.py   renders the inputs/outputs tables out of action.yml

tests/
  send.rs, cli.rs          integration tests against a scripted mock Telegram
  support/mod.rs           the mock
  mock_server.py           the mock used by the bats suites
  action/                  wrapper acceptance tests and action.yml manifest checks
  parity/v1/               the frozen v1 bash implementation
  parity/bats/             v1's original suite, still green
  parity/corpus/*.json     the golden cases
  parity/run.py            the parity harness

docs/                      Astro Starlight site (EN primary, RU locale)
docs/src/generated/        NEVER edit by hand — produced by `make gen`
```

## Rules that are not negotiable

**Never edit `docs/src/generated/`.** Those files come from `make gen`, and `make gen-check`
fails in CI when the committed copy differs. Change the source — `action.yml`, the clap
definitions, `EXIT_CODE_TABLE`, `PLACEHOLDERS` — and regenerate.

**Never add a `${{ ... }}` expression to a `description:` in `action.yml`.** Composite-action
manifests evaluate expressions at load time, before any job context exists, so a literal
example inside a description aborts every workflow that uses the Action. v1.6.0 shipped
exactly that bug. `tests/action/manifest.bats` guards it.

**Never put the bot token in argv.** The wrapper exports it as `NOTIFLOW_BOT_TOKEN`. The
`--bot-token` flag exists for compatibility, warns, and is documented as unsafe.

**Never print anything that has not passed through `redact`.** Use `safe_println!` and
`safe_eprintln!`, or `actions::log`, which redacts. A raw `println!` of an HTTP error can
leak the token through the request URL.

**Never widen the `api_base` allowlist for Action mode.** Inside a workflow, `NF_API_BASE`
could have been planted by an earlier step in the same job. Loopback is allowed because the
smoke test needs it and reaching the runner's own loopback already requires code execution
there.

**Exit codes 10–16 are a contract with v1 users.** 20 and 22 are permanently reserved and
must never be reassigned.

## The parity corpus

`tests/parity/` runs every case through the frozen v1 bash and through the v2 binary and
diffs the results. A case may declare a `divergence` string explaining why the two differ.

- v2 differs and no `divergence` is declared → **failure**. That is a regression.
- a `divergence` is declared but the two agree → **failure**. A stale declaration hides the
  next real difference.

When a change is a deliberate fix: amend the case, write the `divergence`, run
`make parity-bless`, and **read the diff** before committing. A blessed expectation nobody
looked at is worth nothing.

The v1 bash sources are frozen. Do not "improve" anything under `tests/parity/v1/` — its
value is that it does not change.

## Conventions

Rust: `rustfmt` with the repository's `rustfmt.toml`, `clippy -D warnings`. Edition 2024,
MSRV 1.85 — `make msrv` is the check, and let-chains are therefore off limits.

Comments explain why, not what. A comment that restates the line below it is noise; a
comment that records why a non-obvious choice was made is the reason the file is
maintainable. Several already-fixed defects are documented in place — leave them there.

Shell: bash with `set -euo pipefail`, `shellcheck -x` clean, `shfmt -i 2 -ci`.

Commits: [Conventional Commits](https://www.conventionalcommits.org/), English, imperative,
no attribution trailers.

Markdown: no hard wrapping of prose. One paragraph, one line.

## Before opening a PR

```bash
make ci
```

That is `lint`, `test`, `gen-check` and `parity` in the order CI runs them. If the Action
wrapper changed, also run `make test-action` and `make action-smoke`.

## Releasing

```bash
make release-prep VERSION=2.1.0   # stamps Cargo.toml and VERSION
$EDITOR CHANGELOG.md
git commit -am "chore(release): 2.1.0"
git tag v2.1.0 && git push --follow-tags
```

The release workflow refuses to build when `Cargo.toml`, `VERSION` and the tag disagree.
That guard is what makes the Action's version resolution deterministic for `@v2`.
