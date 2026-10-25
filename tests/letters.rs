//! Snapshot tests for every lever letter, and the rule that a letter shows only the
//! entry's identity fields (plan section 8, M2). Run with `UPDATE_SNAPSHOTS=1` to rewrite
//! the snapshots after a deliberate change, then review the diff.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use ghostsoap::core::letters::{self, Letter, Request};
use ghostsoap::core::profile::Profile;
use ghostsoap::core::register::{Entry, Lever};

/// An entry that accepts every lever by email and post, and asks only for name and email.
const ENTRY: &str = r#"
id = "wexler-mcgill-au"
name = "Wexler McGill"
legal_entity = "Wexler McGill Pty Ltd"
abn = "88 000 014 675"
kinds = ["marketing-list"]
priority = "high"
quality = "verified"
verified_at = 2026-10-25
sources = ["https://example.com.au/privacy"]
levers = ["app7-no-marketing", "app7-no-facilitation", "app7-source", "app11-destroy", "app13", "app13-notify-recipients", "app13-statement", "s20t-correction", "s21v-correction", "spam-unsubscribe"]
opt_in_levers = ["app12", "s20r-access", "s20k-ban", "s21t-access"]
identity_fields = ["full_name", "email"]
exemption_risk = "none"
escalation = ["afca", "oaic"]
effort = "easy"

[[channels]]
kind = "email"
address = "privacy@example.com.au"
levers = ["app7-no-marketing", "app7-no-facilitation", "app7-source", "app11-destroy", "app13", "app13-notify-recipients", "app13-statement", "s20t-correction", "s21v-correction", "spam-unsubscribe", "app12", "s20r-access", "s20k-ban", "s21t-access"]

[[channels]]
kind = "post"
address = "PO Box 1, Sydney NSW 2001"
levers = ["app7-source"]
"#;

fn entry() -> Entry {
    toml::from_str(ENTRY).expect("fixture entry parses")
}

/// Every identity field filled, so a test can see which ones leak.
fn profile() -> Profile {
    Profile {
        full_name: "James McGill".to_owned(),
        alias_names: vec!["Saul Goodman".to_owned()],
        email: Some("jimmy@example.com".to_owned()),
        phone: Some("0400 111 222".to_owned()),
        date_of_birth: Some("1960-11-12".to_owned()),
        address: Some("9800 Montgomery Road, Albuquerque NSW 2000".to_owned()),
        previous_addresses: vec!["160 Juan Tabo Boulevard, Cicero NSW 2000".to_owned()],
        postcode: Some("2000".to_owned()),
    }
}

fn compose(entry: &Entry, profile: &Profile, levers: &[Lever], channel: usize) -> Letter {
    let detail = levers
        .iter()
        .any(|&l| letters::takes_detail(l))
        .then_some("These are my own words about this request.");
    let request = Request {
        channel: &entry.channels[channel],
        levers,
        detail,
        date: NaiveDate::from_ymd_opt(2026, 10, 25).unwrap(),
        tag: "GS-TEST1",
    };
    letters::compose(entry, profile, &request).expect("compose")
}

fn snapshot(name: &str, letter: &Letter) {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots")
        .join(format!("{name}.txt"));
    let actual = format!(
        "To: {}\nSubject: {}\n\n{}\n",
        letter.to, letter.subject, letter.body
    );
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = fs::read_to_string(&path)
        .unwrap_or_else(|_| {
            panic!(
                "no snapshot {}; run with UPDATE_SNAPSHOTS=1",
                path.display()
            )
        })
        .replace("\r\n", "\n");
    assert_eq!(actual, expected, "snapshot {name} differs");
}

#[test]
fn every_lever_letter_matches_its_snapshot() {
    let entry = entry();
    for lever in Lever::ALL {
        snapshot(lever.name(), &compose(&entry, &profile(), &[lever], 0));
    }
}

#[test]
fn combined_post_spam_and_small_business_letters_match_their_snapshots() {
    let entry = entry();
    let app7 = [
        Lever::App7NoMarketing,
        Lever::App7NoFacilitation,
        Lever::App7Source,
    ];
    snapshot("combined-app7", &compose(&entry, &profile(), &app7, 0));
    snapshot(
        "post",
        &compose(&entry, &profile(), &[Lever::App7Source], 1),
    );
    let mut small = entry;
    small.exemption_risk = ghostsoap::core::register::ExemptionRisk::SmallBusinessPossible;
    snapshot(
        "small-business",
        &compose(&small, &profile(), &[Lever::App7NoMarketing], 0),
    );
}

#[test]
fn a_letter_contains_only_the_entrys_identity_fields() {
    let entry = entry();
    let profile = profile();
    let withheld = [
        "Saul Goodman",
        "0400 111 222",
        "1960-11-12",
        "Montgomery Road",
        "Juan Tabo",
        "2000",
    ];
    for lever in Lever::ALL {
        for channel in 0..entry.channels.len() {
            if !entry.channels[channel].levers.contains(&lever) {
                continue;
            }
            let letter = compose(&entry, &profile, &[lever], channel);
            assert!(
                letter.body.contains("Name: James McGill"),
                "{}",
                lever.name()
            );
            assert!(letter.body.contains("Email: jimmy@example.com"));
            for value in withheld {
                assert!(
                    !letter.body.contains(value),
                    "{} letter leaks `{value}`",
                    lever.name()
                );
            }
        }
    }
}

#[test]
fn a_missing_profile_value_drops_its_line() {
    let entry = entry();
    let profile = Profile {
        email: None,
        ..profile()
    };
    let letter = compose(&entry, &profile, &[Lever::App7NoMarketing], 0);
    assert!(!letter.body.contains("Email:"));
}
