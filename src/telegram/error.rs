//! What one HTTP attempt produced, and how to read Telegram's error shape out of it.

use serde_json::Value;

/// The outcome of a single POST — never an `Err`, because a transport failure is just
/// another attempt outcome the retry loop has to weigh.
#[derive(Debug, Clone)]
pub struct Attempt {
    /// HTTP status, or 0 when the request never got a response.
    pub status: u16,
    /// Parsed response body, when there was one and it was JSON.
    pub body: Option<Value>,
    /// `Retry-After` response header, in seconds.
    pub retry_after_header: Option<u64>,
    /// Transport-level failure description (DNS, TLS, timeout, connection reset).
    pub network_error: Option<String>,
}

impl Attempt {
    pub fn network(error: impl Into<String>) -> Self {
        Attempt {
            status: 0,
            body: None,
            retry_after_header: None,
            network_error: Some(error.into()),
        }
    }

    /// True when Telegram answered 200 with `ok: true`.
    pub fn is_ok_true(&self) -> bool {
        self.status == 200
            && self.body.as_ref().and_then(|b| b.get("ok")).and_then(Value::as_bool) == Some(true)
    }

    /// `result.message_id` from a successful response.
    pub fn message_id(&self) -> Option<i64> {
        self.body.as_ref()?.get("result")?.get("message_id")?.as_i64()
    }

    /// Telegram's human-readable `description`, when present.
    pub fn description(&self) -> Option<String> {
        self.body.as_ref()?.get("description")?.as_str().map(str::to_string)
    }

    /// Retry delay in seconds: the body's `parameters.retry_after` wins, then the
    /// `Retry-After` header, then a one-second floor.
    ///
    /// v1 read only the body. Telegram's proxies and self-hosted Bot API servers send the
    /// header instead, and ignoring it turned a polite 429 into four hammering retries.
    pub fn retry_after(&self) -> u64 {
        let from_body = self
            .body
            .as_ref()
            .and_then(|b| b.get("parameters"))
            .and_then(|p| p.get("retry_after"))
            .and_then(Value::as_u64);
        from_body.or(self.retry_after_header).unwrap_or(1)
    }

    /// The message stored in the `error` output when this attempt is the last one.
    pub fn error_text(&self) -> String {
        if let Some(net) = &self.network_error {
            return format!("network error ({net})");
        }
        match (self.status, self.description()) {
            (200, Some(d)) => d,
            (200, None) => "200 ok=false".to_string(),
            (s, Some(d)) if (500..600).contains(&s) => format!("HTTP {s}: {d}"),
            (s, Some(d)) => {
                let _ = s;
                d
            }
            (s, None) => format!("HTTP {s}"),
        }
    }
}

/// How the retry loop should treat an attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// Delivered.
    Success,
    /// Worth another attempt after a delay.
    Retry(RetryReason),
    /// Telegram will answer the same way forever; stop.
    Terminal,
}

/// Why an attempt is being retried — decides how the delay is computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryReason {
    /// 429: honour the server's requested delay.
    RateLimited,
    /// 5xx, `200 ok=false`, transport failure, or an unclassified status.
    Transient,
}

/// Classifies an attempt.
///
/// 4xx other than 429 is terminal: a bad token or a malformed message is not going to fix
/// itself, and retrying only delays the failure the user needs to see.
pub fn classify(attempt: &Attempt) -> Disposition {
    if attempt.network_error.is_some() {
        return Disposition::Retry(RetryReason::Transient);
    }
    match attempt.status {
        200 if attempt.is_ok_true() => Disposition::Success,
        200 => Disposition::Retry(RetryReason::Transient),
        429 => Disposition::Retry(RetryReason::RateLimited),
        s if (500..600).contains(&s) => Disposition::Retry(RetryReason::Transient),
        s if (400..500).contains(&s) => Disposition::Terminal,
        _ => Disposition::Retry(RetryReason::Transient),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn with_body(status: u16, body: Value) -> Attempt {
        Attempt { status, body: Some(body), retry_after_header: None, network_error: None }
    }

    #[test]
    fn ok_true_is_success_and_carries_message_id() {
        let a = with_body(200, json!({"ok": true, "result": {"message_id": 4242}}));
        assert_eq!(classify(&a), Disposition::Success);
        assert_eq!(a.message_id(), Some(4242));
    }

    #[test]
    fn ok_false_at_200_is_retried() {
        let a = with_body(200, json!({"ok": false, "description": "nope"}));
        assert_eq!(classify(&a), Disposition::Retry(RetryReason::Transient));
        assert_eq!(a.error_text(), "nope");
    }

    #[test]
    fn four_xx_other_than_429_is_terminal() {
        for status in [400u16, 401, 403, 404] {
            let a = with_body(status, json!({"ok": false, "description": "bad"}));
            assert_eq!(classify(&a), Disposition::Terminal, "status {status}");
            assert_eq!(a.error_text(), "bad");
        }
    }

    #[test]
    fn five_xx_is_transient_and_keeps_the_status_in_the_message() {
        let a = with_body(503, json!({"ok": false, "description": "busy"}));
        assert_eq!(classify(&a), Disposition::Retry(RetryReason::Transient));
        assert_eq!(a.error_text(), "HTTP 503: busy");
    }

    #[test]
    fn retry_after_prefers_body_then_header_then_one() {
        let mut a = with_body(429, json!({"parameters": {"retry_after": 7}}));
        a.retry_after_header = Some(30);
        assert_eq!(a.retry_after(), 7);

        let mut b = with_body(429, json!({"ok": false}));
        b.retry_after_header = Some(30);
        assert_eq!(b.retry_after(), 30);

        let c = with_body(429, json!({"ok": false}));
        assert_eq!(c.retry_after(), 1);
    }

    #[test]
    fn network_failures_are_transient_and_report_zero() {
        let a = Attempt::network("connection reset");
        assert_eq!(a.status, 0);
        assert_eq!(classify(&a), Disposition::Retry(RetryReason::Transient));
        assert_eq!(a.error_text(), "network error (connection reset)");
    }
}
