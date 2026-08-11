//! HTTP transport for the Bot API.
//!
//! `ureq` with rustls: one blocking POST and the process exits. An async runtime would add
//! ~6 MB to every release archive to solve a concurrency problem this tool does not have.

use std::time::Duration;

use serde_json::Value;

use crate::telegram::error::Attempt;

/// Default TCP/TLS handshake budget.
pub const DEFAULT_CONNECT_TIMEOUT: u64 = 5;
/// Default whole-request budget.
pub const DEFAULT_TOTAL_TIMEOUT: u64 = 15;

/// One HTTP POST. Behind a trait so the retry loop can be driven by a scripted fake.
pub trait Transport {
    fn post_json(&self, url: &str, body: &str) -> Attempt;
}

/// The real transport.
pub struct HttpTransport {
    agent: ureq::Agent,
}

impl HttpTransport {
    pub fn new(connect_timeout: Duration, total_timeout: Duration) -> Self {
        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(connect_timeout))
            .timeout_global(Some(total_timeout))
            // Non-2xx is data, not an exception: the retry loop needs the status and body.
            .http_status_as_error(false)
            .user_agent(concat!("notiflow/", env!("CARGO_PKG_VERSION")))
            .build();
        HttpTransport { agent: config.into() }
    }
}

impl Default for HttpTransport {
    fn default() -> Self {
        HttpTransport::new(
            Duration::from_secs(DEFAULT_CONNECT_TIMEOUT),
            Duration::from_secs(DEFAULT_TOTAL_TIMEOUT),
        )
    }
}

impl Transport for HttpTransport {
    fn post_json(&self, url: &str, body: &str) -> Attempt {
        let response = self.agent.post(url).header("Content-Type", "application/json").send(body);

        let mut response = match response {
            Ok(r) => r,
            Err(e) => return Attempt::network(e.to_string()),
        };

        let status = response.status().as_u16();
        let retry_after_header = response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<u64>().ok());

        let text = match response.body_mut().read_to_string() {
            Ok(t) => t,
            Err(e) => return Attempt::network(format!("reading response body: {e}")),
        };
        let body = serde_json::from_str::<Value>(&text).ok();

        Attempt { status, body, retry_after_header, network_error: None }
    }
}

/// Bot API endpoint builder plus the calls notiflow makes.
pub struct Client<T: Transport> {
    api_base: String,
    token: String,
    transport: T,
}

impl<T: Transport> Client<T> {
    pub fn new(api_base: impl Into<String>, token: impl Into<String>, transport: T) -> Self {
        Client {
            api_base: api_base.into().trim_end_matches('/').to_string(),
            token: token.into(),
            transport,
        }
    }

    /// Full URL for a method. Kept private-ish: the token in it must never be logged.
    fn url(&self, method: &str) -> String {
        format!("{}/bot{}/{}", self.api_base, self.token, method)
    }

    /// Posts a pre-serialised JSON body to `method`.
    pub fn call(&self, method: &str, body: &str) -> Attempt {
        self.transport.post_json(&self.url(method), body)
    }

    /// `getMe` — the token check behind `notiflow whoami`.
    pub fn get_me(&self) -> Attempt {
        self.transport.post_json(&self.url("getMe"), "{}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct SpyTransport {
        seen: RefCell<Vec<(String, String)>>,
    }

    impl Transport for SpyTransport {
        fn post_json(&self, url: &str, body: &str) -> Attempt {
            self.seen.borrow_mut().push((url.to_string(), body.to_string()));
            Attempt { status: 200, body: None, retry_after_header: None, network_error: None }
        }
    }

    #[test]
    fn url_joins_base_token_and_method() {
        let client = Client::new("https://api.telegram.org/", "123:ABC", SpyTransport::default());
        client.call("sendMessage", "{}");
        let seen = client.transport.seen.borrow();
        assert_eq!(seen[0].0, "https://api.telegram.org/bot123:ABC/sendMessage");
    }

    #[test]
    fn get_me_posts_an_empty_object() {
        let client = Client::new("http://127.0.0.1:8080", "t:oken", SpyTransport::default());
        client.get_me();
        let seen = client.transport.seen.borrow();
        assert_eq!(seen[0].0, "http://127.0.0.1:8080/bott:oken/getMe");
        assert_eq!(seen[0].1, "{}");
    }
}
