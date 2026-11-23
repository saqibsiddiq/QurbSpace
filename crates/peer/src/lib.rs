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

/// A direct link a phone of this person's offered ([`wire::Request::DirectLink`],
/// decision 0092), for the computer to join.
#[derive(Debug, Clone)]
pub struct DirectLinkOffer {
    pub from: qurb_sync::DeviceId,
    pub how: u8,
    pub name: String,
    pub password: String,
    pub security: String,
    pub device: String,
}

/// The network of a direct link this computer has joined, while it is up:
/// tried first for the device that made it (`Connector::race`).
#[derive(Debug, Clone, Copy)]
pub struct DirectNet {
    pub address: std::net::Ipv4Addr,
    pub prefix: u8,
    /// The phone that made it.
    pub gateway: std::net::IpAddr,
}

impl DirectNet {
    pub fn holds(&self, ip: std::net::IpAddr) -> bool {
        let std::net::IpAddr::V4(ip) = ip else { return false };
        let mask = u32::MAX.checked_shl(32 - u32::from(self.prefix.min(32))).unwrap_or(0);
        u32::from(ip) & mask == u32::from(self.address) & mask
    }
}

static DIRECT_NET: std::sync::Mutex<Option<DirectNet>> = std::sync::Mutex::new(None);
static DIRECT_CHANGED: std::sync::LazyLock<tokio::sync::Notify> = std::sync::LazyLock::new(tokio::sync::Notify::new);

/// Where each device last connected to this one from, by its fingerprint:
/// on a direct link, the only address it can be dialled at.
static HEARD_FROM: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<Fingerprint, std::net::SocketAddr>>> =
    std::sync::LazyLock::new(Default::default);

pub(crate) fn heard_from(peer: Fingerprint, at: std::net::SocketAddr) {
    if let Ok(mut heard) = HEARD_FROM.lock() {
        heard.insert(peer, at);
    }
}

pub(crate) fn last_heard_from(peer: &Fingerprint) -> Option<std::net::SocketAddr> {
    HEARD_FROM.lock().ok().and_then(|heard| heard.get(peer).copied())
}

/// A direct link joined, or ended (None): connections kept to its device
/// are dropped by the daemon, to be made again over the link or without it.
/// Where devices were heard from on the link before is forgotten: a new link
/// gives them new addresses, and a stale one would pass for the device
/// having connected (`heard_on_direct_link`).
pub fn set_direct_link(net: Option<DirectNet>) {
    if let Ok(mut held) = DIRECT_NET.lock() {
        let before = std::mem::replace(&mut *held, net);
        if let Ok(mut heard) = HEARD_FROM.lock() {
            heard.retain(|_, at| ![before, net].iter().flatten().any(|n| n.holds(at.ip())));
        }
    }
    DIRECT_CHANGED.notify_one();
}

/// Whether a device has connected to this one over the direct link up now:
/// on the phone that made it, the computer that joined is on it with an
/// address and has dialled in, so the phone can dial it there. Joining the
/// Wi-Fi is not that: the phone started a pass on it before the computer
/// had an address, dialled where the computer had been, and fetched nothing
/// (2026-11-22, five links in a row).
pub fn heard_on_direct_link() -> bool {
    let Some(net) = direct_link() else { return false };
    HEARD_FROM.lock().map(|heard| heard.values().any(|at| net.holds(at.ip()))).unwrap_or(false)
}

pub fn direct_link() -> Option<DirectNet> {
    DIRECT_NET.lock().ok().and_then(|n| *n)
}

/// Until a direct link is joined or ends.
pub async fn direct_link_changed() {
    DIRECT_CHANGED.notified().await
}

/// Whether this program joins direct links, and how it is told of one: set
/// by the computer's daemon where it can (`qurb_cli::direct`). Unset, offers
/// are declined.
static DIRECT_LINKS: std::sync::OnceLock<tokio::sync::mpsc::UnboundedSender<DirectLinkOffer>> = std::sync::OnceLock::new();

/// Whether an offer would be taken now, asked as it arrives so the phone
/// hears no at once and does not wait for a computer that will not come.
static DIRECT_TAKES: std::sync::OnceLock<fn(u8) -> bool> = std::sync::OnceLock::new();
/// This computer's Wi-Fi Direct address, told a phone that meets it
/// ([`wire::DIRECT_MEET`]).
static DIRECT_ADDRESS: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Take direct links offered from here on, as they come, when `takes` says
/// so at the time for that kind of link; `address` this computer's Wi-Fi
/// Direct address, if it meets phones.
pub fn take_direct_links(
    takes: fn(u8) -> bool,
    address: Option<String>,
) -> Option<tokio::sync::mpsc::UnboundedReceiver<DirectLinkOffer>> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    DIRECT_TAKES.set(takes).ok()?;
    if let Some(address) = address {
        let _ = DIRECT_ADDRESS.set(address);
    }
    DIRECT_LINKS.set(tx).ok().map(|_| rx)
}

/// This computer's Wi-Fi Direct address, where it meets phones.
pub(crate) fn direct_address() -> Option<String> {
    DIRECT_ADDRESS.get().cloned()
}

/// Offered: passed on, if this program takes it now. Whether it does.
pub(crate) fn direct_link_offered(offer: DirectLinkOffer) -> bool {
    let how = offer.how;
    DIRECT_TAKES.get().is_some_and(|takes| takes(how)) && DIRECT_LINKS.get().is_some_and(|tx| tx.send(offer).is_ok())
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

#[cfg(test)]
mod tests {
    use super::*;

    // The direct link is one for the process: the only test that sets it.
    #[test]
    fn heard_on_the_link_only_once_dialled_in_over_it() {
        let link = DirectNet {
            address: std::net::Ipv4Addr::new(192, 168, 49, 1),
            prefix: 24,
            gateway: std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
        };
        let computer = Fingerprint::from_bytes([7; 32]);
        set_direct_link(None);
        assert!(!heard_on_direct_link(), "no link, nothing heard on one");

        // Heard at home before the link: not on it.
        heard_from(computer, "192.168.1.5:47474".parse().unwrap());
        set_direct_link(Some(link));
        assert!(!heard_on_direct_link(), "joined the Wi-Fi, not yet dialled in");

        heard_from(computer, "192.168.49.35:47474".parse().unwrap());
        assert!(heard_on_direct_link());

        // The next link begins with nothing heard on it.
        set_direct_link(None);
        set_direct_link(Some(link));
        assert!(!heard_on_direct_link(), "a stale address is not the computer having connected");
        assert_eq!(last_heard_from(&computer), None);
        set_direct_link(None);
    }
}
