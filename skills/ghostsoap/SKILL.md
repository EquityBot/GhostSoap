---
name: ghostsoap
description: Help someone in Australia choose Privacy Act requests to data brokers, recruiters and other businesses with GhostSoap, and explain each request's legal basis and deadline. Use when they ask to opt out of data brokers, stop marketing, or find out who holds their details. The person keeps their own details and sends every request.
---

# GhostSoap

GhostSoap prepares Australian Privacy Act requests for a person to send from their own mailbox. Your job is to help them decide what to send and to explain it. It provides information and templates, not legal advice; say so when you explain a right or deadline.

## What you run

```bash
ghostsoap plan --pack start-here --json   # what would be sent, to whom and why
ghostsoap plan --entry quantium-au --json # one business
ghostsoap law --json                      # each request's legal basis, deadline and charge
ghostsoap registry list --json            # the packs and businesses to choose from
```

These commands need no profile and no passphrase. Errors go to standard error with a non-zero exit code. `plan` steps have an `action`:

- `letter`: GhostSoap can draft it. `details_used` names the identity fields that business would receive; a letter also holds any `--detail` text the person adds.
- `manual`: run from overseas, with no letter. Show the person `note`, which says Australian law may not reach it, and give them `page` and `forms`; they use them themselves.
- `skip`: nothing to send; give the `reason`.

Show every item in `warnings`. Without the binary, the same data is in `registry/au/` in the [repository](https://github.com/EquityBot/GhostSoap).

## What the person does themselves

Give them these commands to run in their own terminal, on their own computer. Do not run them.

```bash
ghostsoap init                     # creates their encrypted profile
ghostsoap draft --pack start-here  # writes the letters
ghostsoap clean                    # deletes the drafts once sent
```

If you can read their files, tell them so before they run `draft`: these rules are guidance and do not stop you reading a draft. `ghostsoap draft --copy` prints the letters to their terminal and writes no draft files.

## Rules

- Never ask for, accept or store the GhostSoap passphrase, and never use `--passphrase-stdin`.
- Never ask for their name, address, date of birth or other identity details, and do not open the drafts folder. If they paste such details, do not repeat or store them.
- Do not write `--detail` text for them or run a command containing it: it describes their personal circumstances. Explain what the letter needs and let them add it when they run `ghostsoap draft`.
- Do not send letters, fill in web forms, solve CAPTCHAs or scrape people-search sites.
- Make access requests only about the person themselves: requesting someone else's information may be an offence.
- Do not promise deletion. Australian law has no general right to erasure.

## On a cloud computer

If you run on a hosted computer rather than the person's own, do not create a profile or draft letters there, even if they ask: their details would sit on that computer. If `ghostsoap` is not installed and you cannot build it with `cargo build --release`, read the entries in `registry/au/` in the repository and help them choose, then give them the commands above to run at home.
