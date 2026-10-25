//! The drafts folder: plaintext letters waiting to be sent (plan section 5).

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::io::state::create_private_dir;

/// Lists the drafts GhostSoap wrote in a folder, one `<sha256> <name>` line each, so `clean`
/// deletes a listed file only while it still holds the bytes GhostSoap wrote.
const MANIFEST: &str = ".ghostsoap-drafts";

/// What `clean` did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Cleaned {
    /// Drafts deleted.
    pub deleted: usize,
    /// Listed names whose file no longer holds what GhostSoap wrote, left alone.
    pub kept: Vec<String>,
    /// Temporary files from a write that stopped part-way, left for the user to check.
    pub leftovers: Vec<String>,
}

/// The default drafts folder: `drafts` in the data directory, which is private to the user on
/// every platform, unlike a shared temporary folder such as Linux's `/tmp`.
pub fn default_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("drafts")
}

/// Write `text` to `<dir>/<name>`, creating `dir`. The draft is written to a temporary file of
/// this process's own and linked into place once complete, so a draft name never holds a
/// partial letter and an existing file is never replaced. On Unix, refuses a folder others
/// can open.
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
    // Recorded first, with the content's hash, so `clean` finds a draft even if a crash
    // follows its publication, and never deletes a file holding other bytes.
    let mut manifest =
        private_file(OpenOptions::new().append(true).create(true)).open(dir.join(MANIFEST))?;
    writeln!(manifest, "{} {name}", sha256(text.as_bytes()))?;
    manifest.sync_all()?;
    let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    let mut file = private_file(OpenOptions::new().write(true).create_new(true)).open(&tmp)?;
    // From here the temporary file is this call's own, so it may remove it.
    let published = file
        .write_all(text.as_bytes())
        .and_then(|()| file.sync_all())
        .and_then(|()| fs::hard_link(&tmp, &path));
    drop(file);
    let _ = fs::remove_file(&tmp);
    match published {
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Err(io::Error::new(
            e.kind(),
            format!("{} already exists", path.display()),
        )),
        other => other.map(|()| path),
    }
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Open options for a file only the user can read (mode 600 on Unix).
fn private_file(options: &mut OpenOptions) -> &mut OpenOptions {
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(options, 0o600);
    options
}

/// On Unix, `dir` must be a real folder (not a link) that only its owner can open. Windows
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

/// A name GhostSoap gives a draft: `<entry-id>-GS-XXXXX.eml` or `.txt`, where the entry id
/// is lowercase ASCII letters, digits and hyphens and `XXXXX` is 5 digits or capitals.
fn is_draft(name: &str) -> bool {
    let Some(stem) = name
        .strip_suffix(".eml")
        .or_else(|| name.strip_suffix(".txt"))
    else {
        return false;
    };
    let Some((id, tag)) = stem.rsplit_once("-GS-") else {
        return false;
    };
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && tag.len() == 5
        && tag
            .bytes()
            .all(|b| b.is_ascii_digit() || b.is_ascii_uppercase())
}

