// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

//! Flag legal claims the law table does not support, in an edited draft or the user's own
//! words. A statute review on 5 November 2026 settled what is flagged (plan section 14).
//! The rules are word patterns: they read no meaning, so they can miss a claim or flag a
//! harmless one, and an unflagged text is not thereby sound.

use std::collections::BTreeMap;

use crate::core::law::law;
use crate::core::letters;
use crate::core::register::Lever;

/// One claim for the user to recheck.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flag {
    /// The line the claim starts on, from 1.
    pub line: usize,
    /// The words that matched. They come only from the fixed phrases and from numbers, so
    /// a flag repeats nothing personal from the text.
    pub found: String,
    /// Why the claim needs rechecking.
    pub why: &'static str,
}

const ERASURE: &str = "if this claims a right or duty to erase, the Privacy Act levers in the \
                       law table give no right to erasure. APP 11.2 makes a business take \
                       reasonable steps to destroy or de-identify what it no longer needs, which \
                       is its duty and not a right you can use. The tranche 2 Exposure Draft \
                       (read 5 November 2026) is a draft, not law, and its erasure right would \
                       reach large digital platforms only.";
const GDPR: &str = "the law table has no row for the GDPR, which reaches a business only as its \
                    Article 3 provides, so it may not apply here.";
const CCPA: &str = "the law table has no row for California's privacy law, which protects \
                    California residents, so it may not apply here.";
const ARTICLE: &str = "this looks like an article of a foreign law. Australian law cites APPs \
                       and sections, and the law table has no such article.";
const DEADLINE: &str = "if this is a legal deadline, the law table does not give it in these \
                        words. The APPs give an organisation 'a reasonable period', and 30 \
                        days is the OAIC's guide for that, not a limit in those APPs. Other rows \
                        set fixed periods, such as 30 days for a credit correction (ss 20T and \
                        21V), so run `ghostsoap law` for the deadlines the law sets.";
const NO_WORDS: &str = "it has no words to check";
const ENCODED: &str = "it holds an encoded mail body, whose words cannot be matched. Paste the \
                       message into a plain text file and check that";
const HTML: &str = "it holds HTML, not the plain text of a letter. Paste the message into a \
                    plain text file and check that";

// A sentence that holds an erasing word and a legal source or a word of duty is flagged. 'Require'
// and 'requires' are neither, so 'I require you to delete' stays out, and 'the Act requires' is
// caught by its source.
#[rustfmt::skip]
const ERASING_WORDS: [&str; 21] = [
    "erase", "erases", "erased", "erasing", "erasure",
    "delete", "deletes", "deleted", "deleting", "deletion",
    "destroy", "destroys", "destroyed", "destroying", "destruction",
    "remove", "removes", "removed", "removing", "removal",
    "forgotten",
];
#[rustfmt::skip]
const LEGAL_SOURCES: [&str; 13] = [
    "act", "acts", "law", "laws", "legal", "legally", "legislation", "statute",
    "section", "sections", "regulation", "regulations", "apps",
];
/// These count as a legal source only before a number, because 'your app' is not 'APP 11'.
const CITING_WORDS: [&str; 3] = ["app", "s", "ss"];
#[rustfmt::skip]
const DUTY_WORDS: [&str; 14] = [
    "must", "shall", "required", "obliged", "obligated", "obligation", "obligations",
    "compelled", "duty", "duties", "entitled", "entitlement", "right", "rights",
];

/// Word patterns and why each needs rechecking.
const PHRASES: [(&[&[&str]], &str); 2] = [
    (
        &[&["gdpr"], &["general", "data", "protection", "regulation"]],
        GDPR,
    ),
    (
        &[
            &["ccpa"],
            &["cpra"],
            &["california", "consumer", "privacy", "act"],
            &["california", "privacy", "rights", "act"],
        ],
        CCPA,
    ),
];

const HTML_SIGNS: [&str; 2] = ["<!doctype", "<!--"];

