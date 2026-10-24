# Register

One TOML file per Australian entity in `au/`, named after its `id`, and packs in `au/packs/`. The data is dedicated to the public domain under [CC0-1.0](../LICENSE-DATA). Where each entry's lead came from and what verified it is recorded in [docs/provenance.md](../docs/provenance.md).

Check the register before opening a pull request:

```bash
cargo run -- registry check
```

## Entry fields

| Field | Values |
|---|---|
| `id` | Permanent lowercase slug, equal to the file name; never reused. A retired entry stays with an `[obsolete]` table (`reason`, optional `replaced_by`) |
| `name`, `legal_entity` | Trading name; legal entity as in its privacy policy or ABN Lookup |
| `abn` | `NN NNN NNN NNN`, checked against ABN Lookup; the check digit is validated |
| `kinds` | `credit-reporting-body`, `property-data`, `people-search`, `marketing-list`, `ad-tech`, `directory`, `loyalty` |
| `priority` | `crucial`, `high`, `standard` |
| `quality` | `tested` (a GhostSoap request got a reply), `verified` (contact checked against the entity's own privacy policy), `imported` (licence-compatible dataset), `lead` (not yet checked). Only `tested` and `verified` entries may enter packs |
| `verified_at` | TOML date of the last check; `registry check` warns after 12 months and rejects a date more than a day ahead |
| `sources` | At least one https URL from the entity itself |
| `levers` | The requests this entity is eligible for; listing a lever does not mean it is always sent (see below). `app7-no-marketing`, `app7-no-facilitation`, `app7-source`, `app11-destroy`, `app13`, `app13-notify-recipients`, `app13-statement`, `s20r-access`, `s20t-correction`, `s20k-ban`, `s21t-access`, `s21v-correction`, `spam-unsubscribe` |
| `opt_in_levers` | Levers the user must choose per entity. The access levers `app12`, `s20r-access` and `s21t-access` return personal information, so they are allowed only here |
| `identity_fields` | `full_name`, `alias_names`, `email`, `phone`, `date_of_birth`, `address`, `previous_addresses`, `postcode`. Document numbers are never collected |
| `exemption_risk` | `none`, `small-business-possible`, `overseas` |
| `escalation` | Ordered: `afca`, `acma`, `oaic` |
| `effort` | `easy`, `medium`, `hard`, `impossible` |
| `[[channels]]` | `kind` (`email`, `web-form`, `post`, `phone`) and `address` |
| `[manual_check]` | `url` of the entity's own search or opt-out page; GhostSoap only opens it |

## Packs

A pack has `id` (equal to the file name), `name`, `description` and `entries`, a list of entry ids at `tested` or `verified` quality. Packs choose entities, not levers. When the user plans a round from a pack, the planner offers each entry's `levers` and checks each lever's prerequisites: a correction (`app13`, `s20t-correction`) needs the user to say what is wrong, `app13-notify-recipients` follows a correction and `app13-statement` follows a refusal to correct. Pack membership never selects an `opt_in_levers` lever; the user chooses those per entity.
