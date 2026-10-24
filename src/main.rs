use std::process::ExitCode;

use clap::Parser;
use ghostsoap::Cli;

fn main() -> ExitCode {
    ghostsoap::cli::run(Cli::parse())
}
