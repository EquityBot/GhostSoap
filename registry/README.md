# Register

One TOML file per Australian entity in `au/`, named after its `id`, and packs in `au/packs/`. The data is dedicated to the public domain under [CC0-1.0](../LICENSE-DATA). Where each entry's lead came from and what verified it is recorded in [docs/provenance.md](../docs/provenance.md).

The register is built into the `ghostsoap` binary at compile time (`build.rs`), so an installed copy needs no register files. Check a working copy before opening a pull request:

```bash
cargo run -- registry check --path registry/au
```

Without `--path`, `registry check` validates the register built into the binary.

## What belongs here

An entity belongs in the register when all of these hold:

- It holds personal information about many people in Australia, and it fits one of the `kinds` below. That covers data brokers and people-search sites, recruiters, firms behind large Australian data breaches and large consumer firms.
- People can reach it: its own pages give a privacy contact or an opt-out.
- It has an ABN, or it is an overseas entry as `exemption_risk` defines one. An entity that may fall under the small business exemption can stay, marked `small-business-possible`.
- Its details come from its own pages. Another list may point you to it, but nothing is copied from a list whose licence does not allow it ([clean-room rules](../CONTRIBUTING.md#clean-room-rules)).

Individuals do not belong, and no entry names a person as its contact. A lead you cannot yet verify is worth an issue, not an entry. To propose an entity, use the [new entry form](https://github.com/EquityBot/GhostSoap/issues/new?template=new-entry.yml), and leave your own details out: never include a reply you received, unless everything about you is removed.

## Entry fields

| Field | Values |
|---|---|
| `id` | Permanent lowercase slug, equal to the file name; never reused. A retired entry stays with an `[obsolete]` table: `reason` is `acquired`, `merged` or `duplicate` (each needs `replaced_by`, the entry that now handles requests), `closed` or `out-of-scope` |
| `name`, `legal_entity` | Trading name; legal entity as in its privacy policy or ABN Lookup |
| `abn` | `NN NNN NNN NNN`, checked against ABN Lookup; the check digit is validated. Omitted on overseas entries, and only there |
| `kinds` | `credit-reporting-body`, `property-data`, `people-search`, `marketing-list`, `ad-tech`, `directory`, `loyalty`, `recruitment` (agencies and job boards), `telecommunications`, `financial-services`, `insurance`, `retail`, `online-service` |
| `priority` | `crucial`, `high`, `standard` |
| `quality` | `tested` (a GhostSoap request got a reply), `verified` (contact checked against the entity's own privacy policy or contact pages, by a person or an AI agent, with the evidence recorded in provenance), `imported` (licence-compatible dataset), `lead` (not yet checked). Only `tested` and `verified` entries may enter packs |
| `verified_at` | TOML date of the last check; `registry check` warns after 12 months and rejects a date more than a day ahead |
| `sources` | At least one https URL from the entity itself |
| `levers` | The requests this entity is eligible for; listing a lever does not mean it is always sent (see below). `app7-no-marketing`, `app7-no-facilitation`, `app7-source`, `app11-destroy`, `app13`, `app13-notify-recipients`, `app13-statement`, `s20t-correction`, `s20k-ban`, `s21v-correction`, `spam-unsubscribe` |
| `opt_in_levers` | Levers the user must choose per entity. The access levers `app12`, `s20r-access` and `s21t-access` return personal information, so they are allowed only here |
| `identity_fields` | `full_name`, `alias_names`, `email`, `phone`, `date_of_birth`, `address`, `previous_addresses`, `postcode`. Document numbers are never collected |
| `exemption_risk` | `none`, `small-business-possible`, `overseas`. An overseas entry is a site whose responsible entity is overseas with no ABN found (a people-search site that lists Australians but has no Australian presence, or a service whose privacy policy names an overseas company, such as Indeed and LinkedIn; provisional, since section 5B can reach an overseas organisation that carries on business in Australia whether or not it has an ABN, and GhostSoap has not assessed that): it has no `abn`, no levers, a `[manual_check]` page and its own web-form opt-out as a channel with `levers = []` |
| `escalation` | Ordered: `afca`, `acma`, `oaic` |
| `effort` | `easy`, `medium`, `hard`, `impossible` |
| `[[channels]]` | `kind` (`email`, `web-form`, `post`, `phone`), `address`, `levers` (the entry's levers this channel can carry; `[]` for a contact-only channel such as a phone line) and optional `preferred = true`. Every lever in `levers` and `opt_in_levers` needs at least one channel; a channel may list only the entry's levers; at most one channel is preferred for each lever |
| `[manual_check]` | `url` of the entity's own search or opt-out page; GhostSoap only opens it |
| `[[evidence]]` | Optional, one per tracked source page: `url` (one of `sources`), `checked_at` (date) and `sha256` (64 lowercase hex digits; see below) |

## Packs

A pack has `id` (equal to the file name), `name`, `description` and `entries`, a list of entry ids at `tested` or `verified` quality. Packs choose entities, not levers. When the user plans a round from a pack, the planner offers each entry's `levers` and checks each lever's prerequisites: a correction (`app13`, `s20t-correction`) needs the user to say what is wrong, `app13-notify-recipients` follows a correction and `app13-statement` follows a refusal to correct. Pack membership never selects an `opt_in_levers` lever; the user chooses those per entity.

## Evidence hashes and drift

Each `[[evidence]]` table records a source page's state when the entry was verified. A scheduled workflow (`.github/workflows/evidence-drift.yml`) re-fetches every evidence page on the first of each month and opens or updates one issue, 'Register evidence drift', listing pages that changed or could not be read within 30 seconds, and closes that issue once every page matches again. It never commits. When a page changes, recheck the entry against it and update the hash and `checked_at`.

The same run checks the contact channels of every entry that is not obsolete. It looks up each email address's domain through Google Public DNS (`dns.google`) and reports one that does not exist, has no mail or address record, or publishes a null MX record saying it accepts no mail. It also requests each web-form address and reports an HTTP error or a host public DNS says does not exist. A 401, 403, 429 or 5xx, a dropped connection or a failed DNS lookup proves nothing, because sites also answer scripts that way when the form works and such failures pass, so those go to the workflow log and not the issue: a form or domain behind one of them is not confirmed to work. These checks never send an email or submit a form, and a passing domain does not prove the mailbox exists.

`cargo run -p xtask -- evidence-hash <url>` prints a page's hash, computed as follows:

1. GET the URL, following redirects, with the user agent `GhostSoap-evidence-check/0.1 (+https://github.com/EquityBot/GhostSoap)` and a 30-second limit for the whole request.
2. The response is HTML if its `Content-Type` contains `html` in any letter case or, when it has no `Content-Type`, if its body starts (after any byte order mark and whitespace) with `<!doctype html` or `<html` in any letter case. Hash any other response (for example a PDF) as its raw bytes.
3. Decode an HTML body as UTF-8 (invalid bytes replaced). Ignoring comments and the content of `script`, `style`, `noscript`, `template`, `svg` and `iframe` elements, take the content of the first `<main>` element (including any `<main>` nested in it), or else `<body>`, or else the whole document. Markup starts with `<` followed by an ASCII letter, `/` and an ASCII letter, `!` or `?`; any other `<` is text. Inside `template` and `svg`, CDATA sections are skipped too. A `>` inside a quoted attribute value does not end a tag. `script`, `style`, `noscript` and `iframe` end at their first end tag (so `</scripture>` does not end a script); `template` and `svg` nest, and `<svg/>` closes itself.
4. Replace every tag in that content with a space.
5. Decode `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`, `&nbsp;` and numeric character references (`&#` and decimal digits, or `&#x` or `&#X` and hex digits, naming a valid character), each only when its `;` is at most 10 bytes after the `&`; leave any other `&` as it is.
6. Collapse every run of Unicode whitespace (the `White_Space` property, as Rust's `split_whitespace` uses) to one space and trim both ends.
7. If fewer than 500 characters remain, the page is a bot check or needs a browser: record no evidence for it.
8. Hash the text's UTF-8 bytes with SHA-256 and write the digest as lowercase hex.

Record evidence only for a page that gives the same hash on two fetches and a hash no other source shares (several URLs returning one bot-check page share a hash).
