//! Settings: where they come from, how they are layered, and what they must satisfy.
//!
//! Precedence is flags → environment → config-file profile → config-file defaults →
//! built-in defaults. clap resolves the first two itself (each flag declares its `env`),
//! so this module only has to merge what is left and turn strings into the newtypes from
//! [`crate::model`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::actions::{self, Level};
use crate::error::{Error, Result};
use crate::model::{ChatId, MessageId, NotifyOn, ParseMode, Status, ThreadId};
use crate::telegram::client::{DEFAULT_CONNECT_TIMEOUT, DEFAULT_TOTAL_TIMEOUT};
use crate::telegram::retry::{DEFAULT_MAX_ATTEMPTS, DEFAULT_MAX_RETRY_AFTER};
use crate::template::TemplateInputs;

/// Telegram's own API host. Anything else needs a deliberate `--api-base`.
pub const DEFAULT_API_BASE: &str = "https://api.telegram.org";

/// One layer of settings. Every field optional — absent means "ask the next layer".
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub bot_token: Option<String>,
    pub chat_id: Option<String>,
    pub parse_mode: Option<String>,
    pub notify_on: Option<String>,
    pub api_base: Option<String>,
    pub message_template: Option<String>,
    pub template_success: Option<String>,
    pub template_failure: Option<String>,
    pub template_cancelled: Option<String>,
    pub template_skipped: Option<String>,
    pub message_thread_id: Option<String>,
    pub disable_notification: Option<bool>,
    pub disable_web_page_preview: Option<bool>,
    pub connect_timeout: Option<u64>,
    pub timeout: Option<u64>,
    pub retries: Option<u32>,
    pub max_retry_after: Option<u64>,
    pub fail_on_error: Option<bool>,
}

macro_rules! fill {
    ($self:ident, $other:ident, $($field:ident),+ $(,)?) => {
        $( if $self.$field.is_none() { $self.$field = $other.$field.clone(); } )+
    };
}

impl Settings {
    /// Fills every unset field from `lower`. `self` is the higher-priority layer.
    pub fn fill_from(&mut self, lower: &Settings) {
        fill!(
            self,
            lower,
            bot_token,
            chat_id,
            parse_mode,
            notify_on,
            api_base,
            message_template,
            template_success,
            template_failure,
            template_cancelled,
            template_skipped,
            message_thread_id,
            disable_notification,
            disable_web_page_preview,
            connect_timeout,
            timeout,
            retries,
            max_retry_after,
            fail_on_error,
        );
    }
}

/// A parsed `config.toml`: top-level keys are the defaults, `[profile.<name>]` overrides.
#[derive(Debug, Clone, Default)]
pub struct FileConfig {
    pub defaults: Settings,
    pub profile: BTreeMap<String, Settings>,
}

impl FileConfig {
    /// Parses a config file.
    ///
    /// The `profile` table is lifted out by hand rather than with `#[serde(flatten)]`:
    /// flatten swallows unknown keys, which would turn `chat_ids = "-1"` into a config
    /// that silently does nothing instead of an error the user can see.
    pub fn parse(raw: &str) -> std::result::Result<FileConfig, String> {
        let mut table: toml::Table = raw.parse().map_err(|e| format!("{e}"))?;
        let profile_value = table.remove("profile");
        let defaults: Settings =
            toml::Value::Table(table).try_into().map_err(|e| format!("{e}"))?;
        let profile = match profile_value {
            Some(value) => value.try_into().map_err(|e| format!("[profile] {e}"))?,
            None => BTreeMap::new(),
        };
        Ok(FileConfig { defaults, profile })
    }
}

/// Path of the config file: `$NOTIFLOW_CONFIG`, else `$XDG_CONFIG_HOME/notiflow/config.toml`,
/// else `~/.config/notiflow/config.toml`.
pub fn default_config_path() -> Option<PathBuf> {
    if let Some(explicit) = std::env::var_os("NOTIFLOW_CONFIG") {
        return Some(PathBuf::from(explicit));
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("notiflow").join("config.toml"))
}

