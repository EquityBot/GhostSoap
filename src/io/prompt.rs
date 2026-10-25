//! Terminal input: profile questions and the passphrase.

use std::io::{self, BufRead, Lines, StdinLock, Write};

use age::secrecy::SecretString;

use crate::core::profile::Profile;

/// Minimum passphrase length, in characters.
const MIN_PASSPHRASE: usize = 12;

/// Reads answers from standard input, and the passphrase either from the terminal with
/// echo off or, for scripts and tests, as the next line of standard input.
pub struct Input {
    passphrase_from_stdin: bool,
    lines: Lines<StdinLock<'static>>,
}

impl Input {
    pub fn new(passphrase_from_stdin: bool) -> Self {
        Self {
            passphrase_from_stdin,
            lines: io::stdin().lock().lines(),
        }
    }

    /// One line of input after printing `prompt` to standard error.
    pub fn line(&mut self, prompt: &str) -> io::Result<String> {
        eprint!("{prompt}");
        io::stderr().flush()?;
        match self.lines.next() {
            Some(line) => Ok(line?.trim().to_owned()),
            None => Err(io::Error::new(io::ErrorKind::UnexpectedEof, "input ended")),
        }
    }

    pub fn passphrase(&mut self, prompt: &str) -> io::Result<SecretString> {
        let text = if self.passphrase_from_stdin {
            self.line(prompt)?
        } else {
            rpassword::prompt_password(prompt)?
        };
        Ok(SecretString::from(text))
    }

    /// Ask for a new passphrase twice.
    pub fn new_passphrase(&mut self) -> io::Result<SecretString> {
        use age::secrecy::ExposeSecret;
        let first = self.passphrase("New passphrase (at least 12 characters): ")?;
        if first.expose_secret().chars().count() < MIN_PASSPHRASE {
            return Err(io::Error::other(
                "the passphrase must be at least 12 characters",
            ));
        }
        let second = self.passphrase("Repeat the passphrase: ")?;
        if first.expose_secret() != second.expose_secret() {
            return Err(io::Error::other("the passphrases do not match"));
        }
        Ok(first)
    }

    /// Ask for each profile field. Blank answers leave optional fields empty; lists are
    /// separated by semicolons.
    pub fn profile(&mut self) -> io::Result<Profile> {
        let optional = |s: String| Some(s).filter(|s| !s.is_empty());
        let list = |s: String| -> Vec<String> {
            s.split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect()
        };
        eprintln!(
            "GhostSoap puts only the details each business needs into its letters. \
             Leave a question blank to skip it."
        );
        Ok(Profile {
            full_name: self.line("Full name: ")?,
            alias_names: list(self.line("Other names you have used, separated by ';': ")?),
            email: optional(self.line("Email: ")?),
            phone: optional(self.line("Phone: ")?),
            date_of_birth: optional(self.line("Date of birth (YYYY-MM-DD): ")?),
            address: optional(self.line("Current address: ")?),
            previous_addresses: list(self.line("Previous addresses, separated by ';': ")?),
            postcode: optional(self.line("Postcode: ")?),
        })
    }
}
