//! Command-line interface.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{CommandFactory, Parser, Subcommand};

use crate::core::embedded;
use crate::core::register::{self, RegisterFiles, Severity};
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
        /// Check a register directory, such as a working copy's `registry/au`, instead of
        /// the register built into this binary. Packs are read from its `packs` subdirectory.
        #[arg(long)]
        path: Option<PathBuf>,
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
        }) => match path {
            None => registry_check(&embedded::register(), "built-in register"),
            Some(dir) => match io::register::load(&dir) {
                Ok(files) => registry_check(&files, &dir.display().to_string()),
                Err(e) => {
                    eprintln!("error: cannot read {}: {e}", dir.display());
                    ExitCode::FAILURE
                }
            },
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