/// Loads the config file and merges the requested profile over the file's defaults.
///
/// A missing file is not an error unless the user named one explicitly — that is the
/// difference between "I have no config" and "my config is not where I said it was".
pub fn load_file(path: Option<&Path>, profile: Option<&str>, explicit: bool) -> Result<Settings> {
    let Some(path) = path else { return Ok(Settings::default()) };
    if !path.exists() {
        if explicit {
            return Err(Error::Config(format!("{} does not exist", path.display())));
        }
        if profile.is_some() {
            return Err(Error::Config(format!(
                "--profile was given but {} does not exist",
                path.display()
            )));
        }
        return Ok(Settings::default());
    }

    warn_if_world_readable(path);

    let raw = std::fs::read_to_string(path)
        .map_err(|e| Error::Config(format!("cannot read {}: {e}", path.display())))?;
    let parsed = FileConfig::parse(&raw)
        .map_err(|e| Error::Config(format!("cannot parse {}: {e}", path.display())))?;

    let mut merged = match profile {
        Some(name) => parsed.profile.get(name).cloned().ok_or_else(|| {
            let known: Vec<&str> = parsed.profile.keys().map(String::as_str).collect();
            Error::Config(format!(
                "profile '{name}' not found in {} (known: {})",
                path.display(),
                if known.is_empty() { "none".to_string() } else { known.join(", ") }
            ))
        })?,
        None => Settings::default(),
    };
    merged.fill_from(&parsed.defaults);
    Ok(merged)
}

/// Warns when a file that may hold a bot token is readable beyond its owner.
#[cfg(unix)]
fn warn_if_world_readable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let Ok(meta) = std::fs::metadata(path) else { return };
    let mode = meta.permissions().mode() & 0o777;
    if mode & 0o077 != 0 {
        actions::log(
            Level::Warning,
            &format!(
                "{} is mode {:04o}; a bot token there is readable by other users — chmod 600 it",
                path.display(),
                mode
            ),
        );
    }
}

#[cfg(not(unix))]
fn warn_if_world_readable(_path: &Path) {}

/// Resolves `api_base` under the policy for the current mode.
///
/// Inside Actions the allowlist stands: `NF_API_BASE` could be planted by an earlier step
/// in the same job, and following it would hand the bot token to whoever planted it. From
/// the CLI an arbitrary base is the user's own explicit choice — a self-hosted Bot API
/// server is a legitimate, documented setup.
pub fn resolve_api_base(candidate: &str, action_mode: bool) -> Result<String> {
    let candidate = candidate.trim();
    if candidate.is_empty() {
        return Ok(DEFAULT_API_BASE.to_string());
    }
    if !action_mode {
        if !(candidate.starts_with("http://") || candidate.starts_with("https://")) {
            return Err(Error::InvalidArgument(format!(
                "api_base '{candidate}' must start with http:// or https://"
            )));
        }
        return Ok(candidate.to_string());
    }
    if is_allowlisted(candidate) {
        return Ok(candidate.to_string());
    }
    actions::log(
        Level::Warning,
        &format!("ignoring api_base '{candidate}' (host not allowed under Actions); using default"),
    );
    Ok(DEFAULT_API_BASE.to_string())
}

/// api.telegram.org, or loopback so the smoke workflow can intercept.
///
/// The host must be followed by `/`, `:` or end-of-string, which is what stops
/// `https://api.telegram.org.evil.com` and `http://127.0.0.1@evil.com` from passing.
fn is_allowlisted(url: &str) -> bool {
    let after_scheme = match url.split_once("://") {
        Some(("https", rest)) => rest,
        Some(("http", rest)) => rest,
        _ => return false,
    };
    let host_and_port = after_scheme.split('/').next().unwrap_or("");
    if host_and_port.contains('@') {
        return false;
    }
    let host = host_and_port.split(':').next().unwrap_or("");
    match host {
        "api.telegram.org" => url.starts_with("https://"),
        "127.0.0.1" | "localhost" | "[::1]" => true,
        _ => false,
    }
}

/// Fully resolved, fully typed settings for one run.
#[derive(Debug, Clone)]
pub struct Config {
    pub bot_token: String,
    pub chat_id: ChatId,
    pub status: Status,
    pub parse_mode: ParseMode,
    pub notify_on: NotifyOn,
    pub api_base: String,
    pub templates: TemplateInputs,
    pub message_thread_id: Option<ThreadId>,
    pub edit_message_id: Option<MessageId>,
    pub disable_notification: bool,
    pub disable_web_page_preview: bool,
    pub connect_timeout: u64,
    pub timeout: u64,
    pub retries: u32,
    pub max_retry_after: u64,
    pub fail_on_error: bool,
}

