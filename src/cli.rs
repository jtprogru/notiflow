//! The command line: argument definitions and the five commands they drive.
//!
//! `main.rs` does nothing but call [`dispatch`] and turn its error into an exit code, so
//! every decision about defaults, precedence and output lives here.

use std::io::Read;
use std::path::PathBuf;

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

use crate::actions::{self, Level};
use crate::config::{Config, Invocation, Need, Settings, default_config_path, load_file};
use crate::context::{self, Context};
use crate::error::{EXIT_CODE_TABLE, Error, Result};
use crate::model::Status;
use crate::output::{Format, Report};
use crate::telegram::client::{Client, HttpTransport};
use crate::{RunOptions, safe_println};

/// Send a Telegram message from CI or from a terminal.
#[derive(Debug, Parser)]
#[command(
    name = "notiflow",
    version,
    about = "Telegram notifier for CI and the terminal",
    long_about = "notiflow sends a Telegram message when something finishes — a workflow job, \
                  a deploy script, a long build. The same binary backs the notiflow GitHub \
                  Action and the standalone CLI.",
    propagate_version = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Send a message.
    Send(SendArgs),
    /// Replace the text of a message sent earlier.
    Edit(EditArgs),
    /// Render a template locally and print it — no network, no token needed.
    Render(RenderArgs),
    /// Check the bot token with getMe.
    Whoami(WhoamiArgs),
    /// Print a shell completion script.
    Completions(CompletionsArgs),
    /// Emit a generated documentation fragment. Used by `make gen`.
    #[command(hide = true)]
    Docs(DocsArgs),
}

/// Flags shared by every command that produces a report.
#[derive(Debug, Clone, Default, Args)]
pub struct CommonArgs {
    /// Config file to read (default: $XDG_CONFIG_HOME/notiflow/config.toml).
    #[arg(long, value_name = "PATH", global = true)]
    pub config: Option<PathBuf>,

    /// Profile inside the config file.
    #[arg(long, value_name = "NAME", global = true)]
    pub profile: Option<String>,

    /// Output format.
    #[arg(long, value_name = "FORMAT", default_value = "human", global = true)]
    pub output: OutputFormat,

    /// Shorthand for --output json.
    #[arg(long, global = true, conflicts_with = "output")]
    pub json: bool,

    /// Print nothing on success.
    #[arg(short = 'q', long, global = true)]
    pub quiet: bool,

    /// Increase log verbosity; repeat for debug-level detail.
    #[arg(short = 'v', long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,
}

/// How the result is reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum OutputFormat {
    /// One readable line.
    #[default]
    Human,
    /// A single JSON object.
    Json,
    /// $GITHUB_OUTPUT entries and a step summary.
    Github,
}

impl From<OutputFormat> for Format {
    fn from(f: OutputFormat) -> Self {
        match f {
            OutputFormat::Human => Format::Human,
            OutputFormat::Json => Format::Json,
            OutputFormat::Github => Format::Github,
        }
    }
}

/// Credentials and transport knobs.
#[derive(Debug, Clone, Default, Args)]
pub struct DeliveryArgs {
    /// Bot token. Prefer the environment variable: arguments are visible in `ps`.
    #[arg(long, env = "NOTIFLOW_BOT_TOKEN", value_name = "TOKEN", hide_env_values = true)]
    pub bot_token: Option<String>,

    /// Destination chat: an integer id or @channelusername. Exactly one.
    ///
    /// `allow_hyphen_values` is not optional here: supergroup ids start with `-100`, and
    /// without it clap reads `--chat-id -1001234567890` as an unknown flag.
    #[arg(
        short = 'c',
        long,
        env = "NOTIFLOW_CHAT_ID",
        value_name = "CHAT",
        allow_hyphen_values = true
    )]
    pub chat_id: Option<String>,

    /// Bot API base URL, for a self-hosted Bot API server.
    #[arg(long, env = "NOTIFLOW_API_BASE", value_name = "URL")]
    pub api_base: Option<String>,

    /// Whole-request timeout in seconds.
    #[arg(long, value_name = "SECONDS")]
    pub timeout: Option<u64>,

    /// Connection timeout in seconds.
    #[arg(long, value_name = "SECONDS")]
    pub connect_timeout: Option<u64>,

    /// Retries after the first attempt.
    #[arg(long, value_name = "N")]
    pub retries: Option<u32>,

    /// Cap applied to a 429's requested retry delay, in seconds.
    #[arg(long, value_name = "SECONDS")]
    pub max_retry_after: Option<u64>,
}

