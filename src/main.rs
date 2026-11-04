// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: FSL-1.1-ALv2

use std::process::ExitCode;

use clap::Parser;
use ghostsoap::Cli;

fn main() -> ExitCode {
    ghostsoap::cli::run(Cli::parse())
}
