//! Command-line interface.

use std::hash::{BuildHasher, RandomState};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::SystemTime;

use clap::{Args, CommandFactory, Parser, Subcommand};

use crate::core::embedded;
use crate::core::law::{self, Deadline};
use crate::core::letters::{self, Letter, Request};
use crate::core::plan::{self, Selection, Step};
use crate::core::profile::Profile;
use crate::core::register::{
    self, ChannelKind, Entry, IdentityField, Lever, Register, RegisterFiles, Severity,
};
use crate::io;
use crate::io::prompt::Input;
use crate::io::state::{self, State};

/// Prepare Australian Privacy Act requests to data brokers and track the replies.
#[derive(Debug, Parser)]
#[command(name = "ghostsoap", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create your encrypted profile: the identity details letters may include.
    Init {
        /// Read the passphrase from standard input instead of the terminal (for scripts).
        #[arg(long)]
        passphrase_stdin: bool,
    },
    /// Show the requests a selection would make, without your details. Nothing is written.
    Plan(SelectionArgs),
    /// Write the planned letters to the drafts folder for you to send yourself.
    Draft {
        #[command(flatten)]
        selection: SelectionArgs,
        /// Drafts folder; defaults to `ghostsoap-drafts` in the system temporary folder.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Print each letter's To, Subject and body instead of writing files (for webmail).
        #[arg(long)]
        copy: bool,
        /// Read the passphrase from standard input instead of the terminal (for scripts).
        #[arg(long)]
        passphrase_stdin: bool,
    },
    /// Delete the drafts GhostSoap wrote; other files in the folder are left alone.
    Clean {
        /// Drafts folder; defaults to `ghostsoap-drafts` in the system temporary folder.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Show the law table: each lever's legal basis, deadline and charge rule.
    Law,
    /// Work with the register of Australian entities.
    Registry {
        #[command(subcommand)]
        command: RegistryCommand,
    },
}

/// Which entities and requests.
#[derive(Debug, Args)]
pub struct SelectionArgs {
    /// A pack of entities, such as `start-here`; repeat for more.
    #[arg(long)]
    pack: Vec<String>,
    /// One entity by id, such as `quantium-au`; repeat for more.
    #[arg(long)]
    entry: Vec<String>,
    /// A request to make instead of the default APP 7 requests; repeat for more. Access,
    /// correction and ban requests need a single --entry.
    #[arg(long, value_parser = parse_lever)]
    lever: Vec<Lever>,
    /// Your own words for levers that need them, such as what is wrong for a correction.
    #[arg(long)]
    detail: Option<String>,
}

impl SelectionArgs {
    fn selection(&self) -> Selection {
        Selection {
            packs: self.pack.clone(),
            entries: self.entry.clone(),
            levers: self.lever.clone(),
            detail: self.detail.clone(),
        }
    }
}

fn parse_lever(name: &str) -> Result<Lever, String> {
    Lever::from_name(name).ok_or_else(|| {
        let names: Vec<&str> = Lever::ALL.iter().map(|l| l.name()).collect();
        format!("unknown lever; use one of {}", names.join(", "))
    })
}

