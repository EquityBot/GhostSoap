# GhostSoap

_GhostSoap at no Price._

GhostSoap is a free, local-first tool that helps people in Australia reclaim their privacy. It prepares Privacy Act requests to data brokers for you to send from your own mailbox. Planned milestones add tracking of every deadline and reply, an escalation file when a business does not respond, and a guide to the data hygiene steps email cannot fix.

**Status: milestone M2.** GhostSoap has a sourced [register](registry/README.md) of Australian entities and prepares request letters for you to send yourself. It does not yet track what you sent or the replies (milestone M3). The [plan](docs/plan.md) sets out what comes next.

## Using it

```bash
ghostsoap init                                   # your details, encrypted under a passphrase
ghostsoap plan --pack start-here                 # what would be sent, to whom and why
ghostsoap draft --pack start-here                # write the letters to the drafts folder
ghostsoap clean                                  # delete the drafts once sent
ghostsoap law                                    # each request's legal basis and deadline
```

`plan` and `draft` take `--pack` or `--entry` (repeatable). Without `--lever`, they ask each business to stop direct marketing, to stop passing your details to other marketers and to say where it got them (APP 7). Other requests are named with `--lever` for one business at a time, and those that need your own words take `--detail`, for example `ghostsoap draft --entry equifax-au --lever s20t-correction --detail "The default listed for 2024 is not mine."`. Each letter includes only the details that business needs. Overseas people-search sites get no letter: GhostSoap has not established that Australian privacy law reaches them (it can reach an overseas business that carries on business in Australia, under section 5B), so it shows their own search page and opt-out form instead.

Drafts are `.eml` files your mail program opens ready to send, or text files to post or paste into a web form; `draft --copy` prints them for webmail instead. They are written to a `drafts` folder beside your profile (or the folder `--out` names) and stay plaintext until you run `ghostsoap clean`, and your mailbox's Sent folder keeps copies. Each draft is dated the day you made it, so check the date if you send it later, and text you give with `--detail` goes into the letter as you wrote it. `draft --mailto` also prints a ready-to-send link for a letter short enough for one; the link holds the whole letter, so it stays in your terminal's history.

## Safety

- Your profile is encrypted with [age](https://age-encryption.org/) under your passphrase and kept in your user data folder (`%LOCALAPPDATA%\GhostSoap` on Windows). A lost passphrase cannot be recovered.
- Requests for access return personal information. Make them only about yourself: requesting someone else's information may be an offence. GhostSoap never puts them in a pack.
- If someone with access to your device may be watching what you do, read eSafety's guidance on [device safety](https://www.esafety.gov.au/) before you start.

## What leaves your machine

Nothing. GhostSoap makes no network connections; you send each letter yourself.

## What it will do

- Send requests under the Australian Privacy Principles and the credit reporting rules in Part IIIA of the Privacy Act: stop direct marketing, tell me where you got my information, give me access, correct it.
- Keep a small register of Australian data brokers, each entry sourced and dated.
- Track deadlines and replies in an encrypted file on your computer, and export an escalation pack.
- Guide wider data hygiene: breach checks, credit bans, the Do Not Call Register, silent numbers and more.

## What it will not do

- Act for you. You review and send every request yourself.
- Collect anything. There is no server, account, telemetry or analytics.
- Scrape people-search sites, solve CAPTCHAs or fill in web forms automatically.
- Promise deletion. Australian law has no general right to erasure; GhostSoap tells you what each request can and cannot achieve.

GhostSoap provides information and templates, not legal advice. For advice, contact a community legal centre or the [Office of the Australian Information Commissioner](https://www.oaic.gov.au/).

## Building

GhostSoap is written in Rust; `rust-toolchain.toml` pins the compiler.

```bash
cargo build --release
```

## Licences

- Code: [Apache-2.0](LICENSE).
- Register and checklist data (`registry/` and, from milestone M4, `checklist/`): [CC0-1.0](LICENSE-DATA), so other privacy projects can reuse it.

GhostSoap is not affiliated with, endorsed by or connected to Activision or the Call of Duty franchise. The name is a nod to it, nothing more.
