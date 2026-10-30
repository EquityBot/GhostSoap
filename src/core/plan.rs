//! Choosing entries and levers for a round of requests (plan sections 5 and 6a).

use std::collections::BTreeSet;

use crate::core::letters::takes_detail;
use crate::core::register::{Channel, ChannelKind, Entry, Lever, Register};

/// Levers sent when the user names none: they return no personal information and need
/// nothing from the user beyond identity. `app11-destroy` is not one: only the user knows
/// whether they still deal with a business, so they name it with `--lever`.
const DEFAULT_LEVERS: [Lever; 3] = [
    Lever::App7NoMarketing,
    Lever::App7NoFacilitation,
    Lever::App7Source,
];

/// What the user asked for.
#[derive(Debug, Default)]
pub struct Selection {
    pub packs: Vec<String>,
    pub entries: Vec<String>,
    pub levers: Vec<Lever>,
    pub detail: Option<String>,
}

/// One step of a plan.
#[derive(Debug)]
pub enum Step<'a> {
    /// A message to one entry through one channel.
    Letter {
        entry: &'a Entry,
        channel: &'a Channel,
        levers: Vec<Lever>,
    },
    /// An overseas site: GhostSoap has not established that Australian law reaches it, so the
    /// user opts out by hand.
    Overseas { entry: &'a Entry },
    /// Nothing to send, with the reason.
    Skip { entry: &'a Entry, reason: String },
}

/// Turn a selection into steps, in the order the entries were named.
pub fn plan<'a>(register: &'a Register, selection: &Selection) -> Result<Vec<Step<'a>>, String> {
    let entries = resolve(register, selection)?;
    let single = selection.packs.is_empty() && entries.len() == 1;
    let guarded: Vec<&str> = selection
        .levers
        .iter()
        .filter(|&&l| is_guarded(l))
        .map(|l| l.name())
        .collect();
    if !guarded.is_empty() && !single {
        return Err(format!(
            "{} must be chosen for one entity at a time: use a single --entry and no --pack",
            guarded.join(", ")
        ));
    }
    check_detail(selection)?;
    Ok(entries
        .into_iter()
        .flat_map(|entry| steps(entry, selection))
        .collect())
}

/// Levers that return personal information, need the user's own words or have serious
/// effects: chosen for one entity at a time, never through a pack (plan section 6a). Every
/// `opt_in_levers` value in the register is one of these.
fn is_guarded(lever: Lever) -> bool {
    takes_detail(lever) || matches!(lever, Lever::App12 | Lever::S20rAccess | Lever::S21tAccess)
}

/// `--detail` goes with exactly one lever that takes it, so one request's words never land
/// in another's letter, and it must say something.
fn check_detail(selection: &Selection) -> Result<(), String> {
    let mut detailed: Vec<&str> = Vec::new();
    for lever in selection.levers.iter().filter(|&&l| takes_detail(l)) {
        if !detailed.contains(&lever.name()) {
            detailed.push(lever.name());
        }
    }
    if detailed.len() > 1 {
        return Err(format!(
            "{} each need their own --detail: choose one of them per run",
            detailed.join(", ")
        ));
    }
    match (selection.detail.as_deref(), detailed.is_empty()) {
        (None, false) => {
            Err("the chosen lever needs --detail: say what the request is about".to_owned())
        }
        (Some(_), true) => {
            Err("--detail is used only with levers that take it, such as app13".to_owned())
        }
        (Some(detail), false) if detail.trim().is_empty() => {
            Err("--detail must not be empty".to_owned())
        }
        _ => Ok(()),
    }
}

fn resolve<'a>(register: &'a Register, selection: &Selection) -> Result<Vec<&'a Entry>, String> {
    if selection.packs.is_empty() && selection.entries.is_empty() {
        return Err("choose entities with --pack or --entry".to_owned());
    }
    let mut ids: Vec<&str> = Vec::new();
    for pack in &selection.packs {
        let pack = register
            .packs
            .get(pack)
            .ok_or_else(|| format!("unknown pack `{pack}`"))?;
        ids.extend(pack.entries.iter().map(String::as_str));
    }
    ids.extend(selection.entries.iter().map(String::as_str));
    let mut seen = BTreeSet::new();
    let mut entries = Vec::new();
    for id in ids {
        let entry = register
            .entries
            .get(id)
            .ok_or_else(|| format!("unknown entry `{id}`"))?;
        if seen.insert(id) {
            entries.push(entry);
        }
    }
    Ok(entries)
}

