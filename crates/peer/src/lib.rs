//! Talking to another device.
//!
//! [`qurb_sync`] decides what two devices should do about their differences and
//! [`qurb_engine`] carries it out locally. This crate is how the two devices
//! reach each other in the first place.
//!
//! ```text
//!   PeerServer   serves a store, read-only, to peers it recognises
//!   PeerClient   asks: what do you have, what is this file made of, send me a chunk
//!   NetworkSource  plugs the client into the engine's ContentSource seam
//! ```
//!
//! # Transfer is incremental
//!
//! Fetching a file asks for its *manifest* (the list of chunks it is made
//! of) and then only for the chunks this device does not already hold. A large
//! file with a small edit shares nearly all its chunks with the copy already
//! here, so nearly nothing crosses the wire. This is the point of the content-
//! defined chunking in [`qurb_storage`], finally realised over a network.
//!
//! # Identity
//!
//! Both ends present a self-signed certificate and check the other against a
//! fingerprint given in advance, and both verify the handshake signature, so a
//! peer must hold the matching private key rather than merely replay a public
//! certificate.
//!
//! **Pairing is not built.** Deciding which fingerprint to expect (the QR-code
//! exchange in the architecture) is Phase 3. Until then the caller supplies it,
//! which is why [`PeerClient::connect`] demands a fingerprint rather than
//! offering a way to skip one.
//!
//! See ../../docs/CODEBASE.md for where this sits in the system.

pub mod base32;
pub mod client;
pub mod connect;
pub mod convert;
pub mod disk;
pub mod error;
pub mod identity;
pub mod kept;
pub mod limit;
pub mod local;
pub mod nat;
pub mod nostr_path;
pub mod openings;
pub mod pairing;
pub mod pause;
pub mod ranged;
pub mod server;
pub mod short;
pub mod source;
pub mod tls;
pub mod trees;
#[cfg(feature = "tor")]
pub mod tor;
pub mod wire;

pub use client::PeerClient;

/// This computer's onion service, once it is published (`tor::host`): what
/// it answers [`wire::Request::Onion`] with. None on a device hosting none.
static HOSTED_ONION: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Codes this person's phones handed over for their Google accounts, waiting
/// for the computer to exchange them (`Request::DriveCode`).
static DRIVE_CODES: std::sync::Mutex<Vec<(qurb_sync::DeviceId, String)>> = std::sync::Mutex::new(Vec::new());

/// Told when a code is handed over, so the computer exchanges it at once
/// rather than on its next mailbox round (the owner, 2026-11-14).
static DRIVE_CODE_GIVEN: std::sync::LazyLock<tokio::sync::Notify> = std::sync::LazyLock::new(tokio::sync::Notify::new);

pub fn drive_code_given(from: qurb_sync::DeviceId, code: String) {
    if let Ok(mut codes) = DRIVE_CODES.lock() {
        codes.push((from, code));
    }
    DRIVE_CODE_GIVEN.notify_one();
}

/// Until a code has been handed over since last asked.
pub async fn drive_code_handed() {
    DRIVE_CODE_GIVEN.notified().await
}

/// The codes given since last asked, each once.
pub fn drive_codes_taken() -> Vec<(qurb_sync::DeviceId, String)> {
    DRIVE_CODES.lock().map(|mut codes| std::mem::take(&mut *codes)).unwrap_or_default()
}

/// Google accounts this computer was signed in to and Google no longer
/// takes the sign-in of (expired, revoked): a phone of that account asks
/// ([`wire::Request::DriveWanted`]) and hands a fresh one over. The
/// laptop's sign-in for the owner's account, made while the app was in
/// Google's testing, ran out after seven days, and nothing said so but a log
/// line every round (2026-11-20).
static DRIVE_WANTED: std::sync::Mutex<std::collections::BTreeSet<String>> =
    std::sync::Mutex::new(std::collections::BTreeSet::new());

/// Google refused this account's sign-in: asked of the phones.
pub fn drive_sign_in_wanted(account: &str) {
    if let Ok(mut wanted) = DRIVE_WANTED.lock() {
        if wanted.insert(account.to_lowercase()) {
            tracing::info!(%account, "Google no longer takes this account's sign-in; asking its phone for a new one");
        }
    }
}

/// The account works again, or was signed in afresh.
pub fn drive_sign_in_kept(account: &str) {
    if let Ok(mut wanted) = DRIVE_WANTED.lock() {
        wanted.remove(&account.to_lowercase());
    }
}

pub fn drive_sign_in_is_wanted(account: &str) -> bool {
    DRIVE_WANTED.lock().is_ok_and(|wanted| wanted.contains(&account.to_lowercase()))
}

pub fn onion_hosted() -> Option<String> {
    HOSTED_ONION.lock().ok().and_then(|o| o.clone())
}

#[cfg_attr(not(feature = "tor-host"), allow(dead_code))]
pub(crate) fn set_onion_hosted(onion: String) {
    if let Ok(mut held) = HOSTED_ONION.lock() {
        *held = Some(onion);
    }
}
pub use connect::{relay_address_ok, relay_for, resolve_relay, Connector, Finding, Need, RelayAt};
pub use error::{Error, Result};
pub use identity::{Fingerprint, Identity};
pub use local::Beacon;
pub use nat::{NatBehaviour, Reflexive};
pub use ranged::Ranged;
pub use pairing::{accept, join, pairing_number, visit, who, Asking, Invite, Ours, Paired, PairingHost};
pub use server::{accepted_fingerprints, trusted_fingerprints, Generation, PeerServer, Served, ServerStats};
pub use kept::{fetch_kept, learn_kept};
pub use source::{check_holders, confirm_kept, learn_profile, report_holdings, NetworkSource};
pub use wire::{Request, Response};