const NUMBER_WORDS: [&str; 27] = [
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
    "twenty",
    "thirty",
    "forty",
    "fifty",
    "sixty",
    "seventy",
    "eighty",
    "ninety",
];
const QUALIFIERS: [&str; 4] = ["calendar", "business", "working", "clear"];
const UNITS: [&str; 12] = [
    "hour",
    "hours",
    "day",
    "days",
    "week",
    "weeks",
    "fortnight",
    "fortnights",
    "month",
    "months",
    "year",
    "years",
];
/// A period after one of these says how long something lasted or how often it happens, not
/// when it is due.
const NOT_A_DEADLINE: [&str; 8] = [
    "for", "last", "past", "previous", "once", "twice", "times", "every",
];

struct Word {
    line: usize,
    sentence: usize,
    text: String,
}

/// The lowercased words of `text`, without surrounding punctuation or a possessive. Hyphens
/// and angle brackets split words, so '30-day' reads as '30 day', and so does a number joined
/// to a unit, so '30days' reads as '30 days'. A sentence ends at a blank line, or at '.', '!' or
/// '?' before a capital, so it can wrap across lines. A missed end joins two sentences, which
/// can only add a flag.
fn words(text: &str) -> Vec<Word> {
    let mut out = Vec::new();
    let (mut sentence, mut ended, mut blank) = (0, false, false);
    for (index, line) in text.lines().enumerate() {
        blank |= line.trim().is_empty();
        for raw in line.split(|c: char| c.is_whitespace() || "-–—/<>".contains(c)) {
            let word = raw.trim_matches(|c: char| !c.is_alphanumeric());
            if word.is_empty() {
                continue;
            }
            if blank || (ended && word.starts_with(char::is_uppercase)) {
                sentence += 1;
            }
            blank = false;
            ended = raw
                .trim_end_matches([')', '"', '\'', '’', '”'])
                .ends_with(['.', '!', '?']);
            let word = word.to_lowercase().replace('’', "'");
            let word = word.strip_suffix("'s").unwrap_or(&word);
            let line = index + 1;
            for part in number_and_unit(word).into_iter().filter(|p| !p.is_empty()) {
                let text = part.to_owned();
                out.push(Word {
                    line,
                    sentence,
                    text,
                });
            }
        }
    }
    out
}

/// A word such as '30days' as its number and unit; any other word whole.
fn number_and_unit(word: &str) -> [&str; 2] {
    let digits = word.find(|c: char| !c.is_ascii_digit()).unwrap_or(0);
    let (number, unit) = word.split_at(digits);
    if UNITS.contains(&unit) {
        [number, unit]
    } else {
        [word, ""]
    }
}

fn is_number(word: &Word) -> bool {
    word.text.chars().all(|c| c.is_ascii_digit()) || NUMBER_WORDS.contains(&word.text.as_str())
}

/// The index after the period of time that starts at `start`, if one does.
fn period_end(words: &[Word], start: usize) -> Option<usize> {
    let numbers = words[start..].iter().take_while(|w| is_number(w)).count();
    let article = ["a", "an"].contains(&words[start].text.as_str());
    if numbers == 0 && !article {
        return None;
    }
    let mut end = start + numbers.max(1);
    let word_at = |i: usize| words.get(i).map(|w| w.text.as_str());
    if word_at(end).is_some_and(|w| QUALIFIERS.contains(&w)) {
        end += 1;
    }
    word_at(end)
        .is_some_and(|w| UNITS.contains(&w))
        .then_some(end + 1)
}

/// Whether a period states a deadline, rather than how long something lasted or how long
/// ago it happened.
fn is_deadline(words: &[Word], start: usize, end: usize) -> bool {
    let after = words.get(end).map(|w| w.text.as_str());
    let before = start.checked_sub(1).map(|i| words[i].text.as_str());
    after != Some("ago")
        && after != Some("old")
        && !before.is_some_and(|w| NOT_A_DEADLINE.contains(&w))
}

