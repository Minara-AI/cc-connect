//! cc-connect — `host`, `chat`, `doctor`.
//!
//! Subcommands track PROTOCOL.md §3 (Ticket), §6.1–§6.2 (transport),
//! §7.1 (Hook install path), and §8 (active-rooms protocol).

use anyhow::Result;
use clap::{Parser, Subcommand};

mod backfill;
mod chat;
mod doctor;
mod host;
mod ticket_payload;

#[derive(Parser)]
#[command(name = "cc-connect", version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new Room and print its Ticket. Exits after printing.
    Host {
        /// Disable n0's hosted relay servers; LAN-direct only.
        ///
        /// Useful for offline / pure-LAN demos where both peers are on the
        /// same network. Joiners MUST also use `--no-relay` and must be
        /// reachable directly (no NAT between them).
        #[arg(long)]
        no_relay: bool,
    },
    /// Join a Room and run the chat REPL. Long-running.
    Chat {
        /// Room code (`cc1-…`) shared out-of-band by the Host.
        ticket: String,
        /// Disable n0's hosted relay servers; LAN-direct only.
        #[arg(long)]
        no_relay: bool,
    },
    /// Sanity-check the cc-connect installation.
    Doctor,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Command::Host { no_relay } => host::run(no_relay),
        Command::Chat { ticket, no_relay } => chat::run(&ticket, no_relay),
        Command::Doctor => doctor::run(),
    }
}
