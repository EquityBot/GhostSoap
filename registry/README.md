# Register

One TOML file per Australian entity in `au/`, named after its `id`, and packs in `au/packs/`. The data is dedicated to the public domain under [CC0-1.0](../LICENSE-DATA). Where each entry's lead came from and what verified it is recorded in [docs/provenance.md](../docs/provenance.md).

The register is built into the `ghostsoap` binary at compile time (`build.rs`), so an installed copy needs no register files. Check a working copy before opening a pull request:

```bash
cargo run -- registry check --path registry/au
```

Without `--path`, `registry check` validates the register built into the binary.

## Entry fields

| Field | Values |
|---|---|
| `id` | Permanent lowercase slug, equal to the file name; never reused. A retired entry stays with an `[obsolete]` table (`reason`, optional `replaced_by`) |
| `name`, `legal_entity` | Trading name; legal entity as in its privacy policy or ABN Lookup |
| `abn` | `NN NNN NNN NNN`, checked against ABN Lookup; the check digit is validated |
| `kinds` | `credit-reporting-body`, `property-data`, `people-search`, `marketing-list`, `ad-tech`, `directory`, `loyalty` |
| `priority` | `crucial`, `high`, `standard` |
| `quality` | `tested` (a GhostSoap request got a reply), `verified` (contact checked against the entity's own privacy policy or contact pages, by a person or an AI agent, with the evidence recorded in provenance), `imported` (licence-compatible dataset), `lead` (not yet checked). Only `tested` and `verified` entries may enter packs |
| `verified_at` | TOML date of the last check; `registry check` warns after 12 months and rejects a date more than a day ahead |
| `sources` | At least one https URL from the entity itself |
| `levers` | The requests this entity is eligible for; listing a lever does not mean it is always sent (see below). `app7-no-marketing`, `app7-no-facilitation`, `app7-source`, `app11-destroy`, `app13`, `app13-notify-recipients`, `app13-statement`, `s20r-access`, `s20t-correction`, `s20k-ban`, `s21t-access`, `s21v-correction`, `spam-unsubscribe` |
| `opt_in_levers` | Levers the user must choose per entity. The access levers `app12`, `s20r-access` and `s21t-access` return personal information, so they are allowed only here |
| `identity_fields` | `full_name`, `alias_names`, `email`, `phone`, `date_of_birth`, `address`, `previous_addresses`, `postcode`. Document numbers are never collected |
| `exemption_risk` | `none`, `small-business-possible`, `overseas` |
| `escalation` | Ordered: `afca`, `acma`, `oaic` |
| `effort` | `easy`, `medium`, `hard`, `impossible` |
| `[[channels]]` | `kind` (`email`, `web-form`, `post`, `phone`), `address`, `levers` (the entry's levers this channel can carry; `[]` for a contact-only channel such as a phone line) and optional `preferred = true`. Every lever in `levers` and `opt_in_levers` needs at least one channel; a channel may list only the entry's levers; at most one channel is preferred for each lever |
| `[manual_check]` | `url` of the entity's own search or opt-out page; GhostSoap only opens it |
| `[[evidence]]` | Optional, one per tracked source page: `url` (one of `sources`), `checked_at` (date) and `sha256` (64 lowercase hex digits; see below) |

## Packs

A pack has `id` (equal to the file name), `name`, `description` and `entries`, a list of entry ids at `tested` or `verified` quality. Packs choose entities, not levers. When the user plans a round from a pack, the planner offers each entry's `levers` and checks each lever's prerequisites: a correction (`app13`, `s20t-correction`) needs the user to say what is wrong, `app13-notify-recipients` follows a correction and `app13-statement` follows a refusal to correct. Pack membership never selects an `opt_in_levers` lever; the user chooses those per entity.

## Evidence hashes and drift

Each `[[evidence]]` table records a source page's state when the entry was verified. A scheduled workflow (`.github/workflows/evidence-drift.yml`) re-fetches every evidence page on the first of each month and opens or updates one issue, 'Register evidence drift', listing pages that changed or could not be read. It never commits. When a page changes, recheck the entry against it and update the hash and `checked_at`.

`cargo run -p xtask -- evidence-hash <url>` prints a page's hash, computed as follows:

1. GET the URL, following redirects, with the user agent `GhostSoap-evidence-check/0.1`.
2. If the response's `Content-Type` does not contain `html` (for example a PDF), hash the response bytes as they are.
3. Otherwise, decode the body as UTF-8 (invalid bytes replaced) and take the content of the first `<main>` element, or else `<body>`, or else the whole document.
4. Remove comments and the content of `script`, `style`, `noscript`, `template`, `svg` and `iframe` elements, and replace every tag with a space.
5. Decode `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`, `&nbsp;` and numeric character references; leave any other `&` as it is.
6. Collapse every run of whitespace to one space and trim both ends.
7. If fewer than 500 characters remain, the page is a bot check or needs a browser: record no evidence for it.
8. Hash the text's UTF-8 bytes with SHA-256 and write the digest as lowercase hex.

Record evidence only for a page that gives the same hash on two fetches and a hash no other source shares (several URLs returning one bot-check page share a hash).
