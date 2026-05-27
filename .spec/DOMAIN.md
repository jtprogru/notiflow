<!-- generated: 2026-05-27, template: core.md -->

# Domain Model

`notiflow` has no persistent entities — it is a stateless, fire-and-forget HTTP client. Its "domain" is therefore the **set of typed values that flow between the GitHub Actions runtime, the script pipeline, and the Telegram Bot API.** This document catalogues each value, its source, its allowed range, and where it is defined in code.

## 1. Core Entities

### 1.1 `SendMessageRequest` — the JSON body POSTed to Telegram

Built by `_nf::_build_json` in `scripts/send.sh:14-54` via `jq -n`. Shape:

```jsonc
{
  "chat_id":                  <integer | string>,   // numeric tonumber? coercion at send.sh:47; "@username" stays string
  "text":                     <string>,             // rendered + escaped + ≤4096 bytes
  "disable_web_page_preview": <boolean>,            // from NF_DISABLE_WEB_PAGE_PREVIEW; default true
  "disable_notification":     <boolean>,            // from NF_DISABLE_NOTIFICATION;     default false
  "parse_mode":               <string>?,            // OMITTED when NF_PARSE_MODE in {none, ""}; otherwise verbatim
  "message_thread_id":        <integer>?            // INCLUDED only when NF_MESSAGE_THREAD_ID is non-empty
}
```

Source line for each conditional key:
- `parse_mode` conditional: `send.sh:51` (`if $parse_mode == "none" or $parse_mode == "" then {} else {parse_mode: $parse_mode} end`).
- `message_thread_id` conditional: `send.sh:52` (`if $include_thread == 1 then {message_thread_id: ($thread_id | tonumber)} else {} end`).

### 1.2 `SendMessageResponseOk` — successful Telegram response

Parsed at `send.sh:99-110`. Only two fields are consumed:

```jsonc
{
  "ok": true,
  "result": {
    "message_id": <integer>    // exported as the action's message_id output
    /* other fields ignored */
  }
}
```

### 1.3 `SendMessageResponseRateLimit` — 429 Too Many Requests

Parsed at `send.sh:111-118`:

```jsonc
{
  "ok": false,
  "error_code": 429,
  "description": <string>,
  "parameters": {
    "retry_after": <integer>   // seconds to sleep; falls back to 1 via jq `// 1` if missing
  }
}
```

### 1.4 `ActionOutputs` — what gets written to `$GITHUB_OUTPUT`

Set via `nf::set_output` (`lib.sh:32-35`). Always all three keys, in every code path:

| Output | Type | Possible values | Set at |
|--------|------|-----------------|--------|
| `ok` | string | `"true"` (200 + body.ok) / `"false"` (everything else, including filter skip) | `send.sh:104`, `send.sh:136`, `send.sh:150`, `entrypoint.sh:35` |
| `message_id` | string | numeric on success, `""` otherwise | `send.sh:105`, `send.sh:137`, `send.sh:151`, `entrypoint.sh:36` |
| `http_status` | string | `"200"` / `"<4xx>"` / `"<5xx>"` / `"0"` (skip or network error) | `send.sh:106`, `send.sh:138`, `send.sh:152`, `entrypoint.sh:37` |

### 1.5 Placeholder map

Defined in `scripts/render.sh:9` and dispatched by `_nf::_placeholder_value` (`render.sh:23-46`). Sixteen known keys; values are sourced from the GitHub Actions environment.

| Placeholder | Source | Notes |
|-------------|--------|-------|
| `Repo` | `$GITHUB_REPOSITORY` | e.g. `jtprogru/notiflow` |
| `Workflow` | `$GITHUB_WORKFLOW` | workflow name from yaml |
| `Job` | `$GITHUB_JOB` | job id |
| `Status` | `$NF_STATUS` | post-validate; one of the status enum below |
| `StatusEmoji` | from `_nf::_emoji $NF_STATUS` (`render.sh:11-19`) | see Enum 2.2 |
| `Actor` | `$GITHUB_ACTOR` | login who triggered the run |
| `Ref` | `$GITHUB_REF` | full ref (e.g. `refs/heads/main`) |
| `RefName` | `$GITHUB_REF_NAME` | short ref name |
| `Branch` | `$GITHUB_REF_NAME` | alias of `RefName` |
| `Sha` | `$GITHUB_SHA` | full 40-char SHA |
| `ShortSha` | `${GITHUB_SHA:0:7}` | first 7 chars |
| `RunId` | `$GITHUB_RUN_ID` | numeric |
| `RunNumber` | `$GITHUB_RUN_NUMBER` | numeric |
| `RunUrl` | composed: `${GITHUB_SERVER_URL}/${GITHUB_REPOSITORY}/actions/runs/${GITHUB_RUN_ID}` | the only derived value |
| `EventName` | `$GITHUB_EVENT_NAME` | e.g. `push`, `pull_request` |
| `ServerUrl` | `$GITHUB_SERVER_URL` | `https://github.com` on github.com |

