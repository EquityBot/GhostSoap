//! Embeds the register in the binary: generates `include_str!` calls for every entry in
//! `registry/au` and every pack in `registry/au/packs`, sorted by file name.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn toml_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for item in fs::read_dir(dir)? {
        let path = item?.path();
        if path.is_file() && path.extension().is_some_and(|e| e == "toml") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn table(name: &str, files: &[PathBuf]) -> String {
    let mut out = format!("pub static {name}: &[(&str, &str)] = &[\n");
    for path in files {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
        out += &format!(
            "    ({file_name:?}, include_str!({:?})),\n",
            path.display().to_string()
        );
    }
    out + "];\n"
}

fn main() -> io::Result<()> {
    let register =
        Path::new(&env::var("CARGO_MANIFEST_DIR").expect("set by Cargo")).join("registry/au");
    println!("cargo::rerun-if-changed=registry/au");
    let code = table("ENTRIES", &toml_files(&register)?)
        + &table("PACKS", &toml_files(&register.join("packs"))?);
    fs::write(
        Path::new(&env::var("OUT_DIR").expect("set by Cargo")).join("register.rs"),
        code,
    )
}
