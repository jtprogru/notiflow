<!-- generated: 2026-05-27, template: security.md -->

# Security Documentation — notiflow

## 1. Security Overview

**Security model**: a stateless composite GitHub Action that accepts a Telegram bot token via `inputs.bot_token`, masks it before any other observable side-effect, validates every other input against strict allowlists, and POSTs a JSON body — built with `jq` — over TLS 1.2+ to `api.telegram.org`. No persistent storage, no database, no inbound network surface.

### Trust boundary diagram

```
┌─────────────────────────────────────────────────────────────┐
│  External (untrusted)                                       │
│                                                             │
│  GitHub Actions workflow author (consumer repo)             │
│   - controls all `inputs.*` values                          │
│   - controls workflow `secrets.TELEGRAM_BOT_TOKEN`          │
│   - controls how the action is pinned (@v1 vs @<sha>)       │
└─────────────────────────────┬───────────────────────────────┘
                              │ inputs + secrets
┌─────────────────────────────▼───────────────────────────────┐
│  GitHub-hosted runner (semi-trusted)                        │
│                                                             │
│  scripts/entrypoint.sh                                      │
│   ├─ nf::mask "$NF_BOT_TOKEN"   ← FIRST observable action   │
│   ├─ nf::validate               ← allowlists, regex         │
│   ├─ nf::should_notify          ← filter status             │
│   ├─ nf::render                 ← sed-substitute, escape    │
│   └─ nf::send                   ← curl → Telegram API       │
└─────────────────────────────┬───────────────────────────────┘
                              │ HTTPS POST (Telegram Bot API)
┌─────────────────────────────▼───────────────────────────────┐
│  External service (trusted to receive the message)          │
│                                                             │
│  api.telegram.org   (Telegram Bot API)                      │
└─────────────────────────────────────────────────────────────┘
```

### Security-critical components

| Component | File | Responsibility |
|-----------|------|----------------|
| Token masking | `scripts/entrypoint.sh:27-29` | Emits `::add-mask::` so the runner redacts the token in all subsequent log output. |
| Input validation | `scripts/validate.sh:25-103` | Whitelist/regex check on `chat_id`, `status`, `parse_mode`, `notify_on`, `message_thread_id`. Exits 10..15 before any HTTP call. |
| JSON body builder | `scripts/send.sh:14-54` | Uses `jq -n --arg/--argjson` — interpolation happens inside `jq`, not in shell. |
| Template rendering | `scripts/render.sh:98-130` | `sed`-substitutes `{{.Placeholder}}` tokens. RHS sanitised by `_nf::_sed_rhs_escape` (`scripts/render.sh:94-96`). |
| Markdown / HTML escape | `scripts/escape.sh:11-32` | Escapes user-supplied placeholder values for the active `parse_mode`. |
| Release pipeline | `.github/workflows/release.yml` | Force-updates `vN` major tag with the minimum `contents: write` scope. |

For the canonical exit-code list see [`.spec/ERRORS.md`](./ERRORS.md). For operational deploy steps see [`.spec/DEPLOYMENT.md`](./DEPLOYMENT.md). For coding rules see [`.spec/CODE_STYLE.md`](./CODE_STYLE.md).

## 2. Input Validation

**Validation layer** — single layer, executed in `nf::validate` (`scripts/validate.sh:25-103`), called from `scripts/entrypoint.sh:31` **after** masking but **before** rendering or sending. All invalid inputs exit before a single byte goes over the wire (verified by CP-1 / CP-2 in `.spec/features/telegram-notify-action/design.md` §2.6 and by `tests/validate.bats`).

**Validation approach** — POSIX-compatible `grep -E` regex and explicit `_nf::_in_set` allowlists (`scripts/validate.sh:16-23`). No external validation library; no bash 4 associative arrays.

### Validation rules

| Input env var | Source `inputs.*` | Validation | Exit on failure |
|---|---|---|---|
| `NF_BOT_TOKEN` | `bot_token` | Non-empty | 10 (`MISSING_REQUIRED_INPUT`) |
| `NF_CHAT_ID` | `chat_id` | `^(-?[0-9]+\|@[A-Za-z0-9_]{4,32})$` (`scripts/validate.sh:33-53`) | 11 (`INVALID_CHAT_ID`) |
| `NF_STATUS` | `status` | Whitelist: `success failure cancelled skipped` (`scripts/validate.sh:59`) | 12 (`INVALID_STATUS`) |
| `NF_PARSE_MODE` | `parse_mode` | Whitelist: `MarkdownV2 HTML Markdown none` (`scripts/validate.sh:69`) | 13 (`INVALID_PARSE_MODE`) |
| `NF_NOTIFY_ON` | `notify_on` | Comma-split, each item against the status whitelist (`scripts/validate.sh:79-91`) | 14 (`INVALID_NOTIFY_ON`) |
| `NF_MESSAGE_THREAD_ID` | `message_thread_id` | `^[0-9]+$` if set (`scripts/validate.sh:95-100`) | 15 (`INVALID_THREAD_ID`) |
| `NF_MESSAGE` | `message` | **Intentionally NOT validated, NOT escaped** (REQ-5.1, ADR in `design.md`) — see §3 below. | — |
| Template inputs | `message_template`, `template_success`, … | The template body is a `sed` LHS; user controls structure. Placeholder *values* are escaped, not the template itself. | — |

