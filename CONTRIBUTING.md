# Contributing

## Clean-room rules

- Do not copy code, letter text or data from projects without a licence (for example digisamroc/eraser and its forks) or under NonCommercial terms (for example the Big Ass Data Broker Opt-Out List and the Optery directory). They may point you to a broker; the entry must then be verified from the broker's own pages.
- Material under MIT, Apache-2.0 or CC0 may be reused with its notice; list it in [docs/provenance.md](docs/provenance.md).
- Write letters from the Privacy Act and the OAIC guidelines, citing the provision.

## Register entries

Each entry needs at least one source URL from the entity itself, an ABN checked on ABN Lookup and the date you checked it; add a row to [docs/provenance.md](docs/provenance.md) saying where the lead came from and what verified it. [registry/README.md](registry/README.md) lists the fields, and `cargo run -- registry check --path registry/au` validates them. Code contributions are accepted under FSL-1.1-ALv2; register and checklist contributions are released under CC0-1.0.

## Checks

Run these before opening a pull request:

```bash
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked --release
cargo run -- registry check --path registry/au
```

New functionality and bug fixes come with automated tests, in `tests/` or a `#[cfg(test)]` module beside the code, and CI runs them on Linux and Windows.
