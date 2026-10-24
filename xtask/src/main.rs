//! Maintainer tasks for the GhostSoap register. Not part of the `ghostsoap` binary, which
//! makes no network requests of its own.
//!
//! ```text
//! cargo run -p xtask -- evidence-hash <url>...        print the evidence hash of each page
//! cargo run -p xtask -- evidence-drift [registry/au]  report pages whose hash has changed
//! ```

mod text;

use std::path::Path;
use std::process::ExitCode;

use ghostsoap::core::register::Entry;
use sha2::{Digest, Sha256};

/// Pages with less main text than this are bot checks or script-only shells.
const MIN_TEXT: usize = 500;
const USER_AGENT: &str = "GhostSoap-evidence-check/0.1 (+https://github.com/ryanduguid/GhostSoap)";

/// The evidence hash of `url`: SHA-256 of its normalised main text, or of the raw bytes
/// for a non-HTML response.
fn evidence_hash(url: &str) -> Result<String, String> {
    let mut response = ureq::get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| e.to_string())?;
    let html = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("html"));
    let bytes = response
        .body_mut()
        .with_config()
        .limit(20 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| e.to_string())?;
    let digest = if html {
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
    let mut failed = false;
    for url in urls {
        match evidence_hash(url) {
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
    let mut changed = Vec::new();
    let mut failing = Vec::new();
    for file in &files.entries {
        let entry: Entry = match toml::from_str(&file.text) {
            Ok(entry) => entry,
            Err(e) => {
                failing.push(format!("- `{}`: does not parse: {e}", file.name));
                continue;
            }
        };
        for evidence in &entry.evidence {
            match evidence_hash(&evidence.url) {
                Ok(sha) if sha == evidence.sha256 => {}
                Ok(_) => changed.push(format!(
                    "- `{}`: {} (checked {})",
                    entry.id, evidence.url, evidence.checked_at
                )),
                Err(e) => failing.push(format!("- `{}`: {}: {e}", entry.id, evidence.url)),
            }
        }
    }
    if !changed.is_empty() || !failing.is_empty() {
        print!("{}", report(&changed, &failing));
    }
    ExitCode::SUCCESS
}

fn report(changed: &[String], failing: &[String]) -> String {
    let mut out = String::from(
        "The monthly evidence check found register pages that differ from when they were \
         verified. Recheck each entry against the page, then update its `[[evidence]]` hash \
         and `checked_at` (`cargo run -p xtask -- evidence-hash <url>`).\n",
    );
    for (title, lines) in [("Changed", changed), ("Failing", failing)] {
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
