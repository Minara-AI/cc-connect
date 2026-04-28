//! `cc-connect chat <ticket>` — join a Room, run a stdin REPL, and persist
//! every Message (sent + received) to the local log so the Hook can inject
//! them into Claude Code on the next prompt.
//!
//! Implements (most of) the join-side of PROTOCOL.md §3, §6.1, §8.
//!
//! v0.1 simplifications:
//!   - **No Backfill RPC** in this iteration — late-joiners see no history.
//!     The chat REPL prints `[chatroom] (joined late, no history)` once.
//!   - **No multi-Room subscription per Session** — one chat process per Room.
//!   - The hook contract (PID-file lifecycle) is honoured: PID is written
//!     *after* gossip join and removed on clean exit.
//!
//! Magic-moment flow this enables: Bob types in his `chat` REPL → his
//! Message goes via gossip → Alice's `chat` listener appends to her log →
//! Alice's next Claude prompt fires the Hook → Hook reads her log → injects
//! into Claude. No human action on Alice's side.

use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use cc_connect_core::{
    identity::Identity,
    log_io,
    message::Message,
    ticket::decode_room_code,
};
use futures_lite::StreamExt;
use iroh::{
    address_lookup::memory::MemoryLookup, endpoint::presets, endpoint::RelayMode, Endpoint,
    PublicKey, RelayMap, SecretKey,
};
use iroh_blobs::{store::mem::MemStore, BlobsProtocol, Hash};
use iroh_gossip::{
    api::Event,
    net::{Gossip, GOSSIP_ALPN},
    proto::TopicId,
};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::backfill::{try_backfill_from, BackfillHandler, BackfillOutcome, BACKFILL_ALPN};
use crate::ticket_payload::TicketPayload;

pub fn run(ticket_str: &str, no_relay: bool, relay: Option<&str>) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("build tokio runtime")?;
    rt.block_on(run_async(ticket_str, no_relay, relay))
}

