//! Letters: templates in `letters/au`, a strict renderer, and the message formats a draft
//! is written in (plan section 5, 'Letters').

use std::collections::BTreeMap;

use chrono::NaiveDate;

use crate::core::law::law;
use crate::core::profile::Profile;
use crate::core::register::{
    Channel, ChannelKind, Entry, Escalation, ExemptionRisk, IdentityField, Lever,
};

const OPENING: &str = include_str!("../../letters/au/opening.txt");
const CLOSING: &str = include_str!("../../letters/au/closing.txt");

/// The paragraph template for `lever`. The match makes a missing letter a compile error.
fn template(lever: Lever) -> &'static str {
    match lever {
        Lever::App7NoMarketing => include_str!("../../letters/au/app7-no-marketing.txt"),
        Lever::App7NoFacilitation => include_str!("../../letters/au/app7-no-facilitation.txt"),
        Lever::App7Source => include_str!("../../letters/au/app7-source.txt"),
        Lever::App11Destroy => include_str!("../../letters/au/app11-destroy.txt"),
        Lever::App12 => include_str!("../../letters/au/app12.txt"),
        Lever::App13 => include_str!("../../letters/au/app13.txt"),
        Lever::App13NotifyRecipients => {
            include_str!("../../letters/au/app13-notify-recipients.txt")
        }
        Lever::App13Statement => include_str!("../../letters/au/app13-statement.txt"),
        Lever::S20rAccess => include_str!("../../letters/au/s20r-access.txt"),
        Lever::S20tCorrection => include_str!("../../letters/au/s20t-correction.txt"),
        Lever::S20kBan => include_str!("../../letters/au/s20k-ban.txt"),
        Lever::S21tAccess => include_str!("../../letters/au/s21t-access.txt"),
        Lever::S21vCorrection => include_str!("../../letters/au/s21v-correction.txt"),
        Lever::SpamUnsubscribe => include_str!("../../letters/au/spam-unsubscribe.txt"),
    }
}

/// Split a template into its version (first line, `version N`) and its text.
fn parts(template: &str) -> (u32, &str) {
    let (first, text) = template.split_once('\n').unwrap_or((template, ""));
    let version = first
        .trim()
        .strip_prefix("version ")
        .and_then(|v| v.parse().ok())
        .expect("every letter template starts with `version N`");
    (version, text)
}

/// The version of `lever`'s letter, recorded with each request.
pub fn letter_version(lever: Lever) -> u32 {
    parts(template(lever)).0
}

/// Levers whose letter needs the user's own words, such as what is wrong or the grounds
/// for a ban.
pub fn takes_detail(lever: Lever) -> bool {
    template(lever).contains("{{detail}}")
}

/// Fill `{{name}}` placeholders from `values`. A placeholder missing from `values` is an
/// error; a line with a placeholder whose value is `None` is dropped. Values are inserted
/// once and never re-scanned.
pub fn render(text: &str, values: &BTreeMap<&str, Option<String>>) -> Result<String, String> {
    let mut out = Vec::new();
    'lines: for line in text.lines() {
        let mut rendered = String::new();
        let mut rest = line;
        while let Some(start) = rest.find("{{") {
            let Some(len) = rest[start..].find("}}") else {
                return Err(format!("unclosed placeholder in `{line}`"));
            };
            let name = &rest[start + 2..start + len];
            match values.get(name) {
                None => return Err(format!("unknown placeholder `{{{{{name}}}}}`")),
                Some(None) => continue 'lines,
                Some(Some(value)) => {
                    rendered.push_str(&rest[..start]);
                    rendered.push_str(value);
                }
            }
            rest = &rest[start + len + 2..];
        }
        rendered.push_str(rest);
        out.push(rendered);
    }
    Ok(out.join("\n"))
}

/// A drafted message: one per entry and channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Letter {
    pub kind: ChannelKind,
    /// The email address, postal address or form URL.
    pub to: String,
    pub subject: String,
    /// Wrapped at 76 columns, lines separated by `\n`.
    pub body: String,
}

/// Everything a message depends on besides the entry and profile.
pub struct Request<'a> {
    pub channel: &'a Channel,
    pub levers: &'a [Lever],
    pub detail: Option<&'a str>,
    pub date: NaiveDate,
    /// Such as `GS-7K3QX`.
    pub tag: &'a str,
}

