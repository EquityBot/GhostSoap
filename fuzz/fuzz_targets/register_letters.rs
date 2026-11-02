#![no_main]
//! A register entry file, which pull requests add, through the register check and on to the
//! letters a checked entry produces.

use chrono::NaiveDate;
use ghostsoap::core::letters::{self, Request};
use ghostsoap::core::profile::Profile;
use ghostsoap::core::register::{ChannelKind, RegisterFiles, SourceFile, check, parse_register};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let mut file = SourceFile {
        name: "entry.toml".to_owned(),
        text: text.to_owned(),
    };
    // The check wants each entry in a file named after its id.
    let files = RegisterFiles {
        entries: vec![file.clone()],
        packs: Vec::new(),
    };
    if let Some(id) = parse_register(&files)
        .ok()
        .and_then(|r| r.entries.into_keys().next())
    {
        file.name = format!("{id}.toml");
    }
    let today = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
    if check(std::slice::from_ref(&file), &[], today).errors() > 0 {
        return;
    }
    let Ok(register) = parse_register(&RegisterFiles {
        entries: vec![file],
        packs: Vec::new(),
    }) else {
        return;
    };
    let profile = Profile {
        full_name: "Alex Example".to_owned(),
        alias_names: vec!["Alex E".to_owned()],
        email: Some("alex@example.com".to_owned()),
        phone: Some("0400 000 000".to_owned()),
        date_of_birth: Some("1990-01-02".to_owned()),
        address: Some("1 Example St, Sydney NSW 2000".to_owned()),
        previous_addresses: vec!["2 Sample Rd, Hobart TAS 7000".to_owned()],
        postcode: Some("2000".to_owned()),
    };
    for entry in register.entries.values() {
        for channel in entry.channels.iter().filter(|c| !c.levers.is_empty()) {
            let request = Request {
                channel,
                levers: &channel.levers,
                detail: Some("account 123"),
                date: today,
                tag: "GS-7K3QX",
            };
            let Ok(letter) = letters::compose(entry, &profile, &request) else {
                continue;
            };
            if channel.kind != ChannelKind::Email {
                continue;
            }
            // A checked entry's address and subject are one header line each.
            let eml = letters::eml(&letter);
            let headers = eml.split("\r\n\r\n").next().unwrap();
            assert_eq!(headers.split("\r\n").count(), 6, "{headers:?}");
            // Its address cannot end early and add fields to the mailto link.
            if let Some(link) = letters::mailto(&letter) {
                let fields = link.split_once('?').map(|(_, f)| f).unwrap();
                assert!(
                    fields.starts_with("subject=") && !link.contains('#'),
                    "{link}"
                );
                assert_eq!(fields.matches('&').count(), 1, "{link}");
            }
        }
    }
});
