//! The encrypted state file (plan section 5, 'Storage and secrets'). M2 holds the profile;
//! M3 adds requests and events.

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use age::secrecy::SecretString;
use serde::{Deserialize, Serialize};

use crate::core::profile::Profile;

const STATE_FILE: &str = "state.age";
const LOCK_FILE: &str = "state.lock";
const FORMAT: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub format: u32,
    pub profile: Profile,
}

impl State {
    pub fn new(profile: Profile) -> Self {
        Self {
            format: FORMAT,
            profile,
        }
    }
}

/// Where the state lives: `GHOSTSOAP_HOME` if set, else the platform data directory, never
/// a folder a sync service such as OneDrive is likely to copy.
pub fn data_dir() -> io::Result<PathBuf> {
    let var = |name: &str| {
        env::var_os(name)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    if let Some(home) = var("GHOSTSOAP_HOME") {
        return Ok(home);
    }
    let dir = if cfg!(windows) {
        var("LOCALAPPDATA").map(|d| d.join("GhostSoap"))
    } else if cfg!(target_os = "macos") {
        var("HOME").map(|d| d.join("Library/Application Support/GhostSoap"))
    } else {
        var("XDG_DATA_HOME")
            .or_else(|| var("HOME").map(|h| h.join(".local/share")))
            .map(|d| d.join("ghostsoap"))
    };
    dir.ok_or_else(|| io::Error::other("cannot find a data directory; set GHOSTSOAP_HOME"))
}

pub fn state_path(dir: &Path) -> PathBuf {
    dir.join(STATE_FILE)
}

/// Encrypt and write a new state file. Refuses to replace an existing one.
pub fn create(dir: &Path, state: &State, passphrase: SecretString) -> io::Result<()> {
    let path = state_path(dir);
    let text = toml::to_string(state).map_err(io::Error::other)?;
    let recipient = age::scrypt::Recipient::new(passphrase);
    let bytes = age::encrypt(&recipient, text.as_bytes()).map_err(io::Error::other)?;
    create_private_dir(dir)?;
    // Checked under the lock, so 2 `init` runs cannot both see no file and the second replace
    // the first.
    let _lock = Lock::acquire(dir)?;
    if path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already exists", path.display()),
        ));
    }
    write_atomic(&path, &bytes)
}

/// Read and decrypt the state file.
pub fn load(dir: &Path, passphrase: SecretString) -> io::Result<State> {
    let path = state_path(dir);
    let bytes = fs::read(&path).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => io::Error::new(
            e.kind(),
            format!(
                "no profile at {}; run `ghostsoap init` first",
                path.display()
            ),
        ),
        _ => e,
    })?;
    let identity = age::scrypt::Identity::new(passphrase);
    let text = age::decrypt(&identity, &bytes).map_err(|e| match e {
        age::DecryptError::DecryptionFailed | age::DecryptError::NoMatchingKeys => {
            io::Error::new(io::ErrorKind::PermissionDenied, "wrong passphrase")
        }
        other => io::Error::other(format!("cannot decrypt {}: {other}", path.display())),
    })?;
    let text = String::from_utf8(text).map_err(io::Error::other)?;
    let state: State = toml::from_str(&text).map_err(io::Error::other)?;
    if state.format != FORMAT {
        return Err(io::Error::other(format!(
            "state format {} is not supported by this version",
            state.format
        )));
    }
    Ok(state)
}

/// Create `dir` and its parents; on Unix, the folders created are private to the user.
#[cfg(unix)]
pub(crate) fn create_private_dir(dir: &Path) -> io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
}

#[cfg(not(unix))]
pub(crate) fn create_private_dir(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)
}

/// The state lock: a file created exclusively, so 2 processes cannot interleave writes, and
/// removed when dropped.
struct Lock(PathBuf);

impl Lock {
    fn acquire(dir: &Path) -> io::Result<Self> {
        let path = dir.join(LOCK_FILE);
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| match e.kind() {
                io::ErrorKind::AlreadyExists => io::Error::new(
                    e.kind(),
                    format!(
                        "{} exists: another GhostSoap is writing, or one stopped mid-write; \
                         delete it if no GhostSoap is running",
                        path.display()
                    ),
                ),
                _ => e,
            })?;
        Ok(Self(path))
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        // A lock left behind stops the next write with a message saying how to clear it.
        let _ = fs::remove_file(&self.0);
    }
}

/// Write `bytes` to a temporary file beside `path`, flush it to disk and rename it over
/// `path`. The caller holds the [`Lock`]. A rename that fails because antivirus or an indexer
/// briefly holds the file is retried.
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("age.tmp");
    let mut file = File::create(&tmp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    let mut attempt = 0;
    loop {
        match fs::rename(&tmp, path) {
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied && attempt < 10 => {
                attempt += 1;
                thread::sleep(Duration::from_millis(200));
            }
            other => return other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Profile {
        Profile {
            full_name: "Lara Croft".to_owned(),
            email: Some("lara@croft-manor.example".to_owned()),
            ..Profile::default()
        }
    }

    fn secret(s: &str) -> SecretString {
        SecretString::from(s.to_owned())
    }

    #[test]
    fn round_trip_and_wrong_passphrase() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        create(&home, &State::new(profile()), secret("correct horse")).unwrap();
        let raw = fs::read(state_path(&home)).unwrap();
        assert!(
            !String::from_utf8_lossy(&raw).contains("Croft"),
            "state is not plaintext"
        );
        assert!(!home.join(LOCK_FILE).exists());
        assert_eq!(
            load(&home, secret("correct horse")).unwrap().profile,
            profile()
        );
        let err = load(&home, secret("wrong horse")).unwrap_err();
        assert_eq!(err.to_string(), "wrong passphrase");
        let err = create(&home, &State::new(profile()), secret("x")).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
    }

    #[test]
    fn a_held_lock_stops_a_write() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(LOCK_FILE), "").unwrap();
        let err = create(dir.path(), &State::new(profile()), secret("pass phrase")).unwrap_err();
        assert!(err.to_string().contains("another GhostSoap is writing"));
        assert!(!state_path(dir.path()).exists());
    }
}
