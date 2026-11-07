// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Maintainer tasks for the GhostSoap register. Not part of the `ghostsoap` binary, which
//! makes no network requests of its own.
//!
//! ```text
//! cargo run -p xtask -- evidence-hash <url>...        print the evidence hash of each page
//! cargo run -p xtask -- evidence-drift [registry/au [text dir]]
//!                                                     report pages whose hash has changed
//!                                                     and laws or legal pages newer than
//!                                                     the law table, optionally saving
//!                                                     each page's text
//! cargo run -p xtask -- email-changes <base> <head>   warn of email addresses a change added
//! ```

mod text;

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use ghostsoap::core::register::{ChannelKind, Entry, SourceFile};
use sha2::{Digest, Sha256};

/// Pages with less main text than this are bot checks or script-only shells.
const MIN_TEXT: usize = 500;
const USER_AGENT: &str = "GhostSoap-evidence-check/0.1 (+https://github.com/yungSNAT/GhostSoap)";
/// Whole-request limit, so one slow or silent site fails instead of stalling the check.
const TIMEOUT: Duration = Duration::from_secs(30);
/// Each law the plan pins: its name, Federal Register title and the version read.
const LAWS: [(&str, &str, &str); 4] = [
    ("Privacy Act 1988", "C2004A03712", "C2026C00227"),
    ("Spam Act 2003", "C2004A01214", "C2016C00614"),
    ("Spam Regulations 2021", "F2021L00285", "F2021L00285"),
    ("Criminal Code", "C2004A04868", "C2026C00243"),
];

/// Pages outside the register that the escalation targets rest on: a name, a file name
/// for the saved text, the URL and the evidence hash of the text read.
const LEGAL_PAGES: [LegalPage; 1] = [(
    "OAIC recognised external dispute resolution schemes register",
    "oaic-edr-register",
    "https://www.oaic.gov.au/privacy/privacy-registers/recognised-external-dispute-resolution-schemes-register",
    "2e23acc8194089cd50cefeb1d29a4d26df08480b6656738214cde865edbb430d",
)];

type LegalPage<'a> = (&'a str, &'a str, &'a str, &'a str);

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .build()
        .into()
}

/// A fetched evidence page.
struct Page {
    sha256: String,
    /// The normalised main text the hash covers; `None` for a non-HTML page.
    main: Option<String>,
    /// Lowercase raw HTML, main text and Cloudflare-protected addresses, for finding an
    /// email address in a link, a footer, or entity-encoded or obfuscated text; `None` for
    /// a non-HTML page.
    text: Option<String>,
}

/// Why an evidence page could not be hashed.
#[derive(Debug)]
enum PageError {
    /// 401, 403 or 429: sites answer scripts this way even when the page is up.
    Refused(String),
    Failed(String),
}

impl std::fmt::Display for PageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused(e) | Self::Failed(e) => f.write_str(e),
        }
    }
}

