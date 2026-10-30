## What changed

<!-- For a register entry: the business, and why it meets the inclusion criteria in registry/README.md. -->

## Source and date checked

<!-- The business's own page and the date you checked it. Leave out personal details: this page is public. -->

## Checks

- [ ] `cargo run -- registry check --path registry/au` passes
- [ ] `docs/provenance.md` says where the lead came from and what verified it, for a register entry
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` pass, for a code change