/// The pieces that only the command line can supply, kept apart from [`Settings`] because
/// they are per-invocation rather than per-profile.
#[derive(Debug, Clone, Default)]
pub struct Invocation {
    pub status: Option<String>,
    pub message: Option<String>,
    pub edit_message_id: Option<String>,
}

/// Which credentials a command actually needs.
///
/// `notiflow render` never opens a socket and `notiflow whoami` has no destination, so
/// demanding a full set from them would be validation theatre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Need {
    /// A token and a destination: `send`, `edit`.
    Full,
    /// A token only: `whoami`.
    TokenOnly,
    /// Neither: `render`.
    TemplateOnly,
}

/// Stand-in destination for commands that never send. Never reaches the network.
const UNUSED_CHAT_ID: &str = "0";

impl Config {
    /// Validates a merged settings stack into a usable configuration.
    pub fn build(
        settings: Settings,
        invocation: Invocation,
        action_mode: bool,
        need: Need,
    ) -> Result<Config> {
        let bot_token = match settings.bot_token.filter(|t| !t.is_empty()) {
            Some(token) => {
                actions::add_mask(&token);
                token
            }
            None if need == Need::TemplateOnly => String::new(),
            None => return Err(Error::MissingRequiredInput("bot_token is required".into())),
        };

        let chat_id = match settings.chat_id.filter(|c| !c.is_empty()) {
            Some(raw) => raw.parse::<ChatId>()?,
            None if need == Need::Full => {
                return Err(Error::MissingRequiredInput("chat_id is required".into()));
            }
            None => UNUSED_CHAT_ID.parse::<ChatId>().expect("constant is a valid chat id"),
        };

        let status = invocation
            .status
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::MissingRequiredInput("status is required".into()))?
            .parse::<Status>()?;

        let parse_mode = match settings.parse_mode.filter(|p| !p.is_empty()) {
            Some(raw) => {
                let parsed = ParseMode::parse(&raw)?;
                if parsed.upgraded_from_legacy_markdown {
                    actions::log(
                        Level::Warning,
                        "parse_mode=Markdown upgraded to MarkdownV2 (notiflow escapes per V2 rules)",
                    );
                }
                parsed.mode
            }
            None => ParseMode::MarkdownV2,
        };

        let notify_on = match settings.notify_on.filter(|n| !n.is_empty()) {
            Some(raw) => raw.parse::<NotifyOn>()?,
            None => NotifyOn::default_set(),
        };

        let api_base = resolve_api_base(settings.api_base.as_deref().unwrap_or(""), action_mode)?;

        let message_thread_id = match settings.message_thread_id.filter(|t| !t.is_empty()) {
            Some(raw) => Some(raw.parse::<ThreadId>()?),
            None => None,
        };

        let edit_message_id = match invocation.edit_message_id.filter(|m| !m.is_empty()) {
            Some(raw) => Some(raw.parse::<MessageId>()?),
            None => None,
        };

        let timeout = settings.timeout.unwrap_or(DEFAULT_TOTAL_TIMEOUT);
        let connect_timeout = settings.connect_timeout.unwrap_or(DEFAULT_CONNECT_TIMEOUT);
        if timeout == 0 || connect_timeout == 0 {
            return Err(Error::InvalidArgument("timeout must be greater than zero".into()));
        }
        let retries = settings.retries.unwrap_or(DEFAULT_MAX_ATTEMPTS - 1);