### Injection prevention matrix

| Vector | Mitigation | Reference |
|---|---|---|
| SQL injection | N/A — no database, no SQL. | — |
| XSS | N/A — no browser surface. The Telegram client renders messages, not HTML in a browser. | — |
| Command injection | All shell invocations use quoted variables and `printf '%s'`; no `eval`, no `bash -c "$var"`. `curl` URL is built from a static base + masked token + static path (`scripts/send.sh:60-67`). | `scripts/send.sh` |
| JSON injection | Body built with `jq -n --arg/--argjson` — user values are bound parameters inside `jq`, never concatenated into JSON. | `scripts/send.sh:36-53` |
| `sed` RHS injection | Placeholder values are escaped through `_nf::_sed_rhs_escape` which prefixes `\` `&` `/` with `\` and strips newlines. | `scripts/render.sh:94-96` |
| Markdown / HTML injection in Telegram | Placeholder values are escaped per `parse_mode` by `scripts/escape.sh:11-26`. MarkdownV2 escapes the full spec set `_ * [ ] ( ) ~ \` > # + - = | { } . !` plus `\`. HTML escapes `&`, `<`, `>`. | `scripts/escape.sh:11-32` |
| Path traversal | N/A — the action never opens a user-controlled file path. Only `$GITHUB_OUTPUT` (path provided by the runner) is written. | — |
| SSRF | The API base defaults to `https://api.telegram.org` (`scripts/send.sh:60`). `NF_API_BASE` is overridable but is **not** an action input — it is only settable from the action's own shell process (e.g. tests). Not user-reachable through `inputs.*`. | `scripts/send.sh:60` |

## 3. Authentication & Authorization Audit

The action has **no authn / authz surface of its own** — it does not authenticate workflow callers; whoever can run the workflow can call the action. Authority over the action is therefore equivalent to authority over the consumer repository's workflows and secrets.

- **Outbound auth**: bearer token in the URL (`/bot<TOKEN>/sendMessage`) — Telegram's documented scheme.
- **Token strength**: out of scope — the bot token is issued by `@BotFather`; rotate via Telegram if leaked.
- **Privilege escalation**: N/A — there are no roles, no user records.
- **Known gap**: ⚠️ The Telegram token format does not include an expiry. Consumers are responsible for rotation; see §7.

No `AUTH.md` exists — none is required for this action.

## 4. Transport Security

- **TLS to Telegram**: `curl` to `https://api.telegram.org` (`scripts/send.sh:60-66`). Default TLS settings — runner-provided `curl` and OS trust store. `curl -sS` does **not** disable verification.
- **Plain-HTTP override**: there is no `--insecure`, no `-k`, no `--cacert` flag in the code. `NF_API_BASE` is overridable but, as noted in §2, is not exposed as an action input.
- **Internal communication**: N/A — single-process script, no service mesh.
- **Certificate management**: delegated to the GitHub-hosted runner image (`ubuntu-latest`, `macos-latest`, …). Consumers using self-hosted runners must keep the CA bundle current.
- **HTTP→HTTPS redirect**: N/A — outbound only, never serves HTTP.

## 5. CORS & CSP

N/A — notiflow exposes no HTTP server, has no browser-facing surface, and serves no responses. CORS and CSP do not apply.

## 6. Rate Limiting & Abuse Prevention

The action does not impose rate limits — it **honours** them as a client:

| Scope | Behaviour | Source |
|---|---|---|
| Telegram `429 Too Many Requests` | Read `parameters.retry_after` from response; `sleep` that many seconds; retry. | `scripts/send.sh:111-118` |
| Telegram `5xx` | Exponential backoff `1s, 2s, 4s`. | `scripts/send.sh:119-125` |
| Network error (`curl_exit ≠ 0`) | Same exponential backoff. | `scripts/send.sh:126-133` |
| Telegram `4xx` (≠ 429) | **No retry** — fast-fail to avoid hammering on permanent client errors. | `scripts/send.sh:134-140` |
| Maximum attempts | `1 primary + up to 3 retries = 4 total` (`_NF_MAX_ATTEMPTS=4`). | `scripts/send.sh:8` |

