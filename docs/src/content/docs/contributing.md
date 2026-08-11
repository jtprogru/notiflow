---
title: Contributing
description: How to build, test and change notiflow.
---

## Getting set up

```bash
git clone https://github.com/jtprogru/notiflow
cd notiflow
make install-tools      # bats, shellcheck, shfmt, actionlint, jq, cargo-deny, cargo-audit
make build
```

Rust 1.85 or newer. `make help` lists every target.

## The loop

```bash
make lint        # rustfmt, clippy -D warnings, shellcheck, actionlint
make test        # unit, integration and doc tests
make parity      # the golden corpus, v1 bash against v2 Rust
make gen-check   # generated docs match the code
make ci          # all of the above, in the order CI runs them
```

Every CI step calls a make target, so a red build is reproducible with one command.

## Layout

```
src/                     the library and the binary
scripts/action/          the Action wrapper — resolve-version, detect-platform, install, run
tests/send.rs            end-to-end tests against a scripted mock Telegram
tests/cli.rs             the command-line surface
tests/action/            wrapper acceptance tests (bats) and action.yml manifest checks
tests/parity/v1/         the frozen v1 bash implementation
tests/parity/bats/       v1's original suite, still green, keeping the reference honest
tests/parity/corpus/     the golden cases
tests/parity/run.py      the parity harness
docs/                    the Astro Starlight site
```

## Changing behaviour

If a change alters what notiflow produces for some input, the parity corpus will notice.
That is the point. Two outcomes:

**It is a bug fix.** Add or amend the case, give it a `divergence` string saying what was
fixed and why, then `make parity-bless` and read the diff before committing. A blessed
expectation you have not looked at is worth nothing.

**It is a regression.** The corpus fails with both values printed. Fix the code.

A v2 result that differs from v1 without a declared `divergence` always fails. So does a
case that declares one when the two implementations actually agree — a stale declaration
hides the next real difference.

## Adding a placeholder

Add it to `PLACEHOLDERS` in `src/context.rs` and resolve it in `Context::discover`. The
documentation table regenerates itself; `make gen` and commit the result.

## Adding a CLI flag

Add it to the relevant `Args` struct in `src/cli.rs`. The CLI reference regenerates too.
If it should also be settable from a config file or the Action, add it to `Settings` in
`src/config.rs` and map it in `scripts/action/run.sh`.

## Adding an Action input

Add it to `action.yml`, map it to an `NF_*` variable in the same file, and translate it to
an argument in `scripts/action/run.sh`. The inputs table regenerates. Add a case to
`tests/action/run.bats` — the mapping is the part that breaks silently.

## Tests

Unit tests live beside the code they exercise. Integration tests run the real binary
against a scripted mock in `tests/support/mod.rs`; add a `Reply` to the script and assert
on what the mock received. The bats suites cover the shell wrapper, which Rust tests cannot
reach.

Nothing in the suite talks to the real Telegram API.

## Commits and releases

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/).
Versioning is semver; before `v1.0.0` of any component, a breaking change goes in the minor.

Releasing:

```bash
make release-prep VERSION=2.1.0   # stamps Cargo.toml and VERSION
$EDITOR CHANGELOG.md
git commit -am "chore(release): 2.1.0"
git tag v2.1.0 && git push --follow-tags
```

The release workflow refuses to build when `Cargo.toml`, `VERSION` and the tag disagree,
which is what keeps the Action's version resolution deterministic.

## Documentation

```bash
make docs-install
make docs-dev
```

English is the source of truth. Russian pages live under `docs/src/content/docs/ru/`; an
untranslated page falls back to its English original rather than 404ing, so a partial
translation is a valid state.

Never edit anything in `docs/src/generated/` — it is overwritten by `make gen`, and CI
fails if the committed copy disagrees with the code.
