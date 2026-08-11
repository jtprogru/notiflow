//! A scripted stand-in for the Telegram Bot API.
//!
//! Small enough to read in one sitting, which matters more here than features: when an
//! integration test fails, the first question is always whether the mock or the binary is
//! at fault, and that question should be cheap to answer.

// This module is compiled separately into every integration test binary, so anything one
// suite does not touch looks dead to that build. The allow is about the compilation model,
// not about unused code.
#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

/// One request the mock saw.
#[derive(Debug, Clone)]
pub struct Recorded {
    pub path: String,
    pub body: String,
}

impl Recorded {
    /// The request body as JSON. Panics on malformed input — that is a test failure.
    pub fn json(&self) -> serde_json::Value {
        serde_json::from_str(&self.body).expect("request body should be JSON")
    }

    /// The Bot API method, i.e. the last path segment.
    pub fn method(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or("")
    }
}

/// A canned reply.
#[derive(Debug, Clone)]
pub struct Reply {
    pub status: u16,
    pub body: String,
    /// Sent as the `Retry-After` header when present.
    pub retry_after: Option<u64>,
}

impl Reply {
    pub fn ok(message_id: i64) -> Self {
        Reply {
            status: 200,
            body: format!(r#"{{"ok":true,"result":{{"message_id":{message_id}}}}}"#),
            retry_after: None,
        }
    }

    pub fn error(status: u16, description: &str) -> Self {
        Reply {
            status,
            body: format!(r#"{{"ok":false,"description":"{description}"}}"#),
            retry_after: None,
        }
    }

    pub fn rate_limited(retry_after_body: u64) -> Self {
        Reply {
            status: 429,
            body: format!(
                r#"{{"ok":false,"description":"Too Many Requests","parameters":{{"retry_after":{retry_after_body}}}}}"#
            ),
            retry_after: None,
        }
    }

    pub fn with_retry_after_header(mut self, seconds: u64) -> Self {
        self.retry_after = Some(seconds);
        self
    }
}

/// A running mock. Shuts its thread down on drop.
pub struct MockTelegram {
    port: u16,
    requests: Arc<Mutex<Vec<Recorded>>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl MockTelegram {
    /// Starts a server that answers with `replies` in order, repeating the last one once
    /// the script runs out.
    pub fn start(replies: Vec<Reply>) -> Self {
        assert!(!replies.is_empty(), "the mock needs at least one reply");
        let server = tiny_http::Server::http("127.0.0.1:0").expect("bind loopback");
        let port = server.server_addr().to_ip().expect("ipv4 address").port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));

        let thread_requests = Arc::clone(&requests);
        let thread_stop = Arc::clone(&stop);
        let handle = std::thread::spawn(move || {
            let mut index = 0usize;
            while !thread_stop.load(Ordering::Relaxed) {
                let Ok(Some(mut request)) = server.recv_timeout(Duration::from_millis(100)) else {
                    continue;
                };
                let mut body = String::new();
                let _ = request.as_reader().read_to_string(&mut body);
                thread_requests
                    .lock()
                    .expect("mock request log")
                    .push(Recorded { path: request.url().to_string(), body });

                let reply = &replies[index.min(replies.len() - 1)];
                index += 1;

                let mut response = tiny_http::Response::from_string(reply.body.clone())
                    .with_status_code(reply.status)
                    .with_header(
                        tiny_http::Header::from_bytes(
                            &b"Content-Type"[..],
                            &b"application/json"[..],
                        )
                        .expect("static header"),
                    );
                if let Some(seconds) = reply.retry_after {
                    response = response.with_header(
                        tiny_http::Header::from_bytes(
                            &b"Retry-After"[..],
                            seconds.to_string().as_bytes(),
                        )
                        .expect("numeric header"),
                    );
                }
                let _ = request.respond(response);
            }
        });

        MockTelegram { port, requests, stop, handle: Some(handle) }
    }

    /// The value to pass as `--api-base`.
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn requests(&self) -> Vec<Recorded> {
        self.requests.lock().expect("mock request log").clone()
    }

    pub fn request_count(&self) -> usize {
        self.requests().len()
    }

    /// The last request, or a panic with a message better than `unwrap` would give.
    pub fn last_request(&self) -> Recorded {
        self.requests().pop().expect("the mock received no requests")
    }
}

impl Drop for MockTelegram {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// A token shaped like a real one, so redaction has something realistic to hide.
pub const TEST_TOKEN: &str = "123456789:AAHdqTcvCH1vGWJxfSeofSAs0K5PALDsaw";

/// A `notiflow` command with every ambient GitHub and notiflow variable stripped.
///
/// Without this, running the suite inside a workflow would silently change what is tested.
pub fn notiflow() -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::cargo_bin("notiflow").expect("binary is built");
    for key in [
        "GITHUB_ACTIONS",
        "GITHUB_OUTPUT",
        "GITHUB_STEP_SUMMARY",
        "GITHUB_REPOSITORY",
        "GITHUB_WORKFLOW",
        "GITHUB_JOB",
        "GITHUB_RUN_ID",
        "GITHUB_RUN_NUMBER",
        "GITHUB_SHA",
        "GITHUB_ACTOR",
        "GITHUB_REF",
        "GITHUB_REF_NAME",
        "GITHUB_EVENT_NAME",
        "GITHUB_SERVER_URL",
        "NOTIFLOW_BOT_TOKEN",
        "NOTIFLOW_CHAT_ID",
        "NOTIFLOW_API_BASE",
        "NOTIFLOW_CONFIG",
        "NOTIFLOW_DEBUG",
    ] {
        cmd.env_remove(key);
    }
    // A stray user config file on the developer's machine must not reach the test.
    cmd.env("NOTIFLOW_CONFIG", "/nonexistent/notiflow/config.toml");
    cmd
}
