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
        #[arg(long, conflicts_with = "relay")]
        no_relay: bool,
        /// Use this self-hosted iroh-relay instead of n0's hosted relays.
        ///
        /// Pass an HTTPS URL like `https://relay.yourdomain.com`. Joiners
        /// don't need to specify `--relay`: the host's relay URL is baked
        /// into the printed ticket, and joiners pick it up automatically.
        #[arg(long, value_name = "URL")]
        relay: Option<String>,
    },
    /// Join a Room and run the chat REPL. Long-running.
    Chat {
        /// Room code (`cc1-…`) shared out-of-band by the Host.
        ticket: String,
        /// Disable n0's hosted relay servers; LAN-direct only.
        #[arg(long, conflicts_with = "relay")]
        no_relay: bool,
        /// Use this self-hosted iroh-relay (HTTPS URL). If unset, the
        /// joiner uses n0's defaults; the ticket's relay info still drives
        /// connection establishment.
        #[arg(long, value_name = "URL")]
        relay: Option<String>,
    },
    /// Sanity-check the cc-connect installation.
    Doctor,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Command::Host { no_relay, relay } => host::run(no_relay, relay.as_deref()),
        Command::Chat { ticket, no_relay, relay } => {
            chat::run(&ticket, no_relay, relay.as_deref())
        }
        Command::Doctor => doctor::run(),
    }
}
