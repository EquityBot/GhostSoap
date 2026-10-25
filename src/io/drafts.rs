//! The drafts folder: plaintext letters waiting to be sent (plan section 5).

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::io::state::create_private_dir;

/// Lists the drafts GhostSoap wrote in a folder, one name per line, so `clean` deletes those
/// and nothing else.
const MANIFEST: &str = ".ghostsoap-drafts";

/// The default drafts folder: `drafts` in the data directory, which is private to the user on
/// every platform, unlike a shared temporary folder such as Linux's `/tmp`.
pub fn default_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("drafts")
}

/// Write `text` to `<dir>/<name>`, creating `dir`. The draft is written to a temporary file
/// and renamed into place once complete, so a draft name never holds a partial letter. Fails
/// rather than overwrite a file, and on Unix refuses a folder others can read.
pub fn write(dir: &Path, name: &str, text: &str) -> io::Result<PathBuf> {
    if !is_draft(name) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("`{name}` is not a draft name"),
        ));
    }
    create_private_dir(dir)?;
    check_private(dir)?;
    let path = dir.join(name);
    if path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already exists", path.display()),
        ));
    }
    // Recorded first, so `clean` also finds a draft whose write stopped part-way.
    let mut manifest =
        private_file(OpenOptions::new().append(true).create(true)).open(dir.join(MANIFEST))?;
    writeln!(manifest, "{name}")?;
    let tmp = temp_name(dir, name);
    let written = private_file(OpenOptions::new().write(true).create_new(true))
        .open(&tmp)
        .and_then(|mut f| {
            f.write_all(text.as_bytes())?;
            f.sync_all()
        });
    if let Err(e) = written {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    // shortcut: the existence check above and this rename are not one atomic step; only
    // another GhostSoap drawing the same random tag for the same entry in the same instant
    // could slip between them. M3's request store can rule that out by checking tags.
    fs::rename(&tmp, &path)?;
    Ok(path)
}

/// Open options for a file only the user can read (mode 600 on Unix).
fn private_file(options: &mut OpenOptions) -> &mut OpenOptions {
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(options, 0o600);
    options
}

fn temp_name(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!(".{name}.tmp"))
}

/// On Unix, `dir` must be a real folder (not a link) that only its owner can read. Windows
/// folders keep the access list they inherit; the default drafts folder sits in the user's
/// own `%LOCALAPPDATA%`.
#[cfg(unix)]
fn check_private(dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let meta = fs::symlink_metadata(dir)?;
    if meta.is_dir() && meta.permissions().mode() & 0o077 == 0 {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!(
            "{} must be a folder only you can open (chmod 700), not a link",
            dir.display()
        ),
    ))
}

#[cfg(not(unix))]
fn check_private(_dir: &Path) -> io::Result<()> {
    Ok(())
}

/// A name GhostSoap gives a draft: `<entry-id>-GS-XXXXX.eml` or `.txt`, with no folder part.
fn is_draft(name: &str) -> bool {
    let Some(stem) = name
        .strip_suffix(".eml")
        .or_else(|| name.strip_suffix(".txt"))
    else {
        return false;
    };
    !name.contains(['/', '\\'])
        && !name.starts_with('.')
        && stem.len() > 9
        && stem.is_char_boundary(stem.len() - 9)
        && stem[stem.len() - 9..].starts_with("-GS-")
        && stem[stem.len() - 5..]
            .bytes()
            .all(|b| b.is_ascii_digit() || b.is_ascii_uppercase())
}

