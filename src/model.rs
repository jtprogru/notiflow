//! Newtypes for every value that used to be an unvalidated string in the bash version.
//!
//! Parsing happens once, at the edge; the rest of the crate works with types that cannot
//! hold an invalid value. This is what removes the scattered `grep -Eq` checks from v1.

use std::fmt;
use std::str::FromStr;

use crate::error::{Error, Result};

/// Job outcome being reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Status {
    Success,
    Failure,
    Cancelled,
    Skipped,
}

impl Status {
    /// All statuses, in the canonical order used by `notify_on: any`.
    pub const ALL: [Status; 4] =
        [Status::Success, Status::Failure, Status::Cancelled, Status::Skipped];

    pub fn as_str(self) -> &'static str {
        match self {
            Status::Success => "success",
            Status::Failure => "failure",
            Status::Cancelled => "cancelled",
            Status::Skipped => "skipped",
        }
    }

    /// Emoji substituted for `{{.StatusEmoji}}`. Byte-identical to v1's `_nf::_emoji`.
    pub fn emoji(self) -> &'static str {
        match self {
            Status::Success => "✅",
            Status::Failure => "❌",
            Status::Cancelled => "⚠️",
            Status::Skipped => "⏭",
        }
    }
}

impl FromStr for Status {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "success" => Ok(Status::Success),
            "failure" => Ok(Status::Failure),
            "cancelled" => Ok(Status::Cancelled),
            "skipped" => Ok(Status::Skipped),
            other => Err(Error::InvalidStatus(format!(
                "'{other}' (allowed: success|failure|cancelled|skipped)"
            ))),
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How Telegram should interpret the message body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParseMode {
    #[default]
    MarkdownV2,
    Html,
    /// Plain text: no `parse_mode` field on the wire and no escaping.
    None,
}

/// Outcome of parsing a `parse_mode`, so the caller can warn about the legacy alias
/// without `ParseMode` itself carrying a variant that never reaches the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParsedParseMode {
    pub mode: ParseMode,
    /// True when the input was the deprecated `Markdown`, silently upgraded to V2.
    pub upgraded_from_legacy_markdown: bool,
}

impl ParseMode {
    /// The value sent as `parse_mode`, or `None` when the field must be omitted.
    pub fn wire_value(self) -> Option<&'static str> {
        match self {
            ParseMode::MarkdownV2 => Some("MarkdownV2"),
            ParseMode::Html => Some("HTML"),
            ParseMode::None => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ParseMode::MarkdownV2 => "MarkdownV2",
            ParseMode::Html => "HTML",
            ParseMode::None => "none",
        }
    }

    /// Parses a parse_mode, reporting whether the legacy `Markdown` alias was used.
    ///
    /// v1 upgraded `Markdown` to `MarkdownV2` because notiflow escapes placeholder values
    /// per V2 rules; sending those to the legacy parser produces broken messages. v2 keeps
    /// the same behaviour and the same warning.
    pub fn parse(s: &str) -> Result<ParsedParseMode> {
        match s {
            "MarkdownV2" => Ok(ParsedParseMode {
                mode: ParseMode::MarkdownV2,
                upgraded_from_legacy_markdown: false,
            }),
            "Markdown" => Ok(ParsedParseMode {
                mode: ParseMode::MarkdownV2,
                upgraded_from_legacy_markdown: true,
            }),
            "HTML" => {
                Ok(ParsedParseMode { mode: ParseMode::Html, upgraded_from_legacy_markdown: false })
            }
            "none" | "" => {
                Ok(ParsedParseMode { mode: ParseMode::None, upgraded_from_legacy_markdown: false })
            }
            other => Err(Error::InvalidParseMode(format!("'{other}'"))),
        }
    }
}

impl FromStr for ParseMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        ParseMode::parse(s).map(|p| p.mode)
    }
}

impl fmt::Display for ParseMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single Telegram destination.
///
/// v2 accepts exactly one chat. A comma — the v1 multi-chat fan-out syntax — is rejected
/// with a dedicated message rather than being silently sent as a chat named `123,456`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatId(String);

impl ChatId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The JSON representation: a number when the id is numeric, a string for `@username`.
    pub fn to_json(&self) -> serde_json::Value {
        match self.0.parse::<i64>() {
            Ok(n) => serde_json::Value::from(n),
            Err(_) => serde_json::Value::from(self.0.clone()),
        }
    }
}

impl FromStr for ChatId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(Error::InvalidChatId("empty value".to_string()));
        }
        if trimmed.contains(',') {
            return Err(Error::InvalidChatId(format!(
                "'{trimmed}' — v2 sends to exactly one chat; \
                 the v1 comma-separated fan-out was removed (see the migration guide)"
            )));
        }
        if let Some(username) = trimmed.strip_prefix('@') {
            let len = username.chars().count();
            let charset_ok = username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
            if !(4..=32).contains(&len) || !charset_ok {
                return Err(Error::InvalidChatId(format!("'{trimmed}'")));
            }
            return Ok(ChatId(trimmed.to_string()));
        }
        let digits = trimmed.strip_prefix('-').unwrap_or(trimmed);
        if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            return Err(Error::InvalidChatId(format!("'{trimmed}'")));
        }
        Ok(ChatId(trimmed.to_string()))
    }
}

impl fmt::Display for ChatId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The id of a message being edited. Positive integer, single value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageId(i64);

impl MessageId {
    pub fn get(self) -> i64 {
        self.0
    }
}

