//! Maintainer tasks for the GhostSoap register. Not part of the `ghostsoap` binary, which
//! makes no network requests of its own.
//!
//! ```text
//! cargo run -p xtask -- evidence-hash <url>...        print the evidence hash of each page
//! cargo run -p xtask -- evidence-drift [registry/au]  report pages whose hash has changed
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
const USER_AGENT: &str = "GhostSoap-evidence-check/0.1 (+https://github.com/EquityBot/GhostSoap)";
/// Whole-request limit, so one slow or silent site fails instead of stalling the check.
const TIMEOUT: Duration = Duration::from_secs(30);

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .build()
        .into()
}

/// A fetched evidence page.
struct Page {
    sha256: String,
    /// Lowercase raw HTML, main text and Cloudflare-protected addresses, for finding an
    /// email address in a link, a footer, or entity-encoded or obfuscated text; `None` for
    /// a non-HTML page.
    text: Option<String>,
}

/// Fetch `url`. Its evidence hash is SHA-256 of its normalised main text, or of the raw
/// bytes for a non-HTML response.
fn evidence_page(agent: &ureq::Agent, url: &str) -> Result<Page, String> {
    let mut response = agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| e.to_string())?;
    let content_type = response
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap_or_default().to_owned());
    let bytes = response
        .body_mut()
        .with_config()
        .limit(20 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| e.to_string())?;
    let (digest, text) = if text::is_html(content_type.as_deref(), &bytes) {
        let html = String::from_utf8_lossy(&bytes);
        let main = text::main_text(&html);
        if main.chars().count() < MIN_TEXT {
            return Err(format!(
                "only {} characters of main text; the page may need a browser",
                main.chars().count()
            ));
        }
        let hidden = text::cloudflare_emails(&html).join("\n");
        let searchable = format!("{html}\n{main}\n{hidden}").to_lowercase();
        (Sha256::digest(main.as_bytes()), Some(searchable))
    } else {
        (Sha256::digest(&bytes), None)
    };
    Ok(Page {
        sha256: digest.iter().map(|b| format!("{b:02x}")).collect(),
        text,
    })
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
/// page still matches. Exits 0 either way; a register that cannot be read exits 1.
fn drift(dir: &Path) -> ExitCode {
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
                    pages.push(page.text);
                }
                Err(e) => {
                    failing.push(format!("- `{}`: {}: {e}", entry.id, evidence.url));
                    pages.push(None);
                }
            }
        }
        if entry.obsolete.is_none() {
            collect_channels(&entry, &mut mail_domains, &mut forms);
            let ids = std::slice::from_ref(&entry.id);
            for (address, check) in email_checks(&agent, &entry, pages) {
                record(&mut emails, &format!("email {address}"), ids, check);
            }
        }
    }
    let mut channels = channel_problems(&agent, &mail_domains, &forms);
    channels.append(&mut emails);
    if !changed.is_empty() || !failing.is_empty() || !channels.is_empty() {
        print!("{}", report(&changed, &failing, &channels));
    }
    ExitCode::SUCCESS
}

/// Group each mail domain and web form by the entries that use it, so each is checked once.
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
        let what = format!("web form {url}");
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
/// percent-escape such as `%20` in a link may precede it.
fn has_address(text: &str, address: &str) -> bool {
    let local = |c: char| c.is_ascii_alphanumeric() || "._%+-".contains(c);
    let domain = |c: char| c.is_ascii_alphanumeric() || c == '-';
    let escaped = |before: &[u8]| matches!(before, [.., b'%', h, l] if h.is_ascii_hexdigit() && l.is_ascii_hexdigit());
    text.match_indices(address).any(|(i, _)| {
        let after = &text[i + address.len()..];
        (!text[..i].ends_with(local) || escaped(&text.as_bytes()[..i]))
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

fn report(changed: &[String], failing: &[String], channels: &[String]) -> String {
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
    for (title, lines) in [
        ("Changed", changed),
        ("Failing", failing),
        ("Channels", channels),
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
        Some((command, rest)) if command == "evidence-drift" && rest.len() <= 1 => drift(
            Path::new(rest.first().map_or("registry/au", String::as_str)),
        ),
        Some((command, [base, head])) if command == "email-changes" => {
            emails_changed(Path::new(base), Path::new(head))
        }
        _ => {
            eprintln!(
                "usage: xtask evidence-hash <url>... | xtask evidence-drift [registry dir] \
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
        let channels_only = report(&[], &[], &line);
        assert!(channels_only.contains("## Channels"));
        assert!(channels_only.contains("`[[channels]]`"));
        assert!(!channels_only.contains("evidence-hash"));
        let pages_only = report(&line, &[], &[]);
        assert!(pages_only.contains("evidence-hash"));
        assert!(!pages_only.contains("`[[channels]]`"));
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
            result.as_ref().is_err_and(|e| e.contains("timeout")),
            "{result:?}"
        );
        assert!(start.elapsed() < Duration::from_secs(10));
        drop(server.join());
    }
}