async fn run_async(ticket_str: &str, no_relay: bool, relay: Option<&str>) -> Result<()> {
    // 1. Decode ticket → topic + bootstrap peers.
    let payload_bytes = decode_room_code(ticket_str)
        .with_context(|| format!("decode room code: {ticket_str:.20}…"))?;
    let payload = TicketPayload::from_bytes(&payload_bytes)?;
    let topic = payload.topic;
    let bootstrap_peers = payload.peers;
    let topic_id_hex = topic_to_hex(&topic);

    // 2. Load Identity, derive iroh SecretKey (PROTOCOL.md §2 binding).
    let identity = load_identity()?;
    let pubkey_string = identity.pubkey_string();
    let secret_key = SecretKey::from_bytes(&identity.seed_bytes());

    // 3. Build endpoint with a MemoryLookup so we can register the bootstrap
    //    peers' addresses before subscribing.
    let memory_lookup = MemoryLookup::new();
    for peer in &bootstrap_peers {
        memory_lookup.add_endpoint_info(peer.clone());
    }
    let mut builder = Endpoint::builder(presets::N0)
        .secret_key(secret_key)
        .address_lookup(memory_lookup.clone());
    if no_relay {
        builder = builder.relay_mode(RelayMode::Disabled);
    } else if let Some(url) = relay {
        let map = RelayMap::try_from_iter([url])
            .map_err(|e| anyhow!("RELAY_URL_INVALID: {url}: {e}"))?;
        builder = builder.relay_mode(RelayMode::Custom(map));
    }
    let endpoint = builder.bind().await.context("bind iroh endpoint")?;

    // 4. Spawn gossip + iroh-blobs MemStore, then a Router that accepts
    //    gossip / backfill / blobs ALPNs. The MemStore caches every blob
    //    we /drop so other peers can fetch them; backfill targets that
    //    same store.
    let gossip = Gossip::builder().spawn(endpoint.clone());
    let store = MemStore::new();
    let blobs_proto = BlobsProtocol::new(&store, None);
    let log_path = log_path_for(&topic_id_hex);
    let backfill_handler = BackfillHandler::new(log_path.clone());
    let _router = iroh::protocol::Router::builder(endpoint.clone())
        .accept(GOSSIP_ALPN, gossip.clone())
        .accept(BACKFILL_ALPN, backfill_handler)
        .accept(iroh_blobs::ALPN, blobs_proto)
        .spawn();

    // 5. Wait until online so subscription can talk to peers. Skip when
    //    relay is disabled — `online()` blocks on a relay home that
    //    won't ever land in that mode.
    if !no_relay {
        endpoint.online().await;
    }

    // 6. Subscribe to the gossip topic with the bootstrap peers as initial
    //    contacts. Returns a GossipTopic we split into (sender, receiver).
    let peer_ids: Vec<_> = bootstrap_peers.iter().map(|p| p.id).collect();
    let bootstrap_count = peer_ids.len();
    let topic_handle = gossip.subscribe_and_join(topic, peer_ids).await?;
    let (sender, mut receiver) = topic_handle.split();

    // 7. Backfill from the first bootstrap peer (PROTOCOL.md §6.2). Single
    //    attempt with a 5s timeout — if it fails, surface the marker and
    //    proceed (no v0.1 retry across peers; that's a v0.2 polish).
    let backfill_marker = if let Some(first_peer) = bootstrap_peers.first() {
        if first_peer.id == endpoint.id() {
            // We're the only peer in the room (host or self-only join);
            // no one to ask. Skip silently.
            None
        } else {
            let files_dir = files_dir_for(&topic_id_hex);
            match try_backfill_from(
                &endpoint,
                &store,
                first_peer,
                None,
                &log_path,
                &files_dir,
            )
            .await
            {
                BackfillOutcome::Filled { appended } if appended > 0 => {
                    Some(format!(
                        "[chatroom] (backfilled {appended} message{} from peer)",
                        if appended == 1 { "" } else { "s" }
                    ))
                }
                BackfillOutcome::Filled { .. } | BackfillOutcome::Empty => None,
                BackfillOutcome::Timeout => {
                    Some("[chatroom] (joined late, no history available)".to_string())
                }
                BackfillOutcome::Failed(msg) => {
                    eprintln!("[chat] backfill failed: {msg}");
                    Some("[chatroom] (joined late, no history available)".to_string())
                }
            }
        }
    } else {
        None
    };

    // 8. Write active-rooms PID file *after* bootstrap+backfill complete
    //    (PROTOCOL.md §8 + ADR-0003). Cleanup via the guard's Drop.
    let pid_path = pid_file_path(&topic_id_hex)?;
    let _pid_guard = PidFileGuard::new(&pid_path)?;

    // 9. Open the local log (append + read-back) for the REPL half.
    let mut send_log = log_io::open_or_create_log(&log_path)?;

    println!();
    println!("Joined room: {} (peers: {})", &topic_id_hex[..12], bootstrap_count);
    println!("You are:     {}", &pubkey_string[..16]);
    if let Some(marker) = &backfill_marker {
        println!("{marker}");
    }
    println!("Type to send. Ctrl-C / EOF to leave.");
    println!();

    // 9. Spawn gossip listener task. It writes incoming Messages to the same
    //    log file (with its own File handle); fcntl + single-syscall write
    //    keep concurrent appends safe. For file_drop Messages it dials the
    //    author's NodeId over iroh-blobs to fetch the bytes, then exports
    //    them under the per-Room `files/` directory so the hook can
    //    `@file:` the local path on the next prompt.
    let listener_log_path = log_path.clone();
    let listener_files_dir = files_dir_for(&topic_id_hex);
    let listener_store = store.clone();
    let listener_endpoint = endpoint.clone();
    let our_pubkey = pubkey_string.clone();
    let listener_handle = tokio::task::spawn(async move {
        let mut listener_log = match log_io::open_or_create_log(&listener_log_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[chat] open listener log failed: {e:#}");
                return;
            }
        };
        while let Some(event) = receiver.next().await {
            let event = match event {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("[chat] gossip stream error: {e}");
                    continue;
                }
            };
            let payload: &[u8] = match &event {
                Event::Received(m) => m.content.as_ref(),
                _ => continue,
            };
            let msg = match Message::from_wire_bytes(payload) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("[chat] dropped malformed Message: {e}");
                    continue;
                }
            };
            // Defence: don't echo our own broadcasts back into the log
            // (gossip can mirror them).
            if msg.author == our_pubkey {
                continue;
            }
            // For file_drop, fetch the announced blob from the author's
            // NodeId and export it locally.
            if msg.kind == cc_connect_core::message::KIND_FILE_DROP {
                if let Err(e) = fetch_and_export_blob(
                    &listener_store,
                    &listener_endpoint,
                    &msg,
                    &listener_files_dir,
                )
                .await
                {
                    eprintln!("[chat] file_drop blob fetch failed for {}: {e:#}", msg.id);
                    continue;
                }
            }
            // Persist for the Hook to inject into Claude.
            if let Err(e) = log_io::append(&mut listener_log, &msg) {
                eprintln!("[chat] append incoming Message failed: {e:#}");
                continue;
            }
            // Tiny REPL display: short author + body, single-line.
            let nick_short: String = msg.author.chars().take(8).collect();
            let line: String = if msg.kind == cc_connect_core::message::KIND_FILE_DROP {
                format!("dropped {}", msg.body)
            } else {
                msg.body.replace(['\n', '\r', '\t'], " ")
            };
            println!("[{nick_short}] {line}");
        }
    });

    // 10. REPL: read stdin lines, build canonical Messages, append + broadcast.
    let mut stdin_reader = tokio::io::BufReader::new(tokio::io::stdin());
    let mut line = String::new();
    use tokio::io::AsyncBufReadExt;

    let repl_result: Result<()> = loop {
        line.clear();
        let n = match tokio::select! {
            r = stdin_reader.read_line(&mut line) => r,
            _ = tokio::signal::ctrl_c() => {
                println!("\n[chat] Ctrl-C — leaving room");
                break Ok(());
            }
        } {
            Ok(n) => n,
            Err(e) => break Err(anyhow!("read stdin: {e}")),
        };
        if n == 0 {
            // EOF.
            break Ok(());
        }
        let body = line.trim_end_matches(['\n', '\r']).to_string();
        if body.is_empty() {
            continue;
        }

        let msg = if let Some(path_str) = body.strip_prefix("/drop ") {
            // v0.2 file drop: hash the file into our local MemStore, then
            // broadcast a Message announcing (hash, size). Peers fetch the
            // bytes from us over the iroh-blobs ALPN.
            match build_file_drop(&store, path_str.trim(), &pubkey_string, &topic_id_hex).await {
                Ok(m) => {
                    println!(
                        "[chat] dropped {} ({} bytes)",
                        m.body,
                        m.blob_size.unwrap_or(0)
                    );
                    m
                }
                Err(e) => {
                    eprintln!("[chat] /drop failed: {e:#}");
                    continue;
                }
            }
        } else if body.starts_with('/') {
            eprintln!(
                "[chat] unknown slash command. Available: `/drop <path>`. Type plain text to chat."
            );
            continue;
        } else {
            Message::new(&new_ulid(), pubkey_string.clone(), now_ms(), body)
                .context("build Message")?
        };

        // Local log first, then broadcast — if the broadcast fails the local
        // record is intact (PROTOCOL.md §6.1 step 3 ordering).
        if let Err(e) = log_io::append(&mut send_log, &msg) {
            eprintln!("[chat] append outgoing failed: {e:#}");
            continue;
        }
        let bytes = msg.to_canonical_json()?;
        if let Err(e) = sender.broadcast(Bytes::from(bytes)).await {
            eprintln!("[chat] broadcast failed: {e:#}");
        }
    };

    // 11. Cleanup. The pid_guard's Drop already removes the PID file.
    listener_handle.abort();
    drop(sender);
    drop(gossip);
    drop(endpoint);
    repl_result
}

