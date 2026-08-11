//! One place that guarantees the bot token never reaches a stream a human can read.
//!
//! Inside a workflow `::add-mask::` covers the step log, but the CLI has no such thing —
//! and an HTTP error can easily carry the request URL, which contains `bot<TOKEN>`. Every
//! byte notiflow prints goes through [`redact`] first.

use std::sync::{OnceLock, RwLock};

const PLACEHOLDER: &str = "***";

fn registry() -> &'static RwLock<Vec<String>> {
    static SECRETS: OnceLock<RwLock<Vec<String>>> = OnceLock::new();
    SECRETS.get_or_init(|| RwLock::new(Vec::new()))
}

/// Registers a value to be scrubbed from all future output.
///
/// Short values are ignored: redacting a two-character "secret" would turn ordinary text
/// into a wall of asterisks without protecting anything.
pub fn register(secret: &str) {
    if secret.len() < 8 {
        return;
    }
    if let Ok(mut guard) = registry().write() {
        let owned = secret.to_string();
        if !guard.contains(&owned) {
            guard.push(owned);
        }
    }
}

/// Forgets every registered secret. Test-only; the binary never un-redacts.
#[cfg(test)]
pub fn reset() {
    if let Ok(mut guard) = registry().write() {
        guard.clear();
    }
}

/// Replaces every registered secret, then any surviving `/bot<token>/` path segment.
///
/// The second pass is belt-and-braces: it catches a token that reached a URL without ever
/// being registered, which is exactly the failure mode that leaks credentials.
pub fn redact(input: &str) -> String {
    let mut out = input.to_string();
    if let Ok(guard) = registry().read() {
        for secret in guard.iter() {
            if out.contains(secret.as_str()) {
                out = out.replace(secret.as_str(), PLACEHOLDER);
            }
        }
    }
    redact_bot_path(&out)
}

/// Rewrites `…/bot123456:AAA…/sendMessage` to `…/bot***/sendMessage`.
fn redact_bot_path(input: &str) -> String {
    let needle = "/bot";
    let mut out = String::with_capacity(input.len());
    let mut rest = input;

    while let Some(pos) = rest.find(needle) {
        let (head, tail) = rest.split_at(pos + needle.len());
        // A bot token is `<digits>:<35-ish urlsafe chars>`; require the colon so we do not
        // mangle an innocent path like `/bots/list`.
        let seg_end = tail.find('/').unwrap_or(tail.len());
        let segment = &tail[..seg_end];
        if segment.contains(':') && segment.len() >= 8 && segment != PLACEHOLDER {
            out.push_str(head);
            out.push_str(PLACEHOLDER);
            rest = &tail[seg_end..];
        } else {
            out.push_str(head);
            rest = tail;
        }
    }
    out.push_str(rest);
    out
}

/// `println!` that scrubs first.
#[macro_export]
macro_rules! safe_println {
    ($($arg:tt)*) => {{
        println!("{}", $crate::redact::redact(&format!($($arg)*)));
    }};
}

/// `eprintln!` that scrubs first.
#[macro_export]
macro_rules! safe_eprintln {
    ($($arg:tt)*) => {{
        eprintln!("{}", $crate::redact::redact(&format!($($arg)*)));
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    // The registry is process-global, so these run under one lock rather than as
    // independent #[test] functions racing each other.
    #[test]
    fn redaction_behaviour() {
        reset();
        assert_eq!(redact("nothing to hide"), "nothing to hide");

        register("123456:AAHdqTcvCH1vGWJxfSeofSAs0K5PALDsaw");
        let leaked = "POST https://api.telegram.org/bot123456:AAHdqTcvCH1vGWJxfSeofSAs0K5PALDsaw/sendMessage failed";
        let clean = redact(leaked);
        assert!(!clean.contains("AAHdqTcv"), "{clean}");
        assert!(clean.contains("/bot***/sendMessage"), "{clean}");

        // Short values are not worth redacting and would destroy ordinary output.
        register("abc");
        assert_eq!(redact("abc def"), "abc def");
        reset();
    }

    #[test]
    fn unregistered_token_in_url_is_still_scrubbed() {
        reset();
        let s = "url=https://api.telegram.org/bot999:ZZZZZZZZZZZZZZZ/getMe";
        assert_eq!(redact(s), "url=https://api.telegram.org/bot***/getMe");
        reset();
    }

    #[test]
    fn innocent_bot_paths_are_left_alone() {
        reset();
        assert_eq!(redact("https://example.com/bots/list"), "https://example.com/bots/list");
        reset();
    }
}
