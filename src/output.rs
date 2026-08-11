//! The three ways notiflow reports what it did.
//!
//! `github` is the contract v1 established — the same four outputs, same names, same
//! meanings, minus the multi-chat CSV. `json` is for scripts, `human` for a terminal.

use serde::Serialize;

use crate::actions;
use crate::error::Result;
use crate::safe_println;

/// Output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Format {
    /// One readable line on stdout.
    #[default]
    Human,
    /// A single JSON object on stdout.
    Json,
    /// `$GITHUB_OUTPUT` entries plus a step summary table.
    Github,
}

impl Format {
    pub fn as_str(self) -> &'static str {
        match self {
            Format::Human => "human",
            Format::Json => "json",
            Format::Github => "github",
        }
    }
}

/// What one run produced.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// True only when Telegram accepted the message.
    pub ok: bool,
    /// True when `notify_on` filtered the run out before any request was made.
    pub skipped: bool,
    /// Telegram's message id, on success.
    pub message_id: Option<i64>,
    /// Last HTTP status; 0 for a skip, a dry run, or a transport failure.
    pub http_status: u16,
    /// Failure reason, verbatim from Telegram when it gave one.
    pub error: Option<String>,
    /// Destination, echoed back so a matrix job's log says which chat it was.
    pub chat_id: String,
    /// How many HTTP attempts were made.
    pub attempts: u32,
    /// True when `--dry-run` stopped short of sending.
    pub dry_run: bool,
    /// The JSON body that would have been posted. Only populated on a dry run — it is a
    /// debugging aid, not part of the Action's output contract.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<serde_json::Value>,
}

impl Report {
    /// A skip: `notify_on` did not include the reported status.
    pub fn skipped(chat_id: impl Into<String>) -> Self {
        Report {
            ok: false,
            skipped: true,
            message_id: None,
            http_status: 0,
            error: None,
            chat_id: chat_id.into(),
            attempts: 0,
            dry_run: false,
            request: None,
        }
    }

    /// Writes the report in `format`.
    pub fn emit(&self, format: Format) -> Result<()> {
        match format {
            Format::Human => self.emit_human(),
            Format::Json => self.emit_json(),
            Format::Github => self.emit_github(),
        }
    }

    fn emit_human(&self) -> Result<()> {
        if self.skipped {
            safe_println!("skipped: status not in notify_on");
            return Ok(());
        }
        if self.dry_run {
            safe_println!("dry-run: nothing sent to {}", self.chat_id);
            if let Some(request) = &self.request {
                safe_println!("{}", serde_json::to_string_pretty(request).unwrap_or_default());
            }
            return Ok(());
        }
        match (self.ok, &self.error) {
            (true, _) => safe_println!(
                "sent to {} (message_id={})",
                self.chat_id,
                self.message_id.map(|m| m.to_string()).unwrap_or_else(|| "?".into())
            ),
            (false, Some(err)) => {
                safe_println!("failed after {} attempt(s): {}", self.attempts, err)
            }
            (false, None) => safe_println!("failed after {} attempt(s)", self.attempts),
        }
        Ok(())
    }

    fn emit_json(&self) -> Result<()> {
        let rendered = serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string());
        safe_println!("{}", rendered);
        Ok(())
    }

    /// v1's four outputs, unchanged in name and meaning.
    ///
    /// `message_id` is a scalar now rather than a CSV — the one visible consequence of
    /// dropping multi-chat fan-out.
    fn emit_github(&self) -> Result<()> {
        actions::set_output("ok", if self.ok { "true" } else { "false" })?;
        actions::set_output(
            "message_id",
            &self.message_id.map(|m| m.to_string()).unwrap_or_default(),
        )?;
        actions::set_output("http_status", &self.http_status.to_string())?;
        actions::set_output("error", self.error.as_deref().unwrap_or(""))?;
        actions::step_summary(&self.summary_markdown())?;
        Ok(())
    }

    /// The `$GITHUB_STEP_SUMMARY` block.
    pub fn summary_markdown(&self) -> String {
        let state = if self.skipped {
            "skipped"
        } else if self.dry_run {
            "dry-run"
        } else if self.ok {
            "sent"
        } else {
            "failed"
        };
        let mut out = String::from("### notiflow\n\n| Field | Value |\n|---|---|\n");
        out.push_str(&format!("| result | {state} |\n"));
        out.push_str(&format!("| chat_id | `{}` |\n", self.chat_id));
        if let Some(id) = self.message_id {
            out.push_str(&format!("| message_id | `{id}` |\n"));
        }
        out.push_str(&format!("| http_status | {} |\n", self.http_status));
        out.push_str(&format!("| attempts | {} |\n", self.attempts));
        if let Some(err) = &self.error {
            out.push_str(&format!("| error | {err} |\n"));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sent() -> Report {
        Report {
            ok: true,
            skipped: false,
            message_id: Some(42),
            http_status: 200,
            error: None,
            chat_id: "-100123".into(),
            attempts: 1,
            dry_run: false,
            request: None,
        }
    }

    #[test]
    fn json_shape_is_stable() {
        let v: serde_json::Value = serde_json::to_value(sent()).unwrap();
        assert_eq!(v["ok"], serde_json::json!(true));
        assert_eq!(v["message_id"], serde_json::json!(42));
        assert_eq!(v["http_status"], serde_json::json!(200));
        assert_eq!(v["error"], serde_json::Value::Null);
    }

    #[test]
    fn summary_reports_each_state() {
        assert!(sent().summary_markdown().contains("| result | sent |"));
        assert!(Report::skipped("-1").summary_markdown().contains("| result | skipped |"));

        let mut failed = sent();
        failed.ok = false;
        failed.error = Some("Bad Request: chat not found".into());
        let md = failed.summary_markdown();
        assert!(md.contains("| result | failed |"));
        assert!(md.contains("chat not found"));
    }

    #[test]
    fn skip_has_no_message_id_and_zero_status() {
        let r = Report::skipped("@chan_name");
        assert!(!r.ok);
        assert!(r.skipped);
        assert_eq!(r.http_status, 0);
        assert!(r.message_id.is_none());
    }
}