/// What to say and how to format it.
#[derive(Debug, Clone, Default, Args)]
pub struct MessageArgs {
    /// Verbatim message text. No placeholders, no escaping. `-` reads stdin.
    #[arg(short = 'm', long, value_name = "TEXT")]
    pub message: Option<String>,

    /// Read the verbatim message from a file.
    #[arg(long, value_name = "PATH", conflicts_with = "message")]
    pub message_file: Option<PathBuf>,

    /// Read the verbatim message from stdin.
    #[arg(long, conflicts_with_all = ["message", "message_file"])]
    pub stdin: bool,

    /// Template with {{.Field}} placeholders, used for every status.
    #[arg(short = 'T', long = "template", value_name = "TEXT")]
    pub message_template: Option<String>,

    /// Template used when status=success.
    #[arg(long, value_name = "TEXT")]
    pub template_success: Option<String>,

    /// Template used when status=failure.
    #[arg(long, value_name = "TEXT")]
    pub template_failure: Option<String>,

    /// Template used when status=cancelled.
    #[arg(long, value_name = "TEXT")]
    pub template_cancelled: Option<String>,

    /// Template used when status=skipped.
    #[arg(long, value_name = "TEXT")]
    pub template_skipped: Option<String>,

    /// Status being reported.
    #[arg(short = 's', long, value_name = "STATUS", default_value = "success")]
    pub status: String,

    /// Statuses that should produce a message; `any` means all of them.
    #[arg(long, value_name = "CSV")]
    pub notify_on: Option<String>,

    /// MarkdownV2, HTML, Markdown (an alias for MarkdownV2), or none.
    #[arg(long, value_name = "MODE")]
    pub parse_mode: Option<String>,

    /// Forum topic id.
    #[arg(long, value_name = "ID", allow_hyphen_values = true)]
    pub thread_id: Option<String>,

    /// Deliver without a notification sound.
    #[arg(long)]
    pub silent: bool,

    /// Show link previews (they are suppressed by default).
    #[arg(long)]
    pub preview: bool,

    /// Suppress link previews. The default; accepted for symmetry with --preview.
    #[arg(long, conflicts_with = "preview")]
    pub no_preview: bool,
}

/// `notiflow send`
#[derive(Debug, Clone, Default, Args)]
pub struct SendArgs {
    #[command(flatten)]
    pub common: CommonArgs,
    #[command(flatten)]
    pub delivery: DeliveryArgs,
    #[command(flatten)]
    pub message: MessageArgs,

    /// Render and validate, then stop without contacting Telegram.
    #[arg(long)]
    pub dry_run: bool,

    /// Exit non-zero when delivery ultimately fails.
    #[arg(long)]
    pub fail_on_error: bool,
}

/// `notiflow edit`
#[derive(Debug, Clone, Args)]
pub struct EditArgs {
    /// Id of the message to rewrite, typically the message_id of an earlier send.
    /// `allow_hyphen_values` keeps a negative id reaching the validator, so it fails with
    /// INVALID_EDIT_MESSAGE_ID rather than clap's generic usage error.
    #[arg(long, value_name = "ID", required = true, allow_hyphen_values = true)]
    pub message_id: String,

    #[command(flatten)]
    pub send: SendArgs,
}

/// `notiflow render`
#[derive(Debug, Clone, Args)]
pub struct RenderArgs {
    #[command(flatten)]
    pub common: CommonArgs,
    #[command(flatten)]
    pub message: MessageArgs,

    /// Override one placeholder, e.g. --set Repo=owner/name. Repeatable.
    #[arg(long = "set", value_name = "KEY=VALUE")]
    pub overrides: Vec<String>,

    /// Also print which template was chosen and which placeholders were unknown.
    #[arg(long)]
    pub explain: bool,
}

/// `notiflow whoami`
#[derive(Debug, Clone, Args)]
pub struct WhoamiArgs {
    #[command(flatten)]
    pub common: CommonArgs,
    #[command(flatten)]
    pub delivery: DeliveryArgs,
}