/// Compose the message for `request` to `entry`. Only the entry's identity fields appear.
pub fn compose(entry: &Entry, profile: &Profile, request: &Request) -> Result<Letter, String> {
    let mut values = identity_values(entry, profile);
    let spam_only = request.levers.iter().all(|&l| l == Lever::SpamUnsubscribe);
    let has_spam = request.levers.contains(&Lever::SpamUnsubscribe);
    let laws = match (spam_only, has_spam) {
        (true, _) => "the Spam Act 2003 (Cth)",
        (false, true) => "the Privacy Act 1988 (Cth) and the Spam Act 2003 (Cth)",
        (false, false) => "the Privacy Act 1988 (Cth)",
    };
    let exemption_note =
        (entry.exemption_risk == ExemptionRisk::SmallBusinessPossible).then(|| {
            "If you consider that the Privacy Act does not apply to you, for example because you \
         are a small business operator under section 6D, please tell me so in writing."
                .to_owned()
        });
    let postal =
        (request.channel.kind == ChannelKind::Post).then(|| request.channel.address.clone());
    values.extend([
        ("date", Some(request.date.format("%-d %B %Y").to_string())),
        ("legal_entity", Some(entry.legal_entity.clone())),
        ("postal_address", postal),
        ("laws", Some(laws.to_owned())),
        ("exemption_note", exemption_note),
        ("escalation", escalation(entry, spam_only, has_spam)),
        ("tag", Some(format!("[{}]", request.tag))),
        ("detail", request.detail.map(str::to_owned)),
    ]);
    let mut sections = vec![render(parts(OPENING).1, &values)?];
    for &lever in request.levers {
        let row = law(lever);
        let mut lever_values = values.clone();
        lever_values.extend([
            ("title", Some(row.title.to_owned())),
            ("provision", Some(row.provision.to_owned())),
            ("deadline", Some(row.deadline_text.to_owned())),
            ("charge", Some(row.charge.to_owned())),
        ]);
        sections.push(render(parts(template(lever)).1, &lever_values)?);
    }
    sections.push(render(parts(CLOSING).1, &values)?);
    let subject = if spam_only {
        "Unsubscribe request"
    } else {
        "Privacy Act request"
    };
    Ok(Letter {
        kind: request.channel.kind,
        to: request.channel.address.clone(),
        subject: format!("{subject} [{}]", request.tag),
        body: wrap(&tidy(&sections.join("\n\n")), 76),
    })
}

/// The entry's identity fields with the profile's values; every other field is `None`, so
/// its line is dropped.
fn identity_values(entry: &Entry, profile: &Profile) -> BTreeMap<&'static str, Option<String>> {
    let fields = [
        ("full_name", IdentityField::FullName),
        ("alias_names", IdentityField::AliasNames),
        ("email", IdentityField::Email),
        ("phone", IdentityField::Phone),
        ("date_of_birth", IdentityField::DateOfBirth),
        ("address", IdentityField::Address),
        ("previous_addresses", IdentityField::PreviousAddresses),
        ("postcode", IdentityField::Postcode),
    ];
    fields
        .into_iter()
        .map(|(name, field)| {
            let allowed = entry.identity_fields.contains(&field);
            (name, profile.value(field).filter(|_| allowed))
        })
        .collect()
}

/// Where the user can complain: the entry's Privacy Act targets about the Privacy Act
/// requests and ACMA about an unsubscribe request, each named for its own matter.
fn escalation(entry: &Entry, spam_only: bool, has_spam: bool) -> Option<String> {
    let name = |e: Escalation| match e {
        Escalation::Oaic => "the Office of the Australian Information Commissioner (OAIC)",
        Escalation::Afca => "the Australian Financial Complaints Authority (AFCA)",
        Escalation::Acma => "the Australian Communications and Media Authority (ACMA)",
    };
    let privacy: Vec<&str> = entry
        .escalation
        .iter()
        .copied()
        .filter(|&e| e != Escalation::Acma && !spam_only)
        .map(name)
        .collect();
    let privacy = (!privacy.is_empty()).then(|| privacy.join(" or "));
    let spam = has_spam.then(|| name(Escalation::Acma));
    match (privacy, spam) {
        (Some(privacy), Some(spam)) => Some(format!(
            "{privacy} about the Privacy Act requests, or {spam} about the unsubscribe request"
        )),
        (privacy, spam) => privacy.or(spam.map(str::to_owned)),
    }
}

/// Collapse runs of blank lines (left by dropped lines) and trim the ends.
fn tidy(text: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for line in text.lines().map(str::trim_end) {
        if line.is_empty() && out.last().is_none_or(|l| l.is_empty()) {
            continue;
        }
        out.push(line);
    }
    while out.last() == Some(&"") {
        out.pop();
    }
    out.join("\n")
}

