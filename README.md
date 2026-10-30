<p align="center">
  <picture>
    <source media="(max-width: 600px) and (prefers-color-scheme: light)" srcset=".github/assets/banner-phone-light.svg">
    <source media="(max-width: 600px)" srcset=".github/assets/banner-phone.svg">
    <source media="(prefers-color-scheme: light)" srcset=".github/assets/banner-light.svg">
    <img src=".github/assets/banner.svg" width="100%" alt="GhostSoap. Privacy Act requests to data brokers, sent by you. Pixel art of a startled ghost with soap bubbles beside the GhostSoap wordmark in slab-serif block letters.">
  </picture>
</p>

<div align="center">

[![tests](https://github.com/EquityBot/GhostSoap/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/EquityBot/GhostSoap/actions/workflows/ci.yml) [![CodeQL](https://github.com/EquityBot/GhostSoap/actions/workflows/codeql.yml/badge.svg?branch=main)](https://github.com/EquityBot/GhostSoap/actions/workflows/codeql.yml)<br>
[![OpenSSF Scorecard](https://raw.githubusercontent.com/ryanduguid/.github/badges/EquityBot/GhostSoap.svg)](https://scorecard.dev/viewer/?uri=github.com/EquityBot/GhostSoap)<br>
[![rust: 1.99+](.github/assets/badges/rust.svg)](https://www.rust-lang.org/) [![licence: FSL-1.1-ALv2](.github/assets/badges/licence.svg)](#licences)

</div>

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

Drafts are `.eml` files your mail program opens ready to send, or text files to post or paste into a web form; `draft --copy` prints them for webmail instead. They are written to a `drafts` folder beside your profile (or the folder `--out` names) and stay plaintext until you run `ghostsoap clean`, and your mailbox's Sent folder keeps copies. `draft` warns, without stopping, when it finds the drafts folder inside a Git working tree, where the letters could be committed, or inside a OneDrive, Dropbox or iCloud Drive folder, where they could be uploaded. It looks only in the usual places, so a folder it does not warn about may still be committed, synced or shared. Each draft is dated the day you made it, so check the date if you send it later, and text you give with `--detail` goes into the letter as you wrote it. `draft --mailto` also prints a ready-to-send link for a letter short enough for one; the link holds the whole letter, so it stays in your terminal's history.

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

- Code: licensed under the Functional Source License, Version 1.1, ALv2 Future License (FSL-1.1-ALv2). You may use, modify and redistribute it for any purpose except a Competing Use as defined in LICENSE (making it available to others in a competing commercial product or service), and each version becomes available under Apache-2.0 two years after it is made available. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
- Register and checklist data (`registry/` and, from milestone M4, `checklist/`): [CC0-1.0](LICENSE-DATA), so other privacy projects can reuse it.

GhostSoap is not affiliated with, endorsed by or connected to Activision or the Call of Duty franchise. The name is a nod to it, nothing more.

---

<p align="center"><sub>Your details, your mailbox, your call.<br>Made by <a href="https://github.com/EquityBot">EquityBot</a>. <i>Ad gloriam eius, non meam.</i></sub></p>
