//! Register entries and packs: the model and its validation. Pure functions, no I/O.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{Months, NaiveDate};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use toml::value::Datetime;

/// What an entity does with personal information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    CreditReportingBody,
    PropertyData,
    PeopleSearch,
    MarketingList,
    AdTech,
    Directory,
    Loyalty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Priority {
    Crucial,
    High,
    Standard,
}

/// How far an entry has been checked (plan section 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Quality {
    /// A GhostSoap request got a reply.
    Tested,
    /// The contact was checked against the entity's own pages, by a person or an AI agent,
    /// with the evidence recorded in `docs/provenance.md`.
    Verified,
    /// Taken from a licence-compatible dataset.
    Imported,
    /// Named in a non-reusable source and not yet checked.
    Lead,
}

impl Quality {
    /// Only tested and verified entries may enter packs.
    pub fn is_packable(self) -> bool {
        matches!(self, Self::Tested | Self::Verified)
    }
}

/// A request the law lets the user make (plan section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Lever {
    App7NoMarketing,
    App7NoFacilitation,
    App7Source,
    App11Destroy,
    App12,
    App13,
    App13NotifyRecipients,
    App13Statement,
    S20rAccess,
    S20tCorrection,
    S20kBan,
    S21tAccess,
    S21vCorrection,
    SpamUnsubscribe,
}

impl Lever {
    /// The name used in register files.
    pub fn name(self) -> &'static str {
        match self {
            Self::App7NoMarketing => "app7-no-marketing",
            Self::App7NoFacilitation => "app7-no-facilitation",
            Self::App7Source => "app7-source",
            Self::App11Destroy => "app11-destroy",
            Self::App12 => "app12",
            Self::App13 => "app13",
            Self::App13NotifyRecipients => "app13-notify-recipients",
            Self::App13Statement => "app13-statement",
            Self::S20rAccess => "s20r-access",
            Self::S20tCorrection => "s20t-correction",
            Self::S20kBan => "s20k-ban",
            Self::S21tAccess => "s21t-access",
            Self::S21vCorrection => "s21v-correction",
            Self::SpamUnsubscribe => "spam-unsubscribe",
        }
    }
}

