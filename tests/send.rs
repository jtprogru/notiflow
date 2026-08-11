//! End-to-end behaviour of `notiflow send` and `notiflow edit` against a scripted API.

mod support;

use predicates::prelude::*;
use serde_json::Value;
use support::{MockTelegram, Reply, TEST_TOKEN, notiflow};

fn report(stdout: &[u8]) -> Value {
    let text = String::from_utf8_lossy(stdout);
    let last = text.trim().lines().last().expect("some output");
    serde_json::from_str(last).expect("the last stdout line is the JSON report")
}

#[test]
fn sends_and_reports_the_message_id() {
    let mock = MockTelegram::start(vec![Reply::ok(4242)]);

    let output = notiflow()
        .args(["send", "-c", "-1001234567890", "-m", "hello", "--json"])
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .env("NOTIFLOW_API_BASE", mock.base_url())
        .assert()
        .success()
        .get_output()
        .clone();

    let report = report(&output.stdout);
    assert_eq!(report["ok"], true);
    assert_eq!(report["message_id"], 4242);
    assert_eq!(report["http_status"], 200);
    assert_eq!(report["attempts"], 1);

    let request = mock.last_request();
    assert_eq!(request.method(), "sendMessage");
    assert_eq!(request.json()["text"], "hello");
    assert_eq!(request.json()["chat_id"], -1001234567890i64);
}