/// The clauses of the law table and the letter templates that state a period or an erasing
/// word. One of these, word for word, is the table's own and is not flagged.
fn table_clauses() -> Vec<Vec<String>> {
    let texts = Lever::ALL.iter().flat_map(|&lever| {
        let row = law(lever);
        [
            row.title,
            row.deadline_text,
            row.charge,
            letters::template(lever),
        ]
    });
    texts
        .flat_map(|text| text.split([',', ';', '\n']))
        .map(words)
        .filter(|clause| {
            clause.iter().any(is_erasing)
                || (0..clause.len()).any(|i| period_end(clause, i).is_some())
        })
        .map(|clause| clause.into_iter().map(|w| w.text).collect())
        .collect()
}

/// For each word of `words`, whether it lies in a clause of the table.
fn in_table(words: &[Word]) -> Vec<bool> {
    let mut covered = vec![false; words.len()];
    for clause in table_clauses() {
        for start in 0..words.len().saturating_sub(clause.len() - 1) {
            let run = &words[start..start + clause.len()];
            if run.iter().zip(&clause).all(|(w, c)| w.text == *c) {
                covered[start..start + clause.len()].fill(true);
            }
        }
    }
    covered
}

fn phrase_at(words: &[Word], start: usize, phrase: &[&str]) -> bool {
    words.len() - start >= phrase.len()
        && words[start..].iter().zip(phrase).all(|(w, p)| w.text == *p)
}

fn is_erasing(word: &Word) -> bool {
    ERASING_WORDS.contains(&word.text.as_str())
}

/// Whether the word at `index` of `sentence` is a legal source or a word of duty.
fn is_source_or_duty(sentence: &[Word], index: usize) -> bool {
    let word = sentence[index].text.as_str();
    let next = sentence.get(index + 1).map_or("", |w| w.text.as_str());
    let number = |s: &str| s.starts_with(|c: char| c.is_ascii_digit());
    LEGAL_SOURCES.contains(&word)
        || DUTY_WORDS.contains(&word)
        || (CITING_WORDS.contains(&word) && number(next))
        || (word.starts_with('s') && number(word.trim_start_matches(['s', '.'])))
        || (["have", "has"].contains(&word) && next == "to")
}

/// The sentences that may claim a right or duty to erase, keyed by the index of the first
/// matched word, as those matched words: the rest of a sentence may be personal.
fn erasure_claims(words: &[Word], covered: &[bool]) -> BTreeMap<usize, String> {
    let mut claims = BTreeMap::new();
    let mut offset = 0;
    for sentence in words.chunk_by(|a, b| a.sentence == b.sentence) {
        let erasing =
            (0..sentence.len()).find(|&i| !covered[offset + i] && is_erasing(&sentence[i]));
        let cited = (0..sentence.len()).find(|&i| is_source_or_duty(sentence, i));
        if let (Some(erasing), Some(cited)) = (erasing, cited) {
            let (first, second) = (erasing.min(cited), erasing.max(cited));
            let found = format!("{} ... {}", sentence[first].text, sentence[second].text);
            claims.insert(offset + first, found);
        }
        offset += sentence.len();
    }
    claims
}

/// A foreign-law citation at `start`: an article number, or a section of California's
/// Civil Code in the CCPA's title (sections 1798.100 onwards), with why it needs rechecking.
fn citation_at(words: &[Word], start: usize) -> Option<(String, &'static str)> {
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    let before_paren = |s: &str| s.split('(').next().unwrap_or_default().to_owned();
    let word = words[start].text.as_str();
    if let Some(section) = word.strip_prefix("1798.") {
        let section = before_paren(section);
        return digits(&section).then(|| (format!("1798.{section}"), CCPA));
    }
    let (word, number) = match word.strip_prefix("art.") {
        Some(number) => ("art", number),
        None => (word, words.get(start + 1)?.text.as_str()),
    };
    let number = before_paren(number);
    (["article", "articles", "art"].contains(&word) && digits(&number))
        .then(|| (format!("article {number}"), ARTICLE))
}