fn steps<'a>(entry: &'a Entry, selection: &Selection) -> Vec<Step<'a>> {
    if let Some(obsolete) = &entry.obsolete {
        let reason = obsolete.skip_reason();
        return vec![Step::Skip { entry, reason }];
    }
    if entry.is_overseas() {
        return vec![Step::Overseas { entry }];
    }
    let listed = |l: &Lever| entry.levers.contains(l) || entry.opt_in_levers.contains(l);
    let levers: Vec<Lever> = if selection.levers.is_empty() {
        DEFAULT_LEVERS
            .into_iter()
            .filter(|l| entry.levers.contains(l))
            .collect()
    } else {
        selection.levers.iter().copied().filter(listed).collect()
    };
    if levers.is_empty() {
        let available: Vec<&str> = entry
            .levers
            .iter()
            .chain(&entry.opt_in_levers)
            .map(|l| l.name())
            .collect();
        let reason = format!(
            "none of the chosen levers applies; it accepts {}",
            available.join(", ")
        );
        return vec![Step::Skip { entry, reason }];
    }
    by_channel(entry, levers)
}

/// One letter per channel, in the order the levers were chosen.
fn by_channel(entry: &Entry, levers: Vec<Lever>) -> Vec<Step<'_>> {
    let mut groups: Vec<(&Channel, Vec<Lever>)> = Vec::new();
    let mut unsendable = Vec::new();
    for lever in levers {
        match channel_for(entry, lever) {
            Some(channel) => match groups.iter_mut().find(|(c, _)| std::ptr::eq(*c, channel)) {
                Some((_, group)) => group.push(lever),
                None => groups.push((channel, vec![lever])),
            },
            None => unsendable.push(lever.name()),
        }
    }
    let mut steps: Vec<Step> = groups
        .into_iter()
        .map(|(channel, levers)| Step::Letter {
            entry,
            channel,
            levers,
        })
        .collect();
    if !unsendable.is_empty() {
        let reason = format!("no written channel for {}", unsendable.join(", "));
        steps.push(Step::Skip { entry, reason });
    }
    steps
}