Inbound DDoS / WAF / bot protection: N/A — no inbound surface.

## 7. Secrets Management (audit only)

> Operational owner: [`.spec/DEPLOYMENT.md`](./DEPLOYMENT.md) §8. This section flags audit findings only.

**Secrets in scope**: exactly one — the Telegram bot token (`inputs.bot_token`).

### Findings

- ✅ **Masked first, used later.** `scripts/entrypoint.sh:25-29` calls `nf::mask "$NF_BOT_TOKEN"` *before* validation, rendering, or any other observable step. Once `::add-mask::<token>` is emitted, the GitHub Actions runner redacts every subsequent occurrence in logs.
- ✅ **No verbose curl.** `scripts/send.sh:62` uses `curl -sS` — silent + show-errors. No `-v` / `--verbose` / `--trace*` flag anywhere. Verified by `tests/send.bats:128-133`.
- ✅ **No token in error paths.** Retry warnings (`scripts/send.sh:89, 109, 115, 122, 130, 142, 149`) print attempt counters and HTTP codes only — never the URL or the token. The `4xx` error path (`scripts/send.sh:134-140`) logs only the response `description`. Verified by `tests/entrypoint.bats:71-79`.
- ✅ **No file I/O outside `$GITHUB_OUTPUT`.** The action writes nothing to disk under its own control, so there is no on-disk leak vector.
- ✅ **No `set -x`** in production scripts; this would defeat the mask.
- ⚠️ **Anti-pattern to watch in consumer workflows**: passing the token through any intermediate `run:` step that prints it (e.g. `echo "$TELEGRAM_BOT_TOKEN" | nc -v ...`). The action cannot mask what it never sees. **Recommendation**: pass `secrets.TELEGRAM_BOT_TOKEN` directly into `with: bot_token:` — never via env on a separate step.
- ⚠️ **No automated rotation.** Telegram does not auto-rotate bot tokens. **Recommendation**: rotate at least annually and immediately on any suspected leak via `@BotFather → /revoke`.

## 8. Data Protection

- **PII handled**: none. The action handles workflow metadata only (repo name, ref, actor login, SHA, run id). These are already public for public repos; for private repos they remain inside the GitHub Actions trust boundary except for the single field set the consumer chooses to put in the Telegram message.
- **Encryption at rest**: N/A — no persistent storage.
- **Encryption in transit**: TLS to Telegram (see §4).
- **Data retention**: zero — process exits, runner is destroyed.
- **Backups**: N/A.
- **Compliance**: no GDPR / HIPAA / PCI-DSS scope. The action does not process payments, health records, or EU-resident personal data beyond what consumers voluntarily inject into `message` / templates.

## 9. Security Headers

N/A — outbound HTTP client only. Response security headers from `api.telegram.org` are Telegram's responsibility, not notiflow's.

## 10. Dependency Security

| Dependency | Source | Pinning | Risk |
|---|---|---|---|
| `bash` (≥ 3.2) | Runner image | Runner-shipped | Low — macOS-hosted runners ship bash 3.2; checked at startup by `nf::require_bash`. |
| `curl` | Runner image | Runner-shipped | Low — actively maintained, distro-patched. |
| `jq` | Runner image | Runner-shipped | Low — actively maintained, distro-patched. |
| `sed` | Runner image | Runner-shipped | Low — BSD/GNU differences handled by avoiding non-portable extensions; see [`.spec/CODE_STYLE.md`](./CODE_STYLE.md) §portability. |
| `actions/checkout@v4` (release workflow only) | Marketplace | Major-tag pinned (`@v4`) | Acceptable — official GitHub action. **Recommendation (supply-chain hardening)**: consider pinning to a full commit SHA. |