/// Identity loader matching `host` / PROTOCOL.md §2.
fn load_identity() -> Result<Identity> {
    let path = identity_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create_dir_all {}", parent.display()))?;
        let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
    }
    Identity::generate_or_load(&path)
}

fn identity_path() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME not set")?;
    Ok(PathBuf::from(home).join(".cc-connect").join("identity.key"))
}

fn log_path_for(topic_id_hex: &str) -> PathBuf {
    rooms_dir(topic_id_hex).join("log.jsonl")
}

fn files_dir_for(topic_id_hex: &str) -> PathBuf {
    rooms_dir(topic_id_hex).join("files")
}

fn rooms_dir(topic_id_hex: &str) -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"));
    home.join(".cc-connect").join("rooms").join(topic_id_hex)
}

/// Read a file from `path`, hash it into the iroh-blobs `store`, then build
/// a file_drop Message announcing the resulting hash + size. Also copies a
/// plain-bytes copy under `<rooms_dir>/<topic>/files/<id>-<filename>` so our
/// own hook can `@file:` the local path on the next prompt without making a
/// roundtrip.
async fn build_file_drop(
    store: &MemStore,
    path_str: &str,
    author_pubkey: &str,
    topic_id_hex: &str,
) -> Result<Message> {
    let path = std::path::Path::new(path_str);
    let abs_path = std::path::absolute(path)
        .with_context(|| format!("absolute path of {path_str}"))?;
    let metadata = std::fs::metadata(&abs_path)
        .with_context(|| format!("stat {}", abs_path.display()))?;
    let size = metadata.len();
    if size > cc_connect_core::message::FILE_DROP_MAX_BYTES {
        return Err(anyhow!(
            "BLOB_TOO_LARGE: {} exceeds the {} byte cap",
            size,
            cc_connect_core::message::FILE_DROP_MAX_BYTES
        ));
    }
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow!("FILENAME_INVALID: cannot extract filename from {path_str:?}"))?
        .to_string();

    // Import into the iroh-blobs store. The returned tag pins the blob in
    // the store so peers can fetch it for the lifetime of this process.
    let tag = store
        .blobs()
        .add_path(&abs_path)
        .await
        .with_context(|| format!("add_path {}", abs_path.display()))?;
    let hash_hex = tag.hash.to_string();

    let id = new_ulid();
    let msg = Message::new_file_drop(
        &id,
        author_pubkey.to_string(),
        now_ms(),
        filename,
        hash_hex,
        size,
    )
    .context("build file_drop Message")?;

    // Save locally so our own future hook fires can `@file:` it without a
    // round-trip through the blob store.
    let files_dir = files_dir_for(topic_id_hex);
    copy_local_to_files_dir(&msg, &abs_path, &files_dir)
        .context("save local copy for hook")?;
    Ok(msg)
}