/// Whether `text`, which starts with '<', opens with a tag that only HTML has: `</word>`,
/// `<word/>` or `<word name=value>`. A lone `<word>` may be a placeholder.
fn opens_html(text: &str) -> bool {
    let Some(end) = text[1..].find(['<', '>']).map(|i| i + 1) else {
        return false;
    };
    let inner = &text[1..end];
    let closing = inner.strip_prefix('/');
    let inner = closing.unwrap_or(inner);
    let length = inner
        .find(|c: char| !c.is_ascii_alphanumeric() && !":-".contains(c))
        .unwrap_or(inner.len());
    let (name, rest) = inner.split_at(length);
    let attributes = rest.starts_with(char::is_whitespace) && rest.contains('=');
    let shape = if closing.is_some() {
        rest.trim().is_empty()
    } else {
        attributes || rest.trim() == "/"
    };
    text[end..].starts_with('>') && name.starts_with(|c: char| c.is_ascii_alphabetic()) && shape
}

/// Whether `text`, lowercased and following an '&', starts an entity: a numeric reference
/// such as `#101` or `#x65`, whose semicolon is optional, or a name such as `amp;`.
/// shortcut: any name of 2 or more characters before a semicolon counts, so 'Smith&Co;' is
/// refused. Only the full table of entity names would tell it apart.
fn is_entity(text: &str) -> bool {
    if let Some(number) = text.strip_prefix('#') {
        return match number.strip_prefix('x') {
            Some(hex) => hex.starts_with(|c: char| c.is_ascii_hexdigit()),
            None => number.starts_with(|c: char| c.is_ascii_digit()),
        };
    }
    let end = text
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(text.len());
    end >= 2 && text[end..].starts_with(';')
}

/// Why `text` cannot be checked, if it cannot: its words would not match, so a count of no
/// claims would mislead.
pub fn refusal(text: &str) -> Option<&'static str> {
    if words(text).is_empty() {
        return Some(NO_WORDS);
    }
    let lower = text.to_lowercase();
    // A mail header may fold its value onto a line that starts with a space or tab.
    let unfolded = lower.replace("\n ", " ").replace("\n\t", " ");
    let header = |name: &str, values: &[&str]| {
        unfolded.lines().any(|line| {
            line.strip_prefix(name)
                .is_some_and(|value| values.iter().any(|v| value.contains(v)))
        })
    };
    if header(
        "content-transfer-encoding:",
        &["base64", "quoted-printable"],
    ) {
        return Some(ENCODED);
    }
    let html = header("content-type:", &["text/html"])
        || HTML_SIGNS.iter().any(|sign| lower.contains(sign))
        || lower
            .match_indices('<')
            .any(|(i, _)| opens_html(&lower[i..]))
        || lower
            .match_indices('&')
            .any(|(i, _)| is_entity(&lower[i + 1..]));
    html.then_some(HTML)
}

