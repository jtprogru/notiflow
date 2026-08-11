| Code | Name | Meaning |
|---:|---|---|
| 0 | `OK` | Message delivered, or skipped by notify_on, or fail_on_error=false |
| 1 | `SEND_FAILED` | Telegram never accepted the message and fail_on_error=true |
| 2 | `USAGE` | Command-line parsing failed (emitted by clap) |
| 10 | `MISSING_REQUIRED_INPUT` | bot_token, chat_id or status is empty |
| 11 | `INVALID_CHAT_ID` | chat_id is not an integer or @username (a comma is rejected in v2) |
| 12 | `INVALID_STATUS` | status is not success|failure|cancelled|skipped |
| 13 | `INVALID_PARSE_MODE` | parse_mode is not MarkdownV2|HTML|Markdown|none |
| 14 | `INVALID_NOTIFY_ON` | notify_on lists a value outside the status set |
| 15 | `INVALID_THREAD_ID` | message_thread_id is not a non-negative integer |
| 16 | `INVALID_EDIT_MESSAGE_ID` | edit_message_id is not a positive integer |
| 17 | `CONFIG_ERROR` | Config file unreadable, malformed, or the profile does not exist |
| 18 | `INVALID_ARGUMENT` | Argument is well-formed but unusable (bad api_base, zero timeout) |
| 20 | `RESERVED` | v1 UNSUPPORTED_BASH — never emitted by v2 |
| 22 | `RESERVED` | v1 MISSING_DEPENDENCY — never emitted by v2 |
| 30 | `IO_ERROR` | Reading stdin or a message file failed, or an output file is not writable |
