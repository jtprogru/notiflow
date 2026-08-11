//! Template selection and single-pass `{{.Field}}` substitution.
//!
//! v1 substituted key by key in a loop, so a value inserted early was itself scanned for
//! placeholders by every later iteration. Under `parse_mode=MarkdownV2` escaping hid the
//! problem (`{{.` became `\{\{\.`), but under `HTML` and `none` a workflow name containing
//! `{{.Actor}}` really did expand. The scanner below reads the template exactly once, so
//! substituted text is never re-examined and the class of bug is gone rather than masked.

use crate::context::{Context, is_known_placeholder};
use crate::escape::escape;
use crate::model::{ParseMode, Status};

/// The template used when the caller supplies none.
///
/// No trailing newline: v1 built it with a heredoc inside `$(...)`, which strips trailing
/// newlines, and the parity corpus compares byte for byte.
pub const DEFAULT_TEMPLATE: &str = "{{.StatusEmoji}} *{{.Workflow}}* on `{{.Repo}}`\n\
     Status: {{.Status}}\n\
     Branch: {{.Branch}} @ {{.ShortSha}}\n\
     Actor: {{.Actor}}\n\
     [Open run]({{.RunUrl}})";

/// Which of the caller's template inputs won, and what it contained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedTemplate {
    pub body: String,
    pub source: TemplateSource,
}

/// Where the active template came from. Reported by `notiflow render --explain`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateSource {
    /// `message` — verbatim, no substitution, no escaping.
    VerbatimMessage,
    /// `template_<status>` matching the reported status.
    PerStatus(Status),
    /// `message_template`.
    Generic,
    /// Built-in default.
    Default,
}

impl TemplateSource {
    pub fn as_str(self) -> &'static str {
        match self {
            TemplateSource::VerbatimMessage => "message",
            TemplateSource::PerStatus(Status::Success) => "template_success",
            TemplateSource::PerStatus(Status::Failure) => "template_failure",
            TemplateSource::PerStatus(Status::Cancelled) => "template_cancelled",
            TemplateSource::PerStatus(Status::Skipped) => "template_skipped",
            TemplateSource::Generic => "message_template",
            TemplateSource::Default => "default",
        }
    }
}

/// The four per-status template inputs plus the generic one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TemplateInputs {
    pub message: Option<String>,
    pub message_template: Option<String>,
    pub success: Option<String>,
    pub failure: Option<String>,
    pub cancelled: Option<String>,
    pub skipped: Option<String>,
}

impl TemplateInputs {
    /// Applies the v1 precedence: `message` > `template_<status>` > `message_template` >
    /// built-in default. Empty strings count as "not set", same as bash's `-n` test.
    pub fn select(&self, status: Status) -> SelectedTemplate {
        let nonempty = |s: &Option<String>| s.as_ref().filter(|v| !v.is_empty()).cloned();

        if let Some(message) = nonempty(&self.message) {
            return SelectedTemplate { body: message, source: TemplateSource::VerbatimMessage };
        }
        let per_status = match status {
            Status::Success => &self.success,
            Status::Failure => &self.failure,
            Status::Cancelled => &self.cancelled,
            Status::Skipped => &self.skipped,
        };
        if let Some(body) = nonempty(per_status) {
            return SelectedTemplate { body, source: TemplateSource::PerStatus(status) };
        }
        if let Some(body) = nonempty(&self.message_template) {
            return SelectedTemplate { body, source: TemplateSource::Generic };
        }
        SelectedTemplate { body: DEFAULT_TEMPLATE.to_string(), source: TemplateSource::Default }
    }
}

/// Result of one substitution pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub text: String,
    /// Placeholder names that were dropped because nothing defines them. The caller warns
    /// once per name, preserving v1's `UNKNOWN_PLACEHOLDER:<name>` message.
    pub unknown: Vec<String>,
}

/// Substitutes every `{{.Field}}` in `template`, escaping each value for `mode`.
///
/// The template body is copied verbatim — only substituted values are escaped, so a
/// template may contain genuine markup while a hostile branch name cannot inject any.
pub fn render(template: &str, ctx: &Context, mode: ParseMode) -> Rendered {
    let bytes = template.as_bytes();
    let mut out = String::with_capacity(template.len());
    let mut unknown: Vec<String> = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] == b'{' && template[i..].starts_with("{{.") {
            if let Some((name, end)) = scan_placeholder(template, i) {
                match ctx.get(name) {
                    Some(value) => out.push_str(&escape(value, mode)),
                    None => {
                        // Unknown placeholder: dropped, exactly as v1 did.
                        if !unknown.iter().any(|n| n == name) {
                            unknown.push(name.to_string());
                        }
                    }
                }
                i = end;
                continue;
            }
        }
        // Not a placeholder opener — copy one character and move on. Indexing by the
        // char's byte length keeps multi-byte text intact.
        let ch = template[i..].chars().next().expect("index is on a char boundary");
        out.push(ch);
        i += ch.len_utf8();
    }

    Rendered { text: out, unknown }
}