#[derive(Debug, Subcommand)]
pub enum RegistryCommand {
    /// Validate register entries and packs.
    Check {
        /// Check a register directory, such as a working copy's `registry/au`, instead of
        /// the register built into this binary. Packs are read from its `packs` subdirectory.
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

/// Run a parsed command line.
pub fn run(cli: Cli) -> ExitCode {
    let result = match cli.command {
        None => Cli::command().print_help().map_err(|e| e.to_string()),
        Some(Command::Init { passphrase_stdin }) => init(passphrase_stdin),
        Some(Command::Plan(selection)) => show_plan(&selection.selection()),
        Some(Command::Draft {
            selection,
            out,
            copy,
            passphrase_stdin,
        }) => draft(&selection.selection(), out, copy, passphrase_stdin),
        Some(Command::Clean { out }) => clean(out),
        Some(Command::Law) => {
            show_law();
            Ok(())
        }
        Some(Command::Registry {
            command: RegistryCommand::Check { path },
        }) => return registry_command(path),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn init(passphrase_stdin: bool) -> Result<(), String> {
    let dir = state::data_dir().map_err(|e| e.to_string())?;
    let path = state::state_path(&dir);
    if path.exists() {
        return Err(format!(
            "a profile already exists at {}; delete that file to start again",
            path.display()
        ));
    }
    let mut input = Input::new(passphrase_stdin);
    let profile = input.profile().map_err(|e| e.to_string())?;
    let problems = profile.problems();
    if !problems.is_empty() {
        return Err(problems.join("; "));
    }
    let passphrase = input.new_passphrase().map_err(|e| e.to_string())?;
    state::create(&dir, &State::new(profile), passphrase).map_err(|e| e.to_string())?;
    println!("Saved your encrypted profile to {}.", path.display());
    println!("A lost passphrase cannot be recovered; keep it somewhere safe.");
    Ok(())
}

fn built_in_register() -> Result<Register, String> {
    register::parse_register(&embedded::register())
}

fn show_plan(selection: &Selection) -> Result<(), String> {
    let register = built_in_register()?;
    let steps = plan::plan(&register, selection)?;
    for step in &steps {
        match step {
            Step::Letter {
                entry,
                channel,
                levers,
            } => {
                println!(
                    "letter  {}  by {} to {}",
                    entry.id,
                    kind_name(channel.kind),
                    channel.address
                );
                for &lever in levers {
                    let row = law::law(lever);
                    println!(
                        "    {} ({}; {})",
                        lever.name(),
                        row.provision,
                        deadline_summary(row.deadline)
                    );
                }
                println!("    details used: {}", field_names(entry));
            }
            Step::Overseas { entry } => print_overseas(entry),
            Step::Skip { entry, reason } => println!("skip    {}: {reason}", entry.id),
        }
    }
    access_warning(selection);
    Ok(())
}

fn draft(
    selection: &Selection,
    out: Option<PathBuf>,
    copy: bool,
    passphrase_stdin: bool,
) -> Result<(), String> {
    let register = built_in_register()?;
    let steps = plan::plan(&register, selection)?;
    let mut profile = None;
    let dir = out.unwrap_or_else(io::drafts::default_dir);
    let today = chrono::Local::now().date_naive();
    let mut tags = Vec::new();
    for step in &steps {
        let (entry, channel, levers) = match step {
            Step::Letter {
                entry,
                channel,
                levers,
            } => (entry, channel, levers),
            Step::Overseas { entry } => {
                print_overseas(entry);
                continue;
            }
            Step::Skip { entry, reason } => {
                println!("skip    {}: {reason}", entry.id);
                continue;
            }
        };
        if profile.is_none() {
            profile = Some(load_profile(passphrase_stdin)?);
        }
        let profile = profile.as_ref().expect("loaded above");
        let tag = new_tag(&tags);
        tags.push(tag.clone());
        let request = Request {
            channel,
            levers,
            detail: selection.detail.as_deref(),
            date: today,
            tag: &tag,
        };
        let letter = letters::compose(entry, profile, &request)?;
        print_missing_fields(entry, profile);
        if copy {
            print_copy(entry, &letter);
        } else {
            write_draft(&dir, entry, &tag, &letter)?;
        }
    }
    access_warning(selection);
    if !tags.is_empty() && !copy {
        println!(
            "Drafts are plaintext. After sending them from your own mailbox, run `ghostsoap clean`."
        );
    }
    Ok(())
}

fn load_profile(passphrase_stdin: bool) -> Result<Profile, String> {
    let dir = state::data_dir().map_err(|e| e.to_string())?;
    let mut input = Input::new(passphrase_stdin);
    let passphrase = input
        .passphrase("Passphrase: ")
        .map_err(|e| e.to_string())?;
    Ok(state::load(&dir, passphrase)
        .map_err(|e| e.to_string())?
        .profile)
}

/// A tag not already used in this run. `RandomState` is seeded randomly per process, so
/// hashing the time gives unpredictable bits without another dependency; tags only need to
/// be distinct, not secret.
fn new_tag(used: &[String]) -> String {
    let state = RandomState::new();
    let mut n = 0u64;
    loop {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let tag = letters::tag(state.hash_one((now, n)));
        if !used.contains(&tag) {
            return tag;
        }
        n += 1;
    }
}

fn write_draft(dir: &Path, entry: &Entry, tag: &str, letter: &Letter) -> Result<(), String> {
    let (extension, text) = match letter.kind {
        ChannelKind::Email => ("eml", letters::eml(letter)),
        _ => ("txt", format!("{}\n", letter.body)),
    };
    let name = format!("{}-{tag}.{extension}", entry.id);
    let path = io::drafts::write(dir, &name, &text).map_err(|e| e.to_string())?;
    let how = match letter.kind {
        ChannelKind::Email => format!("email to {}", letter.to),
        ChannelKind::Post => format!("print and post to {}", letter.to),
        ChannelKind::WebForm => format!("paste into the form at {}", letter.to),
        ChannelKind::Phone => unreachable!("plans never send letters by phone"),
    };
    println!("draft   {}  {how}: {}", entry.id, path.display());
    if let Some(link) = letters::mailto(letter).filter(|_| letter.kind == ChannelKind::Email) {
        println!("        or open: {link}");
    }
    Ok(())
}

fn print_copy(entry: &Entry, letter: &Letter) {
    println!("==== {} ({})", entry.id, kind_name(letter.kind));
    println!("To: {}", letter.to);
    println!("Subject: {}", letter.subject);
    println!();
    println!("{}", letter.body);
    println!();
}

fn print_overseas(entry: &Entry) {
    let check = entry.manual_check.as_ref().map_or("", |m| m.url.as_str());
    let forms: Vec<&str> = entry
        .channels
        .iter()
        .filter(|c| c.kind == ChannelKind::WebForm)
        .map(|c| c.address.as_str())
        .collect();
    println!(
        "manual  {}: overseas site; no Australian law lever applies. Search for yourself at \
         {check}, then use its own opt-out form at {} (effort: {}).",
        entry.id,
        forms.join(" or "),
        format!("{:?}", entry.effort).to_lowercase()
    );
}

fn access_warning(selection: &Selection) {
    let access = [Lever::App12, Lever::S20rAccess, Lever::S21tAccess];
    if selection.levers.iter().any(|l| access.contains(l)) {
        println!(
            "warning: access requests return personal information. Make them only about \
             yourself; requesting someone else's information may be an offence."
        );
    }
}

fn print_missing_fields(entry: &Entry, profile: &Profile) {
    for &field in &entry.identity_fields {
        if profile.value(field).is_none() {
            println!(
                "note    {}: your profile has no {}, so the letter leaves it out",
                entry.id,
                field_name(field)
            );
        }
    }
}

fn field_names(entry: &Entry) -> String {
    let names: Vec<&str> = entry
        .identity_fields
        .iter()
        .map(|&f| field_name(f))
        .collect();
    names.join(", ")
}

fn field_name(field: IdentityField) -> &'static str {
    match field {
        IdentityField::FullName => "full name",
        IdentityField::AliasNames => "other names",
        IdentityField::Email => "email",
        IdentityField::Phone => "phone",
        IdentityField::DateOfBirth => "date of birth",
        IdentityField::Address => "address",
        IdentityField::PreviousAddresses => "previous addresses",
        IdentityField::Postcode => "postcode",
    }
}

fn kind_name(kind: ChannelKind) -> &'static str {
    match kind {
        ChannelKind::Email => "email",
        ChannelKind::WebForm => "web form",
        ChannelKind::Post => "post",
        ChannelKind::Phone => "phone",
    }
}

fn deadline_summary(deadline: Deadline) -> String {
    match deadline {
        Deadline::Days(n) => format!("{n} days"),
        Deadline::BusinessDays(n) => format!("{n} business days"),
        Deadline::Reasonable {
            guide_days: Some(n),
        } => format!("a reasonable period, OAIC guide {n} days"),
        Deadline::Reasonable { guide_days: None } => "a reasonable period".to_owned(),
        Deadline::NotStated => "no time set".to_owned(),
    }
}

fn clean(out: Option<PathBuf>) -> Result<(), String> {
    let dir = out.unwrap_or_else(io::drafts::default_dir);
    let deleted = io::drafts::clean(&dir).map_err(|e| e.to_string())?;
    println!("Deleted {deleted} drafts from {}.", dir.display());
    Ok(())
}

fn show_law() {
    for lever in Lever::ALL {
        let row = law::law(lever);
        println!("{}: {}", lever.name(), row.title);
        println!("    basis: {}", row.provision);
        println!("    deadline: {}", row.deadline_text);
        println!("    charge: {}", row.charge);
    }
    println!();
    println!("Sources:");
    for source in law::SOURCES {
        println!("    {source}");
    }
    println!("GhostSoap provides information and templates, not legal advice.");
}

fn registry_command(path: Option<PathBuf>) -> ExitCode {
    match path {
        None => registry_check(&embedded::register(), "built-in register"),
        Some(dir) => match io::register::load(&dir) {
            Ok(files) => registry_check(&files, &dir.display().to_string()),
            Err(e) => {
                eprintln!("error: cannot read {}: {e}", dir.display());
                ExitCode::FAILURE
            }
        },
    }
}

fn registry_check(files: &RegisterFiles, place_name: &str) -> ExitCode {
    let today = chrono::Local::now().date_naive();
    let report = register::check(&files.entries, &files.packs, today);
    for finding in &report.findings {
        let label = match finding.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let place = if finding.file.is_empty() {
            place_name.to_owned()
        } else {
            format!("{place_name}/{}", finding.file)
        };
        eprintln!("{label}: {place}: {}", finding.message);
    }
    println!(
        "Checked {} entries and {} packs; errors: {}, warnings: {}.",
        report.entries,
        report.packs,
        report.errors(),
        report.warnings()
    );
    if report.errors() == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
