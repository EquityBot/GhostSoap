//! Maintainer tasks for the GhostSoap register. Not part of the `ghostsoap` binary, which
//! makes no network requests of its own.
//!
//! ```text
//! cargo run -p xtask -- evidence-hash <url>...        print the evidence hash of each page
//! cargo run -p xtask -- evidence-drift [registry/au]  report pages whose hash has changed
//! ```

mod text;

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use ghostsoap::core::register::{ChannelKind, Entry};
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

/// The evidence hash of `url`: SHA-256 of its normalised main text, or of the raw bytes
/// for a non-HTML response.
fn evidence_hash(agent: &ureq::Agent, url: &str) -> Result<String, String> {
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
    let digest = if text::is_html(content_type.as_deref(), &bytes) {
        let text = text::main_text(&String::from_utf8_lossy(&bytes));
        if text.chars().count() < MIN_TEXT {
            return Err(format!(
                "only {} characters of main text; the page may need a browser",
                text.chars().count()
            ));
        }
        Sha256::digest(text.as_bytes())
    } else {
        Sha256::digest(&bytes)
    };
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

fn hash(urls: &[String]) -> ExitCode {
    let agent = agent(TIMEOUT);
    let mut failed = false;
    for url in urls {
        match evidence_hash(&agent, url) {
            Ok(sha) => println!("{sha}  {url}"),
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
    for file in &files.entries {
        let entry: Entry = match toml::from_str(&file.text) {
            Ok(entry) => entry,
            Err(e) => {
                failing.push(format!("- `{}`: does not parse: {e}", file.name));
                continue;
            }
        };
        for evidence in &entry.evidence {
            match evidence_hash(&agent, &evidence.url) {
                Ok(sha) if sha == evidence.sha256 => {}
                Ok(_) => changed.push(format!(
                    "- `{}`: {} (checked {})",
                    entry.id, evidence.url, evidence.checked_at
                )),
                Err(e) => failing.push(format!("- `{}`: {}: {e}", entry.id, evidence.url)),
            }
        }
        if entry.obsolete.is_none() {
            collect_channels(&entry, &mut mail_domains, &mut forms);
        }
    }
    let channels = channel_problems(&agent, &mail_domains, &forms);
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
    let mut record = |what: String, ids: &[String], check: Check| match check {
        Check::Fine => {}
        Check::Dead(problem) => {
            lines.push(format!("- `{}`: {what}: {problem}", ids.join("`, `")));
        }
        // The workflow log keeps these; they stay out of the issue so that it can close.
        Check::Inconclusive(why) => eprintln!("inconclusive: {} {what}: {why}", ids.join(", ")),
    };
    for (domain, ids) in mail_domains {
        record(
            format!("email domain {domain}"),
            ids,
            check_mail(&lookup, domain),
        );
    }
    for (url, ids) in forms {
        record(
            format!("web form {url}"),
            ids,
            check_form(agent, &lookup, url),
        );
    }
    lines
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
/// close the connection, even when the form works; a 5xx may pass; and this machine's
/// resolver may block a host. So only another HTTP error, or a host public DNS says does
/// not exist, counts as dead.
fn check_form(
    agent: &ureq::Agent,
    lookup: &dyn Fn(&str, &str) -> Result<Dns, String>,
    url: &str,
) -> Check {
    match agent.get(url).header("User-Agent", USER_AGENT).call() {
        Ok(_) => Check::Fine,
        Err(e @ ureq::Error::StatusCode(401 | 403 | 429 | 500..=599)) => {
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
    let mut out = String::from(
        "The monthly evidence check found register pages that differ from when they were \
         verified, or contact channels that may no longer work. Recheck each entry against \
         the page, then update its `[[evidence]]` hash and `checked_at` \
         (`cargo run -p xtask -- evidence-hash <url>`).\n",
    );
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
        _ => {
            eprintln!("usage: xtask evidence-hash <url>... | xtask evidence-drift [registry dir]");
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
    fn silent_server_times_out() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        // Accept the connection and hold it open without replying.
        let server = std::thread::spawn(move || listener.accept().map(|(stream, _)| stream));
        let start = Instant::now();
        let result = evidence_hash(&agent(Duration::from_millis(300)), &url);
        assert!(
            result.as_ref().is_err_and(|e| e.contains("timeout")),
            "{result:?}"
        );
        assert!(start.elapsed() < Duration::from_secs(10));
        drop(server.join());
    }
}
