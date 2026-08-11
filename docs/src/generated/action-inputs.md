| Input | Required | Default | Description |
|---|---|---|---|
| `bot_token` | yes | — | Telegram bot token (use a repository secret). |
| `chat_id` | yes | — | Target chat: an integer id (possibly negative) or @channelusername. Exactly one chat — the comma-separated form accepted by v1.5 was removed in v2. To notify several chats, use a job matrix or repeat the step. |
| `status` | yes | — | Job status to report. Must be passed explicitly from the workflow, typically from the job.status or needs.<job>.result expression. Allowed values: success, failure, cancelled, skipped. Cannot be defaulted here — composite-action input defaults do not have access to the job context. |
| `parse_mode` | no | `MarkdownV2` | Telegram parse_mode: MarkdownV2 (default), HTML, Markdown, or none. |
| `notify_on` | no | `success,failure,cancelled` | Comma-separated list of statuses that should trigger a notification. `any` or `all` means every status. |
| `message` | no | — | Verbatim message text. Overrides every template. No placeholder substitution and no escaping are applied. |
| `message_template` | no | — | Template string with {{.Field}} placeholders, used for all statuses not overridden by template_<status>. |
| `template_success` | no | — | Template used when status=success. Overrides message_template. |
| `template_failure` | no | — | Template used when status=failure. Overrides message_template. |
| `template_cancelled` | no | — | Template used when status=cancelled. Overrides message_template. |
| `template_skipped` | no | — | Template used when status=skipped. Overrides message_template. |
| `disable_web_page_preview` | no | `true` | Suppress link previews. Sent as link_preview_options.is_disabled; the flat field Telegram deprecated is no longer used. |
| `disable_notification` | no | `false` | Send the message silently (no sound for recipients). |
| `message_thread_id` | no | — | Forum-chat thread (topic) ID. Integer. |
| `fail_on_error` | no | `false` | If true, the action exits non-zero when the Telegram request ultimately fails. Default false keeps the job result intact. |
| `edit_message_id` | no | — | If set, the message is edited via editMessageText instead of sent fresh. One integer, typically wired from the message_id output of an earlier notiflow step. disable_notification and message_thread_id are dropped when editing, because Telegram rejects them there. |
| `version` | no | — | notiflow release to install. Empty pins to the version stamped in the Action tree; "latest" resolves the newest release at run time. |
| `verify_signature` | no | `false` | Verify the release archive with cosign before installing. Requires sigstore/cosign-installer earlier in the job. |
| `github_token` | no | `${{ github.token }}` | Token used to call the GitHub Releases API when resolving "latest". |
