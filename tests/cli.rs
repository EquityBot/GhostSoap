use std::process::{Command, Output};

fn ghostsoap(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ghostsoap"))
        .args(args)
        .output()
        .expect("run ghostsoap")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn help_prints_usage() {
    let out = ghostsoap(&["--help"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(text(&out.stdout).contains("Usage: ghostsoap"));
    assert!(out.stderr.is_empty());
}

#[test]
fn version_prints_name_and_version() {
    let out = ghostsoap(&["--version"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        text(&out.stdout).trim(),
        format!("ghostsoap {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn no_arguments_prints_help() {
    let out = ghostsoap(&[]);
    assert_eq!(out.status.code(), Some(0));
    assert!(text(&out.stdout).contains("Usage: ghostsoap"));
}

#[test]
fn unknown_flag_is_a_usage_error() {
    let out = ghostsoap(&["--no-such-flag"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(text(&out.stderr).contains("unexpected argument '--no-such-flag'"));
}
