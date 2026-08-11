| Output | Description |
|---|---|
| `ok` | true when the message was delivered, false otherwise. |
| `message_id` | Telegram message_id on success, empty otherwise. A single value — v1 returned a comma-separated list for multi-chat sends. |
| `http_status` | HTTP status of the last attempt (0 when skipped or when no response arrived). |
| `error` | Failure reason — Telegram .description when available, otherwise a synthesized reason. Empty on success and on skip. |
