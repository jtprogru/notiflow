//! The values `{{.Field}}` placeholders resolve to.
//!
//! Inside a workflow they come from the `GITHUB_*` environment. Outside one — the CLI use
//! case that v1 did not have — the repository-shaped fields are recovered from the local
//! git checkout, so the same template renders sensibly in both places.

use std::collections::BTreeMap;
use std::process::Command;

use crate::model::Status;

/// Every placeholder notiflow understands, with the docs blurb `make gen` renders.
///
/// The table in the documentation is generated from this constant; adding a placeholder
/// here is the only edit needed to document it.
pub const PLACEHOLDERS: &[(&str, &str)] = &[
    ("Repo", "`owner/name` of the repository"),
    ("Workflow", "Workflow name"),
    ("Job", "Job id inside the workflow"),
    ("Status", "The reported status, verbatim"),
    ("StatusEmoji", "✅ / ❌ / ⚠️ / ⏭ for the reported status"),
    ("Actor", "User that triggered the run (git `user.name` outside Actions)"),
    ("Ref", "Full ref, e.g. `refs/heads/main`"),
    ("RefName", "Short ref name, e.g. `main`"),
    ("Branch", "Alias of `RefName`"),
    ("Sha", "Full commit SHA"),
    ("ShortSha", "First 7 characters of the SHA"),
    ("RunId", "Workflow run id (empty outside Actions)"),
    ("RunNumber", "Workflow run number (empty outside Actions)"),
    ("RunUrl", "Direct link to the run (empty outside Actions)"),
    ("EventName", "Event that triggered the run (empty outside Actions)"),
    ("ServerUrl", "GitHub server URL"),
];

/// Resolved placeholder values for one render.
#[derive(Debug, Clone, Default)]
pub struct Context {
    values: BTreeMap<String, String>,
}

impl Context {
    /// Builds a context from the process environment, falling back to local git.
    ///
    /// `git_fallback` is disabled inside Actions: the `GITHUB_*` variables are the truth
    /// there, and shelling out to git on every render would be pure cost.
    pub fn discover(status: Status, git_fallback: bool) -> Self {
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());

        let mut repo = env("GITHUB_REPOSITORY");
        let mut sha = env("GITHUB_SHA");
        let mut ref_name = env("GITHUB_REF_NAME");
        let mut full_ref = env("GITHUB_REF");
        let mut actor = env("GITHUB_ACTOR");

        if git_fallback {
            if repo.is_none() {
                repo = git_repo_slug();
            }
            if sha.is_none() {
                sha = git(&["rev-parse", "HEAD"]);
            }
            if ref_name.is_none() {
                ref_name = git(&["rev-parse", "--abbrev-ref", "HEAD"]);
            }
            if full_ref.is_none() {
                full_ref = ref_name.as_ref().map(|r| format!("refs/heads/{r}"));
            }
            if actor.is_none() {
                actor = git(&["config", "user.name"]);
            }
        }

        let server_url =
            env("GITHUB_SERVER_URL").unwrap_or_else(|| "https://github.com".to_string());
        let run_id = env("GITHUB_RUN_ID");
        let short_sha: Option<String> = sha.as_ref().map(|s| s.chars().take(7).collect());

        // RunUrl only means something when there is a run to point at. v1 emitted a
        // half-built `//actions/runs/` string outside Actions; v2 leaves it empty.
        let run_url = match (&run_id, &repo) {
            (Some(id), Some(r)) => Some(format!("{server_url}/{r}/actions/runs/{id}")),
            _ => None,
        };

        let mut values = BTreeMap::new();
        let mut set = |k: &str, v: Option<String>| {
            values.insert(k.to_string(), v.unwrap_or_default());
        };
        set("Repo", repo);
        set("Workflow", env("GITHUB_WORKFLOW"));
        set("Job", env("GITHUB_JOB"));
        set("Status", Some(status.as_str().to_string()));
        set("StatusEmoji", Some(status.emoji().to_string()));
        set("Actor", actor);
        set("Ref", full_ref);
        set("RefName", ref_name.clone());
        set("Branch", ref_name);
        set("Sha", sha);
        set("ShortSha", short_sha);
        set("RunId", run_id);
        set("RunNumber", env("GITHUB_RUN_NUMBER"));
        set("RunUrl", run_url);
        set("EventName", env("GITHUB_EVENT_NAME"));
        set("ServerUrl", Some(server_url));

        Context { values }
    }

    /// Builds a context from an explicit map. Used by tests and by `notiflow render`.
    pub fn from_pairs<I, K, V>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        Context { values: pairs.into_iter().map(|(k, v)| (k.into(), v.into())).collect() }
    }

    /// Value for a placeholder name, or `None` when the name is not a known placeholder.
    pub fn get(&self, key: &str) -> Option<&str> {
        if !is_known_placeholder(key) {
            return None;
        }
        Some(self.values.get(key).map(String::as_str).unwrap_or(""))
    }

    /// Overrides one value; used by `notiflow render --set Key=Value`.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    pub fn entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.values.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

/// True when `name` is one of the documented placeholders.
pub fn is_known_placeholder(name: &str) -> bool {
    PLACEHOLDERS.iter().any(|(k, _)| *k == name)
}

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

/// Turns whatever `origin` points at into `owner/name`.
///
/// Handles both remote spellings — `git@host:owner/name.git` and
/// `https://host/owner/name.git` — and gives up quietly on anything else.
fn git_repo_slug() -> Option<String> {
    let url = git(&["remote", "get-url", "origin"])?;
    let path = match url.split_once("://") {
        Some((_, rest)) => rest.split_once('/').map(|(_, p)| p.to_string())?,
        None => url.split_once(':').map(|(_, p)| p.to_string())?,
    };
    let slug = path.strip_suffix(".git").unwrap_or(&path);
    let parts: Vec<&str> = slug.trim_matches('/').split('/').collect();
    if parts.len() < 2 {
        return None;
    }
    Some(parts[parts.len() - 2..].join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_keys_are_not_resolvable() {
        let ctx = Context::from_pairs([("Repo", "jtprogru/notiflow")]);
        assert_eq!(ctx.get("Repo"), Some("jtprogru/notiflow"));
        assert_eq!(ctx.get("Nope"), None);
    }

    #[test]
    fn known_but_unset_key_resolves_to_empty() {
        let ctx = Context::from_pairs([("Repo", "a/b")]);
        assert_eq!(ctx.get("Workflow"), Some(""));
    }

    #[test]
    fn every_placeholder_is_documented_once() {
        let mut names: Vec<&str> = PLACEHOLDERS.iter().map(|(k, _)| *k).collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), count, "duplicate placeholder name");
    }
}
