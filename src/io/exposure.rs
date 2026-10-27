//! Places where plaintext drafts could leave the machine: a Git working tree, whose files can
//! be committed and pushed, or a folder a sync service uploads.
//!
//! The checks are a best effort and only ever produce warnings. They find a `.git` folder or
//! file at or above the drafts folder (so linked worktrees and submodules count) and a small
//! list of sync folders:
//!
//! - OneDrive: the folders the `OneDrive`, `OneDriveCommercial` and `OneDriveConsumer`
//!   environment variables name (set by OneDrive on Windows).
//! - Dropbox: `Dropbox` in the home folder (`%USERPROFILE%` on Windows, `$HOME` elsewhere).
//! - iCloud Drive: `iCloudDrive` in the home folder on Windows; `Library/Mobile Documents`
//!   on macOS.
//! - macOS cloud storage: `Library/CloudStorage`, where current OneDrive, Dropbox and Google
//!   Drive clients keep their folders.
//!
//! They miss, amongst others, repositories found through `GIT_DIR`, Dropbox or OneDrive
//! folders moved elsewhere, other sync services, backup tools and network shares. A folder
//! with no finding is not thereby private.
//!
//! The folder is checked both as given and with links resolved, so a link into or out of a
//! repository or sync folder is reported either way. A folder that does not exist yet is
//! judged by its nearest existing ancestor.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

/// Something that may copy files in the drafts folder off the machine.
#[derive(Debug, PartialEq, Eq)]
pub enum Exposure {
    /// Inside the Git working tree whose top folder is `root`.
    Git { root: PathBuf },
    /// Inside the folder `root`, which `service` uploads.
    Sync {
        service: &'static str,
        root: PathBuf,
    },
}

/// A folder a sync service uploads.
#[derive(Debug, PartialEq, Eq)]
pub struct SyncRoot {
    pub service: &'static str,
    pub path: PathBuf,
}

/// The sync folders listed in the module documentation, read through `var` (normally
/// `std::env::var_os`) so tests can supply their own. Empty variables are ignored.
pub fn sync_roots(var: impl Fn(&str) -> Option<OsString>) -> Vec<SyncRoot> {
    let var = |name: &str| var(name).filter(|v| !v.is_empty()).map(PathBuf::from);
    let mut roots: Vec<SyncRoot> = ["OneDrive", "OneDriveCommercial", "OneDriveConsumer"]
        .into_iter()
        .filter_map(var)
        .map(|path| SyncRoot {
            service: "OneDrive",
            path,
        })
        .collect();
    let home = if cfg!(windows) {
        var("USERPROFILE")
    } else {
        var("HOME")
    };
    if let Some(home) = home {
        let mut add = |service, sub: &str| {
            roots.push(SyncRoot {
                service,
                path: home.join(sub),
            });
        };
        add("Dropbox", "Dropbox");
        if cfg!(windows) {
            add("iCloud Drive", "iCloudDrive");
        }
        if cfg!(target_os = "macos") {
            add("iCloud Drive", "Library/Mobile Documents");
            add("macOS cloud storage", "Library/CloudStorage");
        }
    }
    roots
}

/// What may copy files written to `dir` off the machine, as far as these checks can tell:
/// the nearest Git working tree above `dir` as given and above it with links resolved, and
/// each sync folder in `roots` containing either. An empty result does not mean the folder
/// is private.
pub fn exposures(dir: &Path, roots: &[SyncRoot]) -> Vec<Exposure> {
    let given = std::path::absolute(dir).unwrap_or_else(|_| dir.to_owned());
    let resolved = resolve(&given);
    let mut found = Vec::new();
    for path in [&given, &resolved] {
        if let Some(root) = path
            .ancestors()
            .find(|a| fs::symlink_metadata(a.join(".git")).is_ok())
        {
            push_new(&mut found, Exposure::Git { root: real(root) });
        }
    }
    for root in roots {
        // A sync folder that does not exist holds nothing.
        let Ok(real_root) = fs::canonicalize(&root.path) else {
            continue;
        };
        let given_root = std::path::absolute(&root.path).unwrap_or_else(|_| root.path.clone());
        if resolved.starts_with(&real_root) || given.starts_with(&given_root) {
            push_new(
                &mut found,
                Exposure::Sync {
                    service: root.service,
                    root: real_root,
                },
            );
        }
    }
    found
}

fn push_new(found: &mut Vec<Exposure>, exposure: Exposure) {
    if !found.contains(&exposure) {
        found.push(exposure);
    }
}

/// `path` with links resolved: its nearest existing ancestor made canonical, then the rest
/// of the path as given. Unchanged if no ancestor can be resolved.
fn resolve(path: &Path) -> PathBuf {
    let mut existing = path;
    let mut rest = Vec::new();
    loop {
        if let Ok(real) = fs::canonicalize(existing) {
            return rest.iter().rev().fold(real, |p, name| p.join(name));
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name);
                existing = parent;
            }
            _ => return path.to_owned(),
        }
    }
}

/// `path` canonical if possible, so the same folder reached two ways is reported once.
fn real(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_owned())
}

