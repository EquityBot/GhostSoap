//! The drafts folder: plaintext letters waiting to be sent (plan section 5).
//!
//! Each folder has a manifest, `.ghostsoap-drafts`. Before a draft is written its name is
//! recorded as `pending <name>`; once the draft is in place, `<sha256> <name>` commits it.
//! `clean` deletes only committed drafts that still hold those bytes. A lock file serialises
//! `write` and `clean` between GhostSoap processes; other programs changing the folder at the
//! same moment are outside this guarantee.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::io::state::{Lock, create_private_dir};

const MANIFEST: &str = ".ghostsoap-drafts";
const LOCK: &str = ".ghostsoap-drafts.lock";
const PENDING: &str = "pending";

/// What `clean` did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Cleaned {
    /// Drafts deleted.
    pub deleted: usize,
    /// Committed names whose file no longer holds what GhostSoap wrote, left alone.
    pub kept: Vec<String>,
    /// Names whose write never committed but whose file exists, left alone.
    pub unconfirmed: Vec<String>,
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
    let _lock = Lock::acquire(dir, LOCK)?;
    let path = dir.join(name);
    if fs::symlink_metadata(&path).is_ok() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already exists", path.display()),
        ));
    }
    let mut manifest = open_manifest(dir)?;
    record(&mut manifest, PENDING, name)?;
    let tmp = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    let mut file = private_file(OpenOptions::new().write(true).create_new(true)).open(&tmp)?;
    // From here the temporary file is this call's own, so it may remove it.
    let published = file
        .write_all(text.as_bytes())
        .and_then(|()| file.sync_all())
        .and_then(|()| fs::hard_link(&tmp, &path));
    drop(file);
    let _ = fs::remove_file(&tmp);
    published?;
    sync_dir(dir)?;
    record(&mut manifest, &sha256(text.as_bytes()), name)?;
    Ok(path)
}

/// Open the manifest for appending, refusing a link or anything but a plain file.
fn open_manifest(dir: &Path) -> io::Result<File> {
    let path = dir.join(MANIFEST);
    match fs::symlink_metadata(&path) {
        Ok(meta) if !meta.is_file() => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} is not a plain file", path.display()),
            ));
        }
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    private_file(OpenOptions::new().append(true).create(true)).open(path)
}

fn record(manifest: &mut File, first: &str, name: &str) -> io::Result<()> {
    writeln!(manifest, "{first} {name}")?;
    manifest.sync_all()
}

