use clap::{CommandFactory, Parser};
use ghostsoap::Cli;

fn main() -> std::io::Result<()> {
    Cli::parse();
    Cli::command().print_help()
}
