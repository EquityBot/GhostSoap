use std::path::Path;
use std::process::{Command, Output};

use ghostsoap::core::register::{self, Quality};
use ghostsoap::io;

fn registry_check(path: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ghostsoap"))
        .args(["registry", "check", "--path", path])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run ghostsoap")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn shipped_register_passes() {
    let out = registry_check("registry/au");
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("errors: 0,"));
}

#[test]
fn shipped_register_meets_the_m1_targets() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("registry/au");
    let files = io::register::load(&dir).expect("read register");
    let today = chrono::Local::now().date_naive();
    let report = register::check(&files.entries, &files.packs, today);
    assert_eq!(report.errors(), 0, "{:?}", report.findings);
    assert!(
        (25..=40).contains(&report.entries),
        "{} entries",
        report.entries
    );
    let mut packs: Vec<_> = files.packs.iter().map(|p| p.name.as_str()).collect();
    packs.sort_unstable();
    assert_eq!(
        packs,
        [
            "credit.toml",
            "loyalty.toml",
            "marketing.toml",
            "people-search.toml",
            "property.toml",
            "start-here.toml"
        ]
    );
    for file in &files.entries {
        let entry: register::Entry = toml::from_str(&file.text).expect("parse entry");
        assert!(
            matches!(entry.quality, Quality::Tested | Quality::Verified),
            "{} is below verified",
            file.name
        );
    }
}

#[test]
fn entry_with_no_source_is_rejected() {
    let out = registry_check("tests/fixtures/registry/no-source");
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("no-source-au.toml: no source"));
    assert!(text(&out.stdout).contains("errors: 1,"));
}

#[test]
fn missing_register_directory_is_an_error() {
    let out = registry_check("tests/fixtures/registry/does-not-exist");
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("error: cannot read"));
}