/// Fetch `url`. Its evidence hash is SHA-256 of its normalised main text, or of the raw
/// bytes for a non-HTML response.
fn evidence_page(agent: &ureq::Agent, url: &str) -> Result<Page, PageError> {
    let mut response = agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| match e {
            ureq::Error::StatusCode(401 | 403 | 429) => PageError::Refused(e.to_string()),
            _ => PageError::Failed(e.to_string()),
        })?;
    let content_type = response
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap_or_default().to_owned());
    let bytes = response
        .body_mut()
        .with_config()
        .limit(20 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| PageError::Failed(e.to_string()))?;
    let (digest, main, text) = if text::is_html(content_type.as_deref(), &bytes) {
        let html = String::from_utf8_lossy(&bytes);
        let main = text::main_text(&html);
        if main.chars().count() < MIN_TEXT {
            return Err(PageError::Failed(format!(
                "only {} characters of main text; the page may need a browser",
                main.chars().count()
            )));
        }
        let hidden = text::cloudflare_emails(&html).join("\n");
        let searchable = format!("{html}\n{main}\n{hidden}").to_lowercase();
        (
            Sha256::digest(main.as_bytes()),
            Some(main),
            Some(searchable),
        )
    } else {
        (Sha256::digest(&bytes), None, None)
    };
    Ok(Page {
        sha256: hex(&digest),
        main,
        text,
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Save a page's main text as `<entry id>-<URL hash>.txt` under `dir`, so a later run's
/// copy can be diffed against it. The URL is the first line; after a blank line each
/// sentence starts a line, and joining those lines with spaces gives the hashed text.
fn save_text(dir: &Path, id: &str, url: &str, main: &str) {
    let (name, body) = text_file(id, url, main);
    if let Err(e) =
        std::fs::create_dir_all(dir).and_then(|()| std::fs::write(dir.join(&name), body))
    {
        eprintln!("warning: cannot save the text of {url}: {e}");
    }
}

/// The file name and contents `save_text` writes.
fn text_file(id: &str, url: &str, main: &str) -> (String, String) {
    let name = format!("{id}-{}.txt", hex(&Sha256::digest(url.as_bytes())[..4]));
    (name, format!("{url}\n\n{}\n", main.replace(". ", ".\n")))
}

fn hash(urls: &[String]) -> ExitCode {
    let agent = agent(TIMEOUT);
    let mut failed = false;
    for url in urls {
        match evidence_page(&agent, url) {
            Ok(page) => println!("{}  {url}", page.sha256),
            Err(e) => {
                failed = true;
                eprintln!("error: {url}: {e}");
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Prints a Markdown report of changed and failing evidence pages, or nothing if every
/// page still matches, and saves each HTML page's text under `text_dir` if given. Exits 0
/// either way; a register that cannot be read exits 1.
fn drift(dir: &Path, text_dir: Option<&Path>) -> ExitCode {
    let files = match ghostsoap::io::register::load(dir) {
        Ok(files) => files,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", dir.display());
            return ExitCode::FAILURE;
        }
    };
    let agent = agent(TIMEOUT);
    let mut changed = Vec::new();
    let mut failing = Vec::new();
    let mut mail_domains = BTreeMap::new();
    let mut forms = BTreeMap::new();
    let mut emails = Vec::new();
    for file in &files.entries {
        let entry: Entry = match toml::from_str(&file.text) {
            Ok(entry) => entry,
            Err(e) => {
                failing.push(format!("- `{}`: does not parse: {e}", file.name));
                continue;
            }
        };
        let ids = std::slice::from_ref(&entry.id);
        let mut pages = Vec::new();
        for evidence in &entry.evidence {
            match evidence_page(&agent, &evidence.url) {
                Ok(page) => {
                    if page.sha256 != evidence.sha256 {
                        changed.push(format!(
                            "- `{}`: {} (checked {})",
                            entry.id, evidence.url, evidence.checked_at
                        ));
                    }
                    if let (Some(dir), Some(main)) = (text_dir, &page.main) {
                        save_text(dir, &entry.id, &evidence.url, main);
                    }
                    pages.push(page.text);
                }
                Err(PageError::Refused(e)) => {
                    record(&mut failing, &evidence.url, ids, Check::Inconclusive(e));
                    pages.push(None);
                }
                Err(PageError::Failed(e)) => {
                    record(&mut failing, &evidence.url, ids, Check::Dead(e));
                    pages.push(None);
                }
            }
        }
        if entry.obsolete.is_none() {
            collect_channels(&entry, &mut mail_domains, &mut forms);
            for (address, check) in email_checks(&agent, &entry, pages) {
                record(&mut emails, &format!("email {address}"), ids, check);
            }
        }
    }
    let mut channels = channel_problems(&agent, &mail_domains, &forms);
    channels.append(&mut emails);
    let mut laws = law_changes(&|title| latest_version(&agent, title));
    laws.append(&mut legal_page_changes(&agent, &LEGAL_PAGES, text_dir));
    if !changed.is_empty() || !failing.is_empty() || !channels.is_empty() || !laws.is_empty() {
        print!("{}", report(&changed, &failing, &channels, &laws));
    }
    ExitCode::SUCCESS
}

/// The Federal Register's latest version of a title.
fn latest_version(agent: &ureq::Agent, title: &str) -> Result<String, String> {
    let body = agent
        .get("https://api.prod.legislation.gov.au/v1/Versions")
        .query(
            "$filter",
            format!("titleId eq '{title}' and isLatest eq true"),
        )
        .query("$select", "registerId")
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    parse_latest(&body)
}

fn parse_latest(body: &str) -> Result<String, String> {
    let reply: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    match reply["value"].as_array().map(Vec::as_slice) {
        Some([version]) => version["registerId"]
            .as_str()
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| "a version has no register ID".to_owned()),
        versions => Err(format!("expected one latest version, got {versions:?}")),
    }
}

/// Laws whose latest version is not the one read, and laws that could not be checked.
fn law_changes(latest: &dyn Fn(&str) -> Result<String, String>) -> Vec<String> {
    LAWS.iter()
        .filter_map(|(name, title, read)| match latest(title) {
            Ok(id) if id == *read => None,
            Ok(id) => Some(format!("- {name}: {id} is now the latest version")),
            Err(e) => Some(format!("- {name}: inconclusive: {e}")),
        })
        .collect()
}

/// Legal pages whose text has changed, that no longer load or that refused the check. A
/// refused page proves nothing, so it stays unverified and keeps the issue open.
fn legal_page_changes(
    agent: &ureq::Agent,
    pages: &[LegalPage],
    text_dir: Option<&Path>,
) -> Vec<String> {
    let mut lines = Vec::new();
    for &(name, id, url, sha256) in pages {
        match evidence_page(agent, url) {
            Ok(page) => {
                if page.sha256 != sha256 {
                    lines.push(format!(
                        "- {name}: {url} has changed. Check the `escalation` values and the \
                         plan's law status against it, then update `LEGAL_PAGES` in xtask"
                    ));
                }
                if let (Some(dir), Some(main)) = (text_dir, &page.main) {
                    save_text(dir, id, url, main);
                }
            }
            Err(PageError::Refused(e)) => lines.push(format!(
                "- {name}: {url}: unverified, because it refused the check ({e}). Compare it \
                 by hand with the `escalation` values and the plan's law status"
            )),
            Err(PageError::Failed(e)) => lines.push(format!("- {name}: {url}: {e}")),
        }
    }
    lines
}

/// Group each mail domain and each web form or manual-check page by the entries that use
/// it, so each is checked once.
fn collect_channels(
    entry: &Entry,
    mail_domains: &mut BTreeMap<String, Vec<String>>,
    forms: &mut BTreeMap<String, Vec<String>>,
) {
    for channel in &entry.channels {
        let (target, map) = match channel.kind {
            ChannelKind::Email => {
                let domain = channel.address.rsplit('@').next().unwrap_or_default();
                (domain.to_lowercase(), &mut *mail_domains)
            }
            ChannelKind::WebForm => (channel.address.clone(), &mut *forms),
            ChannelKind::Post | ChannelKind::Phone => continue,
        };
        let ids = map.entry(target).or_default();
        if !ids.contains(&entry.id) {
            ids.push(entry.id.clone());
        }
    }
    if let Some(manual) = &entry.manual_check {
        let ids = forms.entry(manual.url.clone()).or_default();
        if !ids.contains(&entry.id) {
            ids.push(entry.id.clone());
        }
    }
}

fn channel_problems(
    agent: &ureq::Agent,
    mail_domains: &BTreeMap<String, Vec<String>>,
    forms: &BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    let lookup = |name: &str, record: &str| dns_query(agent, name, record);
    let mut lines = Vec::new();
    for (domain, ids) in mail_domains {
        let what = format!("email domain {domain}");
        record(&mut lines, &what, ids, check_mail(&lookup, domain));
    }
    for (url, ids) in forms {
        let what = format!("web form or manual-check page {url}");
        record(&mut lines, &what, ids, check_form(agent, &lookup, url));
    }
    lines
}

/// Add a failed check to the issue's lines.
fn record(lines: &mut Vec<String>, what: &str, ids: &[String], check: Check) {
    match check {
        Check::Fine => {}
        Check::Dead(problem) => {
            lines.push(format!("- `{}`: {what}: {problem}", ids.join("`, `")));
        }
        // The workflow log keeps these; they stay out of the issue so that it can close.
        // evidence-drift.yml counts lines starting `inconclusive: ` for its closing comment.
        Check::Inconclusive(why) => eprintln!("inconclusive: {} {what}: {why}", ids.join(", ")),
    }
}

/// Check each email channel of `entry` against its source pages. `pages` holds the
/// searchable text of its evidence pages; its other sources, such as a page behind a bot
/// check that cannot have evidence, are fetched only when an address is not on those.
fn email_checks(
    agent: &ureq::Agent,
    entry: &Entry,
    mut pages: Vec<Option<String>>,
) -> Vec<(String, Check)> {
    let addresses: Vec<&str> = entry
        .channels
        .iter()
        .filter(|c| c.kind == ChannelKind::Email)
        .map(|c| c.address.as_str())
        .collect();
    if addresses
        .iter()
        .any(|a| email_on_pages(a, &pages) != Check::Fine)
    {
        let tracked = |url: &String| entry.evidence.iter().any(|e| &e.url == url);
        for url in entry.sources.iter().filter(|url| !tracked(url)) {
            pages.push(evidence_page(agent, url).ok().and_then(|page| page.text));
        }
    }
    addresses
        .into_iter()
        .map(|a| (a.to_owned(), email_on_pages(a, &pages)))
        .collect()
}

/// Whether `address` appears on one of its entry's source pages, given each page's
/// searchable text (`None` for a page that failed or is not HTML). It is missing only when
/// every page could be searched.
fn email_on_pages(address: &str, pages: &[Option<String>]) -> Check {
    let address = address.to_lowercase();
    if pages
        .iter()
        .flatten()
        .any(|text| has_address(text, &address))
    {
        Check::Fine
    } else if pages.is_empty() {
        Check::Inconclusive("the entry has no source page".to_owned())
    } else if pages.iter().any(Option::is_none) {
        Check::Inconclusive("not on the source pages that could be searched".to_owned())
    } else {
        Check::Dead("appears on none of the entry's source pages".to_owned())
    }
}

/// Whether `text` contains `address` as a whole address, not inside a longer one such as
/// `not` + address or address + `.au`. A sentence's full stop may follow it, and a
/// percent-escape in a link, such as `%20`, may precede it unless it encodes a character an
/// address can contain, such as `%2B`.
fn has_address(text: &str, address: &str) -> bool {
    let local = |c: char| c.is_ascii_alphanumeric() || "._%+-".contains(c);
    let domain = |c: char| c.is_ascii_alphanumeric() || c == '-';
    let escaped_boundary = |before: &[u8]| match before {
        [.., b'%', h, l] if h.is_ascii_hexdigit() && l.is_ascii_hexdigit() => {
            let hex = |d: u8| char::from(d).to_digit(16).unwrap_or(0) as u8;
            !local(char::from(hex(*h) * 16 + hex(*l)))
        }
        _ => false,
    };
    text.match_indices(address).any(|(i, _)| {
        let after = &text[i + address.len()..];
        (!text[..i].ends_with(local) || escaped_boundary(&text.as_bytes()[..i]))
            && !after.starts_with(domain)
            && !(after.starts_with('.') && after[1..].starts_with(domain))
    })
}

/// Email addresses in `head` that the same entry in `base` did not have, as `(file, message)`.
/// A change that repoints a privacy address would send users' requests elsewhere, so a
/// reviewer must see each one. Every address of an entry whose id is not in `base` is listed,
/// so renaming an entry cannot hide a changed address. Fails on a file that does not parse.
fn email_changes(
    base: &[SourceFile],
    head: &[SourceFile],
) -> Result<Vec<(String, String)>, String> {
    // Domains are case-insensitive; a local part may not be, so only the domain is folded.
    let emails = |entry: &Entry| -> Vec<String> {
        let fold = |address: &str| match address.rsplit_once('@') {
            Some((local, domain)) => format!("{local}@{}", domain.to_lowercase()),
            None => address.to_owned(),
        };
        entry
            .channels
            .iter()
            .filter(|c| c.kind == ChannelKind::Email)
            .map(|c| fold(&c.address))
            .collect()
    };
    let parsed = |files: &[SourceFile]| -> Result<Vec<(String, Entry)>, String> {
        let parse = |f: &SourceFile| {
            toml::from_str(&f.text)
                .map(|e| (f.name.clone(), e))
                .map_err(|e| format!("{} does not parse: {e}", f.name))
        };
        files.iter().map(parse).collect()
    };
    let before: BTreeMap<String, Vec<String>> = parsed(base)?
        .into_iter()
        .map(|(_, entry)| (entry.id.clone(), emails(&entry)))
        .collect();
    let mut changes = Vec::new();
    for (file, entry) in parsed(head)? {
        let old = before.get(&entry.id);
        for address in emails(&entry)
            .into_iter()
            .filter(|a| old.is_none_or(|old| !old.contains(a)))
        {
            let message = match old {
                None => format!(
                    "new entry `{}` lists email {address}; check it on the entity's own pages",
                    entry.id
                ),
                Some(old) => {
                    let was = if old.is_empty() {
                        "none".to_owned()
                    } else {
                        old.join(", ")
                    };
                    format!(
                        "`{}` gains email {address} (before: {was}); check it on the entity's own pages",
                        entry.id
                    )
                }
            };
            changes.push((file.clone(), message));
        }
    }
    Ok(changes)
}

/// Print a GitHub warning annotation for each email address `email_changes` lists. Exits 0
/// either way, like a stale `verified_at`; an unreadable register or entry exits 1.
fn emails_changed(base: &Path, head: &Path) -> ExitCode {
    let load = |dir: &Path| {
        ghostsoap::io::register::load(dir)
            .map_err(|e| eprintln!("error: cannot read {}: {e}", dir.display()))
    };
    let (Ok(base_files), Ok(head_files)) = (load(base), load(head)) else {
        return ExitCode::FAILURE;
    };
    let changes = match email_changes(&base_files.entries, &head_files.entries) {
        Ok(changes) => changes,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    // A workflow command escapes `%`, CR and LF in its message, and `:` and `,` too in a
    // property value.
    let data = |s: &str| {
        s.replace('%', "%25")
            .replace('\r', "%0D")
            .replace('\n', "%0A")
    };
    for (file, message) in &changes {
        let path = head.join(file).to_string_lossy().replace('\\', "/");
        let path = data(&path).replace(':', "%3A").replace(',', "%2C");
        let message = data(message);
        println!("::warning file={path},title=Register email changed::{message}");
    }
    println!("{} email address(es) to check", changes.len());
    ExitCode::SUCCESS
}

/// One DNS answer: the name does not exist, or the data of its records of the asked type.
#[derive(Debug, PartialEq, Eq)]
enum Dns {
    NoSuchName,
    Records(Vec<String>),
}

/// Ask Google Public DNS, which reports MX records that the standard library cannot look up.
fn dns_query(agent: &ureq::Agent, name: &str, record: &str) -> Result<Dns, String> {
    let body = agent
        .get("https://dns.google/resolve")
        .query("name", name)
        .query("type", record)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    parse_dns(&body, record)
}

fn parse_dns(body: &str, record: &str) -> Result<Dns, String> {
    let number = match record {
        "MX" => 15,
        "A" => 1,
        "AAAA" => 28,
        _ => return Err(format!("unknown record type {record}")),
    };
    let reply: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    match reply["Status"].as_u64() {
        Some(0) => {}
        Some(3) => return Ok(Dns::NoSuchName),
        status => return Err(format!("DNS status {status:?}")),
    }
    let answers = reply["Answer"].as_array().map_or(&[][..], Vec::as_slice);
    Ok(Dns::Records(
        answers
            .iter()
            .filter(|a| a["type"].as_u64() == Some(number))
            .filter_map(|a| a["data"].as_str().map(str::to_owned))
            .collect(),
    ))
}

#[derive(Debug, PartialEq, Eq)]
enum Check {
    Fine,
    Dead(String),
    Inconclusive(String),
}

/// Whether `domain` can receive mail. Mail falls back to the address records of a domain
/// with no MX record (RFC 5321 section 5.1). A failed lookup says nothing about the domain.
fn check_mail(lookup: &dyn Fn(&str, &str) -> Result<Dns, String>, domain: &str) -> Check {
    if domain.is_empty() {
        return Check::Dead("no domain in the address".to_owned());
    }
    for record in ["MX", "A", "AAAA"] {
        match lookup(domain, record) {
            Err(e) => return Check::Inconclusive(format!("lookup failed: {e}")),
            Ok(Dns::NoSuchName) => return Check::Dead("domain does not exist".to_owned()),
            Ok(Dns::Records(records)) if records.is_empty() => {}
            // A null MX record (RFC 7505) says the domain accepts no mail.
            Ok(Dns::Records(records)) if record == "MX" && records == ["0 ."] => {
                return Check::Dead(
                    "domain publishes a null MX record: it accepts no mail".to_owned(),
                );
            }
            Ok(Dns::Records(_)) => return Check::Fine,
        }
    }
    Check::Dead("no MX or address record".to_owned())
}

/// Fetch a web form, never submitting it. Sites answer scripts with 401, 403 and 429, or
/// close the connection, even when the form works; a form may accept only POST (405); a
/// 5xx may pass; and this machine's resolver may block a host. So only another HTTP error,
/// or a host public DNS says does not exist, counts as dead.
fn check_form(
    agent: &ureq::Agent,
    lookup: &dyn Fn(&str, &str) -> Result<Dns, String>,
    url: &str,
) -> Check {
    match agent.get(url).header("User-Agent", USER_AGENT).call() {
        Ok(_) => Check::Fine,
        Err(e @ ureq::Error::StatusCode(401 | 403 | 405 | 429 | 500..=599)) => {
            Check::Inconclusive(e.to_string())
        }
        Err(e @ ureq::Error::StatusCode(_)) => Check::Dead(e.to_string()),
        Err(e) => {
            let host = url.parse::<ureq::http::Uri>().ok();
            match host.as_ref().and_then(|u| u.host()).map(|h| lookup(h, "A")) {
                Some(Ok(Dns::NoSuchName)) => Check::Dead("host does not exist".to_owned()),
                _ => Check::Inconclusive(e.to_string()),
            }
        }
    }
}

fn report(changed: &[String], failing: &[String], channels: &[String], laws: &[String]) -> String {
    let mut out = String::new();
    if !changed.is_empty() || !failing.is_empty() {
        out += "The monthly evidence check found register pages that differ from when they \
                were verified. Recheck each entry against the page, then update its \
                `[[evidence]]` hash and `checked_at` \
                (`cargo run -p xtask -- evidence-hash <url>`).\n";
    }
    if !channels.is_empty() {
        out += "The monthly evidence check found contact channels that may no longer work. \
                Check each against the business's own pages, then correct its `[[channels]]` \
                entry and `verified_at`.\n";
    }
    if !laws.is_empty() {
        out += "The monthly evidence check found a law version the law table has not read, \
                or a legal page that changed or could not be confirmed. For a new version, reread each lever's \
                provisions in it, then update `LAWS` in xtask, `docs/plan.md` and any \
                `src/core/law.rs` line that cites it.\n";
    }
    for (title, lines) in [
        ("Changed", changed),
        ("Failing", failing),
        ("Channels", channels),
        ("Laws", laws),
    ] {
        if !lines.is_empty() {
            out += &format!("\n## {title}\n\n{}\n", lines.join("\n"));
        }
    }
    out
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.split_first() {
        Some((command, urls)) if command == "evidence-hash" && !urls.is_empty() => hash(urls),
        Some((command, rest)) if command == "evidence-drift" && rest.len() <= 2 => drift(
            Path::new(rest.first().map_or("registry/au", String::as_str)),
            rest.get(1).map(Path::new),
        ),
        Some((command, [base, head])) if command == "email-changes" => {
            emails_changed(Path::new(base), Path::new(head))
        }
        _ => {
            eprintln!(
                "usage: xtask evidence-hash <url>... \
                 | xtask evidence-drift [registry dir [page text dir]] \
                 | xtask email-changes <base registry dir> <head registry dir>"
            );
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    const ENTRY: &str = r#"
id = "example-au"
name = "Example"
legal_entity = "Example Pty Ltd"
abn = "51 824 753 556"
kinds = ["marketing-list"]
priority = "standard"
quality = "verified"
verified_at = 2026-10-30
sources = ["https://example.com.au/privacy"]
levers = ["app7-no-marketing"]
identity_fields = ["email"]
exemption_risk = "none"
escalation = ["oaic"]
effort = "easy"

[[channels]]
kind = "email"
address = "privacy@Example.com.au"
levers = ["app7-no-marketing"]

[[channels]]
kind = "web-form"
address = "https://example.com.au/form"
levers = []

[[channels]]
kind = "post"
address = "PO Box 1, Sydney NSW 2000"
levers = []
"#;
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::Instant;

    fn fake_dns(mx: Dns, a: Dns) -> impl Fn(&str, &str) -> Result<Dns, String> {
        let (mx, a) = (
            std::cell::RefCell::new(Some(mx)),
            std::cell::RefCell::new(Some(a)),
        );
        move |_, record| match record {
            "MX" => Ok(mx.borrow_mut().take().expect("one MX query")),
            "A" => Ok(a.borrow_mut().take().expect("one A query")),
            _ => Ok(Dns::Records(Vec::new())),
        }
    }

    fn records(data: &[&str]) -> Dns {
        Dns::Records(data.iter().map(|&d| d.to_owned()).collect())
    }

    #[test]
    fn a_domain_with_mx_or_only_an_address_can_take_mail() {
        let mx = fake_dns(records(&["10 mail.example.com."]), records(&[]));
        assert_eq!(check_mail(&mx, "example.com"), Check::Fine);
        let address_only = fake_dns(records(&[]), records(&["192.0.2.1"]));
        assert_eq!(check_mail(&address_only, "example.com"), Check::Fine);
    }

    #[test]
    fn reports_a_domain_that_cannot_take_mail() {
        let gone = fake_dns(Dns::NoSuchName, records(&[]));
        assert_eq!(
            check_mail(&gone, "x.example"),
            Check::Dead("domain does not exist".to_owned())
        );
        let empty = fake_dns(records(&[]), records(&[]));
        assert_eq!(
            check_mail(&empty, "x.example"),
            Check::Dead("no MX or address record".to_owned())
        );
        let null = fake_dns(records(&["0 ."]), records(&["192.0.2.1"]));
        assert!(matches!(
            check_mail(&null, "x.example"),
            Check::Dead(p) if p.contains("null MX")
        ));
    }

    #[test]
    fn a_channel_only_report_does_not_ask_for_new_hashes() {
        let line = ["- `example-au`: web form x: gone".to_owned()];
        let channels_only = report(&[], &[], &line, &[]);
        assert!(channels_only.contains("## Channels"));
        assert!(channels_only.contains("`[[channels]]`"));
        assert!(!channels_only.contains("evidence-hash"));
        let pages_only = report(&line, &[], &[], &[]);
        assert!(pages_only.contains("evidence-hash"));
        assert!(!pages_only.contains("`[[channels]]`"));
    }

    #[test]
    fn reports_a_law_with_a_version_the_table_has_not_read() {
        let lines = law_changes(&|title| match title {
            "C2004A03712" => Ok("C2026C99999".to_owned()),
            "C2004A01214" => Ok("C2016C00614".to_owned()),
            "F2021L00285" => Err("timed out".to_owned()),
            _ => Ok("C2026C00243".to_owned()),
        });
        assert_eq!(
            lines,
            [
                "- Privacy Act 1988: C2026C99999 is now the latest version",
                "- Spam Regulations 2021: inconclusive: timed out",
            ]
        );
        let report = report(&[], &[], &[], &lines);
        assert!(report.contains("## Laws"));
        assert!(report.contains("`src/core/law.rs`"));
    }

    #[test]
    fn the_plan_and_the_law_table_record_every_version_read() {
        let plan = include_str!("../../docs/plan.md");
        assert!(LAWS.iter().all(|(_, _, read)| plan.contains(read)));
        let pins: Vec<&str> = ghostsoap::core::law::SOURCES
            .iter()
            .flat_map(|s| s.split([' ', ',']))
            .filter(|word| word.starts_with("C20") || word.starts_with("F20"))
            .collect();
        assert_eq!(pins.len(), 3);
        assert!(pins.iter().all(|pin| LAWS.iter().any(|law| law.2 == *pin)));
        assert!(ghostsoap::core::law::STATUS[0].contains(LAWS[0].2));
    }

    fn stale_page(url: &str) -> LegalPage<'_> {
        ("EDR register", "edr", url, "stale")
    }

    #[test]
    fn a_changed_legal_page_is_reported_and_a_matching_one_is_not() {
        let agent = agent(TIMEOUT);
        let url = serve_html("The recognised schemes changed.");
        let lines = legal_page_changes(&agent, &[stale_page(&url)], None);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].starts_with("- EDR register: "));
        assert!(lines[0].contains("has changed"));
        assert!(report(&[], &[], &[], &lines).contains("## Laws"));

        let url = serve_html("The recognised schemes changed.");
        let sha256 = evidence_page(&agent, &url).unwrap().sha256;
        let url = serve_html("The recognised schemes changed.");
        let same = ("EDR register", "edr", url.as_str(), sha256.as_str());
        assert!(legal_page_changes(&agent, &[same], None).is_empty());
    }

    #[test]
    fn a_legal_page_refused_to_scripts_is_unverified_and_a_dead_one_is_reported() {
        let agent = agent(TIMEOUT);
        let refused = serve_once("403 Forbidden");
        let lines = legal_page_changes(&agent, &[stale_page(&refused)], None);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("unverified"), "{lines:?}");
        assert!(report(&[], &[], &[], &lines).contains("## Laws"));
        let dead = serve_once("404 Not Found");
        assert_eq!(
            legal_page_changes(&agent, &[stale_page(&dead)], None).len(),
            1
        );
    }

    #[test]
    fn checks_a_manual_check_page_once_for_each_entry() {
        let text = format!("{ENTRY}\n[manual_check]\nurl = \"https://example.com.au/form\"\n");
        let entry: Entry = toml::from_str(&text).unwrap();
        let (mut mail, mut forms) = (BTreeMap::new(), BTreeMap::new());
        collect_channels(&entry, &mut mail, &mut forms);
        assert_eq!(forms["https://example.com.au/form"], ["example-au"]);

        let text = format!("{ENTRY}\n[manual_check]\nurl = \"https://example.com.au/search\"\n");
        let entry: Entry = toml::from_str(&text).unwrap();
        collect_channels(&entry, &mut mail, &mut forms);
        assert_eq!(forms["https://example.com.au/search"], ["example-au"]);

        let repeat = "[[channels]]\nkind = \"web-form\"\n\
                      address = \"https://example.com.au/repeat\"\nlevers = []\n";
        let text = format!("{ENTRY}\n{repeat}\n{repeat}");
        let entry: Entry = toml::from_str(&text).unwrap();
        collect_channels(&entry, &mut mail, &mut forms);
        assert_eq!(forms["https://example.com.au/repeat"], ["example-au"]);
    }

    #[test]
    fn parses_the_latest_version() {
        let one = r#"{"value":[{"registerId":"C2026C00227"}]}"#;
        assert_eq!(parse_latest(one).as_deref(), Ok("C2026C00227"));
        assert!(parse_latest(r#"{"value":[]}"#).is_err());
        assert!(parse_latest(r#"{"value":[{}]}"#).is_err());
        assert!(parse_latest(r#"{"value":[{"registerId":""}]}"#).is_err());
        assert!(parse_latest("not json").is_err());
    }

    #[test]
    fn a_failed_lookup_is_inconclusive() {
        let broken = |_: &str, _: &str| Err("offline".to_owned());
        assert_eq!(
            check_mail(&broken, "x.example"),
            Check::Inconclusive("lookup failed: offline".to_owned())
        );
    }

    #[test]
    fn parses_dns_replies() {
        let mx = r#"{"Status":0,"Answer":[{"type":5,"data":"alias."},{"type":15,"data":"10 m."}]}"#;
        assert_eq!(parse_dns(mx, "MX"), Ok(records(&["10 m."])));
        assert_eq!(parse_dns(r#"{"Status":0}"#, "MX"), Ok(records(&[])));
        assert_eq!(parse_dns(r#"{"Status":3}"#, "A"), Ok(Dns::NoSuchName));
        assert!(parse_dns(r#"{"Status":2}"#, "A").is_err());
        assert!(parse_dns("not json", "A").is_err());
    }

    fn serve_once(status: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 1024];
            let _ = stream.read(&mut request);
            let reply =
                format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            let _ = stream.write_all(reply.as_bytes());
        });
        url
    }

    #[test]
    fn reports_a_form_that_does_not_load() {
        let agent = agent(Duration::from_secs(5));
        let lookup = |_: &str, _: &str| Ok(records(&["192.0.2.1"]));
        assert_eq!(
            check_form(&agent, &lookup, &serve_once("200 OK")),
            Check::Fine
        );
        let form = check_form(&agent, &lookup, &serve_once("404 Not Found"));
        assert!(matches!(form, Check::Dead(p) if p.contains("404")));
    }

    #[test]
    fn a_failed_connection_counts_only_if_public_dns_has_no_host() {
        let agent = agent(Duration::from_secs(5));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        drop(listener);
        let gone = |_: &str, _: &str| Ok(Dns::NoSuchName);
        assert_eq!(
            check_form(&agent, &gone, &url),
            Check::Dead("host does not exist".to_owned())
        );
        let there = |_: &str, _: &str| Ok(records(&["192.0.2.1"]));
        assert!(matches!(
            check_form(&agent, &there, &url),
            Check::Inconclusive(_)
        ));
        let broken = |_: &str, _: &str| Err("offline".to_owned());
        assert!(matches!(
            check_form(&agent, &broken, &url),
            Check::Inconclusive(_)
        ));
    }

    #[test]
    fn a_bot_block_does_not_count_as_a_dead_form() {
        let agent = agent(Duration::from_secs(5));
        for status in [
            "401 Unauthorized",
            "405 Method Not Allowed",
            "403 Forbidden",
            "429 Too Many Requests",
            "500 Internal Server Error",
            "502 Bad Gateway",
            "503 Service Unavailable",
        ] {
            let lookup = |_: &str, _: &str| Ok(records(&["192.0.2.1"]));
            assert!(
                matches!(
                    check_form(&agent, &lookup, &serve_once(status)),
                    Check::Inconclusive(_)
                ),
                "{status}"
            );
        }
    }

    #[test]
    fn groups_channels_by_target_and_skips_post() {
        let entry: Entry = toml::from_str(ENTRY).unwrap();
        let (mut mail, mut forms) = (BTreeMap::new(), BTreeMap::new());
        collect_channels(&entry, &mut mail, &mut forms);
        assert_eq!(mail["example.com.au"], ["example-au"]);
        assert_eq!(forms["https://example.com.au/form"], ["example-au"]);
        assert_eq!(mail.len() + forms.len(), 2);
    }

    #[test]
    fn an_email_must_appear_on_a_source_page() {
        let page = |text: &str| Some(text.to_owned());
        let found = [page("other"), page("write to privacy@example.com.au.")];
        let linked = [page(
            "<a href=\"/share?body=write%20privacy@example.com.au\">",
        )];
        assert_eq!(
            email_on_pages("privacy@example.com.au", &linked),
            Check::Fine
        );
        for escape in ["%2b", "%2e", "%61"] {
            let longer = [page(&format!("?to=user{escape}privacy@example.com.au"))];
            assert!(matches!(
                email_on_pages("privacy@example.com.au", &longer),
                Check::Dead(_)
            ));
        }
        assert_eq!(
            email_on_pages("Privacy@Example.com.au", &found),
            Check::Fine
        );
        let longer = [page(
            "notprivacy@example.com.au, privacy@example.com.au.example.org",
        )];
        assert!(matches!(
            email_on_pages("privacy@example.com.au", &longer),
            Check::Dead(_)
        ));
        let missing = [page("write to privacy@elsewhere.example")];
        assert!(matches!(
            email_on_pages("privacy@example.com.au", &missing),
            Check::Dead(p) if p.contains("none of the entry's source pages")
        ));
        let unsearched = [page("nothing here"), None];
        assert!(matches!(
            email_on_pages("privacy@example.com.au", &unsearched),
            Check::Inconclusive(_)
        ));
        assert!(matches!(
            email_on_pages("privacy@example.com.au", &[]),
            Check::Inconclusive(_)
        ));
    }

    /// Serve one HTML page whose main element holds 500 or more characters, then `main`.
    fn serve_html(main: &str) -> String {
        let filler = "Privacy policy text. ".repeat(30);
        let html = format!("<html><body><main>{filler} {main}</main></body></html>");
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let _ = stream.read(&mut [0; 1024]);
            let reply = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n{html}",
                html.len()
            );
            let _ = stream.write_all(reply.as_bytes());
        });
        url
    }

    #[test]
    fn a_page_is_searched_in_its_markup_and_its_decoded_text() {
        let url = serve_html(
            "Email privacy&#64;example.com.au. \
             <a href=\"mailto:Help@Example.com.au\">Contact</a> \
             <a href=\"/cdn-cgi/l/email-protection\" \
             data-cfemail=\"5a32333e3e3f341a3f223b372a363f74393537743b2f\">[email protected]</a>",
        );
        let page = evidence_page(&agent(Duration::from_secs(5)), &url).unwrap();
        let pages = [page.text];
        assert_eq!(
            email_on_pages("privacy@example.com.au", &pages),
            Check::Fine
        );
        assert_eq!(email_on_pages("help@example.com.au", &pages), Check::Fine);
        assert_eq!(email_on_pages("hidden@example.com.au", &pages), Check::Fine);
        assert!(matches!(
            email_on_pages("other@example.com.au", &pages),
            Check::Dead(_)
        ));
    }

    #[test]
    fn an_email_is_searched_on_sources_without_evidence() {
        let url = serve_html("Email privacy@example.com.au.");
        let entry: Entry =
            toml::from_str(&ENTRY.replace("https://example.com.au/privacy", &url)).unwrap();
        let checks = email_checks(&agent(Duration::from_secs(5)), &entry, Vec::new());
        assert_eq!(checks, [("privacy@Example.com.au".to_owned(), Check::Fine)]);
    }

    #[test]
    fn an_email_missing_from_evidence_is_searched_on_the_other_sources() {
        let url = serve_html("Email privacy@example.com.au.");
        let sources = format!("sources = [\"https://example.com.au/privacy\", \"{url}\"]");
        let evidence = format!(
            "\n[[evidence]]\nurl = \"https://example.com.au/privacy\"\n\
             checked_at = 2026-10-30\nsha256 = \"{}\"\n",
            "0".repeat(64)
        );
        let toml = ENTRY.replace("sources = [\"https://example.com.au/privacy\"]", &sources);
        let entry: Entry = toml::from_str(&(toml + &evidence)).unwrap();
        let evidence_page = vec![Some("a policy that names no address".to_owned())];
        let checks = email_checks(&agent(Duration::from_secs(5)), &entry, evidence_page);
        assert_eq!(checks, [("privacy@Example.com.au".to_owned(), Check::Fine)]);
    }

    #[test]
    fn a_page_refused_to_scripts_is_not_a_failure() {
        let agent = agent(Duration::from_secs(5));
        for status in ["401 Unauthorized", "403 Forbidden", "429 Too Many Requests"] {
            let page = evidence_page(&agent, &serve_once(status)).map(|p| p.sha256);
            assert!(
                matches!(page, Err(PageError::Refused(_))),
                "{status}: {page:?}"
            );
        }
        for status in ["404 Not Found", "410 Gone", "500 Internal Server Error"] {
            let page = evidence_page(&agent, &serve_once(status)).map(|p| p.sha256);
            assert!(
                matches!(page, Err(PageError::Failed(_))),
                "{status}: {page:?}"
            );
        }
    }

    #[test]
    fn saved_text_rejoins_to_the_hashed_text() {
        let url = serve_html("First sentence. Second sentence. Third.");
        let page = evidence_page(&agent(Duration::from_secs(5)), &url).unwrap();
        let main = page.main.unwrap();
        let (name, saved) = text_file("example-au", &url, &main);
        assert!(
            name.starts_with("example-au-") && name.ends_with(".txt"),
            "{name}"
        );
        assert_ne!(
            name,
            text_file("example-au", "https://example.com.au/", &main).0
        );
        let (head, body) = saved.split_once("\n\n").unwrap();
        assert_eq!(head, url);
        assert!(body.contains("Second sentence.\nThird."), "{body}");
        let rejoined = body.trim_end_matches('\n').replace('\n', " ");
        assert_eq!(hex(&Sha256::digest(rejoined.as_bytes())), page.sha256);
    }

    fn source(text: &str) -> SourceFile {
        SourceFile {
            name: "example-au.toml".to_owned(),
            text: text.to_owned(),
        }
    }

    fn changes(base: &[SourceFile], head: &[SourceFile]) -> Result<Vec<(String, String)>, String> {
        email_changes(base, head)
    }

    #[test]
    fn warns_of_an_email_address_an_entry_gains() {
        let base = [source(ENTRY)];
        let moved = ENTRY.replace("privacy@Example.com.au", "privacy@attacker.example");
        let found = changes(&base, &[source(&moved)]).unwrap();
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].0, "example-au.toml");
        assert!(found[0].1.contains("privacy@attacker.example"));
        assert!(found[0].1.contains("before: privacy@example.com.au"));
        let local_part = ENTRY.replace("privacy@Example.com.au", "optout@example.com.au");
        assert_eq!(changes(&base, &[source(&local_part)]).unwrap().len(), 1);
    }

    #[test]
    fn a_recased_local_part_is_a_different_address_but_a_recased_domain_is_not() {
        let base = [source(ENTRY)];
        let domain = ENTRY.replace("privacy@Example.com.au", "privacy@EXAMPLE.com.au");
        assert_eq!(changes(&base, &[source(&domain)]).unwrap(), []);
        let local_part = ENTRY.replace("privacy@Example.com.au", "Privacy@example.com.au");
        assert_eq!(changes(&base, &[source(&local_part)]).unwrap().len(), 1);
    }

    #[test]
    fn a_renamed_entry_cannot_change_its_email_unflagged() {
        let base = [source(ENTRY)];
        let renamed = ENTRY
            .replace("\"example-au\"", "\"other-au\"")
            .replace("privacy@Example.com.au", "privacy@attacker.example");
        let found = changes(&base, &[source(&renamed)]).unwrap();
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].1.contains("new entry `other-au`"));
        assert!(found[0].1.contains("privacy@attacker.example"));
    }

    #[test]
    fn ignores_unchanged_addresses() {
        let base = [source(ENTRY)];
        assert_eq!(changes(&base, &base).unwrap(), []);
    }

    #[test]
    fn an_entry_that_does_not_parse_is_an_error() {
        let base = [source(ENTRY)];
        let broken = [source("id = ")];
        let error = changes(&base, &broken).unwrap_err();
        assert!(error.contains("example-au.toml"), "{error}");
        assert!(changes(&broken, &base).is_err());
    }

    #[test]
    fn silent_server_times_out() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        // Accept the connection and hold it open without replying.
        let server = std::thread::spawn(move || listener.accept().map(|(stream, _)| stream));
        let start = Instant::now();
        let result = evidence_page(&agent(Duration::from_millis(300)), &url).map(|p| p.sha256);
        assert!(
            matches!(&result, Err(PageError::Failed(e)) if e.contains("timeout")),
            "{result:?}"
        );
        assert!(start.elapsed() < Duration::from_secs(10));
        drop(server.join());
    }
}