#[test]
fn retries_a_rate_limit_then_succeeds() {
    let mock = MockTelegram::start(vec![Reply::rate_limited(0), Reply::ok(7)]);

    notiflow()
        .args(["send", "-c", "-100", "-m", "hi", "--json", "--max-retry-after", "0"])
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .env("NOTIFLOW_API_BASE", mock.base_url())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""ok":true"#))
        .stdout(predicate::str::contains(r#""attempts":2"#));

    assert_eq!(mock.request_count(), 2);
}

#[test]
fn honours_a_retry_after_header_when_the_body_has_none() {
    let mock = MockTelegram::start(vec![
        Reply::error(429, "Too Many Requests").with_retry_after_header(0),
        Reply::ok(9),
    ]);

    notiflow()
        .args(["send", "-c", "-100", "-m", "hi", "--json", "--max-retry-after", "0"])
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .env("NOTIFLOW_API_BASE", mock.base_url())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""ok":true"#));

    assert_eq!(mock.request_count(), 2);
}

#[test]
fn a_four_hundred_is_not_retried() {
    let mock = MockTelegram::start(vec![Reply::error(400, "Bad Request: chat not found")]);

    let output = notiflow()
        .args(["send", "-c", "-100", "-m", "hi", "--json"])
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .env("NOTIFLOW_API_BASE", mock.base_url())
        .assert()
        .success()
        .get_output()
        .clone();

    let report = report(&output.stdout);
    assert_eq!(report["ok"], false);
    assert_eq!(report["http_status"], 400);
    assert_eq!(report["error"], "Bad Request: chat not found");
    assert_eq!(mock.request_count(), 1, "a 4xx must not be retried");
}

#[test]
fn exhausted_retries_report_the_last_status() {
    let mock = MockTelegram::start(vec![Reply::error(503, "busy")]);

    let output = notiflow()
        .args(["send", "-c", "-100", "-m", "hi", "--json", "--retries", "1"])
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .env("NOTIFLOW_API_BASE", mock.base_url())
        .assert()
        .success()
        .get_output()
        .clone();

    let report = report(&output.stdout);
    assert_eq!(report["ok"], false);
    assert_eq!(report["http_status"], 503);
    assert_eq!(report["attempts"], 2);
    assert_eq!(mock.request_count(), 2);
}

#[test]
fn fail_on_error_turns_a_failure_into_exit_one() {
    let mock = MockTelegram::start(vec![Reply::error(400, "nope")]);

    notiflow()
        .args(["send", "-c", "-100", "-m", "hi", "--json", "--fail-on-error"])
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .env("NOTIFLOW_API_BASE", mock.base_url())
        .assert()
        .code(1);
}

#[test]
fn without_fail_on_error_the_job_result_is_left_alone() {
    let mock = MockTelegram::start(vec![Reply::error(400, "nope")]);

    notiflow()
        .args(["send", "-c", "-100", "-m", "hi", "--json"])
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .env("NOTIFLOW_API_BASE", mock.base_url())
        .assert()
        .code(0);
}

#[test]
fn a_filtered_status_never_opens_a_connection() {
    let mock = MockTelegram::start(vec![Reply::ok(1)]);

    notiflow()
        .args(["send", "-c", "-100", "-m", "hi", "--json", "--status", "skipped"])
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .env("NOTIFLOW_API_BASE", mock.base_url())
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""skipped":true"#));

    assert_eq!(mock.request_count(), 0);
}

#[test]
fn edit_targets_edit_message_text_and_drops_send_only_fields() {
    let mock = MockTelegram::start(vec![Reply::ok(42)]);

    notiflow()
        .args([
            "edit",
            "--message-id",
            "42",
            "-c",
            "-100",
            "-m",
            "corrected",
            "--json",
            "--silent",
            "--thread-id",
            "7",
        ])
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .env("NOTIFLOW_API_BASE", mock.base_url())
        .assert()
        .success();

    let request = mock.last_request();
    assert_eq!(request.method(), "editMessageText");
    let body = request.json();
    assert_eq!(body["message_id"], 42);
    assert_eq!(body["text"], "corrected");
    assert!(body.get("disable_notification").is_none());
    assert!(body.get("message_thread_id").is_none());
}

#[test]
fn stdin_becomes_the_message_body() {
    let mock = MockTelegram::start(vec![Reply::ok(1)]);

    notiflow()
        .args(["send", "-c", "-100", "--stdin", "--json"])
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .env("NOTIFLOW_API_BASE", mock.base_url())
        .write_stdin("deploy finished\n")
        .assert()
        .success();

    assert_eq!(mock.last_request().json()["text"], "deploy finished");
}

#[test]
fn github_mode_writes_the_v1_output_contract() {
    let mock = MockTelegram::start(vec![Reply::ok(555)]);
    let dir = tempdir("outputs");
    let output_file = dir.join("gh_output");
    let summary_file = dir.join("gh_summary");
    std::fs::write(&output_file, "").unwrap();

    notiflow()
        .args(["send", "-c", "-100", "-m", "hi", "--output", "github"])
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        // Loopback stays allowlisted under Actions so the smoke workflow can intercept.
        .env("NOTIFLOW_API_BASE", mock.base_url())
        .env("GITHUB_ACTIONS", "true")
        .env("GITHUB_OUTPUT", &output_file)
        .env("GITHUB_STEP_SUMMARY", &summary_file)
        .assert()
        .success();

    let outputs = std::fs::read_to_string(&output_file).unwrap();
    assert!(outputs.contains("ok=true"), "{outputs}");
    assert!(outputs.contains("message_id=555"), "{outputs}");
    assert!(outputs.contains("http_status=200"), "{outputs}");
    assert!(outputs.contains("error="), "{outputs}");

    let summary = std::fs::read_to_string(&summary_file).unwrap();
    assert!(summary.contains("| result | sent |"), "{summary}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn under_actions_a_foreign_api_base_is_ignored() {
    // The threat: a prior step in the same job exports NF_API_BASE and harvests the token.
    // The request must not leave for evil.example, and the run must fail against the real
    // Telegram host rather than succeed against the attacker's.
    let mock = MockTelegram::start(vec![Reply::ok(1)]);

    notiflow()
        .args(["send", "-c", "-100", "-m", "hi", "--json", "--dry-run"])
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .env("NOTIFLOW_API_BASE", "https://api.telegram.org.evil.example")
        .env("GITHUB_ACTIONS", "true")
        .assert()
        .success()
        .stderr(predicate::str::contains("host not allowed"));

    assert_eq!(mock.request_count(), 0);
}

#[test]
fn the_token_never_reaches_stdout_or_stderr() {
    // No mock: the connection fails, which is precisely when a URL-bearing error surfaces.
    let output = notiflow()
        .args(["send", "-c", "-100", "-m", "hi", "--json", "--retries", "0", "-vv"])
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .env("NOTIFLOW_API_BASE", "http://127.0.0.1:1")
        .assert()
        .get_output()
        .clone();

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!combined.contains(TEST_TOKEN), "token leaked: {combined}");
    assert!(!combined.contains("AAHdqTcv"), "token fragment leaked: {combined}");
}

#[test]
fn a_comma_in_chat_id_fails_loudly_instead_of_sending() {
    let mock = MockTelegram::start(vec![Reply::ok(1)]);

    notiflow()
        .args(["send", "-c", "-100,-200", "-m", "hi"])
        .env("NOTIFLOW_BOT_TOKEN", TEST_TOKEN)
        .env("NOTIFLOW_API_BASE", mock.base_url())
        .assert()
        .code(11)
        .stderr(predicate::str::contains("exactly one chat"));

    assert_eq!(mock.request_count(), 0);
}

fn tempdir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("notiflow-it-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}