/// Delete the drafts GhostSoap wrote in `dir`: each file the manifest lists that still holds
/// the bytes GhostSoap wrote. Other files, including a listed name now holding something
/// else, are left alone, as are temporary files from an interrupted write (reported, not
/// deleted). With `remove_dir`, `dir` is also removed once empty; pass it only for
/// GhostSoap's own drafts folder.
pub fn clean(dir: &Path, remove_dir: bool) -> io::Result<Cleaned> {
    let manifest = dir.join(MANIFEST);
    let lines = match fs::read_to_string(&manifest) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Cleaned::default()),
        other => other?,
    };
    check_private(dir)?;
    let mut cleaned = Cleaned::default();
    for (hash, name) in lines.lines().filter_map(|l| l.split_once(' ')) {
        if !is_draft(name) {
            continue;
        }
        let path = dir.join(name);
        match fs::read(&path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
            Ok(bytes) if sha256(&bytes) == hash => {
                fs::remove_file(&path)?;
                cleaned.deleted += 1;
            }
            Ok(_) => cleaned.kept.push(name.to_owned()),
        }
    }
    fs::remove_file(&manifest)?;
    for item in fs::read_dir(dir)? {
        let name = item?.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') && name.ends_with(".tmp") && name.contains("-GS-") {
            cleaned.leftovers.push(name);
        }
    }
    if remove_dir && fs::read_dir(dir)?.next().is_none() {
        fs::remove_dir(dir)?;
    }
    Ok(cleaned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_deletes_only_drafts_it_wrote() {
        let dir = tempfile::tempdir().unwrap();
        let drafts = dir.path().join("drafts");
        write(&drafts, "quantium-au-GS-7K3QX.eml", "x").unwrap();
        write(&drafts, "talefin-au-GS-00AZ9.txt", "y").unwrap();
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
            write(&drafts, "a\nnotes-GS-ABCDE.txt", "x").is_err(),
            "no line break"
        );
        let cleaned = clean(&drafts, true).unwrap();
        assert_eq!(cleaned.deleted, 2);
        assert!(drafts.join("notes.txt").exists());
        assert!(drafts.join("notes-GS-ABCDE.txt").exists());
        assert!(!drafts.join(MANIFEST).exists());
        assert_eq!(clean(&drafts, true).unwrap(), Cleaned::default());
    }

    #[test]
    fn clean_keeps_a_listed_file_that_now_holds_other_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let drafts = dir.path().join("drafts");
        let path = write(&drafts, "a-GS-11111.eml", "letter").unwrap();
        fs::rename(&path, dir.path().join("moved.eml")).unwrap();
        fs::write(&path, "the user's own file").unwrap();
        // A listing GhostSoap never wrote cannot authorise a deletion either.
        let mut manifest = OpenOptions::new()
            .append(true)
            .open(drafts.join(MANIFEST))
            .unwrap();
        writeln!(manifest, "{} b-GS-22222.eml", "0".repeat(64)).unwrap();
        fs::write(drafts.join("b-GS-22222.eml"), "also the user's").unwrap();
        let cleaned = clean(&drafts, true).unwrap();
        assert_eq!(cleaned.deleted, 0);
        assert_eq!(cleaned.kept, ["a-GS-11111.eml", "b-GS-22222.eml"]);
        assert_eq!(fs::read_to_string(&path).unwrap(), "the user's own file");
        assert!(drafts.exists());
    }

    #[test]
    fn clean_removes_the_folder_only_when_asked() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("mine");
        write(&out, "a-GS-11111.eml", "x").unwrap();
        assert_eq!(clean(&out, false).unwrap().deleted, 1);
        assert!(out.exists(), "a folder the user named stays");
        write(&out, "a-GS-11111.eml", "x").unwrap();
        assert_eq!(clean(&out, true).unwrap().deleted, 1);
        assert!(!out.exists(), "GhostSoap's own emptied folder is removed");
    }

    #[test]
    fn a_draft_never_replaces_a_file_and_a_failed_write_leaves_others_alone() {
        let dir = tempfile::tempdir().unwrap();
        let drafts = dir.path().join("drafts");
        let path = write(&drafts, "a-GS-11111.eml", "first").unwrap();
        let err = write(&drafts, "a-GS-11111.eml", "second").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(&path).unwrap(), "first");
        // Another writer's temporary file with this process's name is neither used nor
        // removed.
        let tmp = drafts.join(format!(".b-GS-22222.eml.{}.tmp", std::process::id()));
        fs::write(&tmp, "canary").unwrap();
        assert!(write(&drafts, "b-GS-22222.eml", "x").is_err());
        assert_eq!(fs::read_to_string(&tmp).unwrap(), "canary");
        assert!(!drafts.join("b-GS-22222.eml").exists());
        let cleaned = clean(&drafts, true).unwrap();
        assert_eq!(cleaned.deleted, 1);
        assert_eq!(
            cleaned.leftovers,
            [format!(".b-GS-22222.eml.{}.tmp", std::process::id())]
        );
        assert!(tmp.exists(), "leftovers are reported, not deleted");
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
        assert!(clean(&link, true).is_err(), "clean refuses a link too");
    }
}