/// Parses `{{.Name}}` starting at `start`, returning the name and the byte index just
/// past the closing braces. `Name` is `[A-Za-z][A-Za-z0-9]*`, matching v1's stripper.
fn scan_placeholder(s: &str, start: usize) -> Option<(&str, usize)> {
    let rest = &s[start + 3..];
    let mut end = 0usize;
    for (idx, ch) in rest.char_indices() {
        let ok = if idx == 0 { ch.is_ascii_alphabetic() } else { ch.is_ascii_alphanumeric() };
        if ok {
            end = idx + ch.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        return None;
    }
    if !rest[end..].starts_with("}}") {
        return None;
    }
    Some((&rest[..end], start + 3 + end + 2))
}

/// True when the name looks like a placeholder we could have resolved. Used by the render
/// command's `--strict` mode to distinguish typos from deliberate literal braces.
pub fn placeholder_is_known(name: &str) -> bool {
    is_known_placeholder(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Context {
        Context::from_pairs([
            ("Repo", "jtprogru/notiflow"),
            ("Workflow", "CI"),
            ("Actor", "jtprogru"),
            ("Status", "success"),
            ("StatusEmoji", "✅"),
            ("Branch", "main"),
            ("ShortSha", "abc1234"),
            ("RunUrl", "https://github.com/jtprogru/notiflow/actions/runs/1"),
        ])
    }

    #[test]
    fn substitutes_known_placeholders() {
        let r = render("repo={{.Repo}} actor={{.Actor}}", &ctx(), ParseMode::None);
        assert_eq!(r.text, "repo=jtprogru/notiflow actor=jtprogru");
        assert!(r.unknown.is_empty());
    }

    #[test]
    fn escapes_values_but_not_the_template_body() {
        let c = Context::from_pairs([("Branch", "fix/a.b-c")]);
        let r = render("*Branch:* {{.Branch}}", &c, ParseMode::MarkdownV2);
        assert_eq!(r.text, "*Branch:* fix/a\\.b\\-c");
    }

    /// The v1 defect: a value containing a placeholder was expanded by a later iteration.
    #[test]
    fn substituted_values_are_not_rescanned() {
        let c = Context::from_pairs([("Workflow", "{{.Actor}}"), ("Actor", "attacker")]);
        let r = render("{{.Workflow}}", &c, ParseMode::None);
        assert_eq!(r.text, "{{.Actor}}");
    }

    #[test]
    fn unknown_placeholders_are_dropped_and_reported_once() {
        let r = render("a{{.Nope}}b{{.Nope}}c", &ctx(), ParseMode::None);
        assert_eq!(r.text, "abc");
        assert_eq!(r.unknown, vec!["Nope"]);
    }

    #[test]
    fn malformed_braces_stay_literal() {
        for input in ["{{.}}", "{{.1Bad}}", "{{.Repo}", "{{ .Repo }}", "{{Repo}}"] {
            let r = render(input, &ctx(), ParseMode::None);
            assert_eq!(r.text, input, "input {input:?}");
        }
    }

    #[test]
    fn multibyte_text_survives() {
        let r = render("статус: {{.Status}} 🎉", &ctx(), ParseMode::None);
        assert_eq!(r.text, "статус: success 🎉");
    }

    #[test]
    fn precedence_message_beats_everything() {
        let inputs = TemplateInputs {
            message: Some("verbatim".into()),
            message_template: Some("generic".into()),
            success: Some("per-status".into()),
            ..Default::default()
        };
        let sel = inputs.select(Status::Success);
        assert_eq!(sel.body, "verbatim");
        assert_eq!(sel.source, TemplateSource::VerbatimMessage);
    }

    #[test]
    fn precedence_per_status_beats_generic() {
        let inputs = TemplateInputs {
            message_template: Some("generic".into()),
            failure: Some("boom".into()),
            ..Default::default()
        };
        assert_eq!(inputs.select(Status::Failure).body, "boom");
        assert_eq!(inputs.select(Status::Success).body, "generic");
    }

    #[test]
    fn empty_strings_do_not_count_as_set() {
        let inputs = TemplateInputs {
            message: Some(String::new()),
            message_template: Some(String::new()),
            ..Default::default()
        };
        assert_eq!(inputs.select(Status::Success).source, TemplateSource::Default);
    }

    #[test]
    fn default_template_has_no_trailing_newline() {
        assert!(!DEFAULT_TEMPLATE.ends_with('\n'));
    }
}
