//! The retry loop: four attempts, exponential backoff with jitter, 429 honoured.

use std::time::Duration;

use crate::actions::Level;
use crate::telegram::error::{Attempt, Disposition, RetryReason, classify};

/// Total attempts, matching v1: one primary plus three retries.
pub const DEFAULT_MAX_ATTEMPTS: u32 = 4;
/// Upper bound applied to a server-requested `retry_after`, in seconds.
pub const DEFAULT_MAX_RETRY_AFTER: u64 = 60;

/// Sleeping, behind a trait so tests do not actually wait.
pub trait Clock {
    fn sleep(&self, duration: Duration);
}

/// The real one.
pub struct SystemClock;

impl Clock for SystemClock {
    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

/// Knobs for the loop.
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub max_retry_after: u64,
    /// Adds up to 50% of the computed backoff. Disabled in tests for determinism.
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy {
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            max_retry_after: DEFAULT_MAX_RETRY_AFTER,
            jitter: true,
        }
    }
}

/// What the loop concluded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub ok: bool,
    pub message_id: Option<i64>,
    /// Last HTTP status observed; 0 when every attempt failed at the transport level.
    pub http_status: u16,
    /// Empty on success.
    pub error: Option<String>,
    pub attempts: u32,
}

/// Runs `attempt_fn` until it succeeds, is terminal, or the attempt budget runs out.
///
/// `log` receives every retry decision so the caller can annotate the workflow log without
/// this module knowing whether it is running under Actions.
pub fn execute<F, C>(
    mut attempt_fn: F,
    policy: RetryPolicy,
    clock: &C,
    log: &dyn Fn(Level, &str),
) -> Outcome
where
    F: FnMut(u32) -> Attempt,
    C: Clock + ?Sized,
{
    let mut jitter = Jitter::new();
    let mut last_status = 0u16;
    let mut last_error = String::from("send failed");

    for attempt in 1..=policy.max_attempts {
        let result = attempt_fn(attempt);
        last_status = result.status;
        last_error = result.error_text();

        match classify(&result) {
            Disposition::Success => {
                return Outcome {
                    ok: true,
                    message_id: result.message_id(),
                    http_status: 200,
                    error: None,
                    attempts: attempt,
                };
            }
            Disposition::Terminal => {
                log(
                    Level::Error,
                    &format!("Telegram returned {} (no retry): {last_error}", result.status),
                );
                return Outcome {
                    ok: false,
                    message_id: None,
                    http_status: result.status,
                    error: Some(last_error),
                    attempts: attempt,
                };
            }
            Disposition::Retry(reason) => {
                if attempt == policy.max_attempts {
                    break;
                }
                let delay = match reason {
                    RetryReason::RateLimited => {
                        let requested = result.retry_after();
                        let capped = requested.min(policy.max_retry_after);
                        if capped < requested {
                            log(
                                Level::Warning,
                                &format!("429 retry_after={requested}s capped at {capped}s"),
                            );
                        }
                        Duration::from_secs(capped)
                    }
                    RetryReason::Transient => backoff(attempt, policy.jitter, &mut jitter),
                };
                log(
                    Level::Warning,
                    &format!(
                        "attempt {attempt}/{} failed (http={}, {last_error}); retrying in {:.1}s",
                        policy.max_attempts,
                        result.status,
                        delay.as_secs_f64()
                    ),
                );
                clock.sleep(delay);
            }
        }
    }

    log(
        Level::Warning,
        &format!(
            "send failed after {} attempts (last http_status={last_status})",
            policy.max_attempts
        ),
    );
    Outcome {
        ok: false,
        message_id: None,
        http_status: last_status,
        error: Some(last_error),
        attempts: policy.max_attempts,
    }
}

/// `1s, 2s, 4s, …` plus up to 50% jitter.
///
/// Jitter matters once more than one job notifies the same chat: without it every retry in
/// the fleet lands on the same second and re-triggers the rate limit that caused it.
fn backoff(attempt: u32, use_jitter: bool, jitter: &mut Jitter) -> Duration {
    let base_ms = 1000u64 << (attempt - 1);
    if !use_jitter {
        return Duration::from_millis(base_ms);
    }
    Duration::from_millis(base_ms + jitter.next_below(base_ms / 2))
}

/// A xorshift PRNG. Jitter needs spread, not cryptographic quality, and this keeps the
/// dependency tree at zero for something that would otherwise pull in `rand`.
struct Jitter(u64);

