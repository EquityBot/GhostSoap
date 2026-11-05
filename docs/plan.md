# GhostSoap repository plan

_GhostSoap at no Price._

Draft 2, 24 October 2026. Status: approved (section 12); M0 complete; M1 (register) delivered on 24 October 2026; nothing is public.

GhostSoap is a free, local-first Rust tool that helps people in Australia reclaim their privacy. It sends Australian Privacy Act requests to data brokers and other businesses from the user's own mailbox, tracks every deadline and reply, builds an escalation file for the Office of the Australian Information Commissioner (OAIC), and walks the user through the wider data hygiene steps that email cannot fix.

## 1. Why another tool

Research on 24 October 2026 (sources in section 13) found:

- **Few services use Australian rights.** The 24 October research found none, and Optery was the only paid service found with an Australian list (55 brokers, about 40 of them US business-data or ad-tech firms; from A$49 a year). DeleteMe lists Australia; Incogni does not. All of them cited US or EU law. By 5 November 2026 ClearTrace AU sold a managed service that says it operates under the Privacy Act (A$129 to A$259 for a year of re-scanning), and Serus published a free guide citing the APPs.
- **No open source tool covers Australia.** The closest projects are US or EU focused: digisamroc/eraser (Go, no licence, dormant since January 2026), datenanfragen (CC0 data, MIT site, EU), nulltrace (MIT, Go), symaira-eraseme (Apache-2.0, Rust) and Scrubbr (MIT, Python).
- **No licence-compatible Australian broker dataset exists.** The Optery directory and the Big Ass Data Broker Opt-Out List are CC BY-NC-SA, so they can serve only as leads.
- **The category underperforms.** Consumer Reports (August 2024, 32 volunteers) found paid services removed 35% of people-search profiles within 4 months; manual opt-outs did better.

GhostSoap's edge is to be honest about Australian law, use the levers that law actually provides, keep the user's data on the user's machine, and keep a small, sourced and verified register.

## 2. Principles and lines not to cross

1. **The user acts; GhostSoap prepares.** Every request goes out from the user's own mailbox, in the user's name, after the user has reviewed it. GhostSoap does not act as an agent or representative.
2. **Local first.** No server, account, telemetry or analytics. The only network traffic is what the user starts: sending mail, reading replies, opening a link.
3. **Minimum data.** Each register entry lists the identity fields that entity needs; a letter includes only those. GhostSoap never stores identity document images or Medicare, passport or licence numbers.
4. **No evasion.** No scraping of people-search sites, CAPTCHA solving, IP rotation or automated form filling. GhostSoap opens the broker's own search or opt-out page for the user and records what they found.
5. **Honest outcomes.** Results read 'requested', 'confirmed' or 'refused', never 'deleted' unless the entity confirms deletion. An honesty panel states what Australian law cannot force.
6. **Requests about the user only.** No requests for another person; household use means a separate profile per consenting adult.
7. **Information, not legal advice.** GhostSoap supplies templates and guidance and points to the OAIC, IDCARE and community legal centres. It never makes statutory declarations or Australian Electoral Commission (AEC) applications for the user.
8. **No conflicts.** No affiliate links, broker partnerships or paid tiers.

## 3. Legal model

Australia has no general right to erasure today. The tranche 2 Privacy Act reform is an Exposure Draft, not law (submissions closed 18 September 2026), and its erasure right would reach large digital platforms only (see 'Law status' below). GhostSoap therefore uses these levers, each stored in a law table with its source and deadline:

Every row below was read in the primary text on 25 October 2026: the Privacy Act 1988 compilation C2026C00227 (in force from 4 June 2026; versions start 10 December 2026 and 1 January 2027, and the monthly evidence check reports each once the Federal Register records it as latest), the Spam Act 2003 compilation C2016C00614, the Spam Regulations 2021 (F2021L00285), the Criminal Code compilation C2026C00243 and the OAIC's APP guidelines (paragraph numbers given). The law table in `src/core/law.rs` holds the same values with their citations, and `ghostsoap law` prints it.

| Lever | Request | Deadline basis |
|---|---|---|
| APP 7.6(c), 7.7(a) | Stop sending me direct marketing; free | Reasonable period, OAIC: generally no more than 30 days (para 7.37) |
| APP 7.6(d), 7.7(a) | Stop using or disclosing my information to facilitate others' direct marketing, including by sale; free | Reasonable period, OAIC: no more than 30 days (para 7.41, without 'generally'). Selling data to another organisation is the OAIC's own example of facilitating (paras 7.40, 7.43), so APP 7 reaches brokers that only sell |
| APP 7.6(e), 7.7(b) | Tell me the source of my information; free | Generally 30 days (para 7.47), unless impracticable or unreasonable; the organisation must justify that (para 7.46); source tracing finds the next broker upstream |
| APP 12 | Give me access to the information you hold | Organisations: reasonable period (APP 12.4(a)(ii)), OAIC: should not exceed 30 calendar days (para 12.67); agencies: 30 days. No charge for the request; a non-excessive charge for giving access is allowed (APP 12.8, paras 12.77–12.81), so the letter asks for any charge to be stated first. 10 refusal grounds (APP 12.3); a refusal needs written reasons and complaint options (APP 12.9) |
| APP 13 | Correct information that is inaccurate, out of date, incomplete, irrelevant or misleading; free (APP 13.5(b)) | Reasonable period, OAIC: should not exceed 30 calendar days (para 13.63) |
| APP 13.2 | Tell the entities you disclosed my information to about the correction | No time set: reasonable steps unless impracticable or unlawful |
| APP 13.4 | Attach my statement where you refuse to correct | As APP 13 (APP 13.5 covers it; para 13.63) |
| APP 11.2 | Destroy or de-identify information you no longer need | An entity obligation, not an individual right; asked for, never demanded |
| s 20R | Credit reporting body access | Within a reasonable period, not longer than 10 days (s 20R(3)); free if no request in the previous 3 months (s 20R(5)) |
| s 20T | Credit reporting body correction | Within 30 days or a longer period the individual agrees to in writing (s 20T(2)); free (s 20T(5)) |
| s 21T, s 21V | Credit provider access; credit provider correction | Access: reasonable period (s 21T(3)); correction: 30 days (s 21V(2)) |
| s 20K | Ban period after suspected fraud | Starts on the request and ends 21 days later, extendable on request, more than once (s 20K(3)–(5)); free (s 20K(6)) |
| Spam Act 2003 | Unsubscribe | Withdrawal of consent takes effect at the end of 5 business days from the day an electronic unsubscribe message is sent (from service for post, from delivery otherwise), with public holidays where the message was sent to (Sch 2 cl 6); only email channels may serve this lever, so letters state the sending rule. The unsubscribe address must be reasonably likely to receive unsubscribe messages for at least 30 days after the message is sent (s 18(1)(e)) and must not need a fee, an account or personal information beyond the address (Spam Regulations 2021 s 7) |
| s 40(1A)–(1B), s 41 | Escalation | Complain to the entity first; the Act sets no waiting period, and OAIC guidance allows about 30 days (para 12.87). Then the entity's external dispute resolution scheme if it has one (AFCA for credit entities, the TIO for telcos), then the OAIC, within 12 months of becoming aware (the OAIC may decline later complaints, s 41(1)(c)). The OAIC has a discretion not to investigate (s 41(1)(da), (dc), (dd)) and publishes a long wait (see 'Law status' below). The complain-first precondition does not apply to complaints about ss 20R, 20T, 21T or 21V. Spam complaints go to ACMA through its spam complaint form with a copy of the message; telemarketing complaints go to donotcall.gov.au. Each register entry lists its escalation targets |