- **Audit tool**: not applicable — no `npm` / `pip` / `go.mod` / `cargo` manifest exists. Nothing to `audit` in the package-manager sense.
- **Lock file**: N/A.
- **Update policy**: monitor [GitHub Security Advisories](https://github.com/advisories) for `bash` / `curl` / `jq`. Runner-shipped versions update with image updates; consumers control runner image refresh.

### Supply-chain guidance for consumers of `jtprogru/notiflow`

| Pin style | Example | Security trade-off |
|---|---|---|
| Major tag (moving) | `uses: jtprogru/notiflow@v1` | Convenient; auto-upgrades within `v1`. Trusts the action author not to retag maliciously. The release pipeline (`.github/workflows/release.yml`) force-pushes `vN` on every release. |
| Full SHA (immutable) | `uses: jtprogru/notiflow@<40-char-sha>` | **Recommended for security-sensitive workflows.** Pinning to a SHA makes the action immutable even if a tag is rewritten. Use `dependabot` or `renovate` to bump. |

## 11. OWASP Top 10 Mapping (2021)

| # | OWASP Category | Project Mitigation | Status |
|---|---|---|---|
| A01 | Broken Access Control | No access-control surface in the action. Auth boundary lives in the consumer repo's workflow / secret config. | N/A |
| A02 | Cryptographic Failures | TLS to Telegram (`scripts/send.sh:60`). Token masked in logs (`scripts/entrypoint.sh:28`). No custom crypto. | ✅ Mitigated |
| A03 | Injection | `jq --arg/--argjson` for JSON (`scripts/send.sh:36-53`); `_nf::_sed_rhs_escape` for `sed` RHS (`scripts/render.sh:94-96`); allowlist validation in `scripts/validate.sh`; MarkdownV2/HTML escape in `scripts/escape.sh`. | ✅ Mitigated |
| A04 | Insecure Design | Threat-modelled in `.spec/features/telegram-notify-action/design.md` §2.7. Audited in `.spec/features/telegram-notify-action/review.md` § Security. | ✅ Mitigated |
| A05 | Security Misconfiguration | No `-v` / `-k` on curl; no `set -x`; release workflow uses `permissions: contents: write` only (`.github/workflows/release.yml:10-11`). | ✅ Mitigated |
| A06 | Vulnerable & Outdated Components | Zero managed dependencies; relies on runner-shipped `bash`/`curl`/`jq`. See §10. | ✅ Mitigated |
| A07 | Identification & Auth Failures | N/A — no caller authentication; outbound token is Telegram's responsibility. | N/A |
| A08 | Software & Data Integrity Failures | Release workflow runs with minimum scope; recommend consumers pin to a full SHA for immutability. See §10. | ⚠️ Partial — consumers may choose moving tags. |
| A09 | Logging & Monitoring Failures | Structured log levels (`info`/`warn`/`error`) via `nf::log`. Token never logged. No external monitoring (out of scope). | ✅ Mitigated |
| A10 | SSRF | No user-controlled URL: target host is the constant `https://api.telegram.org` (`scripts/send.sh:60`). `NF_API_BASE` override is not exposed as an action input. | ✅ Mitigated |

### Notes on Partial findings

- **A08**: the project provides a force-updated moving major tag (`v1`). This is convenient but enables a supply-chain attacker with push access to retroactively change what `@v1` resolves to. Mitigation = consumer-side pin to a SHA. Documented in §10.

## 12. Incident Response

The action itself is stateless and rolls forward on every workflow run — incidents are short-lived. The realistic incident surface is **token compromise**.

| Phase | Action |
|---|---|
| Detection | Unexpected Telegram messages, `@BotFather` activity, audit of GitHub Actions logs for the secret name. |
| Containment | (1) `@BotFather → /revoke` to invalidate the bot token immediately. (2) Rotate `secrets.TELEGRAM_BOT_TOKEN` in every repo that referenced it. |
| Communication | If the bot was used to notify a public channel, post a correction message. |
| Recovery | Re-issue token via `@BotFather → /token`, update secret, re-run failed workflows. |
| Post-mortem | Identify how the token leaked — most likely cause is an upstream `run:` step that printed it *before* the masking action ran. Add `::add-mask::` early in any custom step that touches the token. |

### Consumer responsibilities (explicit)

These are the consumer's burden, not notiflow's:

1. **Pin the action.** Either `@vN` (convenience) or `@<sha>` (recommended for security). See §10.
2. **Treat `message` as raw.** When `inputs.message` is set, notiflow performs **no placeholder substitution and no escaping** (REQ-5.1; `scripts/render.sh:100-103`). Whoever constructed that string owns its safety against MarkdownV2/HTML breakage and against leaking secrets the runner did not mask.
3. **Treat `parse_mode=none` as raw.** With `parse_mode=none`, `nf::escape_none` is the identity function (`scripts/escape.sh:30-32`). Placeholder values are inserted verbatim — fine for plaintext, dangerous if downstream code re-interprets the output as Markdown.
4. **Do not feed the token to other steps that log.** Pass `secrets.TELEGRAM_BOT_TOKEN` directly into `with: bot_token:` so notiflow can mask it on entry.
5. **Rotate proactively.** Telegram bot tokens have no expiry. Rotate annually and on any suspected leak.

For exit codes used by failure paths see [`.spec/ERRORS.md`](./ERRORS.md). For deployment / release procedure see [`.spec/DEPLOYMENT.md`](./DEPLOYMENT.md). For shell-style and portability rules see [`.spec/CODE_STYLE.md`](./CODE_STYLE.md).