        Ok(Config {
            bot_token,
            chat_id,
            status,
            parse_mode,
            notify_on,
            api_base,
            templates: TemplateInputs {
                message: invocation.message,
                message_template: settings.message_template,
                success: settings.template_success,
                failure: settings.template_failure,
                cancelled: settings.template_cancelled,
                skipped: settings.template_skipped,
            },
            message_thread_id,
            edit_message_id,
            disable_notification: settings.disable_notification.unwrap_or(false),
            disable_web_page_preview: settings.disable_web_page_preview.unwrap_or(true),
            connect_timeout,
            timeout,
            retries,
            max_retry_after: settings.max_retry_after.unwrap_or(DEFAULT_MAX_RETRY_AFTER),
            fail_on_error: settings.fail_on_error.unwrap_or(false),
        })
    }

    /// True when this run should produce a message at all.
    pub fn should_notify(&self) -> bool {
        self.notify_on.contains(self.status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal() -> Settings {
        Settings {
            bot_token: Some("123456:AAHdqTcvCH1vGWJxfSeofSAs0K5PALDsaw".into()),
            chat_id: Some("-100123".into()),
            ..Default::default()
        }
    }

    fn invocation(status: &str) -> Invocation {
        Invocation { status: Some(status.into()), ..Default::default() }
    }

    #[test]
    fn higher_layer_wins_field_by_field() {
        let mut high = Settings { chat_id: Some("1".into()), ..Default::default() };
        let low = Settings {
            chat_id: Some("2".into()),
            parse_mode: Some("HTML".into()),
            ..Default::default()
        };
        high.fill_from(&low);
        assert_eq!(high.chat_id.as_deref(), Some("1"));
        assert_eq!(high.parse_mode.as_deref(), Some("HTML"));
    }

    #[test]
    fn defaults_match_v1() {
        let cfg = Config::build(minimal(), invocation("success"), false, Need::Full).unwrap();
        assert_eq!(cfg.parse_mode, ParseMode::MarkdownV2);
        assert_eq!(cfg.notify_on, NotifyOn::default_set());
        assert!(cfg.disable_web_page_preview);
        assert!(!cfg.disable_notification);
        assert!(!cfg.fail_on_error);
        assert_eq!(cfg.api_base, DEFAULT_API_BASE);
        assert_eq!(cfg.retries, 3);
    }

    #[test]
    fn missing_required_inputs_map_to_ten() {
        let err = Config::build(Settings::default(), invocation("success"), false, Need::Full)
            .unwrap_err();
        assert_eq!(err.exit_code(), 10);

        let no_status =
            Config::build(minimal(), Invocation::default(), false, Need::Full).unwrap_err();
        assert_eq!(no_status.exit_code(), 10);
    }

    #[test]
    fn notify_on_gate() {
        let mut s = minimal();
        s.notify_on = Some("failure".into());
        let cfg = Config::build(s.clone(), invocation("success"), false, Need::Full).unwrap();
        assert!(!cfg.should_notify());
        let cfg = Config::build(s, invocation("failure"), false, Need::Full).unwrap();
        assert!(cfg.should_notify());
    }

    #[test]
    fn action_mode_allowlist_rejects_lookalikes() {
        for bad in [
            "https://api.telegram.org.evil.com",
            "http://127.0.0.1@evil.com",
            "https://evil.com/api.telegram.org",
            "ftp://api.telegram.org",
            "http://api.telegram.org",
        ] {
            assert_eq!(resolve_api_base(bad, true).unwrap(), DEFAULT_API_BASE, "{bad}");
        }
    }

    #[test]
    fn action_mode_allowlist_accepts_telegram_and_loopback() {
        for good in [
            "https://api.telegram.org",
            "https://api.telegram.org/",
            "http://127.0.0.1:8081",
            "http://localhost:9000/base",
        ] {
            assert_eq!(resolve_api_base(good, true).unwrap(), good, "{good}");
        }
    }

    #[test]
    fn cli_mode_allows_self_hosted_but_still_needs_a_scheme() {
        assert_eq!(
            resolve_api_base("https://bot.internal.example.com", false).unwrap(),
            "https://bot.internal.example.com"
        );
        assert!(resolve_api_base("bot.internal.example.com", false).is_err());
    }

    #[test]
    fn profile_overrides_file_defaults() {
        let dir = std::env::temp_dir().join(format!("notiflow-cfg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(
            &path,
            "chat_id = \"-1\"\nparse_mode = \"HTML\"\n\n[profile.work]\nchat_id = \"-2\"\n",
        )
        .unwrap();

        let merged = load_file(Some(&path), Some("work"), true).unwrap();
        assert_eq!(merged.chat_id.as_deref(), Some("-2"));
        assert_eq!(merged.parse_mode.as_deref(), Some("HTML"));

        let plain = load_file(Some(&path), None, true).unwrap();
        assert_eq!(plain.chat_id.as_deref(), Some("-1"));

        let missing = load_file(Some(&path), Some("nope"), true).unwrap_err();
        assert_eq!(missing.exit_code(), 17);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn absent_file_is_fine_unless_named() {
        let path = Path::new("/nonexistent/notiflow/config.toml");
        assert_eq!(load_file(Some(path), None, false).unwrap(), Settings::default());
        assert_eq!(load_file(Some(path), None, true).unwrap_err().exit_code(), 17);
    }
}
