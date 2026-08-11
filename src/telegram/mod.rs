//! Everything that talks to the Telegram Bot API.
//!
//! v1 had an aggregating layer here that walked a CSV of chat ids and stitched their
//! results into one output. v2 sends to exactly one chat, so the layer is gone and this
//! module is a straight line: build request → post with retries → report.

pub mod client;
pub mod error;
pub mod request;
pub mod retry;

pub use client::{Client, HttpTransport, Transport};
pub use error::{Attempt, Disposition, RetryReason};
pub use request::Request;
pub use retry::{Clock, Outcome, RetryPolicy, SystemClock};

use crate::actions::Level;

/// Sends (or edits) one message, retrying per `policy`.
pub fn deliver<T: Transport, C: Clock>(
    client: &Client<T>,
    request: &Request,
    policy: RetryPolicy,
    clock: &C,
    log: &dyn Fn(Level, &str),
) -> Outcome {
    let endpoint = request.endpoint();
    let body = request.to_body();
    retry::execute(|_| client.call(endpoint, &body), policy, clock, log)
}