/// Claims in `text` that the law table does not support, in the order they appear.
pub fn check(text: &str) -> Vec<Flag> {
    let words = words(text);
    let covered = in_table(&words);
    let mut claims = erasure_claims(&words, &covered);
    let mut flags = Vec::new();
    let mut start = 0;
    while start < words.len() {
        let line = words[start].line;
        if let Some(found) = claims.remove(&start) {
            let why = ERASURE;
            flags.push(Flag { line, found, why });
        }
        for (phrases, why) in PHRASES {
            if let Some(phrase) = phrases.iter().find(|p| phrase_at(&words, start, p)) {
                let found = phrase.join(" ");
                flags.push(Flag { line, found, why });
            }
        }
        if let Some((found, why)) = citation_at(&words, start) {
            flags.push(Flag { line, found, why });
        }
        if let Some(end) = period_end(&words, start) {
            if !covered[start] && is_deadline(&words, start, end) {
                let found: Vec<&str> = words[start..end].iter().map(|w| w.text.as_str()).collect();
                let found = found.join(" ");
                flags.push(Flag {
                    line,
                    found,
                    why: DEADLINE,
                });
            }
            start = end;
        } else {
            start += 1;
        }
    }
    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(text: &str) -> Vec<String> {
        check(text).into_iter().map(|f| f.found).collect()
    }

    #[test]
    fn flags_a_claimed_right_to_erasure_in_its_common_forms() {
        for text in [
            "I exercise my right to erasure.",
            "My Right to be Forgotten applies.",
            "I have a right-to-erasure.",
            "You must honour my right to\ndeletion.",
            "These are my rights to erasure.",
            "I have the right to have my data deleted.",
            "You are legally required to delete my details.",
            "You are required by law to remove them.",
            "My right to be forgotten applies.",
        ] {
            let flags = check(text);
            assert_eq!(flags.len(), 1, "{text}");
            assert_eq!(flags[0].why, ERASURE);
        }
    }

    #[test]
    fn asking_for_destruction_is_not_a_claim_of_right() {
        assert!(found("Please delete my details and destroy your copies.").is_empty());
        assert!(found("I have the right to complain. Please delete my details.").is_empty());
    }

    #[test]
    fn flags_a_stated_legal_duty_to_erase() {
        for text in [
            "The Privacy Act requires you to delete my data.",
            "The Privacy Act requires that you delete my data.",
            "Deletion is required by law.",
            "APP 11.2 requires you to destroy it.",
            "Section 13 obliges the business to erase it.",
            "Regulation 5 compels erasure.",
            "Legislation says to delete it.",
            "See s 20T and remove it.",
            "See s20T and remove it.",
            "See s. 20T and remove it.",
            "The law compels you to remove it.",
            "The Act requires deletion of my data.",
            "The law requires the erasure of it.",
            "You must by law delete it.",
            "You are obliged to erase it.",
            "You are compelled to delete it.",
            "You are required to delete it.",
            "Under the Privacy Act you have to delete it.",
            "You have to delete it.",
            "The business has to erase it.",
            "Under the Act you must delete it.",
            "It must be deleted.",
            "You shall destroy it.",
            "You have a duty to delete it.",
            "You have an obligation to erase it.",
            "I am entitled to have my records erased.",
            "You are required under the Privacy Act to delete it.",
        ] {
            let flags = check(text);
            assert_eq!(flags.len(), 1, "{text}");
            assert_eq!(flags[0].why, ERASURE);
        }
        assert_eq!(
            found("The Privacy Act requires that you delete my data."),
            ["act ... delete"]
        );
    }

    #[test]
    fn an_ordinary_request_to_delete_is_not_flagged() {
        for text in [
            "Please delete my data.",
            "Please delete my account from your app.",
            "I ask you to remove my details.",
            "I would like you to erase my records.",
            "I require you to delete my data.",
            "We require you to delete it.",
            "The Act requires you to reply. Please delete my data.",
            "I am required to give my name. Please delete it.",
            "I have a right (to complain.) Please delete it.",
            "You must reply.\n\nplease delete it",
        ] {
            assert!(found(text).is_empty(), "{text}");
        }
    }

    #[test]
    fn each_sentence_flags_once_and_may_wrap_across_lines() {
        let two = "You must delete it. The Act requires erasure.";
        assert_eq!(found(two), ["must ... delete", "act ... erasure"]);
        assert_eq!(found("You must\ndelete it."), ["must ... delete"]);
    }

    #[test]
    fn the_tables_wording_is_no_claim_but_excuses_nothing_else() {
        let clause = "APP 11.2 requires you to take reasonable steps to destroy it or to ensure \
                      that it is de-identified";
        assert!(found(clause).is_empty());
        assert_eq!(
            found(&format!("{clause}, and you must delete it.")),
            ["app ... delete"]
        );
    }

    #[test]
    fn a_tag_cannot_hide_a_claim() {
        for text in [
            "my right to <b>erasure</b>",
            "my right to <blink>erasure</blink>",
            "my right <o:p></o:p>to erasure",
            "my right to <span style=\"color:red\">erasure</span>",
            "my right to<br/>erasure",
        ] {
            assert_eq!(found(text), ["right ... erasure"], "{text}");
        }
    }

    #[test]
    fn angle_brackets_round_words_do_not_hide_them() {
        assert_eq!(found("<right to erasure>"), ["right ... erasure"]);
        assert_eq!(found("5<6 and right to erasure>2"), ["right ... erasure"]);
        assert_eq!(found("under the <GDPR>"), ["gdpr"]);
    }

    #[test]
    fn refuses_a_closing_tag_an_entity_or_other_html_by_its_shape() {
        for text in [
            "<main>My right to &#101;rasure</main>",
            "<main>Hello</main>",
            "<p>I claim my right to erasure</p>",
            "<SPAN style=\"color:red\">Hello</SPAN>",
            "<span\nstyle=\"color:red\">Hello",
            "<a href=\"https://example.com.au\">Hello",
            "<br />Hello",
            "<br/>Hello",
            "Hello </o:p>",
            "<!DOCTYPE html>\nHello",
            "<!-- a comment -->Hello",
            "My right to &#101;rasure",
            "My right to &#x65;rasure",
            "My right&nbsp;to erasure",
            "Terms &amp; conditions",
        ] {
            assert_eq!(refusal(text), Some(HTML), "{text:?}");
        }
    }

    #[test]
    fn refuses_a_numeric_reference_with_or_without_its_semicolon() {
        for text in [
            "My right to &#101rasure",
            "My right to &#x65rasure",
            "My right to &#X65rasure",
            "My right to &#101;rasure",
            "My right to &#x65;rasure",
        ] {
            assert_eq!(refusal(text), Some(HTML), "{text:?}");
        }
        assert_eq!(refusal("Ticket &# and &#x for review"), None);
    }

    #[test]
    fn a_name_of_one_character_is_no_entity_and_a_longer_name_is() {
        for text in ["R&D;", "AT&T;", "Q&A;", "Smith&Co and sons"] {
            assert_eq!(refusal(text), None, "{text:?}");
        }
        for text in ["1 &gt; 0", "a&nbsp;b", "Smith&Co;"] {
            assert_eq!(refusal(text), Some(HTML), "{text:?}");
        }
    }

    #[test]
    fn placeholders_and_addresses_in_angle_brackets_are_not_refused() {
        for text in [
            "Dear <name>,\nHello",
            "Dear <your name>,\nHello",
            "Hello<br>there",
            "To: Jo Example <a@example.com.au>\nHello",
            "See <https://example.com.au/privacy?a=b>",
            "5 < 6 and 7 > 6",
            "AT&T and Q&A",
        ] {
            assert_eq!(refusal(text), None, "{text:?}");
        }
    }

    #[test]
    fn refuses_a_folded_header() {
        for text in [
            "Content-Transfer-Encoding:\r\n base64\r\n\r\nSSBleGVyY2lzZQ==\r\n",
            "Content-Transfer-Encoding:\n\tquoted-printable\n\nHello\n",
        ] {
            assert_eq!(refusal(text), Some(ENCODED), "{text:?}");
        }
        assert_eq!(refusal("Content-Type:\n text/html\n\nHello\n"), Some(HTML));
    }

    #[test]
    fn every_template_is_checked_and_flags_nothing() {
        for lever in Lever::ALL {
            let template = letters::template(lever);
            assert_eq!(refusal(template), None, "{}", lever.name());
            assert_eq!(found(template), Vec::<String>::new(), "{}", lever.name());
        }
    }

    #[test]
    fn a_flag_on_a_claimed_right_leaves_out_the_words_it_spans() {
        assert_eq!(
            found("my right to have Jane Citizen's records deleted"),
            ["right ... deleted"]
        );
    }

    #[test]
    fn flags_foreign_law_and_its_citations() {
        assert_eq!(
            found("Under the GDPR's Article 17:"),
            ["gdpr", "article 17"]
        );
        assert_eq!(found("under art. 15"), ["article 15"]);
        assert_eq!(found("under Article 17(1)"), ["article 17"]);
        assert_eq!(found("under Art.17"), ["article 17"]);
        assert_eq!(found("under Art.17(1)(a)."), ["article 17"]);
        assert_eq!(found("(CCPA) and §1798.105(a)"), ["ccpa", "1798.105"]);
        assert_eq!(
            found("the General Data Protection Regulation"),
            ["general data protection regulation"]
        );
        assert_eq!(check("CCPA")[0].why, CCPA);
    }

    #[test]
    fn flags_a_fixed_deadline_in_any_common_form() {
        for (text, period) in [
            ("Delete it within 14 days.", "14 days"),
            ("You have fourteen days.", "fourteen days"),
            ("a 30-day deadline", "30 day"),
            ("within one month of this letter", "one month"),
            ("in twenty one calendar days", "twenty one calendar days"),
            ("within 72 hours", "72 hours"),
            ("You must reply within 30 days.", "30 days"),
            ("You have a month to reply.", "a month"),
            ("within a fortnight", "a fortnight"),
            ("within an hour", "an hour"),
            ("Keep it no longer than two years.", "two years"),
            ("in twelve days", "twelve days"),
            ("Reply within 30days.", "30 days"),
        ] {
            assert_eq!(found(text), [period], "{text}");
            assert_eq!(check(text)[0].why, DEADLINE);
        }
    }

    #[test]
    fn a_period_that_is_no_deadline_is_left_alone() {
        for text in [
            "I moved 3 months ago.",
            "I was a customer for 6 months.",
            "It has been wrong for the last 2 weeks and the past 3 days.",
            "Born 12 November 1960. 2 children.",
            "You mail me twice a month.",
            "My son is 9 years old.",
            "It came 3 times a week.",
        ] {
            assert!(found(text).is_empty(), "{text}");
        }
    }

    #[test]
    fn the_deadline_warning_keeps_the_fixed_periods_apart_from_the_apps() {
        let why = check("within 30 days")[0].why;
        assert!(
            why.contains("an organisation 'a reasonable period'"),
            "{why}"
        );
        assert!(why.contains("ss 20T and 21V"), "{why}");
        assert!(!why.contains("limit in the Act"), "{why}");
    }

    #[test]
    fn refuses_text_whose_words_cannot_be_matched() {
        for text in ["", " \n\n", "\u{feff}", "---\n"] {
            assert_eq!(refusal(text), Some(NO_WORDS), "{text:?}");
        }
        for text in [
            "Content-Transfer-Encoding: base64\r\n\r\nSSBleGVyY2lzZSBteQ==\r\n",
            "CONTENT-TRANSFER-ENCODING: Quoted-Printable\n\nmy right to=\n erasure\n",
        ] {
            assert_eq!(refusal(text), Some(ENCODED), "{text:?}");
        }
        for text in [
            "Content-Type: text/html; charset=utf-8\n\nHello\n",
            "<HTML><body>Hello</body></HTML>\n",
        ] {
            assert_eq!(refusal(text), Some(HTML), "{text:?}");
        }
    }

    #[test]
    fn plain_text_and_an_eight_bit_mail_are_not_refused() {
        assert_eq!(refusal("Dear Privacy Officer\n"), None);
        let mail = "Content-Type: text/plain; charset=utf-8\r\n\
                    Content-Transfer-Encoding: 8bit\r\n\r\nDear Privacy Officer\r\n";
        assert_eq!(refusal(mail), None);
    }

    #[test]
    fn the_law_tables_own_wording_is_not_flagged() {
        for lever in Lever::ALL {
            let row = law(lever);
            assert!(found(row.deadline_text).is_empty(), "{}", lever.name());
            assert!(found(row.charge).is_empty(), "{}", lever.name());
        }
    }

    #[test]
    fn a_table_clause_does_not_excuse_a_period_elsewhere_in_the_text() {
        let table = law(Lever::S20tCorrection).deadline_text;
        assert_eq!(
            found(&format!("{table}. Delete it in 30 days.")),
            ["30 days"]
        );
    }

    #[test]
    fn reports_the_line_a_claim_starts_on() {
        let flags = check("Dear Sir\r\n\r\nI claim my right to\r\nerasure.\r\n");
        assert_eq!(flags[0].line, 3);
    }

    #[test]
    fn a_flag_repeats_only_its_pattern() {
        let flags = check("Jane Citizen of 1 Test Street claims the GDPR");
        assert_eq!(flags[0].found, "gdpr");
    }
}