APP 12 and 13 set no form for a request: an entity may publish a preferred procedure but 'cannot require an individual to follow a particular procedure, use a designated form or explain the reason for making the request' (paras 12.18, 12.21, 13.20, 13.22). A letter to any published privacy address is therefore a valid request.

APP 7 applies only where the entity uses or discloses information for direct marketing, and is displaced to the extent the Interactive Gambling Act 2001 (Pt 7B Div 5), the Do Not Call Register Act 2006 or the Spam Act applies (APP 7.8). A broker that does neither gets APP 12 and 13 letters, not APP 7, and a reply to that effect is recorded as `not-direct-marketing`.

The Privacy and Other Legislation Amendment Act 2024 added a statutory tort for serious invasions of privacy (Privacy Act Schedule 2, s 94A; commenced 10 June 2025) and doxxing offences (Criminal Code ss 474.17C and 474.17D; commenced 11 December 2024). GhostSoap prepares neither claims nor reports; the honesty panel and checklist point to a community legal centre, police and eSafety.

Each register entry records an **exemption risk**: `none`, `small-business-possible` (turnover under A$3 million, unless it discloses personal information for a benefit, service or advantage, or pays to collect it, under s 6D(4)(c)–(d); s 6D(7)–(8) narrow both carve-outs), or `overseas` (responsible entity overseas with no ABN found: no levers, a manual check only; the Act can still reach an overseas organisation that carries on business in Australia (s 5B(1A), (3)), so GhostSoap says it has not established a legal basis rather than that none exists). The letter text and the honesty panel change with it.

A future `au-erasure` letter waits for passed legislation. Adding it needs a `Lever` variant, a law table row and a letter template; the compiler rejects a lever missing either.

### Law status, read 5 November 2026

This panel records what the primary sources said on that date, or on the date a row gives; `ghostsoap law` prints its short form. Sources are in the list at the end of this plan. Re-read the lever provisions when the 10 December 2026 compilation registers, because nothing below has been checked against that text.

| When | Source | What it does | Effect on GhostSoap |
|---|---|---|---|
| In force | Privacy Act 1988, C2026C00227 | The Federal Register shows it as the latest version, ending 10 December 2026; no later compilation has a register ID | Every lever above rests on this text |
| 10 December 2026 | Privacy and Other Legislation Amendment Act 2024, sch 1 items 87 and 88 | Insert APP 1.7 to 1.9 and add APP 1.7 to s 13K(1)(b). Item 89 applies them to decisions made after commencement | Transparency duty only; see 'Automated decisions' below |
| 1 January 2027 | Interactive Gambling Amendment (Gambling Reform) Act 2026, sch 1 items 46 and 47 | Add s 33C(1)(h) and insert 'and subsection 62P(2)' in APP 7.8(aa) | The APP 7.8 note in this section needs rereading; no lever changes on the face of the items |
| By 10 December 2026 | Privacy Act s 26GC, inserted by the 2024 Act (assented to 10 December 2024); read 6 November 2026 | Section 26GC(10): 'The Commissioner must develop and register the Children's Online Privacy Code within the period of 24 months beginning on the day the Privacy and Other Legislation Amendment Act 2024 receives the Royal Assent.' The OAIC says 'The final Code must be registered by 10 December 2026'. It consulted on an Exposure Draft from 31 March to 5 June 2026, and the Federal Register had no such title on 6 November 2026 | No lever uses it. It would bind APP entities within s 26GC(5), so read the final code for any change to the APP levers when it registers |
| Not law | Exposure Draft, Privacy Amendment (Personal Data Protection) Bill 2026 | Released with a consultation paper; consultation opened 31 August and closed 18 September 2026. The Attorney-General's Department says the Bill 'remains subject to further consideration by government'. The draft's commencement table is blank. At the 31 August press conference the Attorney-General spoke of 'the introduction of the Bill this year' | No lever or letter relies on it; see 'Tranche 2 levers to watch' below |

**Automated decisions.** From 10 December 2026, APP 1.7 to 1.9 require an APP privacy policy to state the kinds of personal information used in certain computer programs and the kinds of decisions those programs make or substantially and directly support, where a decision could reasonably be expected to significantly affect an individual's rights or interests. The OAIC's September 2026 fact sheet and APP 1 guidelines (version 2) say a decision can be in scope even where a person reviews the program's output. This is a transparency duty on the entity. It gives the user no request to make, so it gets no lever and no register field. The monthly evidence hashes already flag a changed privacy policy where that page is tracked.

