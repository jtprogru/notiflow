//! Error taxonomy and the exit codes it maps to.
//!
//! Codes 10–16 are inherited verbatim from notiflow v1 (`scripts/validate.sh`) so a
//! workflow that branches on them keeps working across the v1 → v2 upgrade. 20
//! (`UNSUPPORTED_BASH`) and 22 (`MISSING_DEPENDENCY`) belonged to the bash runtime and
//! are permanently reserved: the Rust binary has no bash and no external dependencies,
//! but nothing else may claim those numbers.

use std::fmt;

/// Exit code emitted for a terminal delivery failure when `fail_on_error` is set.
pub const EXIT_SEND_FAILED: i32 = 1;
/// Reserved. v1 used it for `UNSUPPORTED_BASH`; v2 never emits it.
pub const EXIT_RESERVED_UNSUPPORTED_BASH: i32 = 20;
/// Reserved. v1 used it for `MISSING_DEPENDENCY`; v2 never emits it.
pub const EXIT_RESERVED_MISSING_DEPENDENCY: i32 = 22;

/// Everything that can go wrong before or during a send.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// bot_token, chat_id or status missing.
    #[error("MISSING_REQUIRED_INPUT: {0}")]
    MissingRequiredInput(String),

    /// chat_id is neither an integer nor a `@username` of 4–32 word characters.
    #[error("INVALID_CHAT_ID: {0}")]
    InvalidChatId(String),

    /// status is not one of success|failure|cancelled|skipped.
    #[error("INVALID_STATUS: {0}")]
    InvalidStatus(String),

    /// parse_mode is not one of MarkdownV2|HTML|Markdown|none.
    #[error("INVALID_PARSE_MODE: {0}")]
    InvalidParseMode(String),

    /// notify_on contains a value outside the status set.
    #[error("INVALID_NOTIFY_ON: {0}")]
    InvalidNotifyOn(String),

    /// message_thread_id is not a non-negative integer.
    #[error("INVALID_THREAD_ID: {0}")]
    InvalidThreadId(String),

    /// edit_message_id is not a positive integer.
    #[error("INVALID_EDIT_MESSAGE_ID: {0}")]
    InvalidEditMessageId(String),

    /// The config file exists but cannot be read, parsed, or names a missing profile.
    #[error("CONFIG_ERROR: {0}")]
    Config(String),

    /// A well-formed but unusable argument (bad URL, zero timeout, unreadable file).
    #[error("INVALID_ARGUMENT: {0}")]
    InvalidArgument(String),

    /// Telegram never accepted the message.
    #[error("SEND_FAILED: {0}")]
    SendFailed(String),

    /// Local I/O failed (stdin, message file, `$GITHUB_OUTPUT`).
    #[error("IO_ERROR: {0}")]
    Io(String),
}

impl Error {
    /// The process exit code this error maps to.
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::SendFailed(_) => EXIT_SEND_FAILED,
            Error::MissingRequiredInput(_) => 10,
            Error::InvalidChatId(_) => 11,
            Error::InvalidStatus(_) => 12,
            Error::InvalidParseMode(_) => 13,
            Error::InvalidNotifyOn(_) => 14,
            Error::InvalidThreadId(_) => 15,
            Error::InvalidEditMessageId(_) => 16,
            Error::Config(_) => 17,
            Error::InvalidArgument(_) => 18,
            Error::Io(_) => 30,
        }
    }

    /// Stable machine-readable label, the part before the colon in [`Display`].
    pub fn code_name(&self) -> &'static str {
        match self {
            Error::SendFailed(_) => "SEND_FAILED",
            Error::MissingRequiredInput(_) => "MISSING_REQUIRED_INPUT",
            Error::InvalidChatId(_) => "INVALID_CHAT_ID",
            Error::InvalidStatus(_) => "INVALID_STATUS",
            Error::InvalidParseMode(_) => "INVALID_PARSE_MODE",
            Error::InvalidNotifyOn(_) => "INVALID_NOTIFY_ON",
            Error::InvalidThreadId(_) => "INVALID_THREAD_ID",
            Error::InvalidEditMessageId(_) => "INVALID_EDIT_MESSAGE_ID",
            Error::Config(_) => "CONFIG_ERROR",
            Error::InvalidArgument(_) => "INVALID_ARGUMENT",
            Error::Io(_) => "IO_ERROR",
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

/// Every exit code the binary can produce, in the order the docs table lists them.
///
/// `make gen` renders this into `docs/src/content/docs/**/exit-codes` — the table cannot
/// drift from the code because it is not written by hand.
pub const EXIT_CODE_TABLE: &[(i32, &str, &str)] = &[
    (0, "OK", "Message delivered, or skipped by notify_on, or fail_on_error=false"),
    (1, "SEND_FAILED", "Telegram never accepted the message and fail_on_error=true"),
    (2, "USAGE", "Command-line parsing failed (emitted by clap)"),
    (10, "MISSING_REQUIRED_INPUT", "bot_token, chat_id or status is empty"),
    (11, "INVALID_CHAT_ID", "chat_id is not an integer or @username (a comma is rejected in v2)"),
    (12, "INVALID_STATUS", "status is not success|failure|cancelled|skipped"),
    (13, "INVALID_PARSE_MODE", "parse_mode is not MarkdownV2|HTML|Markdown|none"),
    (14, "INVALID_NOTIFY_ON", "notify_on lists a value outside the status set"),
    (15, "INVALID_THREAD_ID", "message_thread_id is not a non-negative integer"),
    (16, "INVALID_EDIT_MESSAGE_ID", "edit_message_id is not a positive integer"),
    (17, "CONFIG_ERROR", "Config file unreadable, malformed, or the profile does not exist"),
    (18, "INVALID_ARGUMENT", "Argument is well-formed but unusable (bad api_base, zero timeout)"),
    (20, "RESERVED", "v1 UNSUPPORTED_BASH — never emitted by v2"),
    (22, "RESERVED", "v1 MISSING_DEPENDENCY — never emitted by v2"),
    (30, "IO_ERROR", "Reading stdin or a message file failed, or an output file is not writable"),
];

/// Convenience alias used across the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Wrapper that renders an error the way the human output format wants it.
pub struct Rendered<'a>(pub &'a Error);

impl fmt::Display for Rendered<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "notiflow: {}", self.0)
    }
}