/// `path` for people to read: without the `\\?\` prefix Windows gives canonical paths on a
/// drive.
pub fn display(path: &Path) -> String {
    let text = path.display().to_string();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if cfg!(windows) && !rest.starts_with(r"UNC\") => rest.to_owned(),
        _ => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roots(pairs: &[(&'static str, &Path)]) -> Vec<SyncRoot> {
        pairs
            .iter()
            .map(|&(service, path)| SyncRoot {
                service,
                path: path.to_owned(),
            })
            .collect()
    }

    fn git(root: &Path) -> Exposure {
        Exposure::Git {
            root: fs::canonicalize(root).unwrap(),
        }
    }

    #[test]
    fn a_git_folder_or_file_marks_a_working_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        let out = repo.join("letters");
        fs::create_dir(&out).unwrap();
        assert_eq!(exposures(&out, &[]), [git(&repo)]);
        // A linked worktree or submodule has a `.git` file pointing at the repository.
        let worktree = tmp.path().join("worktree");
        fs::create_dir(&worktree).unwrap();
        fs::write(worktree.join(".git"), "gitdir: ../repo/.git/worktrees/w\n").unwrap();
        assert_eq!(exposures(&worktree, &[]), [git(&worktree)]);
    }

    #[test]
    fn a_nested_repository_reports_the_nearest_working_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let outer = tmp.path().join("outer");
        let inner = outer.join("vendor").join("inner");
        fs::create_dir_all(outer.join(".git")).unwrap();
        fs::create_dir_all(&inner).unwrap();
        fs::write(inner.join(".git"), "gitdir: ../../.git/modules/inner\n").unwrap();
        assert_eq!(exposures(&inner.join("drafts"), &[]), [git(&inner)]);
        assert_eq!(exposures(&outer.join("vendor"), &[]), [git(&outer)]);
    }

    #[test]
    fn a_folder_not_yet_made_is_judged_by_its_nearest_ancestor() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        let out = repo.join("not").join("made").join("yet");
        assert_eq!(exposures(&out, &[]), [git(&repo)]);
        assert!(!out.exists(), "checking creates nothing");
        let dropbox = tmp.path().join("Dropbox");
        fs::create_dir(&dropbox).unwrap();
        let found = exposures(&dropbox.join("new"), &roots(&[("Dropbox", &dropbox)]));
        assert_eq!(
            found,
            [Exposure::Sync {
                service: "Dropbox",
                root: fs::canonicalize(&dropbox).unwrap()
            }]
        );
    }

    #[test]
    fn sync_folders_are_matched_and_reported_once() {
        let tmp = tempfile::tempdir().unwrap();
        let onedrive = tmp.path().join("OneDrive");
        let out = onedrive.join("Documents").join("drafts");
        fs::create_dir_all(&out).unwrap();
        let missing = tmp.path().join("no-such-sync-folder");
        let list = roots(&[
            ("OneDrive", &onedrive),
            ("OneDrive", &onedrive),
            ("Dropbox", &missing),
        ]);
        let expected = Exposure::Sync {
            service: "OneDrive",
            root: fs::canonicalize(&onedrive).unwrap(),
        };
        assert_eq!(exposures(&out, &list), [expected]);
        assert!(exposures(tmp.path(), &list).is_empty(), "above the root");
        let beside = tmp.path().join("OneDriveNot");
        fs::create_dir(&beside).unwrap();
        assert!(
            exposures(&beside, &list).is_empty(),
            "a sibling sharing the name's prefix is outside it"
        );
    }

    #[test]
    fn sync_roots_come_from_the_variables_given() {
        let vars = |name: &str| match name {
            "OneDrive" => Some(OsString::from("od")),
            "OneDriveCommercial" => Some(OsString::new()),
            "USERPROFILE" | "HOME" => Some(OsString::from("home")),
            _ => None,
        };
        let found = sync_roots(vars);
        assert_eq!(
            found[..2],
            [
                SyncRoot {
                    service: "OneDrive",
                    path: PathBuf::from("od")
                },
                SyncRoot {
                    service: "Dropbox",
                    path: Path::new("home").join("Dropbox")
                },
            ],
            "an empty variable is ignored"
        );
        let expected_extra = if cfg!(windows) || cfg!(target_os = "macos") {
            1 + usize::from(cfg!(target_os = "macos"))
        } else {
            0
        };
        assert_eq!(found.len(), 2 + expected_extra);
        assert!(sync_roots(|_| None).is_empty());
    }

    /// A link is followed and also judged as given, so either side being exposed counts.
    #[test]
    fn links_are_judged_both_as_given_and_resolved() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        let inside = repo.join("drafts");
        fs::create_dir_all(repo.join(".git")).unwrap();
        fs::create_dir(&inside).unwrap();
        let plain = tmp.path().join("plain");
        fs::create_dir(&plain).unwrap();
        let into_repo = tmp.path().join("into-repo");
        let out_of_repo = repo.join("out");
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(&inside, &into_repo)
            .and_then(|()| std::os::unix::fs::symlink(&plain, &out_of_repo));
        #[cfg(windows)]
        let made = std::os::windows::fs::symlink_dir(&inside, &into_repo)
            .and_then(|()| std::os::windows::fs::symlink_dir(&plain, &out_of_repo));
        if let Err(e) = made {
            // Windows needs Developer Mode or an administrator to make links.
            eprintln!("skipped: cannot create a directory link here: {e}");
            return;
        }
        assert_eq!(exposures(&into_repo, &[]), [git(&repo)], "link into a repo");
        assert_eq!(
            exposures(&out_of_repo, &[]),
            [git(&repo)],
            "link inside a repo"
        );
        assert_eq!(
            exposures(&into_repo.join("new"), &[]),
            [git(&repo)],
            "not yet made, below a link"
        );
    }

    #[test]
    fn display_drops_only_the_drive_verbatim_prefix() {
        assert_eq!(display(Path::new("plain")), "plain");
        if cfg!(windows) {
            assert_eq!(display(Path::new(r"\\?\C:\x")), r"C:\x");
            assert_eq!(
                display(Path::new(r"\\?\UNC\server\share")),
                r"\\?\UNC\server\share"
            );
        }
    }
}
