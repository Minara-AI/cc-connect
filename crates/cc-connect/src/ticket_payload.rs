//! The ticket *payload* — opaque bytes wrapped by `cc-connect-core::ticket`.
//!
//! Per PROTOCOL.md §3, the cc-connect Room code is `cc1-` + base32 +
//! CRC32 of these bytes. The bytes themselves are postcard-encoded
//! `{ topic: TopicId, peers: Vec<EndpointAddr> }`, matching the
//! convention from the iroh-gossip `examples/chat.rs` so a non-cc-connect
//! iroh-gossip client could in principle read our peer list.

use anyhow::{Context, Result};
use iroh::EndpointAddr;
use iroh_gossip::proto::TopicId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketPayload {
    pub topic: TopicId,
    pub peers: Vec<EndpointAddr>,
}

impl TicketPayload {
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        postcard::to_stdvec(self).context("postcard encode TicketPayload")
    }

    #[allow(dead_code)] // used by `chat` subcommand once implemented
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        postcard::from_bytes(bytes).context("postcard decode TicketPayload")
    }
}
