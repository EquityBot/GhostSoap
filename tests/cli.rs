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

fn json_of(args: &[&str]) -> serde_json::Value {
    let out = ghostsoap(args);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    serde_json::from_slice(&out.stdout).expect("stdout is one JSON document")
}

#[test]
fn plan_json_lists_steps_with_their_levers() {
    let plan = json_of(&["plan", "--pack", "start-here", "--json"]);
    let steps = plan["steps"].as_array().expect("steps");
    assert!(!steps.is_empty());
    let letter = steps
        .iter()
        .find(|s| s["action"] == "letter")
        .expect("a letter step");
    assert!(letter["entry"].is_string());
    assert!(letter["address"].is_string());
    assert!(!letter["levers"].as_array().expect("levers").is_empty());
    assert!(
        !letter["details_used"]
            .as_array()
            .expect("details")
            .is_empty()
    );
}

#[test]
fn plan_json_never_echoes_detail_text() {
    let out = ghostsoap(&[
        "plan",
        "--entry",
        "equifax-au",
        "--lever",
        "s20t-correction",
        "--detail",
        "PRIVATE-WORDS",
        "--json",
    ]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(!text(&out.stdout).contains("PRIVATE-WORDS"));
}

#[test]
fn plan_json_carries_the_access_warning() {
    let plan = json_of(&[
        "plan",
        "--entry",
        "equifax-au",
        "--lever",
        "s20r-access",
        "--json",
    ]);
    assert!(
        plan["warnings"][0]
            .as_str()
            .expect("warning")
            .contains("yourself")
    );
}

#[test]
fn plan_json_error_stays_off_stdout() {
    let out = ghostsoap(&["plan", "--entry", "no-such-entry", "--json"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert!(text(&out.stderr).starts_with("error: "));
}

#[test]
fn law_json_covers_every_lever_with_the_notice() {
    let law = json_of(&["law", "--json"]);
    let levers = law["levers"].as_array().expect("levers");
    assert!(!levers.is_empty());
    assert!(
        levers
            .iter()
            .all(|l| l["basis"].is_string() && l["deadline"].is_string())
    );
    assert!(
        law["notice"]
            .as_str()
            .expect("notice")
            .contains("not legal advice")
    );
}

#[test]
fn plan_json_keeps_the_overseas_caveat_of_the_text_output() {
    let plan = json_of(&["plan", "--entry", "infotracer-au", "--json"]);
    let note = plan["steps"][0]["note"].as_str().expect("note");
    assert!(note.contains("has not established"));
    let out = ghostsoap(&["plan", "--entry", "infotracer-au"]);
    assert!(text(&out.stdout).contains(note));
}

#[test]
fn registry_list_json_names_every_pack_entry() {
    let list = json_of(&["registry", "list", "--json"]);
    let ids: Vec<&str> = list["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .map(|e| e["id"].as_str().expect("id"))
        .collect();
    let packs = list["packs"].as_array().expect("packs");
    assert!(packs.iter().any(|p| p["id"] == "start-here"));
    for pack in packs {
        for entry in pack["entries"].as_array().expect("pack entries") {
            assert!(ids.contains(&entry.as_str().expect("entry id")));
        }
    }
    assert!(list["entries"][0]["kinds"][0].is_string());
}
