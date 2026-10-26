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
fn built_in_register_passes() {
    let out = Command::new(env!("CARGO_BIN_EXE_ghostsoap"))
        .args(["registry", "check"])
        .current_dir(env!("CARGO_TARGET_TMPDIR"))
        .output()
        .expect("run ghostsoap");
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(text(&out.stdout).contains("Checked 80 entries and 12 packs; errors: 0,"));
}

#[test]
fn built_in_register_matches_the_files() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("registry/au");
    let files = io::register::load(&dir).expect("read register");
    let built_in = ghostsoap::core::embedded::register();
    let pairs = |f: &[register::SourceFile]| -> Vec<(String, String)> {
        f.iter().map(|s| (s.name.clone(), s.text.clone())).collect()
    };
    assert_eq!(pairs(&built_in.entries), pairs(&files.entries));
    assert_eq!(pairs(&built_in.packs), pairs(&files.packs));
}

#[test]
fn shipped_register_meets_its_targets() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("registry/au");
    let files = io::register::load(&dir).expect("read register");
    let today = chrono::Local::now().date_naive();
    let report = register::check(&files.entries, &files.packs, today);
    assert_eq!(report.errors(), 0, "{:?}", report.findings);
    assert!(report.entries >= 25, "{} entries", report.entries);
    let mut packs: Vec<_> = files.packs.iter().map(|p| p.name.as_str()).collect();
    packs.sort_unstable();
    assert_eq!(
        packs,
        [
            "banks-and-lenders.toml",
            "credit.toml",
            "data-breaches.toml",
            "insurers.toml",
            "loyalty.toml",
            "marketing.toml",
            "people-search.toml",
            "property.toml",
            "recruitment.toml",
            "retail.toml",
            "start-here.toml",
            "telecommunications.toml"
        ]
    );
    // Ryan's decision (plan decision 8): SEEK is sent only when named with --entry.
    for pack in &files.packs {
        let pack: register::Pack = toml::from_str(&pack.text).expect("parse pack");
        assert!(
            !pack.entries.iter().any(|e| e == "seek-au"),
            "seek-au is in pack {}",
            pack.id
        );
    }
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

#[test]
fn empty_register_directory_is_rejected() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("empty-register");
    std::fs::create_dir_all(&dir).expect("create empty register");
    let out = registry_check(dir.to_str().expect("utf-8 path"));
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("the register has no entry files"));
    assert!(text(&out.stdout).contains("errors: 1,"));
}