**The OAIC's published wait.** The OAIC's blog of 2 March 2026 says: 'As at February 2026, it is unlikely that we will be able to substantially progress new validly lodged individual privacy complaints for some 6 to 12 months after they are lodged (unless we determine there are exceptional circumstances which may warrant expeditious consideration).' It also says 'Not all individual privacy complaints will be taken through to investigation', that a person should generally give the entity 30 days, and that a person should generally try an approved external dispute resolution scheme first, listing telecommunications providers among the areas covered. The OAIC's register lists 10 recognised schemes, including AFCA and the TIO. One search of the OAIC's site found no later statement of the wait; the February figure may have changed.

**Tranche 2 levers to watch.** Each row describes the Exposure Draft or the consultation paper, neither of which is law. The draft may change before introduction, and every effect is conditional on enactment as drafted.

| Lever or field | What the draft proposes | What GhostSoap would then need |
|---|---|---|
| `app7-no-marketing` | Draft APP 7.2: on a request, an organisation 'must take such steps as are reasonable in the circumstances to give effect to the request'. The draft clause has no 'reasonable period' and no bar on charging, both of which current APP 7.7 has | New deadline and charge text |
| `app7-no-facilitation`, `app7-source` | Item 27 repeals clause 7 and substitutes draft APP 7.1 to 7.7, which have no counterpart to current APP 7.6(d) or (e). Draft APP 4.2 instead says an organisation 'must not trade the information unless the individual has consented to the trading of the information', subject to exceptions; draft s 6FC makes a disclosure a trade if it is for money or other consideration or for the purposes of direct marketing, with carve-outs | Both letters would lose their present basis; a replacement would need its own reading of draft APP 4 |
| `app11-destroy` | Draft APP 11.2 adds a duty to 'consider whether to destroy the personal information' before destroying or de-identifying it. It stays a duty on the entity | Reword the letter; still asked for, not demanded |
| `app12` | Draft Schedule 4 Part 1 adds an exception for technical impossibility or infeasibility. The consultation paper says the existing timeframes for responding remain unchanged | Reply handling for the new ground |
| `au-erasure` (not built) | Draft APP 14.1: 'If an organisation that is a large digital platform holds personal information that relates to an individual, the organisation must, on request by the individual, destroy the information.' Draft s 6EB limits this to providers of services defined in the Online Safety Act 2021 that meet a gross revenue test of at least $500 million or an end users test of at least 2.5 million, or are prescribed. Draft APP 14.2 and 14.3 give exceptions | A new lever for large digital platforms only, and only after the law passes |
| `exemption_risk` | Draft s 6D(4)(c) and (d) restate the small business carve-outs in terms of trading | Recheck `small-business-possible` |
| `escalation` | The consultation paper, with no draft provision, proposes that APP entities 'respond within 60 days' to complaints and that individuals generally raise concerns with the entity first | Recheck the 30-day guidance in this section |

## 4. Scope

**In v0.1 (milestones M0 to M4):** register, letters, draft-only sending (`.eml` files and `mailto:` links), encrypted local tracker with deadlines and reminders, OAIC escalation pack export, Australian hygiene checklist, CLI.

**Later (M5 to M7):** SMTP sending, read-only IMAP reply queue, an interface for non-technical users.

**Out of scope:** non-Australian brokers (the user can use other tools; GhostSoap may link to them), except overseas people-search sites that list Australians, which get manual opt-out entries only (section 6), account creation on brokers, automated web forms, people-search scanning, any hosted service.

## 5. Architecture

One crate, `ghostsoap`, with a library and a thin `src/main.rs` (house convention: standalone crate, `[lib]` split). Edition 2024, `rust-version = "1.99"`, `rust-toolchain.toml` pinned to `1.99.1`, release profile `lto = true`, `codegen-units = 1`.

The library keeps two layers apart:

- **`core` (no I/O):** register model and validation, law table, letter rendering, request state machine, deadline calculation, checklist model. Pure functions over values, so the same code can later compile to WebAssembly (M7) without a rewrite.
- **`io`:** files, encryption, mail client hand-off, SMTP and IMAP (later), terminal prompts.

### Data model

**Register entry**, one TOML file per entity under `registry/au/`:

```toml
id = "example-broker-au"          # permanent slug; never reused
name = "Example Broker"
legal_entity = "Example Broker Pty Ltd"
abn = "00 000 000 000"            # checked against ABN Lookup
kinds = ["marketing-list"]        # credit-reporting-body, property-data, people-search, marketing-list, ad-tech, directory, loyalty, recruitment, telecommunications, financial-services, insurance, retail, online-service
priority = "high"                 # crucial | high | standard
quality = "verified"              # tested | verified | imported | lead
verified_at = 2026-10-24
sources = ["https://example.com.au/privacy"]
levers = ["app7-no-marketing", "app7-no-facilitation", "app7-source", "app13", "app13-notify-recipients", "app13-statement"]
opt_in_levers = ["app12"]         # never in a default pack; see section 6a
identity_fields = ["full_name", "email", "postcode"]
exemption_risk = "none"
escalation = ["oaic"]             # ordered; for example ["afca", "oaic"] for a credit entity, ["tio", "oaic"] for a telco
effort = "easy"                   # easy | medium | hard | impossible

[[channels]]
kind = "email"                    # email | web-form | post | phone
address = "privacy@example.com.au"

[manual_check]
url = "https://example.com.au/search"   # the broker's own page; GhostSoap only opens it

# obsolete = { reason = "merged", replaced_by = "other-id" }
```

Quality grades follow datenanfragen: `tested` means a GhostSoap request got a reply; `verified` means the contact was checked against the entity's own privacy policy or contact pages, by a person or an AI agent, with the evidence recorded in `docs/provenance.md`; `imported` means taken from a licence-compatible dataset; `lead` means named in a non-reusable source and not yet checked. Only `tested` and `verified` entries enter packs by default.

