//! Command-line surface: rendering, config files, completions, generated docs.

mod support;

use predicates::prelude::*;
use support::{MockTelegram, Reply, TEST_TOKEN, notiflow};

#[test]
fn render_needs_no_token_and_makes_no_request() {
    notiflow()
        .args(["render", "-T", "{{.Repo}} is {{.Status}}", "--set", "Repo=owner/name"])
        .assert()
        .success()
        .stdout("owner/name is success\n");
}

#[test]
fn render_explains_which_template_won() {
    notiflow()
        .args([
            "render",
            "--status",
            "failure",
            "-T",
            "generic",
            "--template-failure",
            "specific",
            "--explain",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("# template: template_failure"))
        .stdout(predicate::str::contains("specific"));
}

#[test]
fn render_rejects_a_set_for_an_unknown_placeholder() {
    notiflow()
        .args(["render", "-T", "x", "--set", "Nope=1"])
        .assert()
        .code(18)
        .stderr(predicate::str::contains("not a placeholder"));
}

#[test]
fn a_config_file_supplies_defaults() {
    let dir = tempdir("config-defaults");
    let path = dir.join("config.toml");
    std::fs::write(&path, "chat_id = \"-100999\"\nparse_mode = \"none\"\n").unwrap();

    let mock = MockTelegram::start(vec![Reply::ok(1)]);
    notiflow()
        .args(["send", "-m", "hi", "--json", "--config"])
        .arg(&path)
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .env("NOTIFLOW_API_BASE", mock.base_url())
        .assert()
        .success();

    let body = mock.last_request().json();
    assert_eq!(body["chat_id"], -100999);
    assert!(body.get("parse_mode").is_none(), "parse_mode=none must be omitted");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_flag_outranks_the_config_file() {
    let dir = tempdir("config-precedence");
    let path = dir.join("config.toml");
    std::fs::write(&path, "chat_id = \"-100999\"\n").unwrap();

    let mock = MockTelegram::start(vec![Reply::ok(1)]);
    notiflow()
        .args(["send", "-m", "hi", "--json", "-c", "-100111", "--config"])
        .arg(&path)
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .env("NOTIFLOW_API_BASE", mock.base_url())
        .assert()
        .success();

    assert_eq!(mock.last_request().json()["chat_id"], -100111);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_profile_overrides_the_file_defaults() {
    let dir = tempdir("config-profile");
    let path = dir.join("config.toml");
    std::fs::write(&path, "chat_id = \"-100999\"\n\n[profile.work]\nchat_id = \"-100777\"\n")
        .unwrap();

    let mock = MockTelegram::start(vec![Reply::ok(1)]);
    notiflow()
        .args(["send", "-m", "hi", "--json", "--profile", "work", "--config"])
        .arg(&path)
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .env("NOTIFLOW_API_BASE", mock.base_url())
        .assert()
        .success();

    assert_eq!(mock.last_request().json()["chat_id"], -100777);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_missing_profile_is_a_config_error_not_a_silent_default() {
    let dir = tempdir("config-bad-profile");
    let path = dir.join("config.toml");
    std::fs::write(&path, "chat_id = \"-1\"\n").unwrap();

    notiflow()
        .args(["send", "-m", "hi", "--profile", "nope", "--config"])
        .arg(&path)
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .assert()
        .code(17)
        .stderr(predicate::str::contains("profile 'nope' not found"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_unknown_config_key_is_reported_rather_than_ignored() {
    let dir = tempdir("config-typo");
    let path = dir.join("config.toml");
    std::fs::write(&path, "chat_ids = \"-1\"\n").unwrap();

    notiflow()
        .args(["send", "-m", "hi", "--config"])
        .arg(&path)
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .assert()
        .code(17)
        .stderr(predicate::str::contains("cannot parse"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn whoami_reports_the_bot_identity() {
    let mock = MockTelegram::start(vec![Reply {
        status: 200,
        body: r#"{"ok":true,"result":{"id":1,"is_bot":true,"first_name":"Notiflow","username":"notiflow_bot"}}"#
            .to_string(),
        retry_after: None,
    }]);

    notiflow()
        .args(["whoami"])
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .env("NOTIFLOW_API_BASE", mock.base_url())
        .assert()
        .success()
        .stdout(predicate::str::contains("Notiflow (@notiflow_bot, id=1)"));

    assert_eq!(mock.last_request().method(), "getMe");
}

#[test]
fn whoami_fails_when_the_token_is_rejected() {
    let mock = MockTelegram::start(vec![Reply::error(401, "Unauthorized")]);

    notiflow()
        .args(["whoami"])
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .env("NOTIFLOW_API_BASE", mock.base_url())
        .assert()
        .code(1)
        .stderr(predicate::str::contains("Unauthorized"));
}

#[test]
fn a_token_on_argv_earns_a_warning() {
    notiflow()
        .args(["send", "-c", "-100", "-m", "x", "--dry-run", "--bot-token", TEST_TOKEN])
        .assert()
        .success()
        .stderr(predicate::str::contains("any process on this machine can read"));
}

#[test]
fn completions_are_generated_for_every_supported_shell() {
    for shell in ["bash", "zsh", "fish", "powershell", "elvish"] {
        notiflow()
            .args(["completions", shell])
            .assert()
            .success()
            .stdout(predicate::str::contains("notiflow"));
    }
}

#[test]
fn generated_doc_fragments_are_non_empty_tables() {
    for fragment in ["exit-codes", "placeholders", "cli"] {
        notiflow()
            .args(["docs", fragment])
            .assert()
            .success()
            .stdout(predicate::str::contains("|"));
    }
}

#[test]
fn version_matches_the_crate() {
    notiflow()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

fn tempdir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("notiflow-it-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}
