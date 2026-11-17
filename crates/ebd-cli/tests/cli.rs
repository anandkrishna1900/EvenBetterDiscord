//! End-to-end checks of the CLI surface (arguments, output, exit codes).
use std::process::{Command, Output};

fn ebd(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ebd-cli"))
        .args(args)
        .output()
        .expect("failed to run ebd-cli")
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