/// On Unix, flush the folder so a new name survives a power cut.
fn sync_dir(dir: &Path) -> io::Result<()> {
    #[cfg(unix)]
    File::open(dir)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
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

/// Delete the drafts GhostSoap wrote in `dir`: each committed draft that still holds the
/// bytes GhostSoap wrote. Other files, including a listed name now holding something else
/// and a name whose write never committed, are left alone and reported, as are temporary
/// files from an interrupted write. With `remove_dir`, `dir` is also removed once empty;
/// pass it only for GhostSoap's own drafts folder.
pub fn clean(dir: &Path, remove_dir: bool) -> io::Result<Cleaned> {
    if !dir.exists() {
        return Ok(Cleaned::default());
    }
    check_private(dir)?;
    let lock = Lock::acquire(dir, LOCK)?;
    let mut cleaned = Cleaned::default();
    let manifest = dir.join(MANIFEST);
    if fs::symlink_metadata(&manifest).is_ok() {
        drop(open_manifest(dir)?); // refuses a link or a folder
        let lines = fs::read_to_string(&manifest)?;
        let records: Vec<(&str, &str)> = lines
            .lines()
            .filter_map(|l| l.split_once(' '))
            .filter(|&(_, name)| is_draft(name))
            .collect();
        for &(hash, name) in records.iter().filter(|&&(first, _)| first != PENDING) {
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
        for &(_, name) in records.iter().filter(|&&(first, _)| first == PENDING) {
            let committed = records.iter().any(|&(f, n)| n == name && f != PENDING);
            if !committed && dir.join(name).exists() {
                cleaned.unconfirmed.push(name.to_owned());
            }
        }
        fs::remove_file(&manifest)?;
    }
    for item in fs::read_dir(dir)? {
        let name = item?.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') && name.ends_with(".tmp") && name.contains("-GS-") {
            cleaned.leftovers.push(name);
        }
    }
    drop(lock);
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
        assert!(!drafts.join(LOCK).exists());
        assert_eq!(clean(&drafts, true).unwrap(), Cleaned::default());
    }

    #[test]
    fn a_failed_write_cannot_authorise_deleting_an_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let drafts = dir.path().join("drafts");
        write(&drafts, "seed-GS-11111.eml", "seed").unwrap();
        let path = drafts.join("a-GS-22222.eml");
        fs::write(&path, "letter").unwrap();
        let err = write(&drafts, "a-GS-22222.eml", "letter").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        // A write that published but stopped before committing leaves a pending name only.
        let mut manifest = open_manifest(&drafts).unwrap();
        record(&mut manifest, PENDING, "b-GS-33333.eml").unwrap();
        fs::write(drafts.join("b-GS-33333.eml"), "unconfirmed").unwrap();
        let cleaned = clean(&drafts, false).unwrap();
        assert_eq!(cleaned.deleted, 1, "only the seed draft");
        assert_eq!(fs::read_to_string(&path).unwrap(), "letter");
        assert_eq!(cleaned.unconfirmed, ["b-GS-33333.eml"]);
        assert!(drafts.join("b-GS-33333.eml").exists());
    }

    #[test]
    fn clean_keeps_a_committed_file_that_now_holds_other_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let drafts = dir.path().join("drafts");
        let path = write(&drafts, "a-GS-11111.eml", "letter").unwrap();
        fs::rename(&path, dir.path().join("moved.eml")).unwrap();
        fs::write(&path, "the user's own file").unwrap();
        let mut manifest = open_manifest(&drafts).unwrap();
        record(&mut manifest, &"0".repeat(64), "b-GS-22222.eml").unwrap();
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
    fn a_draft_never_replaces_a_file_and_leftovers_stay_reported() {
        let dir = tempfile::tempdir().unwrap();
        let drafts = dir.path().join("drafts");
        let path = write(&drafts, "a-GS-11111.eml", "first").unwrap();
        let err = write(&drafts, "a-GS-11111.eml", "second").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(&path).unwrap(), "first");
        // Another writer's temporary file with this process's name is neither used nor
        // removed.
        let tmp_name = format!(".b-GS-22222.eml.{}.tmp", std::process::id());
        let tmp = drafts.join(&tmp_name);
        fs::write(&tmp, "canary").unwrap();
        assert!(write(&drafts, "b-GS-22222.eml", "x").is_err());
        assert_eq!(fs::read_to_string(&tmp).unwrap(), "canary");
        assert!(!drafts.join("b-GS-22222.eml").exists());
        let cleaned = clean(&drafts, true).unwrap();
        assert_eq!(cleaned.deleted, 1);
        assert_eq!(cleaned.leftovers, std::slice::from_ref(&tmp_name));
        assert_eq!(
            clean(&drafts, true).unwrap().leftovers,
            [tmp_name],
            "still reported without a manifest"
        );
        assert!(tmp.exists(), "leftovers are reported, not deleted");
    }

    #[test]
    fn a_held_lock_stops_write_and_clean() {
        let dir = tempfile::tempdir().unwrap();
        let drafts = dir.path().join("drafts");
        write(&drafts, "a-GS-11111.eml", "x").unwrap();
        fs::write(drafts.join(LOCK), "").unwrap();
        let err = write(&drafts, "b-GS-22222.eml", "y").unwrap_err();
        assert!(err.to_string().contains("another GhostSoap"), "{err}");
        assert!(clean(&drafts, true).is_err());
        assert!(drafts.join("a-GS-11111.eml").exists());
        assert!(
            drafts.join(LOCK).exists(),
            "someone else's lock is left alone"
        );
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

    #[cfg(unix)]
    #[test]
    fn a_linked_manifest_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let drafts = dir.path().join("drafts");
        write(&drafts, "a-GS-11111.eml", "x").unwrap();
        fs::remove_file(drafts.join(MANIFEST)).unwrap();
        let canary = dir.path().join("canary.txt");
        fs::write(&canary, "unchanged").unwrap();
        std::os::unix::fs::symlink(&canary, drafts.join(MANIFEST)).unwrap();
        assert!(write(&drafts, "b-GS-22222.eml", "y").is_err());
        assert!(clean(&drafts, true).is_err());
        assert_eq!(fs::read_to_string(&canary).unwrap(), "unchanged");
    }
}
