---
title: Parse modes and escaping
description: MarkdownV2, HTML and none — what each escapes, and where verbatim text is dangerous.
---

## The three modes

| Mode | Sent as | Values are escaped by |
|---|---|---|
| `MarkdownV2` (default) | `parse_mode: MarkdownV2` | prefixing every metacharacter with `\` |
| `HTML` | `parse_mode: HTML` | `&` → `&amp;`, `<` → `&lt;`, `>` → `&gt;` |
| `none` | the field is omitted | nothing |

`Markdown` is accepted as an alias for `MarkdownV2` and logs a warning. notiflow escapes
values per V2 rules; handing those to Telegram's legacy parser produces broken or rejected
messages, so silently upgrading is the only behaviour that yields a message the user meant.

## MarkdownV2

These 18 characters are markup and are escaped inside substituted values:

```
_ * [ ] ( ) ~ ` > # + - = | { } . !
```

The backslash itself is escaped first, so a value containing `\.` becomes `\\\.` — one
escaped backslash followed by one escaped dot — rather than collapsing into a single
escape.

```bash
notiflow render -T 'branch {{.Branch}}' --set Branch='fix/a.b-c_d[e](f)'
# branch fix/a\.b\-c\_d\[e\]\(f\)
```

Ordinary text is left alone: letters, digits, spaces, `/`, `:`, Cyrillic and emoji all pass
through untouched.

Markup you write in the template still works, because only values are escaped:

```yaml
message_template: |
  *{{.Workflow}}* finished on `{{.Branch}}`
  [open the run]({{.RunUrl}})
```

## HTML

Telegram accepts a small tag set: `b`, `strong`, `i`, `em`, `u`, `ins`, `s`, `strike`,
`del`, `span class="tg-spoiler"`, `tg-spoiler`, `a href`, `code`, `pre`, `blockquote`.

Substituted values have `&`, `<` and `>` escaped, with the ampersand first so `&lt;` does
not become `&amp;lt;`.

```yaml
parse_mode: HTML
message_template: |
  <b>{{.Workflow}}</b> on <code>{{.Repo}}</code>
  <a href="{{.RunUrl}}">open the run</a>
```

A caveat that is easy to hit: `{{.RunUrl}}` inside an `href` attribute is escaped as HTML
text, not as a URL. That is correct for the URLs GitHub produces. If you interpolate a URL
you built yourself and it contains `&`, it will arrive as `&amp;` — which is also correct
HTML, and Telegram unescapes it.

## none

Nothing is escaped and no `parse_mode` is sent. The message arrives exactly as composed.
The right choice when the text comes from somewhere you do not control — a commit message,
a log tail, a test failure — because there is no markup to break.

```bash
tail -50 build.log | notiflow send --parse-mode none --stdin
```

## Where verbatim text is dangerous

`message` (and its `--message-file` / `--stdin` equivalents) is passed through untouched:
no substitution and no escaping. That is the point of it — it is the escape hatch for text
you have already formatted.

It also means this is a live grenade:

```yaml
# don't
message: "Build failed: ${{ github.event.head_commit.message }}"
parse_mode: MarkdownV2
```

A commit message containing `_` or `[` produces `400 Bad Request: can't parse entities`,
and one containing `[x](https://evil.example)` produces a link you did not write. Two ways
out:

```yaml
# either: no markup to break
message: "Build failed: ${{ github.event.head_commit.message }}"
parse_mode: none
```

```yaml
# or: let notiflow escape it — templates escape every value they substitute
message_template: "❌ {{.Workflow}} broke on {{.Branch}} ({{.ShortSha}})"
```

## Checking your work

```bash
notiflow render --parse-mode MarkdownV2 --template '…' --explain
notiflow send --parse-mode HTML --template '…' --dry-run
```

`render` shows the text; `--dry-run` shows the whole JSON body, including whether
`parse_mode` is on the wire at all.