Planned register fields (M3, not yet in the schema), after datenanfragen's `needs-id-document` and the Big Ass Data Broker Opt-Out List's flags: `last_tested` (the date of the latest `tested` reply, beside `verified_at`), `needs_id_document`, `needs_phone` and `paid_removal`. `plan` and `draft` warn before a letter goes to an entry carrying one of these flags.

**Packs** (`registry/au/packs/*.toml`) group entries: `credit` (Equifax, Experian, which now includes illion, and any other credit reporting body), `property`, `marketing`, `people-search`, `loyalty`, plus `start-here` for users with little time.

**Request**, held in the tracker: id, entity id, lever, letter version, channel, timestamps, status, reason and an append-only event list. Statuses: `drafted`, `sent`, `acknowledged`, `charge-quoted`, `fulfilled`, `refused-with-reasons`, `refused`, `no-record`, `id-requested`, `exempt-claimed`, `overdue`, `escalated`, `closed`. Reasons include `not-direct-marketing` and otherwise borrow the Data Rights Protocol vocabulary where it fits. `ghostsoap status` groups the statuses into 5 action states, after datenanfragen's proceedings: overdue (`overdue`), action needed (`charge-quoted`, `id-requested`, `exempt-claimed`, `refused`), waiting (`drafted`, `sent`, `acknowledged`, `escalated`), done (`fulfilled`, `refused-with-reasons`, `no-record`, `closed` after a result) and abandoned (`closed` without one). Overdue items are listed first.

**Profile**: the user's identity fields, previous addresses and alias names; never document numbers.

### Storage and secrets

- One state file (profile, requests, events) encrypted with the `age` crate under a passphrase (scrypt), entered through a hidden prompt (`rpassword`).
- State lives in the platform data directory (`%LOCALAPPDATA%\GhostSoap` on Windows, the XDG data directory on Linux), never under `Documents` or another folder OneDrive or a similar service may sync.
- Writes go to a temporary file in the same directory, `sync_all`, then rename, retrying a rename that fails because antivirus or an indexer holds the file. A lock file stops 2 processes overwriting each other. GhostSoap keeps only what it reported as saved: after an interrupted update the state file still holds the last complete state, and after an interrupted `init` the user runs it again.
- The OS keyring cache for the key is optional and off by default: it trades the passphrase for OS-login protection, since any process running as the user can read it. SMTP credentials (M5) live only in the keyring.
- Plaintext outside the state file is the weakest point: drafts, exports, the escalation pack and the mailbox's Sent folder. Drafts go to a `drafts` folder in the data directory (private to the user, unlike a shared temporary folder such as Linux's `/tmp`; on Unix the folder is mode 700 and each draft 600) and `ghostsoap clean` removes them. Each draft is recorded as pending in a `.ghostsoap-drafts` manifest in its folder, written to a temporary file of its own, hard-linked into place once complete (so it never replaces an existing file) and then committed with the SHA-256 of its content; the manifest keeps only each name's latest record and is replaced whole by renaming a complete copy over it, never edited in place; a lock file serialises `write` and `clean` between GhostSoap processes. `clean` deletes a committed draft only while it still holds those bytes (a file put at that name with exactly those bytes counts as the draft), and reports, without deleting, a listed file that changed, a pending name whose write never committed and temporary files left by an interrupted write or manifest update, on every run until they are dealt with; it refuses a manifest that is a link, keeps a `--out` folder's other files and removes only GhostSoap's own emptied folder, never one named with `--out`. `draft` warns, never refuses, when the drafts folder (checked as given and with links resolved, or at its nearest existing ancestor if not yet made) is inside a Git working tree (a `.git` folder or file at or above it) or a OneDrive, Dropbox or iCloud Drive folder at its usual location; the check is incomplete, so no warning does not make a folder private. On Unix a drafts folder others can open, or a link, is refused; export and escalation warn where they write; the README says plainly that the Sent folder holds copies.
- `ghostsoap export` writes a readable archive; `ghostsoap forget` deletes the state file, drafts and keyring entries after confirmation. Docs say 'deletes', not 'erases', because an SSD cannot promise erasure.
- Leak tests check the state file, drafts folder after `clean`, temporary files and logs for the profile's plaintext values.
- A 'What leaves your machine' page in the README lists every outbound path.

### Letters

Plain text templates in `letters/au/<lever>.txt` with `{{field}}` placeholders, rendered by a small strict renderer: an unknown placeholder is an error, and a line whose optional field is empty is dropped. Letters are written fresh from the Privacy Act, the APP guidelines and Part IIIA; none is adapted from another project's text. Each letter carries a version so a request records exactly what was sent. Subject lines carry a tag such as `[GS-7K3QX]` to match replies. From M3, a new tag is checked against the request store, since 5 Crockford base32 characters give only 33,554,432 values.