impl Jitter {
    fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64 ^ (d.as_secs().rotate_left(17)))
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Jitter(seed | 1)
    }

    fn next_below(&mut self, bound: u64) -> u64 {
        if bound == 0 {
            return 0;
        }
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % bound
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::cell::RefCell;

    #[derive(Default)]
    struct FakeClock {
        slept: RefCell<Vec<Duration>>,
    }

    impl Clock for FakeClock {
        fn sleep(&self, duration: Duration) {
            self.slept.borrow_mut().push(duration);
        }
    }

    fn silent() -> impl Fn(Level, &str) {
        |_, _| {}
    }

    fn ok_attempt() -> Attempt {
        Attempt {
            status: 200,
            body: Some(json!({"ok": true, "result": {"message_id": 7}})),
            retry_after_header: None,
            network_error: None,
        }
    }

    fn status_attempt(status: u16, description: &str) -> Attempt {
        Attempt {
            status,
            body: Some(json!({"ok": false, "description": description})),
            retry_after_header: None,
            network_error: None,
        }
    }

    fn policy() -> RetryPolicy {
        RetryPolicy { jitter: false, ..RetryPolicy::default() }
    }

    #[test]
    fn first_attempt_success_does_not_sleep() {
        let clock = FakeClock::default();
        let out = execute(|_| ok_attempt(), policy(), &clock, &silent());
        assert_eq!(
            out,
            Outcome { ok: true, message_id: Some(7), http_status: 200, error: None, attempts: 1 }
        );
        assert!(clock.slept.borrow().is_empty());
    }

    #[test]
    fn transient_failures_back_off_exponentially_then_give_up() {
        let clock = FakeClock::default();
        let out = execute(|_| status_attempt(503, "busy"), policy(), &clock, &silent());
        assert!(!out.ok);
        assert_eq!(out.attempts, 4);
        assert_eq!(out.http_status, 503);
        assert_eq!(out.error.as_deref(), Some("HTTP 503: busy"));
        assert_eq!(
            *clock.slept.borrow(),
            vec![Duration::from_secs(1), Duration::from_secs(2), Duration::from_secs(4)]
        );
    }

    #[test]
    fn recovers_on_a_later_attempt() {
        let clock = FakeClock::default();
        let out = execute(
            |n| if n < 3 { status_attempt(500, "oops") } else { ok_attempt() },
            policy(),
            &clock,
            &silent(),
        );
        assert!(out.ok);
        assert_eq!(out.attempts, 3);
        assert_eq!(clock.slept.borrow().len(), 2);
    }

    #[test]
    fn terminal_status_stops_immediately() {
        let clock = FakeClock::default();
        let out = execute(|_| status_attempt(401, "Unauthorized"), policy(), &clock, &silent());
        assert_eq!(out.attempts, 1);
        assert_eq!(out.http_status, 401);
        assert_eq!(out.error.as_deref(), Some("Unauthorized"));
        assert!(clock.slept.borrow().is_empty());
    }

    #[test]
    fn rate_limit_honours_retry_after_and_the_cap() {
        let clock = FakeClock::default();
        let attempt = || Attempt {
            status: 429,
            body: Some(
                json!({"ok": false, "description": "Too Many Requests", "parameters": {"retry_after": 300}}),
            ),
            retry_after_header: None,
            network_error: None,
        };
        let out = execute(
            |_| attempt(),
            RetryPolicy { max_retry_after: 30, ..policy() },
            &clock,
            &silent(),
        );
        assert!(!out.ok);
        assert_eq!(*clock.slept.borrow(), vec![Duration::from_secs(30); 3]);
    }

    #[test]
    fn network_failures_report_status_zero() {
        let clock = FakeClock::default();
        let out = execute(|_| Attempt::network("timed out"), policy(), &clock, &silent());
        assert_eq!(out.http_status, 0);
        assert_eq!(out.error.as_deref(), Some("network error (timed out)"));
    }

    #[test]
    fn jitter_stays_within_half_the_base_delay() {
        let mut j = Jitter::new();
        for attempt in 1..=3u32 {
            let base = Duration::from_millis(1000u64 << (attempt - 1));
            let d = backoff(attempt, true, &mut j);
            assert!(d >= base, "{d:?} < {base:?}");
            assert!(d < base + base / 2, "{d:?} too large for base {base:?}");
        }
    }
}