/// Copy a freshly-dropped local file under
/// `<files_dir>/<id>-<filename>` with 0600 perms. Idempotent.
fn copy_local_to_files_dir(
    msg: &Message,
    src: &std::path::Path,
    files_dir: &std::path::Path,
) -> Result<()> {
    std::fs::create_dir_all(files_dir)
        .with_context(|| format!("create_dir_all {}", files_dir.display()))?;
    let _ = std::fs::set_permissions(files_dir, std::fs::Permissions::from_mode(0o700));
    let target = files_dir.join(format!("{}-{}", msg.id, msg.body));
    if target.exists() {
        return Ok(());
    }
    std::fs::copy(src, &target)
        .with_context(|| format!("copy {} → {}", src.display(), target.display()))?;
    let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600));
    Ok(())
}

/// Fetch a file_drop's blob from the author's NodeId and export it under
/// `<files_dir>/<id>-<filename>`. Idempotent — skips the download if the
/// destination file already exists.
pub(crate) async fn fetch_and_export_blob(
    store: &MemStore,
    endpoint: &Endpoint,
    msg: &Message,
    files_dir: &std::path::Path,
) -> Result<()> {
    let hash_hex = msg
        .blob_hash
        .as_deref()
        .ok_or_else(|| anyhow!("BLOB_HASH_MISSING for {}", msg.id))?;
    let hash = Hash::from_str(hash_hex)
        .map_err(|e| anyhow!("BLOB_HASH_PARSE: {hash_hex} ({e})"))?;
    let author_id = PublicKey::from_str(&msg.author)
        .map_err(|e| anyhow!("AUTHOR_PARSE: {} ({e})", msg.author))?;

    std::fs::create_dir_all(files_dir)
        .with_context(|| format!("create_dir_all {}", files_dir.display()))?;
    let _ = std::fs::set_permissions(files_dir, std::fs::Permissions::from_mode(0o700));
    let target = files_dir.join(format!("{}-{}", msg.id, msg.body));
    if target.exists() {
        return Ok(());
    }

    let downloader = store.downloader(endpoint);
    downloader
        .download(hash, Some(author_id))
        .await
        .with_context(|| format!("download blob {hash}"))?;
    store
        .blobs()
        .export(hash, &target)
        .await
        .with_context(|| format!("export {} → {}", hash, target.display()))?;
    let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600));
    Ok(())
}

fn pid_file_path(topic_id_hex: &str) -> Result<PathBuf> {
    let uid = rustix::process::geteuid().as_raw();
    let dir = std::env::temp_dir()
        .join(format!("cc-connect-{uid}"))
        .join("active-rooms");
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("create_dir_all {}", dir.display()))?;
    let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    Ok(dir.join(format!("{topic_id_hex}.active")))
}

fn topic_to_hex(topic: &TopicId) -> String {
    let bytes = topic.as_bytes();
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
    }
    out
}

fn new_ulid() -> String {
    ulid::Ulid::new().to_string()
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Owns the active-rooms PID file for the duration of `chat`.
struct PidFileGuard {
    path: PathBuf,
}

impl PidFileGuard {
    fn new(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
            let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
        }
        let pid = std::process::id().to_string();
        std::fs::write(path, pid).with_context(|| format!("write PID file {}", path.display()))?;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
        Ok(Self {
            path: path.to_path_buf(),
        })
    }
}

impl Drop for PidFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