impl FromStr for MessageId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let trimmed = s.trim();
        if trimmed.contains(',') {
            return Err(Error::InvalidEditMessageId(format!(
                "'{trimmed}' — v2 edits exactly one message; \
                 the v1 comma-separated form was removed (see the migration guide)"
            )));
        }
        match trimmed.parse::<i64>() {
            Ok(n) if n > 0 && trimmed.chars().all(|c| c.is_ascii_digit()) => Ok(MessageId(n)),
            _ => Err(Error::InvalidEditMessageId(format!("'{trimmed}' is not a positive integer"))),
        }
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Forum topic id. Non-negative integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThreadId(i64);

impl ThreadId {
    pub fn get(self) -> i64 {
        self.0
    }
}

impl FromStr for ThreadId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let trimmed = s.trim();
        if trimmed.is_empty() || !trimmed.chars().all(|c| c.is_ascii_digit()) {
            return Err(Error::InvalidThreadId(format!("'{trimmed}'")));
        }
        trimmed
            .parse::<i64>()
            .map(ThreadId)
            .map_err(|_| Error::InvalidThreadId(format!("'{trimmed}'")))
    }
}

impl fmt::Display for ThreadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The set of statuses that should produce a notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotifyOn(Vec<Status>);

impl NotifyOn {
    /// v1's default: everything except `skipped`.
    pub fn default_set() -> Self {
        NotifyOn(vec![Status::Success, Status::Failure, Status::Cancelled])
    }

    pub fn contains(&self, status: Status) -> bool {
        self.0.contains(&status)
    }

    pub fn statuses(&self) -> &[Status] {
        &self.0
    }
}

impl Default for NotifyOn {
    fn default() -> Self {
        Self::default_set()
    }
}

impl FromStr for NotifyOn {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Ok(NotifyOn::default_set());
        }
        // `any` and `all` are interchangeable shorthands for the full set.
        if trimmed == "any" || trimmed == "all" {
            return Ok(NotifyOn(Status::ALL.to_vec()));
        }
        let mut out = Vec::new();
        for item in trimmed.split(',') {
            let item = item.trim();
            let status = item.parse::<Status>().map_err(|_| {
                Error::InvalidNotifyOn(format!(
                    "'{item}' not in {{success,failure,cancelled,skipped}}"
                ))
            })?;
            if !out.contains(&status) {
                out.push(status);
            }
        }
        Ok(NotifyOn(out))
    }
}

impl fmt::Display for NotifyOn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let joined: Vec<&str> = self.0.iter().map(|s| s.as_str()).collect();
        f.write_str(&joined.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_roundtrip() {
        for s in Status::ALL {
            assert_eq!(s.as_str().parse::<Status>().unwrap(), s);
        }
    }

    #[test]
    fn status_rejects_unknown() {
        let err = "SUCCESS".parse::<Status>().unwrap_err();
        assert_eq!(err.exit_code(), 12);
    }

    #[test]
    fn parse_mode_upgrades_legacy_markdown() {
        let parsed = ParseMode::parse("Markdown").unwrap();
        assert_eq!(parsed.mode, ParseMode::MarkdownV2);
        assert!(parsed.upgraded_from_legacy_markdown);
    }

    #[test]
    fn parse_mode_none_has_no_wire_value() {
        assert_eq!(ParseMode::None.wire_value(), None);
        assert_eq!(ParseMode::Html.wire_value(), Some("HTML"));
    }

    #[test]
    fn chat_id_accepts_negative_integer_and_username() {
        assert_eq!("-1001234567890".parse::<ChatId>().unwrap().as_str(), "-1001234567890");
        assert_eq!("@my_channel".parse::<ChatId>().unwrap().as_str(), "@my_channel");
        assert_eq!("  42  ".parse::<ChatId>().unwrap().as_str(), "42");
    }

    #[test]
    fn chat_id_rejects_csv_with_migration_hint() {
        let err = "123,456".parse::<ChatId>().unwrap_err();
        assert_eq!(err.exit_code(), 11);
        assert!(err.to_string().contains("exactly one chat"));
    }

    #[test]
    fn chat_id_rejects_short_username_and_junk() {
        for bad in ["@abc", "@way_too_long_username_that_exceeds_limit", "12a", "-", "@bad-char"] {
            assert!(bad.parse::<ChatId>().is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn chat_id_json_is_number_for_integers() {
        assert_eq!("-100".parse::<ChatId>().unwrap().to_json(), serde_json::json!(-100));
        assert_eq!(
            "@chan_name".parse::<ChatId>().unwrap().to_json(),
            serde_json::json!("@chan_name")
        );
    }

    #[test]
    fn message_id_rejects_csv_and_zero() {
        assert_eq!("42".parse::<MessageId>().unwrap().get(), 42);
        assert!("42,43".parse::<MessageId>().is_err());
        assert!("0".parse::<MessageId>().is_err());
        assert!("-1".parse::<MessageId>().is_err());
    }

    #[test]
    fn thread_id_accepts_zero_rejects_negative() {
        assert_eq!("0".parse::<ThreadId>().unwrap().get(), 0);
        assert!("-1".parse::<ThreadId>().is_err());
    }

    #[test]
    fn notify_on_any_expands_to_all() {
        for keyword in ["any", "all"] {
            let n: NotifyOn = keyword.parse().unwrap();
            assert_eq!(n.statuses(), &Status::ALL);
        }
    }

    #[test]
    fn notify_on_trims_and_dedups() {
        let n: NotifyOn = " success , failure ,success".parse().unwrap();
        assert_eq!(n.statuses(), &[Status::Success, Status::Failure]);
    }

    #[test]
    fn notify_on_rejects_unknown_status() {
        let err = "success,bogus".parse::<NotifyOn>().unwrap_err();
        assert_eq!(err.exit_code(), 14);
    }
}