/// `notiflow completions`
#[derive(Debug, Clone, Args)]
pub struct CompletionsArgs {
    /// Shell to generate for.
    #[arg(value_name = "SHELL")]
    pub shell: Shell,
}

/// `notiflow docs`
#[derive(Debug, Clone, Args)]
pub struct DocsArgs {
    /// Which fragment to print.
    #[arg(value_name = "FRAGMENT")]
    pub fragment: DocsFragment,
}

/// Generated documentation fragments. Each one is the single source of truth for a table
/// the handbook would otherwise hand-maintain and let rot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DocsFragment {
    /// The exit-code table.
    ExitCodes,
    /// The placeholder table.
    Placeholders,
    /// The full CLI reference.
    Cli,
}

/// Parses the command line and runs the requested command.
pub fn dispatch() -> Result<i32> {
    let cli = Cli::parse();
    warn_if_token_on_argv();

    match cli.command {
        Command::Send(args) => run_send(args, None),
        Command::Edit(args) => {
            let message_id = args.message_id.clone();
            run_send(args.send, Some(message_id))
        }
        Command::Render(args) => run_render(args),
        Command::Whoami(args) => run_whoami(args),
        Command::Completions(args) => {
            let mut cmd = Cli::command();
            let bin = cmd.get_name().to_string();
            clap_complete::generate(args.shell, &mut cmd, bin, &mut std::io::stdout());
            Ok(0)
        }
        Command::Docs(args) => {
            print!("{}", docs_fragment(args.fragment));
            Ok(0)
        }
    }
}

fn run_send(args: SendArgs, edit_message_id: Option<String>) -> Result<i32> {
    apply_verbosity(args.common.verbose);
    let format = resolve_format(&args.common);
    let message = resolve_message_body(&args.message)?;

    let settings = merge_settings(
        settings_from(Some(&args.delivery), &args.message, args.fail_on_error),
        &args.common,
    )?;
    let invocation =
        Invocation { status: Some(args.message.status.clone()), message, edit_message_id };
    let config = Config::build(settings, invocation, actions::is_github_actions(), Need::Full)?;

    let report = crate::run(&config, RunOptions { dry_run: args.dry_run })?;
    if !args.common.quiet || format != Format::Human {
        report.emit(format)?;
    }

    if !report.ok && !report.skipped && !report.dry_run && config.fail_on_error {
        return Err(Error::SendFailed(
            report.error.clone().unwrap_or_else(|| "delivery failed".into()),
        ));
    }
    Ok(0)
}

fn run_render(args: RenderArgs) -> Result<i32> {
    apply_verbosity(args.common.verbose);
    let message = resolve_message_body(&args.message)?;
    let settings = merge_settings(settings_from(None, &args.message, false), &args.common)?;
    let invocation =
        Invocation { status: Some(args.message.status.clone()), message, edit_message_id: None };
    let config = Config::build(settings, invocation, false, Need::TemplateOnly)?;

    let mut ctx = Context::discover(config.status, true);
    for pair in &args.overrides {
        let (key, value) = pair.split_once('=').ok_or_else(|| {
            Error::InvalidArgument(format!("--set expects KEY=VALUE, got '{pair}'"))
        })?;
        if !context::is_known_placeholder(key) {
            return Err(Error::InvalidArgument(format!("--set: '{key}' is not a placeholder")));
        }
        ctx.set(key, value);
    }

    let rendered = crate::render_message(&config, &ctx);
    if args.explain {
        safe_println!("# template: {}", rendered.source.as_str());
        safe_println!("# parse_mode: {}", config.parse_mode);
        safe_println!("# truncated: {}", rendered.truncated);
        if !rendered.unknown_placeholders.is_empty() {
            safe_println!("# unknown: {}", rendered.unknown_placeholders.join(", "));
        }
    }
    safe_println!("{}", rendered.text);
    Ok(0)
}

