//! The drafts folder: plaintext letters waiting to be sent (plan section 5).

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The default drafts folder, in the system temporary directory.
pub fn default_dir() -> PathBuf {
    env::temp_dir().join("ghostsoap-drafts")
}

/// Write `text` to `<dir>/<name>`, creating `dir`. Fails rather than overwrite a file.
pub fn write(dir: &Path, name: &str, text: &str) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let path = dir.join(name);
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .and_then(|mut f| io::Write::write_all(&mut f, text.as_bytes()))?;
    Ok(path)
}

/// A file GhostSoap wrote: `<entry-id>-GS-XXXXX.eml` or `.txt`.
fn is_draft(name: &str) -> bool {
    let Some(stem) = name
        .strip_suffix(".eml")
        .or_else(|| name.strip_suffix(".txt"))
    else {
        return false;
    };
    stem.len() > 9
        && stem.is_char_boundary(stem.len() - 9)
        && stem[stem.len() - 9..].starts_with("-GS-")
        && stem[stem.len() - 5..]
            .bytes()
            .all(|b| b.is_ascii_digit() || b.is_ascii_uppercase())
}

/// Delete the drafts in `dir`, and `dir` itself once empty. Other files are left alone.
/// Returns how many drafts were deleted.
pub fn clean(dir: &Path) -> io::Result<usize> {
    let items = match fs::read_dir(dir) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
        other => other?,
    };
    let mut deleted = 0;
    for item in items {
        let item = item?;
        if item.file_type()?.is_file() && is_draft(&item.file_name().to_string_lossy()) {
            fs::remove_file(item.path())?;
            deleted += 1;
        }
    }
    if fs::read_dir(dir)?.next().is_none() {
        fs::remove_dir(dir)?;
    }
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_deletes_only_drafts() {
        let dir = tempfile::tempdir().unwrap();
        let drafts = dir.path().join("drafts");
        write(&drafts, "quantium-au-GS-7K3QX.eml", "x").unwrap();
        write(&drafts, "talefin-au-GS-00AZ9.txt", "x").unwrap();
        write(&drafts, "notes.txt", "keep").unwrap();
        write(&drafts, "x-GS-abcde.txt", "keep").unwrap();
        assert!(
            write(&drafts, "notes.txt", "again").is_err(),
            "no overwrite"
        );
        assert_eq!(clean(&drafts).unwrap(), 2);
        assert!(drafts.join("notes.txt").exists());
        fs::remove_file(drafts.join("notes.txt")).unwrap();
        fs::remove_file(drafts.join("x-GS-abcde.txt")).unwrap();
        write(&drafts, "a-GS-11111.eml", "x").unwrap();
        assert_eq!(clean(&drafts).unwrap(), 1);
        assert!(!drafts.exists(), "an emptied folder is removed");
        assert_eq!(clean(&drafts).unwrap(), 0);
    }
}
