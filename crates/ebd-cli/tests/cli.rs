//! End-to-end checks of the CLI surface (arguments, output, exit codes).
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn test_config(name: &str) -> PathBuf {
    std::env::temp_dir()
        .join(format!("ebd-cli-it-{}-{name}", std::process::id()))
        .join("config.toml")
}

fn ebd_at(config: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ebd-cli"))
        .args(args)
        .env("EBD_CONFIG", config)
        .output()
        .expect("failed to run ebd-cli")
}

fn ebd(args: &[&str]) -> Output {
    ebd_at(&test_config("default"), args)
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is utf-8")
}

#[test]
fn demo_reports_the_mock_conversation() {
    let output = ebd(&["demo"]);
    let text = stdout(&output);

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    for expected in [
        "Welcome, EBD Developer!",
        "Community: Example Community",
        "#general",
        "This channel exists entirely offline.",
        "Sent mock message: mock-message-sent",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in:\n{text}");
    }
}

#[test]
fn demo_is_the_default_command() {
    let explicit = ebd(&["demo"]);
    let implicit = ebd(&[]);

    assert_eq!(explicit.status.success(), implicit.status.success());
    assert_eq!(stdout(&explicit), stdout(&implicit));
}

#[test]
fn capabilities_lists_every_contract_capability() {
    let output = ebd(&["capabilities"]);
    let text = stdout(&output);

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    for capability in [
        "read_current_user",
        "read_guilds",
        "read_channels",
        "read_messages",
        "send_messages",
        "open_external_url",
    ] {
        assert!(
            text.contains(capability),
            "missing {capability} in:\n{text}"
        );
    }
    assert_eq!(text.matches("[yes]").count(), 6);
}

#[test]
fn self_check_passes_with_the_mock_adapter() {
    let output = ebd(&["self-check"]);
    let text = stdout(&output);

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert!(text.contains("EBD self-check: OK"), "{text}");
    assert!(text.contains("Capabilities: 6/6"), "{text}");
}

#[test]
fn help_succeeds_and_unknown_commands_exit_with_usage_error() {
    assert!(ebd(&["--help"]).status.success());

    let unknown = ebd(&["nope"]);
    assert_eq!(unknown.status.code(), Some(2));
    assert!(unknown.stdout.is_empty());
}

#[test]
fn config_shows_defaults_without_writing_them() {
    let path = test_config("show");
    let _ = std::fs::remove_dir_all(path.parent().expect("parent"));

    let output = ebd_at(&path, &["config"]);
    let text = stdout(&output);

    assert!(output.status.success(), "stderr: {:?}", output.stderr);
    assert!(text.contains("not created yet"), "{text}");
    assert!(text.contains("\"level\": \"info\""), "{text}");
    assert!(text.contains("\"format\": \"text\""), "{text}");
    assert!(!path.exists(), "`config` must not create the file");
}

#[test]
fn config_init_writes_defaults_and_refuses_to_overwrite() {
    let path = test_config("init");
    let _ = std::fs::remove_dir_all(path.parent().expect("parent"));

    let first = ebd_at(&path, &["config", "--init"]);
    assert!(first.status.success(), "stderr: {:?}", first.stderr);
    assert!(path.exists());

    let contents = std::fs::read_to_string(&path).expect("read config");
    assert!(contents.contains("\"logging\""), "{contents}");

    let second = ebd_at(&path, &["config", "--init"]);
    assert_eq!(second.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&second.stderr).contains("refusing to overwrite"));
}

#[test]
fn invalid_config_fails_loudly() {
    let path = test_config("invalid");
    std::fs::create_dir_all(path.parent().expect("parent")).expect("create temp dir");
    std::fs::write(&path, r#"{"logging": {"levl": "debug"}}"#).expect("write config");

    let output = ebd_at(&path, &["demo"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid configuration"));
}
