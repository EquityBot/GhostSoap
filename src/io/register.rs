//! Reading register files from disk.

use std::fs;
use std::io;
use std::path::Path;

use crate::core::register::{RegisterFiles, SourceFile};

/// Read the register at `dir`. A missing `packs` directory means no packs.
pub fn load(dir: &Path) -> io::Result<RegisterFiles> {
    let packs = match read_toml_files(&dir.join("packs")) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
        other => other?,
    };
    Ok(RegisterFiles {
        entries: read_toml_files(dir)?,
        packs,
    })
}

fn read_toml_files(dir: &Path) -> io::Result<Vec<SourceFile>> {
    let mut files = Vec::new();
    for item in fs::read_dir(dir)? {
        let path = item?.path();
        if path.is_file() && path.extension().is_some_and(|e| e == "toml") {
            files.push(SourceFile {
                name: path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                text: fs::read_to_string(&path)?,
            });
        }
    }
    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}