Since M2, a draft is one message per entry and channel: an opening (date, addressee, the entry's identity fields only, a request not to use them for anything else), one paragraph per lever with its deadline and charge rule from the law table, and a closing (written reasons for a refusal, the escalation targets, the tag). Without `--lever`, a round asks for the 3 APP 7 requests, which return no personal information and need nothing from the user; any other lever is named with `--lever`. Access, correction, notification, statement, ban and unsubscribe levers need a single `--entry` and no `--pack`, and those needing the user's own words take `--detail`.

### Sending and replies

- **v0.1:** `ghostsoap draft` writes `.eml` files with `X-Unsent: 1` (so Outlook opens them as drafts) and, with `--mailto`, prints `mailto:` links for letters short enough (2,000 characters; a 3-request APP 7 letter is longer); links are opt-in because each holds the whole letter and stays in the terminal's history; `ghostsoap draft --copy` prints the To, Subject and Body for webmail, which cannot open `.eml` files and truncates long links. The user sends from their own client, then runs `ghostsoap sent <id>` (or marks a batch). No credentials are stored, and the user's Sent folder doubles as evidence for an escalation.
- **M3, replies to pushback:** `ghostsoap reply <id> <status>` offers a ready follow-up letter for the common answers, as datenanfragen's Reactor does: a demand for an identity document or more identity than needed (APP 12 identity checks; OAIC paragraph to check), the wrong channel or a required form (OAIC 12.21 and 13.22), a charge for making the request or an excessive charge (APP 12.8), or any charge where the law table records that none may be made, a claim that the use is not direct marketing or that the small business exemption applies, and silence. Each follow-up carries the original tag, date and letter version, and its ground is cited from the law table.
- **M3, reminders that escalate:** after Your Digital Rights' reminder (ideas only; GPL-3.0), a reminder's wording follows the status: before the deadline it asks for a reply; after it, it states the days elapsed against the law table's period for that lever, says the period has passed and names the escalation target. It quotes the original letter's date and tag.
- **M3, a failed read is not an all-clear:** after yingzi (ideas only; MIT), a status view that cannot read the state says DEGRADED and shows `?` for its counts, never 0, and a removal or clear result recorded without a date is due for re-check at once, since an undated removal proves nothing.
- **M5:** SMTP through `lettre` with rustls, a review step before each batch, and a conservative rate limit.
- **M6:** read-only IMAP that matches tags and suggests a status in a review queue. It never sends, replies, deletes or follows links on its own; a confirmation link is offered only when it stays on the entity's own registered domain.

### CLI (clap 4, derive)

```text
ghostsoap init                       create the encrypted profile (M2)
ghostsoap packs | brokers [--pack]   browse the register
ghostsoap plan --pack start-here     choose entities and levers; dry run by default (M2)
ghostsoap draft [--all-planned]      write .eml files, mailto links with --mailto (M2, without --all-planned)
ghostsoap clean                      delete drafts (M2)
ghostsoap law                        print the law table and dated law status (M2)
ghostsoap check <file>               warn of legal claims the law table does not support
ghostsoap sent <id>...               record what was sent
ghostsoap reply <id> <status> [--note]
ghostsoap status [--due]             deadlines, overdue items, next actions
ghostsoap escalate <id>              export an OAIC complaint pack
ghostsoap checklist [--next]         the hygiene checklist and progress
ghostsoap registry check             validate the register (also run in CI)
ghostsoap export | forget
```

Every command is planned to support `--json`; output honours `NO_COLOR`, and no status is shown by colour alone. (So far `--json` is built only for `plan`, `law` and `registry list`, which hold no personal details, for the agent skill in `skills/ghostsoap`; M2 output has no colour.)

### Dependencies (initial)

`clap`, `serde` with `derive`, `toml`, `serde_json`, `age`, `rpassword`, `keyring`, `chrono` (`default-features = false, features = ["clock", "std"]`, as the house ports use), `tempfile`. Later: `lettre`, an IMAP client chosen at M6. `serde` derive departs from the ports' `serde_json::Value` habit because the register and state have fixed schemas; the ports' reason (byte parity with Python) does not apply.

## 6. Register

- **Seed:** the 3 credit reporting bodies; the firms in the ACCC's 2024 data broker report (CoreLogic, PropTrack, Quantium, LiveRamp, Nielsen, Oracle and others named there); Australian people-search sites (leads from the Optery list such as Reverse Australia, verified independently); marketing-list firms; directories; the large loyalty programmes as APP 7 targets. Target for v0.1: 25 to 40 entries at `verified` or better. Small and correct beats large and stale. On 26 October 2026 Ryan widened the register to recruiters, job boards and large consumer firms (decision 8), which takes it past that target.
- **Data licence:** CC0-1.0 for `registry/` and `checklist/`, so other projects (datenanfragen, eraseme) can reuse it; code under FSL-1.1-ALv2.
- **Freshness:** CI fails an entry with no source, an unknown lever or a malformed ABN, and warns when `verified_at` is older than 12 months. A scheduled workflow opens one issue a quarter listing stale entries. Since M1, entries carry `[[evidence]]` hashes of their source pages, and a monthly workflow opens or updates one issue listing pages that changed or could not be read, and closes it when every page matches again (normalisation in `registry/README.md`). Since 5 November 2026 the same run requests each `manual_check` page and hashes the OAIC's recognised external dispute resolution register.
- **Backlog (M3):** unvetted leads go in `registry/au/backlog.toml` (name, lead source, date found, reason not yet an entry), as markramm/data-broker-registry keeps `backlog.json`, so a lead is neither guessed at nor silently dropped. `registry check` prints coverage counts: entries by quality grade, kind and pack, and backlog size.
- **Contribution:** an issue form and a pull request template asking for the source URL and the date checked. Replies from `tested` requests (with personal details removed) can raise an entry's grade.
- **Feedback loop:** APP 7.6(e) replies name upstream sources; `ghostsoap reply --source "<name>"` records them, and the README invites users to report new brokers this way.
- **Overseas people-search sites (M2):** a site that lists Australians but has no Australian presence gets an entry with `exemption_risk = "overseas"` and no ABN. It carries only a `manual_check` page, the site's own web-form opt-out and an effort marker, and no levers or letters; the honesty panel says GhostSoap has not established that Australian privacy law reaches the site. The validator accepts a missing ABN only on such entries. From decision 8 the same treatment covers services whose privacy policy names an overseas company with no ABN as responsible for Australians' data (Indeed, LinkedIn), although they have Australian offices. Leads may come from Optery's Australian list (names only, as section 9 requires); the priority tiers of the Big Ass Data Broker Opt-Out List and JustDeleteMe's difficulty ratings are the pattern for the effort marker, with no data copied.
- **Outcome reporting (M3, opt-in):** `ghostsoap report-outcome <id>` writes a summary with personal details removed (entry id, lever, status, days to reply; a leak check refuses the summary if it carries the user's details in any common form: name slugs, phone formats and street-name abbreviations, after yingzi) for the user to review and paste into an issue form; nothing is sent automatically. Reports raise entries to `tested` (as datenanfragen does) and feed a public table of how each entity responded; Consumer Reports measured removal rates once, and no paid service publishes this.
- **Sharing with datenanfragen (after M1):** offer the Australian entries to datenanfragen (CC0 both ways), mapped to its schema. This is a pull request to another project and needs Ryan's authorisation first.

## 6a. Threat model and abuse

| Actor | Threat | Response |
|---|---|---|
| The user | Loses the passphrase or device | Export; clear warning that a lost passphrase cannot be recovered |
| Someone with access to the device (for example an abusive partner) | Reads the profile, requests or checklist progress, which reveal what the user is hiding | Encryption at rest, keyring cache off by default, drafts cleaned, no plaintext logs; the README's safety section links to eSafety and DV support guidance on device safety |
| An impersonator | Uses GhostSoap to send APP 12 requests in a victim's name from a fresh webmail address and collect the victim's details from many brokers at once | APP 12 is opt-in per entity with a warning, never in a pack or bulk command; every APP 12 letter invites the entity to verify identity as APP 12 requires (para 12.15); APP 7 and 13 letters, which return no data, stay the default; the README states misuse may be an offence |
| A hostile broker | Uses the request to confirm or enrich its record of the user | Minimum fields per entry; letters ask the entity not to use request details for any other purpose |

## 7. Hygiene checklist

Data under `checklist/au/`, one TOML file per item, with why it matters, the steps, the official link, when it applies, effort and a re-check interval. Initial items:

- Have I Been Pwned breach check, and IDCARE if identity documents leaked
- Credit report access from each credit reporting body, and a ban period after fraud or a breach
- Do Not Call Register
- Silent number through the user's telco
- AEC silent elector enrolment (only where there is a risk to personal safety; the user applies)
- Google 'Results about you'
- Closing dormant accounts (JustDeleteMe's MIT `sites.json` as an optional import, with its notice)
- My Health Record privacy settings
- Loyalty programme marketing opt-outs (feeds APP 7 requests)
- Social media profile audit
- ASIC director address: under ASIC's CP 391 proposal (11 September 2026), an officeholder could lodge a service address from 1 July 2027, and from 1 August 2028 an officeholder's residential address would be open to general access (free and paid extracts) unless a service address is lodged; ASIC plans its report on submissions, with a draft legislative instrument, for March 2027 (confirm the final position before inclusion)
- Doxxing or serious invasion of privacy: report to police and eSafety; a community legal centre for the statutory tort

The checklist is guidance: it links to the official page and records progress; it submits nothing.

## 8. Milestones and acceptance checks

| Milestone | Delivers | Done when |
|---|---|---|
| M0 Scaffold | Repository, licences, README with tagline, CI | CI matrix (`ubuntu-latest`, `windows-latest`) runs `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`, `cargo build --locked --release` green; `cargo audit` and Dependabot enabled |
| M1 Register | Schema, validator, 25 to 40 entries, packs | `ghostsoap registry check` passes; CI rejects a fixture entry with no source; every entry has an ABN Lookup check recorded |
| M2 Letters and drafts | Profile, plan, draft, law table; overseas people-search entries (section 6) | Snapshot tests for every lever letter; tests show a letter contains only the entry's identity fields; deadline values verified against primary sources and cited in the law table |
| M3 Tracker | Encrypted state, statuses, deadlines, reminders, OAIC pack; re-check reminders by kind (defaults of 60 days for people-search and 90 days for other kinds, after Scrubbr) to catch re-listing; opt-in `report-outcome`; replies to pushback, reminders that escalate and 5 action states (section 5); planned register fields and backlog (sections 5 and 6) | Unit tests for every status transition and deadline; snapshot tests for every reply and reminder letter, with each ground cited from a primary source; plaintext-leak test on the state file and the outcome report, matching name, phone and street variants; DEGRADED and undated-removal tests; export and forget round-trip test |
| M4 Checklist | Checklist data and command | Each item links to an official page checked within the release month |
| v0.1 | First release candidate | Security audit skill run, Alcuin final review, letters reviewed against the statute; release gates per the house binary-release decision when Ryan authorises a release |
| M5 SMTP | Optional sending | Review step, rate limit and keyring-only credentials, with tests against a local SMTP test server |
| M6 IMAP queue | Reply matching | Read-only by test; domain check on confirmation links |
| M7 Reach | Interface for non-technical users | Decision recorded first (section 11) |

## 9. Clean-room and provenance

- No code, letter text or data from digisamroc/eraser (no licence) or its forks. Its README and public behaviour informed the design only.
- CC BY-NC-SA sources (Big Ass Data Broker Opt-Out List, Optery directory) are leads: an entry may name a broker found there only after independent verification from the broker's own pages, and copies no text.
- MIT or Apache-2.0 sources may be reused with their notices; each reuse is listed in `NOTICE` and `docs/provenance.md`.
- GPL-3.0 and AGPL-3.0 projects (Your Digital Rights, OpenOptOut) and CC BY-NC-SA data (markramm/data-broker-registry) informed the design only; no code, text or data is copied. The M3 additions in sections 5 and 6 (25 October 2026) came from reading datenanfragen (code MIT, data CC0-1.0), Your Digital Rights, markramm/data-broker-registry and the Big Ass Data Broker Opt-Out List, on that date. The DEGRADED, undated-removal and leak-check rules (3 November 2026) came from reading NidusVespae/yingzi (MIT) on that date.
- `docs/provenance.md` records, per register entry, where the lead came from and what verified it.

## 10. Repository layout

```text
GhostSoap/
  Cargo.toml  Cargo.lock  rust-toolchain.toml  clippy.toml
  LICENSE (FSL-1.1-ALv2)  LICENSE-DATA (CC0-1.0)  NOTICE  README.md  SECURITY.md  CONTRIBUTING.md
  src/  main.rs  lib.rs  core/  io/  cli.rs
  registry/au/  *.toml  packs/
  letters/au/   *.txt
  law/au.toml
  checklist/au/ *.toml
  tests/        cli.rs  registry.rs  letters.rs  tracker.rs  fixtures/
  docs/         plan.md  provenance.md  what-leaves-your-machine.md
  .github/      workflows/ci.yml  ISSUE_TEMPLATE/  dependabot.yml
```

The GitHub repository is `yungSNAT/GhostSoap`.

## 11. Alternatives considered

1. **Fork digisamroc/eraser (Go).** Rejected: no licence, dormant, US-focused data, and Ryan's Rust-first rule.
2. **Contribute an Australian pack to symaira-eraseme (Rust, Apache-2.0, active, single maintainer).** The strongest alternative: it already has an encrypted identity store, deadlines, inbox sorting and a YAML registry by region. Not chosen as the base because its registry contract closes jurisdictions, laws and templates to GDPR- or CCPA-style erasure (no AU jurisdiction; it files ReverseAustralia under `us/` with CCPA), so an Australian pack would need upstream contract changes; it includes CAPTCHA-solving integrations (capsolver, 2captcha), which break principle 4; and it is a large workspace (git dependencies on the author's own crates, LLM-drafted replies, a macOS Swift app, Go parity fixtures) whose direction GhostSoap cannot steer. CC0 register data keeps this open: GhostSoap's Australian entries can be offered upstream later.
3. **Contribute to datenanfragen.** Its schema and grades are the model for GhostSoap's register, but its requests are GDPR-shaped and it is a TypeScript web app. Possible later contribution of Australian company records.

## 12. Decisions

Ryan approved draft 2 on 24 October 2026 and decided:

1. **Licences:** Apache-2.0 for code, CC0-1.0 for register and checklist data.
2. **Visibility:** the repository stays private until M1.
3. **Interface:** decided at M7, from a static WebAssembly web app (no install; data in the browser, like datenanfragen), a local web UI served by the CLI, or a desktop app.
4. **Branding:** the name and tagline only; no Call of Duty art, logos or fonts; the README disclaims any affiliation.
5. **Register quality (24 October 2026, at M1):** `verified` covers a check against the entity's own pages with recorded evidence, made by a person or an AI agent; provenance says which.
6. **Evidence coverage (24 October 2026, at M1):** an entry records `[[evidence]]` for each source page that gives the same hash on 2 fetches and a hash no other source shares. Pages that return bot checks, errors or browser-only content stay untracked, and provenance lists them; at M1, 22 of 29 entries carry evidence and 7 have none.
7. **Letters and drafts (25 October 2026, at M2, agent's choices within the approved plan):** the law table is Rust code with one match arm per lever, so a missing row or letter fails to compile; a message bundles an entry's levers per channel rather than one message per lever; the default round is the 3 APP 7 levers; drafts are cleaned by a `clean` command; the encrypted profile arrives in M2 (`init`), with requests and events added in M3. Ryan kept Locate Family as an overseas entry although its operator is not named.
8. **Removal requests beyond brokers (26 October 2026):** Ryan asked for data removal requests to recruitment agencies and to the companies people most often ask to delete their data. No dataset ranks companies by requests received, so he chose recruiters and job boards, firms behind large Australian data breaches and large consumer firms (telcos, banks and lenders, insurers, retailers), with a `recruitment` kind and pack. SEEK is in the register but in no pack, so a letter goes to it only with `--entry`. The agent added the kinds `telecommunications`, `financial-services`, `insurance`, `retail` and `online-service`. `app11-destroy` stays out of the default levers: the user names it with `--lever app11-destroy`, because only the user knows whether they still deal with a business (a Alcuin review found that a default would reach current customers, such as Qantas members through the loyalty pack).

## 13. Risks

| Risk | Mitigation |
|---|---|
| Wrong legal statement in a letter | Letters cite primary sources; Alcuin review before v0.1; honesty panel; no legal advice |
| Stale register | Grades, `verified_at`, CI warnings, quarterly stale issue, obsolete redirects |
| The tool becomes a dossier of the user | Encryption at rest, minimum fields, no document numbers, forget command |
| Brokers demand identity documents | Guidance to send a redacted copy only where APP 12 verification is reasonable; record the demand in the register |
| Deliverability problems | Draft-first; small Australian list; rate limit at M5 |
| Abandonment | Small surface; data in CC0 so it survives the code |
| Trademark complaint over the name | No game assets; rename path kept simple |
| Impersonation requests | No bulk APP 12; letters invite identity verification; misuse warning (section 6a) |
| The state file syncs to the cloud | Platform data directory, never `Documents`; a check at `init` warns if the path is inside a known sync folder |

## 14. Review record

- **Alcuin:** not done. Three attempts on 24 October 2026 stopped before dispatch: a ChatGPT sign-in timeout, a 20-minute wait for a browser slot (one slot held by a stale lease from 21 October), and 'Picker selection is unverified'. Rerun before v0.1, and before M2 if it becomes available.
- **Independent Opus review** (24 October 2026, read-only, against OAIC guidance and the Privacy Act): verdict 'sound enough to start M0'. Accepted and applied: the APP 7 split and APP 7.8, APP 12 charges and refusals, APP 13 grounds and levers, the escalation order (EDR, then OAIC; ACMA for spam), the 2024 Act, the storage changes, the draft usability fixes, the threat model, accessibility, the agent wording, and the chrono `clock` feature. Its eraseme findings were checked locally (13 CAPTCHA-solver matches in that repository's code search).
- **Statute check:** ss 6D, 20K, 20R, 20T, 21T, 21V and 40 read on AustLII on 24 October 2026.
- **Statute review for `ghostsoap check`** (5 November 2026, by an AI agent, not a lawyer): read the Privacy Act compilation C2026C00227 (APPs 7, 11, 12 and 13; ss 20K, 20R, 20T, 21T, 21V, 35A, 40 and 41), the Spam Act compilation C2016C00614 (Sch 2 cl 6), the GDPR (Articles 3 and 17, EUR-Lex) and California Civil Code s 1798.140 (California Legislative Information). Findings: 'erasure' and 'forgotten' do not appear in the compilation; APP 11.2 is a duty on the entity with no request attached; the 30-day limits in APPs 12.4(a)(i) and 13.5(a)(i) bind agencies, and organisations have 'a reasonable period' (APPs 7.7, 12.4(a)(ii), 13.5(a)(ii)); the fixed periods are those the law table gives (ss 20K(3), 20R(3), 20T(2), 21V(2); Spam Act Sch 2 cl 6); GDPR Article 3 and the CCPA's definitions of 'business' and 'consumer' limit whom each law reaches. Hence `check` flags a sentence that holds a word for erasing and a legal source or a word of duty, the GDPR, California's privacy law, foreign article numbers and any period written as a number and a unit that is not in the law table's or a letter's own words, and it passes that wording whichever business the text addresses. It never says a letter is sound, because word patterns can miss a claim. Not reviewed: Australian laws outside the law table, so `check` says only that the table's levers give no right to erasure, and whether a table deadline fits the lever a text uses it for.
- **Alcuin M1 final review** (24 October 2026, GPT-6 Pro at Extra High tier): verdict 'request changes'. Applied: access levers opt-in only, stricter date, URL and obsolete checks, an empty register rejected, entity names narrowed, every channel matched to the entity's pages. Its finding that agent checks did not meet the `verified` definition was settled by decision 5 in section 12.

## Sources

- [Optery Australia launch](https://www.optery.com/?p=27466), [Optery Australian pricing and list](https://www.optery.com/?p=27451)
- [Incogni pricing](https://incogni.com/pricing), [DeleteMe international](https://joindeleteme.com/international/)
- [ClearTrace AU](https://cleartrace.au/), [Serus Australian guide](https://www.serus.ai/privacy-laws/remove-your-data-australia/) (both read on 5 November 2026)
- [Consumer Reports evaluation of removal services](https://advocacy.consumerreports.org/press_release/consumer-reports-evaluation-of-people-search-site-removal-services-finds-that-they-are-largely-ineffective/)
- [OAIC APP 7 guidelines](https://www.oaic.gov.au/privacy/australian-privacy-principles/australian-privacy-principles-guidelines/chapter-7-app-7-direct-marketing)
- [ACCC data broker report](https://www.accc.gov.au/inquiries-and-consultations/finalised-inquiries/digital-platform-services-inquiry-2020-25/march-2024-interim-report)
- Read on 5 November 2026: [Exposure Draft, Privacy Amendment (Personal Data Protection) Bill 2026](https://consultations.ag.gov.au/rights-and-protections/privacy-reform/user_uploads/exposure-draft-bill-2026.pdf), [consultation paper](https://consultations.ag.gov.au/rights-and-protections/privacy-reform/user_uploads/consultation_paper.pdf), [consultation page](https://consultations.ag.gov.au/rights-and-protections/privacy-reform/), [Attorney-General's press conference, 31 August 2026](https://ministers.ag.gov.au/media-centre/transcripts/press-conference-blue-room-31-08-2026)
- Read on 5 November 2026: [Federal Register API](https://api.prod.legislation.gov.au/v1/versions) for the Privacy Act's versions and their reasons, [Privacy and Other Legislation Amendment Act 2024](https://www.legislation.gov.au/C2024A00128), [Interactive Gambling Amendment (Gambling Reform) Act 2026](https://www.legislation.gov.au/C2026A00072), [Telecommunications (Consumer Protection and Service Standards) Act 1999](https://www.legislation.gov.au/C2025C00623)
- Read on 6 November 2026: [OAIC, Children's Online Privacy Code](https://www.oaic.gov.au/privacy/privacy-registers/privacy-codes/childrens-online-privacy-code), [Federal Register](https://www.legislation.gov.au/), [AFCA, what is an ombudsman](https://www.afca.org.au/about-afca/corporate-information/what-is-an-ombudsman) ('We can't accept complaints about certain insurance products, including private health insurance'), [AFCA, complaints we consider](https://www.afca.org.au/what-to-expect/complaints-we-consider) (its member list), [Commonwealth Ombudsman, private health insurance complaints](https://www.ombudsman.gov.au/complaints/private-health-insurance-complaints). AFCA's Rules themselves refused the request, so this rests on AFCA's own pages
- Read on 5 November 2026: [OAIC, Handling privacy complaints](https://www.oaic.gov.au/news/blog/handling-privacy-complaints-a-new-approach-for-a-new-era), [OAIC recognised EDR schemes register](https://www.oaic.gov.au/privacy/privacy-registers/recognised-external-dispute-resolution-schemes-register), [APP 1.7-1.9 fact sheet](https://www.oaic.gov.au/__data/assets/pdf_file/0021/269013/APP-1.7-1.9-Transparency-Obligation-Fact-Sheet.PDF), [APP 1 guidelines, version 2](https://www.oaic.gov.au/__data/assets/pdf_file/0027/269064/APP-Guidelines-APP-1-Open-and-transparent-management-of-personal-information-September-2026-v2.pdf), [TIO Terms of Reference, from 1 January 2025](https://assets.tio.com.au/files/069470690e522a48de334ab0dab6b566919c61c6.pdf)
- [AEC silent electors](https://www.aec.gov.au/enrolling_to_vote/special_category/silent-electors.htm), [IDCARE credit bans](https://www.idcare.org/fact-sheets/credit-bans-australia)
- [datenanfragen/data](https://github.com/datenanfragen/data), [nulltrace](https://github.com/ide-digital/nulltrace), [symaira-eraseme](https://github.com/danieljustus/symaira-eraseme), [Scrubbr](https://github.com/jamessk22/Scrubbr), [JustDeleteMe](https://github.com/jdm-contrib/jdm), [Data Rights Protocol](https://github.com/datarightsprotocol/data-rights-protocol)
