//! GhostSoap: prepare Australian Privacy Act requests to data brokers and track the replies.

use clap::Parser;

/// Prepare Australian Privacy Act requests to data brokers and track the replies.
///
/// GhostSoap is at milestone M0: it has no commands yet.
#[derive(Debug, Parser)]
#[command(name = "ghostsoap", version, about, long_about = None)]
pub struct Cli {}