fn run_whoami(args: WhoamiArgs) -> Result<i32> {
    apply_verbosity(args.common.verbose);
    let format = resolve_format(&args.common);
    let settings = merge_settings(
        settings_from(Some(&args.delivery), &MessageArgs::default(), false),
        &args.common,
    )?;
    let invocation = Invocation { status: Some("success".into()), ..Default::default() };
    let config =
        Config::build(settings, invocation, actions::is_github_actions(), Need::TokenOnly)?;

    let transport = HttpTransport::new(
        std::time::Duration::from_secs(config.connect_timeout),
        std::time::Duration::from_secs(config.timeout),
    );
    let attempt = Client::new(&config.api_base, &config.bot_token, transport).get_me();

    if !attempt.is_ok_true() {
        return Err(Error::SendFailed(attempt.error_text()));
    }
    let result = attempt.body.as_ref().and_then(|b| b.get("result")).cloned().unwrap_or_default();
    match format {
        Format::Json => safe_println!("{}", serde_json::to_string(&result).unwrap_or_default()),
        _ => {
            let username = result.get("username").and_then(|v| v.as_str()).unwrap_or("?");
            let name = result.get("first_name").and_then(|v| v.as_str()).unwrap_or("?");
            let id = result.get("id").and_then(|v| v.as_i64()).unwrap_or_default();
            safe_println!("{name} (@{username}, id={id})");
        }
    }
    Ok(0)
}

/// Builds the CLI layer of settings from parsed arguments.
fn settings_from(
    delivery: Option<&DeliveryArgs>,
    message: &MessageArgs,
    fail_on_error: bool,
) -> Settings {
    let delivery = delivery.cloned().unwrap_or_default();
    Settings {
        bot_token: delivery.bot_token,
        chat_id: delivery.chat_id,
        api_base: delivery.api_base,
        timeout: delivery.timeout,
        connect_timeout: delivery.connect_timeout,
        retries: delivery.retries,
        max_retry_after: delivery.max_retry_after,
        parse_mode: message.parse_mode.clone(),
        notify_on: message.notify_on.clone(),
        message_template: message.message_template.clone(),
        template_success: message.template_success.clone(),
        template_failure: message.template_failure.clone(),
        template_cancelled: message.template_cancelled.clone(),
        template_skipped: message.template_skipped.clone(),
        message_thread_id: message.thread_id.clone(),
        // `--silent` and `--preview` are absence-means-default flags, so only a set flag
        // becomes an opinion the lower layers cannot override.
        disable_notification: message.silent.then_some(true),
        disable_web_page_preview: if message.preview {
            Some(false)
        } else if message.no_preview {
            Some(true)
        } else {
            None
        },
        fail_on_error: fail_on_error.then_some(true),
    }
}

/// Layers the config file underneath the command-line settings.
fn merge_settings(mut cli: Settings, common: &CommonArgs) -> Result<Settings> {
    let explicit = common.config.is_some();
    let path = common.config.clone().or_else(default_config_path);
    let file = load_file(path.as_deref(), common.profile.as_deref(), explicit)?;
    cli.fill_from(&file);
    Ok(cli)
}

/// Resolves `--message`, `--message-file`, `--stdin` and the `-` convention into one body.
fn resolve_message_body(args: &MessageArgs) -> Result<Option<String>> {
    if args.stdin || args.message.as_deref() == Some("-") {
        return read_stdin().map(Some);
    }
    if let Some(path) = &args.message_file {
        let body = std::fs::read_to_string(path)
            .map_err(|e| Error::Io(format!("cannot read {}: {e}", path.display())))?;
        return Ok(Some(body));
    }
    Ok(args.message.clone())
}

fn read_stdin() -> Result<String> {
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| Error::Io(format!("cannot read stdin: {e}")))?;
    // A pipeline almost always ends in a newline that the sender did not mean to include.
    Ok(buf.trim_end_matches(['\n', '\r']).to_string())
}

fn resolve_format(common: &CommonArgs) -> Format {
    if common.json { Format::Json } else { common.output.into() }
}

fn apply_verbosity(level: u8) {
    if level >= 2 {
        // Safety: called once, before any thread is spawned.
        unsafe { std::env::set_var("NOTIFLOW_DEBUG", "1") };
    }
}

/// Warns when the token arrived through argv, where every other user on the box can read it.
fn warn_if_token_on_argv() {
    let on_argv = std::env::args().any(|a| a == "--bot-token" || a.starts_with("--bot-token="));
    if on_argv {
        actions::log(
            Level::Warning,
            "--bot-token puts the token in argv, which any process on this machine can read; \
             prefer NOTIFLOW_BOT_TOKEN or a 0600 config file",
        );
    }
}

