//! notiflow — send a Telegram message from CI or from a terminal.
//!
//! The same pipeline backs both faces of the tool: resolve settings → decide whether this
//! status is worth notifying about → pick and render a template → truncate to Telegram's
//! limit → post with retries → report. `notiflow` the GitHub Action is this library with
//! its outputs pointed at `$GITHUB_OUTPUT`; `notiflow` the CLI is the same code printing
//! to a terminal.
//!
//! ```no_run
//! use notiflow::config::{Config, Invocation, Need, Settings};
//! use notiflow::{RunOptions, run};
//!
//! let settings = Settings {
//!     bot_token: Some(std::env::var("NOTIFLOW_BOT_TOKEN").unwrap()),
//!     chat_id: Some("-1001234567890".into()),
//!     ..Default::default()
//! };
//! let invocation = Invocation { status: Some("success".into()), ..Default::default() };
//! let config = Config::build(settings, invocation, false, Need::Full).unwrap();
//! let report = run(&config, RunOptions::default()).unwrap();
//! println!("delivered: {}", report.ok);
//! ```

pub mod actions;
pub mod cli;
pub mod config;
pub mod context;
pub mod error;
pub mod escape;
pub mod model;
pub mod output;
pub mod redact;
pub mod telegram;
pub mod template;
pub mod truncate;

use std::time::Duration;

use crate::actions::Level;
use crate::config::Config;
use crate::context::Context;
use crate::error::Result;
use crate::output::Report;
use crate::telegram::client::{Client, HttpTransport};
use crate::telegram::request::Request;
use crate::telegram::retry::{RetryPolicy, SystemClock};
use crate::template::{TemplateSource, render};
use crate::truncate::truncate;

/// Per-invocation switches that are not settings.
#[derive(Debug, Clone, Copy, Default)]
pub struct RunOptions {
    /// Render and validate, but make no HTTP request.
    pub dry_run: bool,
}

/// The rendered message plus how it was produced.
#[derive(Debug, Clone)]
pub struct RenderResult {
    pub text: String,
    pub source: TemplateSource,
    /// Placeholder names that were dropped. Already warned about by [`render_message`].
    pub unknown_placeholders: Vec<String>,
    /// True when the text had to be cut to fit Telegram's limit.
    pub truncated: bool,
}

/// Picks the active template and renders it against `ctx`.
///
/// A `message` input is passed through verbatim: no substitution, no escaping. That is
/// deliberate — it is the escape hatch for text the caller has already formatted, and it
/// is why the docs warn that placeholders do not work there.
pub fn render_message(config: &Config, ctx: &Context) -> RenderResult {
    let selected = config.templates.select(config.status);

    let (text, unknown) = match selected.source {
        TemplateSource::VerbatimMessage => (selected.body.clone(), Vec::new()),
        _ => {
            let rendered = render(&selected.body, ctx, config.parse_mode);
            (rendered.text, rendered.unknown)
        }
    };

    for name in &unknown {
        actions::log(Level::Warning, &format!("UNKNOWN_PLACEHOLDER:{name}"));
    }

    let truncated_text = truncate(&text, config.parse_mode);
    let truncated = truncated_text != text;
    if truncated {
        actions::log(
            Level::Warning,
            "message exceeded Telegram's 4096-unit limit and was truncated",
        );
    }

    RenderResult {
        text: truncated_text,
        source: selected.source,
        unknown_placeholders: unknown,
        truncated,
    }
}

/// Builds the Bot API request for `config` and an already-rendered `text`.
pub fn build_request(config: &Config, text: String) -> Request {
    Request {
        chat_id: config.chat_id.clone(),
        text,
        parse_mode: config.parse_mode,
        disable_web_page_preview: config.disable_web_page_preview,
        disable_notification: config.disable_notification,
        message_thread_id: config.message_thread_id,
        edit_message_id: config.edit_message_id,
    }
}

/// Runs the whole pipeline and returns what happened.
///
/// Never returns `Err` for a delivery failure — that is a [`Report`] with `ok: false`, so
/// the caller decides whether `fail_on_error` turns it into a non-zero exit.
pub fn run(config: &Config, options: RunOptions) -> Result<Report> {
    if !config.should_notify() {
        actions::log(
            Level::Notice,
            &format!("skipping (status={} not in notify_on={})", config.status, config.notify_on),
        );
        return Ok(Report::skipped(config.chat_id.to_string()));
    }

    let ctx = Context::discover(config.status, !actions::is_github_actions());
    let rendered = render_message(config, &ctx);
    let request = build_request(config, rendered.text);

    if options.dry_run {
        return Ok(Report {
            ok: false,
            skipped: false,
            message_id: None,
            http_status: 0,
            error: None,
            chat_id: config.chat_id.to_string(),
            attempts: 0,
            dry_run: true,
            request: Some(request.to_json()),
        });
    }

    let transport = HttpTransport::new(
        Duration::from_secs(config.connect_timeout),
        Duration::from_secs(config.timeout),
    );
    let client = Client::new(&config.api_base, &config.bot_token, transport);
    let policy = RetryPolicy {
        max_attempts: config.retries + 1,
        max_retry_after: config.max_retry_after,
        jitter: true,
    };
    let outcome = telegram::deliver(&client, &request, policy, &SystemClock, &|level, msg| {
        actions::log(level, msg)
    });

    Ok(Report {
        ok: outcome.ok,
        skipped: false,
        message_id: outcome.message_id,
        http_status: outcome.http_status,
        error: outcome.error,
        chat_id: config.chat_id.to_string(),
        attempts: outcome.attempts,
        dry_run: false,
        request: None,
    })
}

/// The crate version, used by `--version` and by the Action's install sanity check.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