/// Wrap each line at `width` columns on spaces; a longer word stays whole.
pub fn wrap(text: &str, width: usize) -> String {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut current = String::new();
        for word in line.split(' ').filter(|w| !w.is_empty()) {
            if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > width {
                out.push(std::mem::take(&mut current));
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
        out.push(current);
    }
    out.join("\n")
}

/// An `.eml` draft. `X-Unsent: 1` makes Outlook open it as a draft; there is no `From`, so
/// the mail client fills in the user's own address.
pub fn eml(letter: &Letter) -> String {
    let body = letter.body.replace('\n', "\r\n");
    format!(
        "To: {}\r\nSubject: {}\r\nX-Unsent: 1\r\nMIME-Version: 1.0\r\n\
         Content-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n{body}\r\n",
        letter.to, letter.subject
    )
}

/// A `mailto:` link (RFC 6068), or `None` when it would be longer than mail clients and
/// browsers reliably accept.
pub fn mailto(letter: &Letter) -> Option<String> {
    const LIMIT: usize = 2000;
    let link = format!(
        "mailto:{}?subject={}&body={}",
        letter.to,
        percent_encode(&letter.subject),
        percent_encode(&letter.body.replace('\n', "\r\n"))
    );
    (link.len() <= LIMIT).then_some(link)
}

fn percent_encode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// A request tag such as `GS-7K3QX` from 64 random bits, in Crockford base32.
pub fn tag(random: u64) -> String {
    const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let code: String = (0..5)
        .map(|i| char::from(ALPHABET[((random >> (i * 5)) & 31) as usize]))
        .collect();
    format!("GS-{code}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&'static str, Option<&str>)]) -> BTreeMap<&'static str, Option<String>> {
        pairs
            .iter()
            .map(|&(k, v)| (k, v.map(str::to_owned)))
            .collect()
    }

    #[test]
    fn render_fills_drops_and_rejects() {
        let v = values(&[("name", Some("Lara {{x}}")), ("phone", None)]);
        assert_eq!(
            render("Hi {{name}}\nPhone: {{phone}}\nBye", &v).unwrap(),
            "Hi Lara {{x}}\nBye",
            "values are not re-scanned; a line with an empty value is dropped"
        );
        assert_eq!(
            render("{{nope}}", &v).unwrap_err(),
            "unknown placeholder `{{nope}}`"
        );
        assert!(render("Hi {{name", &v).unwrap_err().contains("unclosed"));
    }

    #[test]
    fn every_template_has_a_version() {
        for lever in Lever::ALL {
            assert_eq!(letter_version(lever), 1, "{}", lever.name());
        }
        assert!(takes_detail(Lever::App13) && takes_detail(Lever::S20kBan));
        assert!(!takes_detail(Lever::App7Source));
    }

    #[test]
    fn wrap_breaks_on_spaces_and_keeps_long_words() {
        assert_eq!(wrap("aaa bbb ccc", 7), "aaa bbb\nccc");
        assert_eq!(
            wrap("x https://a.example/very/long", 10),
            "x\nhttps://a.example/very/long"
        );
        assert_eq!(wrap("one\n\ntwo", 10), "one\n\ntwo");
    }

    fn letter(body: &str) -> Letter {
        Letter {
            kind: ChannelKind::Email,
            to: "privacy@example.com.au".to_owned(),
            subject: "Privacy Act request [GS-TEST1]".to_owned(),
            body: body.to_owned(),
        }
    }

    #[test]
    fn eml_is_an_unsent_draft_with_crlf_lines() {
        let eml = eml(&letter("Hello\nWorld"));
        assert!(eml.starts_with(
            "To: privacy@example.com.au\r\nSubject: Privacy Act request [GS-TEST1]\r\nX-Unsent: 1\r\n"
        ));
        assert!(eml.ends_with("\r\n\r\nHello\r\nWorld\r\n"));
        assert!(!eml.contains("From:"));
    }

    #[test]
    fn mailto_encodes_and_has_a_length_limit() {
        let link = mailto(&letter("Café & co\n50%")).unwrap();
        assert_eq!(
            link,
            "mailto:privacy@example.com.au?subject=Privacy%20Act%20request%20%5BGS-TEST1%5D\
             &body=Caf%C3%A9%20%26%20co%0D%0A50%25"
        );
        assert_eq!(mailto(&letter(&"x".repeat(2000))), None);
    }

    #[test]
    fn tags_use_crockford_base32() {
        assert_eq!(tag(0), "GS-00000");
        assert_eq!(tag(u64::MAX), "GS-ZZZZZ");
        assert_eq!(tag(1 | (31 << 20)), "GS-1000Z");
    }
}