Any other `{{.Name}}` in a template is treated as **unknown** and stripped after emitting `::warning::UNKNOWN_PLACEHOLDER:<name>` (`render.sh:117-127`).

## 2. Enums / Value Objects

### 2.1 `Status` enum

Allowed values for `NF_STATUS` (and for items inside `NF_NOTIFY_ON`):

| Value | Default for | Defined at |
|-------|-------------|------------|
| `success` | `${{ job.status }}` when the job succeeded | `validate.sh:59` |
| `failure` | `${{ job.status }}` when the job failed | `validate.sh:59` |
| `cancelled` | `${{ job.status }}` when the job was cancelled | `validate.sh:59` |
| `skipped` | when an upstream job/step skipped this one | `validate.sh:59` |

Any other value → exit `12` (`INVALID_STATUS`).

### 2.2 `StatusEmoji` mapping

Defined in `_nf::_emoji` (`scripts/render.sh:11-19`):

| Status | Emoji |
|--------|-------|
| `success` | ✅ |
| `failure` | ❌ |
| `cancelled` | ⚠️ |
| `skipped` | ⏭ |
| anything else | empty string |

### 2.3 `ParseMode` enum

Allowed values for `NF_PARSE_MODE` (`validate.sh:69`):

| Value | Behaviour |
|-------|-----------|
| `MarkdownV2` | default; `_nf::_escape_value` dispatches to `nf::escape_md_v2` (`render.sh:83`) |
| `HTML` | dispatches to `nf::escape_html` (`render.sh:84`) |
| `Markdown` | dispatches to `nf::escape_md_v2` (treat legacy as V2 for safety, `render.sh:85`) |
| `none` | dispatches to `nf::escape_none`; **`parse_mode` key is omitted from the JSON body** (`send.sh:51`) |

Any other value → exit `13` (`INVALID_PARSE_MODE`).

### 2.4 `BoolString` value object

`NF_DISABLE_WEB_PAGE_PREVIEW`, `NF_DISABLE_NOTIFICATION`, and `NF_FAIL_ON_ERROR` are bool-like strings. Two different acceptance rules:

| Variable | `true` set | `false` set | Default | Decoded at |
|----------|-----------|------------|---------|------------|
| `NF_DISABLE_WEB_PAGE_PREVIEW` | `true` | `false` | `true` | `send.sh:19-23` (anything else → true) |
| `NF_DISABLE_NOTIFICATION` | `true` | `false` | `false` | `send.sh:24-28` (anything else → false) |
| `NF_FAIL_ON_ERROR` | `true`, `TRUE`, `True`, `1`, `yes` | everything else | `false` | `entrypoint.sh:47-56` |

### 2.5 `ChatId` value object

Three valid shapes (`validate.sh:33-53`):

| Shape | Regex | Example |
|-------|-------|---------|
| Public username | `^@[A-Za-z0-9_]{4,32}$` | `@my_channel` |
| Negative integer | `^-[0-9]+$` | `-100123456789` (supergroups) |
| Non-negative integer | `^[0-9]+$` | `42` |

The `jq` body builder applies `tonumber?` and falls back to the string form for the `@username` case (`send.sh:47`).

### 2.6 `NotifyOn` CSV

Comma-separated subset of `Status` (Enum 2.1). Whitespace around items is trimmed at `validate.sh:86` and `filter.sh:19`. Default: `success,failure,cancelled` (`validate.sh:77`).

## 3. Business Errors

For the full business error catalog (codes, exit semantics, retry policy) **see `ERRORS.md`** once it is generated by the `errors.md` template.

Until then, the authoritative inline lists are:

- **Validation exit codes 10–15**: `scripts/validate.sh:8-14`, fully tabled in `.spec/PACKAGES.md` §3 (validate.sh entry).
- **Dependency / runtime exit codes 20–22**: `scripts/lib.sh:55-67` (20 = `UNSUPPORTED_BASH`) and `scripts/lib.sh:45-50` (22 = `MISSING_DEPENDENCY:<cmd>`).
- **Send-time error semantics**: `scripts/send.sh:99-147` — 4xx is terminal with `ok=false`; 429/5xx/network are retried up to `_NF_MAX_ATTEMPTS=4`; on exhaustion `ok=false` and the entrypoint applies the `NF_FAIL_ON_ERROR` policy (`entrypoint.sh:47-56`).