/// The channel to send `lever` through: the preferred one, else email, post, then web form.
/// Phone channels never carry letters.
fn channel_for(entry: &Entry, lever: Lever) -> Option<&Channel> {
    let serving = || {
        entry
            .channels
            .iter()
            .filter(|c| c.levers.contains(&lever) && c.kind != ChannelKind::Phone)
    };
    serving().find(|c| c.preferred).or_else(|| {
        [ChannelKind::Email, ChannelKind::Post, ChannelKind::WebForm]
            .into_iter()
            .find_map(|kind| serving().find(|c| c.kind == kind))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::register::{RegisterFiles, SourceFile, parse_register};

    const BROKER: &str = r#"
id = "pollos-au"
name = "Los Pollos Hermanos"
legal_entity = "Los Pollos Hermanos Pty Ltd"
abn = "88 000 014 675"
kinds = ["marketing-list"]
priority = "high"
quality = "verified"
verified_at = 2026-10-25
sources = ["https://example.com.au/privacy"]
levers = ["app7-no-marketing", "app7-source", "app11-destroy", "app13"]
opt_in_levers = ["app12"]
identity_fields = ["full_name", "email"]
exemption_risk = "none"
escalation = ["oaic"]
effort = "easy"

[[channels]]
kind = "email"
address = "privacy@example.com.au"
levers = ["app7-no-marketing", "app11-destroy", "app13", "app12"]

[[channels]]
kind = "post"
address = "PO Box 1, Sydney NSW 2001"
levers = ["app7-source", "app7-no-marketing"]
preferred = true

[[channels]]
kind = "phone"
address = "1300 000 000"
levers = []
"#;

    const OVERSEAS: &str = r#"
id = "fsociety-au"
name = "Fsociety People Search"
legal_entity = "E Corp"
kinds = ["people-search"]
priority = "standard"
quality = "verified"
verified_at = 2026-10-25
sources = ["https://example.com/privacy"]
identity_fields = ["full_name"]
exemption_risk = "overseas"
escalation = []
effort = "medium"

[[channels]]
kind = "web-form"
address = "https://example.com/opt-out"
levers = []

[manual_check]
url = "https://example.com/au"
"#;

    const PACK: &str = r#"
id = "start-here"
name = "Start here"
description = "Test pack"
entries = ["pollos-au", "fsociety-au"]
"#;

    fn register() -> Register {
        let file = |name: &str, text: &str| SourceFile {
            name: name.to_owned(),
            text: text.to_owned(),
        };
        parse_register(&RegisterFiles {
            entries: vec![
                file("pollos-au.toml", BROKER),
                file("fsociety-au.toml", OVERSEAS),
            ],
            packs: vec![file("start-here.toml", PACK)],
        })
        .unwrap()
    }

    fn select(
        packs: &[&str],
        entries: &[&str],
        levers: &[Lever],
        detail: Option<&str>,
    ) -> Selection {
        Selection {
            packs: packs.iter().map(|s| (*s).to_owned()).collect(),
            entries: entries.iter().map(|s| (*s).to_owned()).collect(),
            levers: levers.to_vec(),
            detail: detail.map(str::to_owned),
        }
    }

    fn letters<'a>(steps: &'a [Step<'a>]) -> Vec<(&'a str, Vec<Lever>)> {
        steps
            .iter()
            .filter_map(|s| match s {
                Step::Letter {
                    channel, levers, ..
                } => Some((channel.address.as_str(), levers.clone())),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_pack_gets_default_levers_by_preferred_channel_and_overseas_steps() {
        let register = register();
        let steps = plan(&register, &select(&["start-here"], &[], &[], None)).unwrap();
        assert_eq!(
            letters(&steps),
            [(
                "PO Box 1, Sydney NSW 2001",
                vec![Lever::App7NoMarketing, Lever::App7Source]
            )]
        );
        assert!(
            matches!(steps.last(), Some(Step::Overseas { entry }) if entry.id == "fsociety-au")
        );
    }

    #[test]
    fn a_removal_request_goes_only_when_named() {
        let register = register();
        let selection = select(&["start-here"], &[], &[Lever::App11Destroy], None);
        let steps = plan(&register, &selection).unwrap();
        assert_eq!(
            letters(&steps),
            [("privacy@example.com.au", vec![Lever::App11Destroy])]
        );
    }

    #[test]
    fn guarded_levers_need_a_single_entry_and_no_pack() {
        let register = register();
        let err = plan(
            &register,
            &select(&["start-here"], &[], &[Lever::App12], None),
        )
        .unwrap_err();
        assert!(err.contains("one entity at a time"), "{err}");
        let both = ["pollos-au", "fsociety-au"];
        let err = plan(&register, &select(&[], &both, &[Lever::App12], None)).unwrap_err();
        assert!(err.contains("one entity at a time"), "{err}");
        let steps = plan(
            &register,
            &select(&[], &["pollos-au"], &[Lever::App12], None),
        )
        .unwrap();
        assert_eq!(
            letters(&steps),
            [("privacy@example.com.au", vec![Lever::App12])]
        );
    }

    #[test]
    fn detail_is_required_exactly_when_a_lever_takes_it() {
        let register = register();
        let err = plan(
            &register,
            &select(&[], &["pollos-au"], &[Lever::App13], None),
        )
        .unwrap_err();
        assert!(err.contains("needs --detail"), "{err}");
        let err = plan(&register, &select(&[], &["pollos-au"], &[], Some("x"))).unwrap_err();
        assert!(err.contains("--detail is used only"), "{err}");
        let two = [Lever::App13, Lever::S20kBan];
        let err = plan(&register, &select(&[], &["pollos-au"], &two, Some("x"))).unwrap_err();
        assert!(err.contains("each need their own --detail"), "{err}");
        let twice = [Lever::App13, Lever::App13];
        assert!(plan(&register, &select(&[], &["pollos-au"], &twice, Some("x"))).is_ok());
        let err = plan(
            &register,
            &select(&[], &["pollos-au"], &[Lever::App13], Some("  ")),
        )
        .unwrap_err();
        assert_eq!(err, "--detail must not be empty");
        let err = plan(
            &register,
            &select(&[], &["pollos-au"], &[Lever::S20kBan], None),
        )
        .unwrap_err();
        assert!(
            err.contains("needs --detail"),
            "a ban states its grounds: {err}"
        );
        let selection = select(&[], &["pollos-au"], &[Lever::App13], Some("x"));
        let steps = plan(&register, &selection).unwrap();
        assert_eq!(
            letters(&steps),
            [("privacy@example.com.au", vec![Lever::App13])]
        );
    }

    #[test]
    fn unknown_names_and_unlisted_levers() {
        let register = register();
        assert_eq!(
            plan(&register, &select(&["nope"], &[], &[], None)).unwrap_err(),
            "unknown pack `nope`"
        );
        assert_eq!(
            plan(&register, &select(&[], &["nope-au"], &[], None)).unwrap_err(),
            "unknown entry `nope-au`"
        );
        assert!(plan(&register, &select(&[], &[], &[], None)).is_err());
        let selection = select(&[], &["pollos-au"], &[Lever::App7NoFacilitation], None);
        let steps = plan(&register, &selection).unwrap();
        assert!(
            matches!(&steps[..], [Step::Skip { reason, .. }] if reason.contains("accepts app7-no-marketing"))
        );
    }

    #[test]
    fn levers_split_across_channels_become_separate_letters() {
        let register = register();
        let levers = [Lever::App7Source, Lever::App13];
        let steps = plan(&register, &select(&[], &["pollos-au"], &levers, Some("x"))).unwrap();
        assert_eq!(
            letters(&steps),
            [
                ("PO Box 1, Sydney NSW 2001", vec![Lever::App7Source]),
                ("privacy@example.com.au", vec![Lever::App13]),
            ]
        );
    }
}