/// Identity details a letter may include. Document numbers are deliberately absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityField {
    FullName,
    AliasNames,
    Email,
    Phone,
    DateOfBirth,
    Address,
    PreviousAddresses,
    Postcode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExemptionRisk {
    None,
    SmallBusinessPossible,
    Overseas,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Escalation {
    Oaic,
    Afca,
    Acma,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Effort {
    Easy,
    Medium,
    Hard,
    Impossible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChannelKind {
    Email,
    WebForm,
    Post,
    Phone,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    pub kind: ChannelKind,
    /// An email address, form URL, postal address or phone number, by `kind`.
    pub address: String,
    /// The entry's levers this channel can carry; empty for a contact-only channel.
    pub levers: Vec<Lever>,
    /// The channel to use for each of its levers; at most one per lever.
    #[serde(default)]
    pub preferred: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManualCheck {
    /// The entity's own search or opt-out page; GhostSoap only opens it.
    pub url: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Obsolete {
    pub reason: String,
    pub replaced_by: Option<String>,
}

/// One entity in `registry/au/<id>.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Permanent slug, equal to the file name; never reused.
    pub id: String,
    pub name: String,
    pub legal_entity: String,
    pub abn: String,
    pub kinds: Vec<Kind>,
    pub priority: Priority,
    pub quality: Quality,
    pub verified_at: Datetime,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub levers: Vec<Lever>,
    /// Levers the user must choose per entity; never part of a pack (plan section 6a).
    #[serde(default)]
    pub opt_in_levers: Vec<Lever>,
    pub identity_fields: Vec<IdentityField>,
    pub exemption_risk: ExemptionRisk,
    /// Ordered escalation targets.
    pub escalation: Vec<Escalation>,
    pub effort: Effort,
    #[serde(default)]
    pub channels: Vec<Channel>,
    pub manual_check: Option<ManualCheck>,
    pub obsolete: Option<Obsolete>,
    /// Hashes of source pages at verification, for the monthly drift check.
    #[serde(default)]
    pub evidence: Vec<Evidence>,
}

/// A source page's state when the entry was verified (normalisation in `registry/README.md`).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    /// One of the entry's `sources`.
    pub url: String,
    pub checked_at: Datetime,
    /// Lowercase hex SHA-256 of the page's normalised main text.
    pub sha256: String,
}

/// A named group of entries in `registry/au/packs/<id>.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pack {
    pub id: String,
    pub name: String,
    pub description: String,
    pub entries: Vec<String>,
}

/// Entry files and pack files, each sorted by name.
#[derive(Debug, Clone)]
pub struct RegisterFiles {
    pub entries: Vec<SourceFile>,
    pub packs: Vec<SourceFile>,
}

/// A register file: its file name and contents.
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub name: String,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    /// The file name the finding is about.
    pub file: String,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct Report {
    /// Entries and packs that parsed.
    pub entries: usize,
    pub packs: usize,
    pub findings: Vec<Finding>,
}

impl Report {
    pub fn errors(&self) -> usize {
        self.count(Severity::Error)
    }

    pub fn warnings(&self) -> usize {
        self.count(Severity::Warning)
    }

    fn count(&self, severity: Severity) -> usize {
        self.findings
            .iter()
            .filter(|f| f.severity == severity)
            .count()
    }

    fn push(&mut self, severity: Severity, file: &str, message: String) {
        self.findings.push(Finding {
            severity,
            file: file.to_owned(),
            message,
        });
    }
}

/// What later checks need to know about an entry already seen.
struct Known<'a> {
    file: &'a str,
    quality: Quality,
    obsolete: bool,
}

/// Levers that return the user's personal information. An impersonator could use them to
/// collect a victim's details, so they are opt-in per entity (plan section 6a).
const ACCESS_LEVERS: [Lever; 3] = [Lever::App12, Lever::S20rAccess, Lever::S21tAccess];

/// Validate register entries and packs. `today` dates the staleness and future-date checks.
/// Findings about packs name the file as `packs/<name>`; a finding with an empty file name
/// is about the register as a whole.
pub fn check(entries: &[SourceFile], packs: &[SourceFile], today: NaiveDate) -> Report {
    let mut report = Report::default();
    if entries.is_empty() {
        let message = "the register has no entry files".to_owned();
        report.push(Severity::Error, "", message);
    }
    let mut ids: BTreeMap<String, Known> = BTreeMap::new();
    let mut replacements = Vec::new();
    for file in entries {
        let Some(entry) = parse::<Entry>(&file.text, &file.name, &mut report) else {
            continue;
        };
        report.entries += 1;
        check_entry(&entry, &file.name, today, &mut report);
        if let Some(first) = ids.get(&entry.id) {
            let message = format!("id `{}` is already used by {}", entry.id, first.file);
            report.push(Severity::Error, &file.name, message);
            continue;
        }
        if let Some(target) = entry.obsolete.as_ref().and_then(|o| o.replaced_by.clone()) {
            replacements.push((file.name.as_str(), target));
        }
        let known = Known {
            file: &file.name,
            quality: entry.quality,
            obsolete: entry.obsolete.is_some(),
        };
        ids.insert(entry.id, known);
    }
    for (file, target) in replacements {
        if !ids.contains_key(&target) {
            let message = format!("obsolete.replaced_by names unknown entry `{target}`");
            report.push(Severity::Error, file, message);
        }
    }
    check_packs(packs, &ids, &mut report);
    report
}

fn parse<T: DeserializeOwned>(text: &str, name: &str, report: &mut Report) -> Option<T> {
    match toml::from_str(text) {
        Ok(value) => Some(value),
        Err(e) => {
            report.push(Severity::Error, name, e.to_string().trim_end().to_owned());
            None
        }
    }
}

fn check_entry(entry: &Entry, file: &str, today: NaiveDate, report: &mut Report) {
    let mut errors = Vec::new();
    check_id(&entry.id, file, &mut errors);
    if entry.name.trim().is_empty() || entry.legal_entity.trim().is_empty() {
        errors.push("name and legal_entity must not be empty".to_owned());
    }
    if !abn_is_valid(&entry.abn) {
        errors.push(format!(
            "ABN `{}` is malformed: use 11 digits as `NN NNN NNN NNN` with a valid check digit",
            entry.abn
        ));
    }
    if entry.kinds.is_empty() {
        errors.push("kinds must not be empty".to_owned());
    }
    if entry.sources.is_empty() {
        errors.push("no source: list at least one URL in `sources`".to_owned());
    }
    for source in entry.sources.iter().filter(|s| !is_https(s)) {
        errors.push(format!("source `{source}` is not an https URL"));
    }
    check_levers(entry, &mut errors);
    check_channels(entry, &mut errors);
    check_channel_levers(entry, &mut errors);
    check_obsolete(entry, &mut errors);
    check_evidence(entry, today, &mut errors);
    match date_of(&entry.verified_at) {
        None => errors.push("verified_at must be a date such as 2026-10-24".to_owned()),
        Some(date) if is_future(date, today) => {
            errors.push(format!("verified_at {date} is in the future"));
        }
        Some(date) if is_stale(date, today) => {
            let message =
                format!("verified_at {date} is more than 12 months old; recheck the entry");
            report.push(Severity::Warning, file, message);
        }
        Some(_) => {}
    }
    for message in errors {
        report.push(Severity::Error, file, message);
    }
}

fn check_id(id: &str, file: &str, errors: &mut Vec<String>) {
    if !is_slug(id) {
        errors.push(format!(
            "id `{id}` must be lowercase letters, digits and single hyphens"
        ));
    } else if file.strip_suffix(".toml") != Some(id) {
        errors.push(format!("id `{id}` does not match the file name"));
    }
}

fn check_levers(entry: &Entry, errors: &mut Vec<String>) {
    if entry.obsolete.is_none() && entry.levers.is_empty() && entry.opt_in_levers.is_empty() {
        errors.push("an entry must list at least one lever".to_owned());
    }
    if entry.levers.iter().any(|l| ACCESS_LEVERS.contains(l)) {
        errors.push(
            "access levers (app12, s20r-access, s21t-access) belong in opt_in_levers only"
                .to_owned(),
        );
    }
    if entry.opt_in_levers.iter().any(|l| entry.levers.contains(l)) {
        errors.push("a lever is listed in both levers and opt_in_levers".to_owned());
    }
}

fn check_channels(entry: &Entry, errors: &mut Vec<String>) {
    if entry.obsolete.is_none() && entry.channels.is_empty() {
        errors.push("channels must list at least one way to send a request".to_owned());
    }
    for channel in &entry.channels {
        let address = &channel.address;
        match channel.kind {
            ChannelKind::Email if !is_email(address) => {
                errors.push(format!("`{address}` is not an email address"));
            }
            ChannelKind::WebForm if !is_https(address) => {
                errors.push(format!("web form `{address}` is not an https URL"));
            }
            _ if address.trim().is_empty() => {
                errors.push("a channel address must not be empty".to_owned());
            }
            _ => {}
        }
    }
    if let Some(manual) = &entry.manual_check
        && !is_https(&manual.url)
    {
        errors.push(format!(
            "manual_check.url `{}` is not an https URL",
            manual.url
        ));
    }
}

/// Every lever has a channel, every channel lever belongs to the entry, and no lever has
/// two preferred channels.
fn check_channel_levers(entry: &Entry, errors: &mut Vec<String>) {
    let listed = || entry.levers.iter().chain(&entry.opt_in_levers);
    for channel in &entry.channels {
        for lever in channel.levers.iter().filter(|l| !listed().any(|x| x == *l)) {
            errors.push(format!(
                "channel `{}` serves `{}`, which the entry does not list",
                channel.address,
                lever.name()
            ));
        }
    }
    for lever in listed() {
        let serving: Vec<&Channel> = entry
            .channels
            .iter()
            .filter(|c| c.levers.contains(lever))
            .collect();
        if serving.is_empty() {
            errors.push(format!("no channel serves `{}`", lever.name()));
        }
        if serving.iter().filter(|c| c.preferred).count() > 1 {
            errors.push(format!(
                "more than one channel is preferred for `{}`",
                lever.name()
            ));
        }
    }
}

fn check_evidence(entry: &Entry, today: NaiveDate, errors: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    for evidence in &entry.evidence {
        let url = &evidence.url;
        if !seen.insert(url) {
            errors.push(format!("evidence url `{url}` is listed twice"));
        }
        if !entry.sources.contains(url) {
            errors.push(format!("evidence url `{url}` is not one of the sources"));
        }
        let hex = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
        if evidence.sha256.len() != 64 || !evidence.sha256.chars().all(hex) {
            errors.push(format!(
                "evidence sha256 for `{url}` must be 64 lowercase hex digits"
            ));
        }
        if date_of(&evidence.checked_at).is_none_or(|date| is_future(date, today)) {
            errors.push(format!(
                "evidence checked_at for `{url}` must be a date, not in the future"
            ));
        }
    }
}

fn check_obsolete(entry: &Entry, errors: &mut Vec<String>) {
    let Some(obsolete) = &entry.obsolete else {
        return;
    };
    if obsolete.reason.trim().is_empty() {
        errors.push("obsolete.reason must not be empty".to_owned());
    }
    if obsolete.replaced_by.as_deref() == Some(entry.id.as_str()) {
        errors.push("obsolete.replaced_by must name another entry".to_owned());
    }
}

fn check_packs(packs: &[SourceFile], ids: &BTreeMap<String, Known>, report: &mut Report) {
    for file in packs {
        let name = format!("packs/{}", file.name);
        let Some(pack) = parse::<Pack>(&file.text, &name, report) else {
            continue;
        };
        report.packs += 1;
        let mut errors = Vec::new();
        check_id(&pack.id, &file.name, &mut errors);
        if pack.entries.is_empty() {
            errors.push("pack lists no entries".to_owned());
        }
        let mut listed = BTreeSet::new();
        for id in &pack.entries {
            if !listed.insert(id) {
                errors.push(format!("pack lists `{id}` twice"));
                continue;
            }
            match ids.get(id) {
                None => errors.push(format!("pack lists unknown entry `{id}`")),
                Some(known) if known.obsolete => {
                    errors.push(format!("pack lists obsolete entry `{id}`"));
                }
                Some(known) if !known.quality.is_packable() => {
                    errors.push(format!(
                        "pack lists `{id}`, which is below verified quality"
                    ));
                }
                Some(_) => {}
            }
        }
        for message in errors {
            report.push(Severity::Error, &name, message);
        }
    }
}

/// An ABN in the form `NN NNN NNN NNN` whose digits pass the ATO check-digit test.
pub fn abn_is_valid(abn: &str) -> bool {
    const WEIGHTS: [u32; 11] = [10, 1, 3, 5, 7, 9, 11, 13, 15, 17, 19];
    let shape = abn.len() == 14 && [2, 6, 10].iter().all(|&i| abn.as_bytes()[i] == b' ');
    let digits: Option<Vec<u32>> = abn
        .chars()
        .filter(|&c| c != ' ')
        .map(|c| c.to_digit(10))
        .collect();
    match digits {
        Some(mut d) if shape && d.len() == 11 && d[0] > 0 => {
            d[0] -= 1;
            d.iter().zip(WEIGHTS).map(|(x, w)| x * w).sum::<u32>() % 89 == 0
        }
        _ => false,
    }
}

fn is_slug(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('-')
        && !id.ends_with('-')
        && !id.contains("--")
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// An https URL with a dotted host name and, optionally, a numeric port.
fn is_https(url: &str) -> bool {
    url.strip_prefix("https://").is_some_and(|rest| {
        let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
        let (host, port) = authority.split_once(':').unwrap_or((authority, "1"));
        let port_ok =
            port.bytes().all(|b| b.is_ascii_digit()) && port.parse::<u16>().is_ok_and(|p| p > 0);
        is_domain(host) && port_ok && !url.contains(char::is_whitespace)
    })
}

/// Two or more DNS labels of 1 to 63 letters, digits and inner hyphens, at most 253
/// bytes in all.
fn is_domain(host: &str) -> bool {
    let labels: Vec<&str> = host.split('.').collect();
    labels.len() >= 2
        && host.len() <= 253
        && labels.iter().all(|label| {
            (1..=63).contains(&label.len())
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

fn is_email(address: &str) -> bool {
    address.split_once('@').is_some_and(|(local, domain)| {
        !local.is_empty()
            && local
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "!#$%&'*+-/=?^_`{|}~.".contains(c))
            && !local.starts_with('.')
            && !local.ends_with('.')
            && !local.contains("..")
            && is_domain(domain)
            && !address.contains(char::is_whitespace)
    })
}

/// A plain TOML date (no time or offset) as a calendar date.
fn date_of(value: &Datetime) -> Option<NaiveDate> {
    match (value.date, value.time, value.offset) {
        (Some(d), None, None) => {
            NaiveDate::from_ymd_opt(i32::from(d.year), u32::from(d.month), u32::from(d.day))
        }
        _ => None,
    }
}

/// More than a day ahead. The day of grace covers entries dated in Australian time
/// whilst CI runs in UTC.
fn is_future(date: NaiveDate, today: NaiveDate) -> bool {
    today.succ_opt().is_some_and(|tomorrow| date > tomorrow)
}

fn is_stale(verified: NaiveDate, today: NaiveDate) -> bool {
    today
        .checked_sub_months(Months::new(12))
        .is_some_and(|cutoff| verified < cutoff)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
id = "example-au"
name = "Example"
legal_entity = "Example Pty Ltd"
abn = "88 000 014 675"
kinds = ["marketing-list"]
priority = "high"
quality = "verified"
verified_at = 2026-10-24
sources = ["https://example.com.au/privacy"]
levers = ["app7-no-marketing", "app7-source", "app13"]
opt_in_levers = ["app12"]
identity_fields = ["full_name", "email"]
exemption_risk = "none"
escalation = ["oaic"]
effort = "easy"

[[channels]]
kind = "email"
address = "privacy@example.com.au"
levers = ["app12", "app13", "app7-no-marketing", "app7-source"]
preferred = true
"#;

    const CHANNEL_LEVERS: &str = "[\"app12\", \"app13\", \"app7-no-marketing\", \"app7-source\"]";

    const PACK: &str = r#"
id = "marketing"
name = "Marketing"
description = "Marketing lists"
entries = ["example-au"]
"#;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 24).unwrap()
    }

    fn file(name: &str, text: &str) -> SourceFile {
        SourceFile {
            name: name.to_owned(),
            text: text.to_owned(),
        }
    }

    /// Check one entry file, `example-au.toml`, with `edit` applied to the valid text.
    fn check_one(edit: impl Fn(&str) -> String) -> Report {
        check(&[file("example-au.toml", &edit(VALID))], &[], today())
    }

    fn messages(report: &Report, severity: Severity) -> Vec<&str> {
        report
            .findings
            .iter()
            .filter(|f| f.severity == severity)
            .map(|f| f.message.as_str())
            .collect()
    }

    fn assert_error(report: &Report, needle: &str) {
        let errors = messages(report, Severity::Error);
        assert!(
            errors.iter().any(|m| m.contains(needle)),
            "no error containing {needle:?} in {errors:?}"
        );
    }

    #[test]
    fn valid_entry_and_pack_pass() {
        let report = check(
            &[file("example-au.toml", VALID)],
            &[file("marketing.toml", PACK)],
            today(),
        );
        assert_eq!(report.findings, []);
        assert_eq!((report.entries, report.packs), (1, 1));
    }

    #[test]
    fn rejects_missing_or_empty_sources() {
        let report = check_one(|t| t.replace("sources = [\"https://example.com.au/privacy\"]", ""));
        assert_error(&report, "no source");
        let report = check_one(|t| t.replace("[\"https://example.com.au/privacy\"]", "[]"));
        assert_error(&report, "no source");
    }

    #[test]
    fn rejects_a_non_https_source() {
        let report = check_one(|t| t.replace("https://example.com.au/privacy", "http://x.au"));
        assert_error(&report, "is not an https URL");
    }

    #[test]
    fn rejects_unknown_lever_and_kind() {
        let report = check_one(|t| t.replace("\"app7-source\"", "\"app99\""));
        assert_error(&report, "unknown variant `app99`");
        let report = check_one(|t| t.replace("\"marketing-list\"", "\"broker\""));
        assert_error(&report, "unknown variant `broker`");
    }

    #[test]
    fn rejects_malformed_abns() {
        for abn in [
            "88 000 014 676",
            "88000014675",
            "88 000 014 67",
            "8a 000 014 675",
        ] {
            let report = check_one(|t| t.replace("88 000 014 675", abn));
            assert_error(&report, "is malformed");
        }
    }

    #[test]
    fn abn_check_digit() {
        assert!(abn_is_valid("88 000 014 675"));
        assert!(abn_is_valid("51 824 753 556"));
        assert!(!abn_is_valid("51 824 753 557"));
        assert!(!abn_is_valid("00 000 000 000"));
    }

    #[test]
    fn rejects_a_reused_id() {
        let report = check(
            &[
                file("example-au.toml", VALID),
                file("example-au-2.toml", VALID),
            ],
            &[],
            today(),
        );
        assert_error(
            &report,
            "id `example-au` is already used by example-au.toml",
        );
    }

    #[test]
    fn rejects_an_id_that_does_not_match_the_file() {
        let report = check(&[file("other.toml", VALID)], &[], today());
        assert_error(&report, "does not match the file name");
        let report = check_one(|t| t.replace("\"example-au\"", "\"Example_AU\""));
        assert_error(&report, "must be lowercase");
    }

    #[test]
    fn rejects_access_levers_outside_opt_in_levers() {
        for lever in ["app12", "s20r-access", "s21t-access"] {
            let report =
                check_one(|t| t.replace("\"app13\"]", &format!("\"app13\", \"{lever}\"]")));
            assert_error(&report, "belong in opt_in_levers only");
        }
    }

    #[test]
    fn accepts_an_entry_with_only_opt_in_levers() {
        let report = check_one(|t| {
            t.replace("[\"app7-no-marketing\", \"app7-source\", \"app13\"]", "[]")
                .replace(CHANNEL_LEVERS, "[\"app12\"]")
        });
        assert_eq!(report.findings, []);
        let report = check_one(|t| {
            t.replace("[\"app7-no-marketing\", \"app7-source\", \"app13\"]", "[]")
                .replace("[\"app12\"]", "[]")
        });
        assert_error(&report, "at least one lever");
    }

    #[test]
    fn rejects_unserved_and_unlisted_channel_levers() {
        let report = check_one(|t| t.replace(CHANNEL_LEVERS, "[\"app12\", \"app7-no-marketing\"]"));
        assert_error(&report, "no channel serves `app13`");
        assert_error(&report, "no channel serves `app7-source`");
        let report = check_one(|t| {
            t.replace(
                "[\"app12\", \"app13\",",
                "[\"app12\", \"app13\", \"s20k-ban\",",
            )
        });
        assert_error(&report, "serves `s20k-ban`, which the entry does not list");
    }

    #[test]
    fn rejects_two_preferred_channels_for_a_lever() {
        let second = "\n[[channels]]\nkind = \"post\"\naddress = \"PO Box 1\"\nlevers = [\"app13\"]\npreferred = true\n";
        let report = check_one(|t| format!("{t}{second}"));
        assert_error(&report, "more than one channel is preferred for `app13`");
        let report = check_one(|t| format!("{t}{}", second.replace("preferred = true\n", "")));
        assert_eq!(report.findings, []);
    }

    #[test]
    fn checks_channel_levers_on_obsolete_entries() {
        let obsolete = "\n[obsolete]\nreason = \"closed\"\n";
        let second = "\n[[channels]]\nkind = \"post\"\naddress = \"PO Box 1\"\nlevers = [\"app13\"]\npreferred = true\n";
        let report = check_one(|t| format!("{t}{second}{obsolete}"));
        assert_error(&report, "more than one channel is preferred for `app13`");
        let report = check_one(|t| {
            let t = t.replace(CHANNEL_LEVERS, "[\"app12\", \"app7-no-marketing\"]");
            format!("{t}{obsolete}")
        });
        assert_error(&report, "no channel serves `app13`");
    }

    #[test]
    fn checks_evidence() {
        let sha = "a".repeat(64);
        let evidence = |url: &str, date: &str, sha: &str| {
            format!("\n[[evidence]]\nurl = \"{url}\"\nchecked_at = {date}\nsha256 = \"{sha}\"\n")
        };
        let good = evidence("https://example.com.au/privacy", "2026-10-24", &sha);
        assert_eq!(check_one(|t| format!("{t}{good}")).findings, []);
        let report = check_one(|t| {
            format!(
                "{t}{}",
                evidence("https://other.com.au/", "2026-10-24", &sha)
            )
        });
        assert_error(&report, "is not one of the sources");
        let report = check_one(|t| {
            format!(
                "{t}{}",
                evidence(
                    "https://example.com.au/privacy",
                    "2026-10-24",
                    &"A".repeat(64)
                )
            )
        });
        assert_error(&report, "64 lowercase hex digits");
        let report = check_one(|t| {
            format!(
                "{t}{}",
                evidence("https://example.com.au/privacy", "2026-10-26", &sha)
            )
        });
        assert_error(&report, "not in the future");
        let report = check_one(|t| format!("{t}{good}{good}"));
        assert_error(&report, "is listed twice");
    }

    #[test]
    fn rejects_an_empty_register() {
        let report = check(&[], &[], today());
        assert_error(&report, "no entry files");
    }

    #[test]
    fn rejects_a_future_verified_at() {
        let report = check_one(|t| t.replace("2026-10-24", "2026-10-25"));
        assert_eq!(report.findings, [], "one day of grace");
        let report = check_one(|t| t.replace("2026-10-24", "2026-10-26"));
        assert_error(&report, "is in the future");
    }

    #[test]
    fn rejects_urls_without_a_host_and_bad_domains() {
        for url in [
            "https:///",
            "https://?",
            "https://example",
            "https://example.com:bad/path",
            "https://-bad.example/path",
            "https://example.com:+443/path",
            &format!("https://{}.com/", "a".repeat(64)),
        ] {
            let report = check_one(|t| t.replace("https://example.com.au/privacy", url));
            assert_error(&report, "is not an https URL");
        }
        for email in [
            "privacy@example..au",
            "privacy@-bad.example",
            "privacy..team@example.com.au",
            ".privacy@example.com.au",
            "privacy,team@example.com.au",
        ] {
            let report = check_one(|t| t.replace("privacy@example.com.au", email));
            assert_error(&report, "is not an email address");
        }
        let report = check_one(|t| {
            t.replace(
                "https://example.com.au/privacy",
                "https://example.com.au:443/privacy?x=1#y",
            )
            .replace("privacy@example.com.au", "privacy.team+au@example.com.au")
        });
        assert_eq!(report.findings, []);
    }

    #[test]
    fn rejects_unknown_fields_and_bad_channels() {
        let report = check_one(|t| format!("drivers_licence = \"x\"\n{t}"));
        assert_error(&report, "unknown field `drivers_licence`");
        let report = check_one(|t| t.replace("privacy@example.com.au", "privacy"));
        assert_error(&report, "is not an email address");
    }

    #[test]
    fn warns_but_passes_when_verified_at_is_stale() {
        let report = check_one(|t| t.replace("2026-10-24", "2025-10-23"));
        assert_eq!(report.errors(), 0);
        assert_eq!(report.warnings(), 1);
        assert!(messages(&report, Severity::Warning)[0].contains("more than 12 months old"));
        let report = check_one(|t| t.replace("2026-10-24", "2025-10-24"));
        assert_eq!(report.findings, []);
    }

    #[test]
    fn rejects_a_verified_at_with_a_time() {
        let report = check_one(|t| t.replace("2026-10-24", "2026-10-24T10:00:00"));
        assert_error(&report, "verified_at must be a date");
    }

    #[test]
    fn rejects_bad_pack_entries() {
        let lead = VALID.replace("\"verified\"", "\"lead\"");
        let pack = PACK.replace("[\"example-au\"]", "[\"example-au\", \"nobody\"]");
        let report = check(
            &[file("example-au.toml", &lead)],
            &[file("marketing.toml", &pack)],
            today(),
        );
        assert_error(&report, "below verified quality");
        assert_error(&report, "unknown entry `nobody`");
    }

    #[test]
    fn rejects_levers_in_a_pack() {
        let pack = format!("{PACK}levers = [\"app12\"]\n");
        let report = check(
            &[file("example-au.toml", VALID)],
            &[file("marketing.toml", &pack)],
            today(),
        );
        assert_error(&report, "unknown field `levers`");
    }

    #[test]
    fn rejects_blank_or_self_obsolete_details() {
        let report = check_one(|t| {
            format!(
                "{t}
[obsolete]
reason = \" \"
replaced_by = \"example-au\"
"
            )
        });
        assert_error(&report, "obsolete.reason must not be empty");
        assert_error(&report, "must name another entry");
    }

    #[test]
    fn names_pack_findings_by_their_folder() {
        let pack = PACK.replace("[\"example-au\"]", "[\"nobody\"]");
        let report = check(
            &[file("example-au.toml", VALID)],
            &[file(
                "example-au.toml",
                &pack.replace("\"marketing\"", "\"example-au\""),
            )],
            today(),
        );
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].file, "packs/example-au.toml");
    }

    #[test]
    fn rejects_an_unknown_replacement() {
        let report = check_one(|t| {
            format!("{t}\n[obsolete]\nreason = \"merged\"\nreplaced_by = \"gone-au\"\n")
        });
        assert_error(&report, "unknown entry `gone-au`");
    }
}
