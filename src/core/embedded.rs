//! The shipped register, embedded at compile time by `build.rs`, so an installed binary
//! (or a later WebAssembly build) needs no register files on disk.

use crate::core::register::{RegisterFiles, SourceFile};

mod data {
    include!(concat!(env!("OUT_DIR"), "/register.rs"));
}

fn files(table: &[(&str, &str)]) -> Vec<SourceFile> {
    table
        .iter()
        .map(|(name, text)| SourceFile {
            name: (*name).to_owned(),
            text: (*text).to_owned(),
        })
        .collect()
}

/// The register built into this binary.
pub fn register() -> RegisterFiles {
    RegisterFiles {
        entries: files(data::ENTRIES),
        packs: files(data::PACKS),
    }
}