/// Renders one generated documentation fragment.
pub fn docs_fragment(fragment: DocsFragment) -> String {
    match fragment {
        DocsFragment::ExitCodes => {
            let mut out = String::from("| Code | Name | Meaning |\n|---:|---|---|\n");
            for (code, name, meaning) in EXIT_CODE_TABLE {
                out.push_str(&format!("| {code} | `{name}` | {meaning} |\n"));
            }
            out
        }
        DocsFragment::Placeholders => {
            let mut out = String::from("| Placeholder | Value |\n|---|---|\n");
            for (name, description) in context::PLACEHOLDERS {
                out.push_str(&format!("| `{{{{.{name}}}}}` | {description} |\n"));
            }
            out
        }
        DocsFragment::Cli => render_cli_reference(),
    }
}

/// Turns the clap command tree into the CLI reference page body.
fn render_cli_reference() -> String {
    let cmd = Cli::command();
    let mut out = String::new();
    out.push_str(&format!(
        "`notiflow` — {}\n\n",
        cmd.get_about().map(|s| s.to_string()).unwrap_or_default()
    ));

    for sub in cmd.get_subcommands() {
        if sub.is_hide_set() {
            continue;
        }
        out.push_str(&format!("## `notiflow {}`\n\n", sub.get_name()));
        if let Some(about) = sub.get_about() {
            out.push_str(&format!("{about}\n\n"));
        }
        out.push_str("| Flag | Value | Description |\n|---|---|---|\n");
        for arg in sub.get_arguments() {
            if arg.is_hide_set() {
                continue;
            }
            let flag = match (arg.get_short(), arg.get_long()) {
                (Some(s), Some(l)) => format!("`-{s}`, `--{l}`"),
                (None, Some(l)) => format!("`--{l}`"),
                (Some(s), None) => format!("`-{s}`"),
                (None, None) => format!("`<{}>`", arg.get_id()),
            };
            let value = arg
                .get_value_names()
                .and_then(|n| n.first().map(|v| format!("`{v}`")))
                .unwrap_or_else(|| "—".to_string());
            let help = arg.get_help().map(|h| h.to_string()).unwrap_or_default().replace('\n', " ");
            out.push_str(&format!("| {flag} | {value} | {help} |\n"));
        }
        out.push('\n');
    }
    out
}

/// Prints a skip report without touching the network. Exposed for the smoke tests.
pub fn skipped_report(chat_id: &str, format: Format) -> Result<()> {
    Report::skipped(chat_id).emit(format)
}

/// The default status used when none is given, so docs and code agree.
pub const DEFAULT_CLI_STATUS: Status = Status::Success;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clap_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn exit_code_fragment_lists_every_code() {
        let md = docs_fragment(DocsFragment::ExitCodes);
        for (code, _, _) in EXIT_CODE_TABLE {
            assert!(md.contains(&format!("| {code} |")), "missing exit code {code}");
        }
    }

    #[test]
    fn placeholder_fragment_lists_every_placeholder() {
        let md = docs_fragment(DocsFragment::Placeholders);
        for (name, _) in context::PLACEHOLDERS {
            assert!(md.contains(&format!("{{{{.{name}}}}}")), "missing placeholder {name}");
        }
    }

    #[test]
    fn cli_reference_covers_the_public_subcommands() {
        let md = docs_fragment(DocsFragment::Cli);
        for name in ["send", "edit", "render", "whoami", "completions"] {
            assert!(md.contains(&format!("## `notiflow {name}`")), "missing {name}");
        }
        assert!(!md.contains("notiflow docs`"), "hidden command leaked into the reference");
    }

    #[test]
    fn preview_flags_produce_opposite_opinions() {
        let mut m = MessageArgs { preview: true, ..Default::default() };
        assert_eq!(settings_from(None, &m, false).disable_web_page_preview, Some(false));
        m = MessageArgs { no_preview: true, ..Default::default() };
        assert_eq!(settings_from(None, &m, false).disable_web_page_preview, Some(true));
        m = MessageArgs::default();
        assert_eq!(settings_from(None, &m, false).disable_web_page_preview, None);
    }

    #[test]
    fn unset_flags_do_not_shadow_the_config_file() {
        // `--silent` absent must stay None, otherwise a profile could never enable it.
        let s = settings_from(None, &MessageArgs::default(), false);
        assert_eq!(s.disable_notification, None);
        assert_eq!(s.fail_on_error, None);
    }
}
