//! Command-line interface.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};

use crate::core::register::{self, Severity};
use crate::io;

/// Prepare Australian Privacy Act requests to data brokers and track the replies.
#[derive(Debug, Parser)]
#[command(name = "ghostsoap", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Work with the register of Australian entities.
    Registry {
        #[command(subcommand)]
        command: RegistryCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum RegistryCommand {
    /// Validate register entries and packs.
    Check {
        /// Register directory; packs are read from its `packs` subdirectory.
        #[arg(long, default_value = "registry/au")]
        path: PathBuf,
    },
}

/// Run a parsed command line.
pub fn run(cli: Cli) -> ExitCode {
    match cli.command {
        None => match Cli::command().print_help() {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        },
        Some(Command::Registry {
            command: RegistryCommand::Check { path },
        }) => registry_check(&path),
    }
}

fn registry_check(dir: &Path) -> ExitCode {
    let files = match io::register::load(dir) {
        Ok(files) => files,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", dir.display());
            return ExitCode::FAILURE;
        }
    };
    let today = chrono::Local::now().date_naive();
    let report = register::check(&files.entries, &files.packs, today);
    for finding in &report.findings {
        let label = match finding.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let place = if finding.file.is_empty() {
            dir.display().to_string()
        } else {
            format!("{}/{}", dir.display(), finding.file)
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