/// Delete the drafts GhostSoap wrote in `dir`, as its manifest lists them, and the manifest.
/// Other files are left alone. With `remove_dir`, `dir` is also removed once empty; pass it
/// only for GhostSoap's own drafts folder. Returns how many drafts were deleted.
pub fn clean(dir: &Path, remove_dir: bool) -> io::Result<usize> {
    let manifest = dir.join(MANIFEST);
    let names = match fs::read_to_string(&manifest) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
        other => other?,
    };
    let mut deleted = 0;
    for name in names.lines().filter(|n| is_draft(n)) {
        let _ = fs::remove_file(temp_name(dir, name));
        match fs::remove_file(dir.join(name)) {
            Ok(()) => deleted += 1,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    fs::remove_file(&manifest)?;
    if remove_dir && fs::read_dir(dir)?.next().is_none() {
        fs::remove_dir(dir)?;
    }
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_deletes_only_drafts_it_wrote() {
        let dir = tempfile::tempdir().unwrap();
        let drafts = dir.path().join("drafts");
        write(&drafts, "quantium-au-GS-7K3QX.eml", "x").unwrap();
        write(&drafts, "talefin-au-GS-00AZ9.txt", "x").unwrap();
        fs::write(drafts.join("notes.txt"), "keep").unwrap();
        fs::write(
            drafts.join("notes-GS-ABCDE.txt"),
            "keep: named like a draft",
        )
        .unwrap();
        assert!(
            write(&drafts, "notes.txt", "x").is_err(),
            "not a draft name"
        );
        assert!(
            write(&drafts, "../x-GS-ABCDE.txt", "x").is_err(),
            "no folder part"
        );
        assert!(
            write(&drafts, "talefin-au-GS-00AZ9.txt", "again").is_err(),
            "no overwrite"
        );
        assert_eq!(clean(&drafts, true).unwrap(), 2);
        assert!(drafts.join("notes.txt").exists());
        assert!(drafts.join("notes-GS-ABCDE.txt").exists());
        assert!(!drafts.join(MANIFEST).exists());
        assert_eq!(
            clean(&drafts, true).unwrap(),
            0,
            "no manifest, nothing to delete"
        );
    }

    #[test]
    fn clean_removes_the_folder_only_when_asked() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("mine");
        write(&out, "a-GS-11111.eml", "x").unwrap();
        assert_eq!(clean(&out, false).unwrap(), 1);
        assert!(out.exists(), "a folder the user named stays");
        write(&out, "a-GS-11111.eml", "x").unwrap();
        assert_eq!(clean(&out, true).unwrap(), 1);
        assert!(!out.exists(), "GhostSoap's own emptied folder is removed");
    }

    #[test]
    fn a_draft_is_complete_or_absent_and_clean_removes_leftovers() {
        let dir = tempfile::tempdir().unwrap();
        let drafts = dir.path().join("drafts");
        let path = write(&drafts, "a-GS-11111.eml", "whole letter").unwrap();
        assert_eq!(fs::read_to_string(path).unwrap(), "whole letter");
        assert!(!temp_name(&drafts, "a-GS-11111.eml").exists());
        // A write that stopped before its rename leaves only a listed temporary file.
        fs::write(drafts.join(MANIFEST), "b-GS-22222.eml\n").unwrap();
        fs::write(temp_name(&drafts, "b-GS-22222.eml"), "part").unwrap();
        assert!(!drafts.join("b-GS-22222.eml").exists());
        assert_eq!(clean(&drafts, true).unwrap(), 0);
        assert!(!temp_name(&drafts, "b-GS-22222.eml").exists());
    }

    #[cfg(unix)]
    #[test]
    fn drafts_are_private_on_unix() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let drafts = dir.path().join("drafts");
        let path = write(&drafts, "a-GS-11111.eml", "x").unwrap();
        let mode = |p: &Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&drafts), 0o700);
        assert_eq!(mode(&path), 0o600);
        assert_eq!(mode(&drafts.join(MANIFEST)), 0o600);
        let open = dir.path().join("open");
        fs::create_dir(&open).unwrap();
        fs::set_permissions(&open, fs::Permissions::from_mode(0o755)).unwrap();
        let err = write(&open, "a-GS-11111.eml", "x").unwrap_err();
        assert!(err.to_string().contains("chmod 700"), "{err}");
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&drafts, &link).unwrap();
        assert!(
            write(&link, "b-GS-22222.eml", "x").is_err(),
            "a link is refused"
        );
    }
}
