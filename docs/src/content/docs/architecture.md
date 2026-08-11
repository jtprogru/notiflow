---
title: Architecture
description: How the binary is put together, and the decisions behind it.
---

## The pipeline

Both faces of notiflow run the same sequence:

```
settings  →  filter  →  select template  →  render  →  truncate  →  request  →  retry  →  report
```

- **settings** — flags, environment, config profile, config defaults, built-in defaults, merged in that order and parsed into types that cannot hold an invalid value.
- **filter** — is the reported status in `notify_on`? If not, the run stops here, exits 0 and reports a skip. Nothing is rendered and no connection is opened.
- **select template** — `message` > `template_<status>` > `message_template` > built-in default.
- **render** — one pass over the template, substituting `{{.Field}}` and escaping each value for the parse mode. A `message` skips this step entirely.
- **truncate** — to 4096 UTF-16 code units, cutting between markup tokens.
- **request** — a JSON body for `sendMessage` or `editMessageText`.
- **retry** — one attempt plus retries, per the [policy](/notiflow/reliability/).
- **report** — human text, a JSON object, or `$GITHUB_OUTPUT` plus a step summary.

## Modules

| Module | Responsibility |
|---|---|
| `model` | Newtypes: `Status`, `ParseMode`, `ChatId`, `MessageId`, `ThreadId`, `NotifyOn` |
| `config` | Layering, validation, the `api_base` policy, the config file |
| `context` | Placeholder values, from `GITHUB_*` or from local git |
| `escape` | MarkdownV2 / HTML / none |
| `template` | Template selection and the single-pass substitution scanner |
| `truncate` | UTF-16 counting and markup-safe cutting |
| `telegram` | Request building, HTTP transport, error classification, the retry loop |
| `actions` | Workflow commands: masking, annotations, outputs, step summary |
| `output` | The three report formats |
| `redact` | The filter every printed byte passes through |
| `cli` | Argument definitions and the five commands |
| `error` | The error taxonomy and its exit codes |

`main.rs` calls `cli::dispatch` and maps the error onto an exit code. Nothing else.

## Decisions

### One crate, library plus binary

The project is too small for a workspace. `lib.rs` holds the logic and is unit-tested;
`main.rs` is a dozen lines. The crate publishes to crates.io and gets an API reference on
docs.rs for free.

A separate `notiflow-core` was considered and dropped: splitting it only pays off once
there is a real external consumer of the library, and until one exists its public API would
have to be maintained blind. If demand appears, extracting it is a minor release that does
not touch the CLI.

### `ureq`, not `reqwest`

The tool makes one POST and exits. `ureq` with rustls gives a ~3 MB binary and an instant
start; `reqwest` pulls in tokio for roughly 9 MB, buying async that has nothing to do here.
Dropping multi-chat fan-out removed even the hypothetical need for concurrency.

rustls rather than the system TLS stack also means no OpenSSL, which is why the static
`musl` builds need nothing beyond `musl-gcc`.

### A composite Action that downloads a binary

The alternatives, and why not:

| Option | Verdict |
|---|---|
| Docker action | Linux runners only; macOS and Windows fall off |
| Binaries committed to git | ~3 MB × 7 platforms in the history of every release |
| A JavaScript wrapper (`ncc`) | `node_modules` in the repository for forty lines of glue |
| **Composite + released binary** | Works on every runner, caches, verifies. **Chosen.** |

The cost is about five small bash scripts. Their behaviour is trivial and is covered by the
wrapper's own test suite on all three operating systems.

### Version resolution reads a file

The wrapper reads `VERSION` from the checked-out Action tree before falling back to
`GITHUB_ACTION_REF` or the Releases API. A moving tag like `@v2` cannot be resolved from
`GITHUB_ACTION_REF`, and falling back to `latest` makes a pinned workflow depend on a
network call whose answer can change between runs. The file is part of the tree, so `@v2`,
`@v2.1.0` and `@<sha>` all give the same deterministic answer. CI refuses to release when
`VERSION`, `Cargo.toml` and the tag disagree.

### The parity corpus

Before any logic was written, the cases in v1's bats suite were turned into a corpus of
`(inputs) → (rendered text | request JSON | exit code)`. `make parity` runs every case
through both implementations and diffs them.

A case may declare a `divergence` with a reason. A v2 result that differs *without* one
fails the build — the corpus distinguishes a fix from a regression, and forces the
difference to be written down at the moment it is introduced.

It paid for itself immediately: it caught a defect nobody had listed, where BSD `iconv`
exits non-zero on a severed surrogate pair, so a long emoji-bearing message aborted v1's
send on macOS.

The v1 bash sources live on in `tests/parity/v1/`, with their original bats suite, so the
reference implementation stays verified rather than assumed.

### Generated documentation

The Action's inputs and outputs come from `action.yml`; the CLI reference from the `clap`
command tree; the exit-code and placeholder tables from the Rust enums. `make gen-check`
regenerates them in CI and fails on any difference. This is the mechanism that stops the
v1 problem, where `README.md` and `action.yml` described different things.

### Everything through `make`

No workflow contains a build command of its own; every step calls a make target. A CI
failure is reproducible locally by running the same target, and CI cannot drift away from
what a developer runs.

## Security properties

- The bot token never becomes an argument. The Action wrapper exports it as an environment variable; the CLI flag exists, warns, and is documented as unsafe.
- Every stream notiflow writes passes through the redaction filter, which also rewrites `/bot<token>/` in any URL — including a token it was never told about.
- Inside a workflow, `api_base` is restricted to `api.telegram.org` and loopback, so a variable set by an earlier step in the same job cannot redirect the token. Loopback stays allowed because the smoke test needs to intercept, and reaching a listener on the runner's own loopback already requires code execution there.
- Substituted values are escaped for the parse mode, so a hostile branch or workflow name cannot inject markup or links.
- Release archives carry a sha256, a keyless cosign signature, a GPG signature and a SLSA build-provenance attestation. The checksum proves the download is intact; the signatures prove the artefact is the one the release workflow built.