These should be migrated into `ERRORS.md` when the `errors.md` workflow runs.

## 4. Environment Variable Contract

The `NF_*` namespace is the **canonical interface** between `action.yml` and the script pipeline. Every variable below is set in `action.yml:85-100` from the corresponding `inputs.<x>`.

| Env var | Type | Default | Required | Read by |
|---------|------|---------|----------|---------|
| `NF_BOT_TOKEN` | string | — | yes | `entrypoint.sh:27`, `send.sh:61` |
| `NF_CHAT_ID` | `ChatId` (§2.5) | — | yes | `validate.sh:33`, `send.sh:37` |
| `NF_STATUS` | `Status` (§2.1) | `${{ job.status }}` then `JOB_STATUS` | no | `validate.sh:56`, `filter.sh:1`, `render.sh:30` |
| `NF_PARSE_MODE` | `ParseMode` (§2.3) | `MarkdownV2` | no | `validate.sh:66`, `render.sh:82`, `send.sh:34` |
| `NF_NOTIFY_ON` | `NotifyOn` CSV (§2.6) | `success,failure,cancelled` | no | `validate.sh:76`, `entrypoint.sh:33` |
| `NF_MESSAGE` | string (verbatim) | `""` | no | `render.sh:100` (REQ-5.1 passthrough) |
| `NF_MESSAGE_TEMPLATE` | string (template) | `""` | no | `render.sh:74` |
| `NF_TEMPLATE_SUCCESS` | string (template) | `""` | no | `render.sh:64` |
| `NF_TEMPLATE_FAILURE` | string (template) | `""` | no | `render.sh:65` |
| `NF_TEMPLATE_CANCELLED` | string (template) | `""` | no | `render.sh:66` |
| `NF_TEMPLATE_SKIPPED` | string (template) | `""` | no | `render.sh:67` |
| `NF_DISABLE_WEB_PAGE_PREVIEW` | `BoolString` (§2.4) | `true` | no | `send.sh:19` |
| `NF_DISABLE_NOTIFICATION` | `BoolString` (§2.4) | `false` | no | `send.sh:24` |
| `NF_MESSAGE_THREAD_ID` | integer-string or `""` | `""` | no | `validate.sh:95`, `send.sh:30`, `send.sh:42` |
| `NF_FAIL_ON_ERROR` | `BoolString` (§2.4) | `false` | no | `entrypoint.sh:47` |
| `NF_API_BASE` | URL | `https://api.telegram.org` | no (test-only) | `send.sh:60` |

`NF_API_BASE` is not declared in `action.yml` — it is only set by `tests/helpers.bash:59` to redirect HTTP to the mock server.

## 5. Template Selection Decision Tree

The four-level priority that resolves `NF_MESSAGE` / `NF_TEMPLATE_<STATUS>` / `NF_MESSAGE_TEMPLATE` / built-in default. Implemented at `render.sh:98-103` (passthrough) and `render.sh:61-79` (template picker).

```
nf::render
│
├── NF_MESSAGE non-empty?
│     yes ─► return NF_MESSAGE truncated to 4096 bytes (NO substitution, NO escape)
│     no  ─►
│
└── _nf::_pick_template
      │
      ├── per_status = NF_TEMPLATE_${STATUS^^}
      │     case status of
      │       success   → NF_TEMPLATE_SUCCESS
      │       failure   → NF_TEMPLATE_FAILURE
      │       cancelled → NF_TEMPLATE_CANCELLED
      │       skipped   → NF_TEMPLATE_SKIPPED
      │
      ├── per_status non-empty? → return per_status
      ├── NF_MESSAGE_TEMPLATE non-empty? → return NF_MESSAGE_TEMPLATE
      └── otherwise → return _nf::_default_template (render.sh:48-56)
                       """
                       {{.StatusEmoji}} *{{.Workflow}}* on `{{.Repo}}`
                       Status: {{.Status}}
                       Branch: {{.Branch}} @ {{.ShortSha}}
                       Actor: {{.Actor}}
                       [Open run]({{.RunUrl}})
                       """
```

After selection, `nf::render` substitutes placeholders, warns on unknowns, and truncates to 4096 bytes.

## 6. Search/Filter Parameters

Not applicable — `notiflow` has no query surface. The only filter in the system is `nf::should_notify` (Enum 2.6), already covered above.
