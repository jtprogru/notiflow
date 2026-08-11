//! GitHub Actions workflow commands: masking, annotations, outputs, step summary.
//!
//! This is the only part of v1's `lib.sh` that survives into v2 — everything else it did
//! (dependency checks, bash version gate) has no meaning for a static binary.

use std::fs::OpenOptions;
use std::io::Write;

use crate::error::{Error, Result};
use crate::redact;

/// True when the process is running inside a GitHub Actions job.
///
/// Drives two decisions: where outputs go, and whether `api_base` is restricted to the
/// allowlist. Both only make sense when a workflow — not a person — is in control.
pub fn is_github_actions() -> bool {
    std::env::var("GITHUB_ACTIONS").map(|v| v == "true").unwrap_or(false)
}

/// Severity of a workflow annotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Debug,
    Notice,
    Warning,
    Error,
}

impl Level {
    fn command(self) -> &'static str {
        match self {
            Level::Debug => "debug",
            Level::Notice => "notice",
            Level::Warning => "warning",
            Level::Error => "error",
        }
    }
}

/// Hides `value` from every subsequent line of the step log.
pub fn add_mask(value: &str) {
    if value.is_empty() {
        return;
    }
    redact::register(value);
    if is_github_actions() {
        // Deliberately not routed through `redact`: this line is what teaches the runner
        // the secret, and the value is registered above so nothing else can print it.
        eprintln!("::add-mask::{}", escape_data(value));
    }
}

/// Writes an annotation to stderr — as a workflow command in CI, plain text elsewhere.
pub fn log(level: Level, message: &str) {
    let message = redact::redact(message);
    if is_github_actions() {
        eprintln!("::{}::{}", level.command(), escape_data(&message));
    } else if level != Level::Debug || std::env::var_os("NOTIFLOW_DEBUG").is_some() {
        eprintln!("notiflow: {}: {}", level.command(), message);
    }
}

/// Appends `key=value` to `$GITHUB_OUTPUT`, using the heredoc form for multi-line values.
///
/// No-op when `$GITHUB_OUTPUT` is unset, which is the normal CLI case.
pub fn set_output(key: &str, value: &str) -> Result<()> {
    let Some(path) = std::env::var_os("GITHUB_OUTPUT") else {
        return Ok(());
    };
    let value = redact::redact(value);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| Error::Io(format!("cannot open $GITHUB_OUTPUT: {e}")))?;

    if value.contains('\n') {
        // The delimiter must not occur in the value; a fixed random-looking string plus a
        // length suffix is enough for values we generate ourselves.
        let delimiter = format!("notiflow_eof_{}", value.len());
        writeln!(file, "{key}<<{delimiter}\n{value}\n{delimiter}")
    } else {
        writeln!(file, "{key}={value}")
    }
    .map_err(|e| Error::Io(format!("cannot write $GITHUB_OUTPUT: {e}")))
}

/// Appends markdown to `$GITHUB_STEP_SUMMARY`. No-op when unset.
pub fn step_summary(markdown: &str) -> Result<()> {
    let Some(path) = std::env::var_os("GITHUB_STEP_SUMMARY") else {
        return Ok(());
    };
    let markdown = redact::redact(markdown);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| Error::Io(format!("cannot open $GITHUB_STEP_SUMMARY: {e}")))?;
    writeln!(file, "{markdown}")
        .map_err(|e| Error::Io(format!("cannot write $GITHUB_STEP_SUMMARY: {e}")))
}

/// Percent-encodes the characters that would otherwise terminate a workflow command.
fn escape_data(value: &str) -> String {
    value.replace('%', "%25").replace('\r', "%0D").replace('\n', "%0A")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_data_neutralises_newlines_and_percent() {
        assert_eq!(escape_data("a\nb"), "a%0Ab");
        assert_eq!(escape_data("100%"), "100%25");
        assert_eq!(escape_data("a\r\nb"), "a%0D%0Ab");
        // Percent must be escaped before the others, or `%0A` would become `%250A`.
        assert_eq!(escape_data("%\n"), "%25%0A");
    }

    #[test]
    fn set_output_is_a_noop_without_the_env_var() {
        // Safety: single-threaded test that restores nothing because it removes only.
        unsafe { std::env::remove_var("GITHUB_OUTPUT") };
        assert!(set_output("ok", "true").is_ok());
    }
}
