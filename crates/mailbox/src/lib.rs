//! A person's devices leaving each other, in a cloud folder they share, what
//! they cannot say live (2026-11-13): the path that works when nothing else
//! does, and when the other device is asleep or off.
//!
//! The owner, on making QurbSpace work from anywhere: "can we use services
//! like google drive ... to make the system always work", then "include
//! backups too". Live paths -- direct, the relay, Tor, the Nostr relays --
//! need both devices awake at once and a way through between them. A shared
//! folder needs neither: one device posts, the other collects when it can.
//!
//! # What is posted
//!
//! Nothing new is said this way. A device answers, offline, the requests the
//! other would make live (`qurb_peer::server::answer_offline`), so every rule
//! about what each device is shown and what a report means holds here too:
//!
//! - a **list**: its files as the other device would be shown them;
//! - **packs**: the contents of files on that list the other device is not
//!   known to hold, a few megabytes each;
//! - **receipts**: the contents a device took, answered as `Got` is live.
//!
//! The receiving device plans from the list as it would from one fetched
//! live, applies only what it has the contents for, and posts its receipt;
//! the sender records the deliveries and its packs are gone. While both are
//! awake this runs as a pipe, seconds behind; while one sleeps the folder
//! fills up to [`CAP`] and waits.
//!
//! # What the provider sees
//!
//! Each letter is sealed with a key derived from the person's master key
//! (`Purpose::Mailbox`): the provider holds bytes it cannot read, named by
//! random device identifiers, and their sizes and times. Only the person's
//! own devices hold the key: another person's never does, and so cannot use
//! this path (decision 0060's guests stay on the live ones).

pub mod drive;

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use qurb_storage::db::TrustedPeer;
use qurb_storage::Store;
use qurb_sync::{Action, Content, DeviceId, FileVersion};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The most a device leaves waiting for another: once this much is in the
/// folder from it, it posts no more packs until some are collected. The
/// owner's figure, 2026-11-13.
pub const CAP: u64 = 2 << 30;

/// The size a pack is made up to: one upload each, under Google Drive's
/// limit for uploading a file and its name in one request (5 MB).
pub const PACK: usize = 4 << 20;

/// The packs read under the engine at a time, before it is let go to send
/// them: sixteen megabytes.
const ROUND: usize = 4;

/// The largest file sent this way: a larger one is read whole into memory to
/// be packed, which a phone should not do, and waits for a live path.
pub const LARGEST: u64 = 64 << 20;

/// What the mailbox needs of the engine, held only while it is used: the
/// store to read and record, the engine to plan and apply.
pub trait Holds {
    fn store<R>(&self, f: impl FnOnce(&mut Store) -> R) -> R;
    fn engine<R>(&self, f: impl FnOnce(&mut qurb_engine::Engine) -> R) -> R;
}

/// An engine owned outright, as the computer's daemon owns its own.
pub struct Owned<'a>(pub std::sync::Mutex<&'a mut qurb_engine::Engine>);

impl Holds for Owned<'_> {
    fn store<R>(&self, f: impl FnOnce(&mut Store) -> R) -> R {
        f(self.0.lock().expect("engine").store_mut())
    }
    fn engine<R>(&self, f: impl FnOnce(&mut qurb_engine::Engine) -> R) -> R {
        f(&mut self.0.lock().expect("engine"))
    }
}

/// Contents posted and not yet taken are posted again after this long: the
/// pack may have been lost, or taken by a device that never said.
const POST_AGAIN: u64 = 24 * 60 * 60;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Place(String),
    #[error("not signed in to the cloud folder")]
    NotSignedIn,
    #[error("storage: {0}")]
    Storage(#[from] qurb_storage::Error),
    #[error("peer: {0}")]
    Peer(#[from] qurb_peer::Error),
    #[error("engine: {0}")]
    Engine(#[from] qurb_engine::Error),
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
}

pub type Result<T> = std::result::Result<T, Error>;

fn io(path: &Path) -> impl Fn(std::io::Error) -> Error + '_ {
    move |source| Error::Io { path: path.to_path_buf(), source }
}

/// One letter in the folder.
#[derive(Debug, Clone)]
pub struct Letter {
    /// What the place calls it: a file name, or Google Drive's file id.
    pub id: String,
    pub name: String,
    pub size: u64,
}

/// Where letters are left.
pub enum Place {
    /// A folder on this machine: for tests, and for any folder both devices
    /// see another way.
    Folder(PathBuf),
    /// The hidden app folder in a Google account's Drive.
    Drive(drive::Drive),
}

impl Place {
    pub async fn list(&self) -> Result<Vec<Letter>> {
        match self {
            Place::Folder(dir) => {
                let mut out = Vec::new();
                for entry in std::fs::read_dir(dir).map_err(io(dir))?.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with(PREFIX) && !name.ends_with(".part") {
                        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        out.push(Letter { id: name.clone(), name, size });
                    }
                }
                Ok(out)
            }
            Place::Drive(drive) => drive.list().await,
        }
    }

    pub async fn put(&self, name: &str, bytes: Vec<u8>) -> Result<()> {
        match self {
            Place::Folder(dir) => {
                // Written aside and moved in, so a reader never sees half.
                let part = dir.join(format!("{name}.part"));
                std::fs::write(&part, bytes).map_err(io(&part))?;
                std::fs::rename(&part, dir.join(name)).map_err(io(dir))
            }
            Place::Drive(drive) => drive.put(name, bytes).await,
        }
    }

    pub async fn get(&self, letter: &Letter) -> Result<Vec<u8>> {
        match self {
            Place::Folder(dir) => std::fs::read(dir.join(&letter.id)).map_err(io(dir)),
            Place::Drive(drive) => drive.get(&letter.id).await,
        }
    }

    pub async fn delete(&self, letter: &Letter) -> Result<()> {
        match self {
            Place::Folder(dir) => match std::fs::remove_file(dir.join(&letter.id)) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(io(dir)(e)),
                _ => Ok(()),
            },
            Place::Drive(drive) => drive.delete(&letter.id).await,
        }
    }
}

// ---------------------------------------------------------------------------
// Letters: their names and their seals
// ---------------------------------------------------------------------------

const PREFIX: &str = "qm1.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    List,
    Pack,
    Got,
    /// A device saying it uses this folder: how a computer signed in to
    /// several accounts learns which is each phone's (2026-11-13).
    Hello,
    /// The contents a device's plan still wants from the other, so the other
    /// posts those and nothing else (2026-11-14): without it a computer
    /// posted every file the phone was not known to hold, shared files the
    /// phone opens from the computer and never keeps among them, again each
    /// day -- 137 MB waiting on the OPPO, none of it applied.
    Want,
}

impl Kind {
    fn word(self) -> &'static str {
        match self {
            Kind::List => "list",
            Kind::Pack => "pack",
            Kind::Got => "got",
            Kind::Hello => "hello",
            Kind::Want => "want",
        }
    }
}

/// A device as letters name it: the first sixteen hex digits of its id,
/// which is random and says nothing of the person.
fn short(device: &DeviceId) -> String {
    hex(&device.as_bytes()[..8])
}

/// The name letters give `device`, as [`Mailbox::heard_from`] reports it.
pub fn short_name(device: &DeviceId) -> String {
    short(device)
}

fn letter_name(to: &DeviceId, from: &DeviceId, kind: Kind) -> String {
    let mut tag = [0u8; 6];
    getrandom::fill(&mut tag).expect("the system's random source");
    format!("{PREFIX}{}.{}.{}.{}{}", short(to), short(from), kind.word(), now(), hex(&tag))
}

/// Who a letter is to and from, and what it is, read from its name.
fn read_name(name: &str) -> Option<(String, String, Kind)> {
    let mut parts = name.strip_prefix(PREFIX)?.split('.');
    let (to, from, kind) = (parts.next()?, parts.next()?, parts.next()?);
    let kind = match kind {
        "list" => Kind::List,
        "pack" => Kind::Pack,
        "got" => Kind::Got,
        "hello" => Kind::Hello,
        "want" => Kind::Want,
        _ => return None,
    };
    Some((to.to_string(), from.to_string(), kind))
}

fn seal(key: &[u8; 32], plain: &[u8]) -> Vec<u8> {
    let mut nonce = [0u8; 12];
    getrandom::fill(&mut nonce).expect("the system's random source");
    let mut out = nonce.to_vec();
    out.extend(ChaCha20Poly1305::new(key.into()).encrypt(Nonce::from_slice(&nonce), plain).expect("sealing"));
    out
}

fn open(key: &[u8; 32], sealed: &[u8]) -> Option<Vec<u8>> {
    if sealed.len() < 12 + 16 {
        return None;
    }
    ChaCha20Poly1305::new(key.into()).decrypt(Nonce::from_slice(&sealed[..12]), &sealed[12..]).ok()
}

/// A piece of a file's content in a pack: which content, its whole size,
/// where this piece starts, and the bytes.
struct Piece {
    content: [u8; 32],
    size: u64,
    offset: u64,
    bytes: Vec<u8>,
}

fn write_pieces(pieces: &[Piece]) -> Vec<u8> {
    let mut out = Vec::with_capacity(pieces.iter().map(|p| p.bytes.len() + 52).sum());
    for p in pieces {
        out.extend_from_slice(&p.content);
        out.extend_from_slice(&p.size.to_be_bytes());
        out.extend_from_slice(&p.offset.to_be_bytes());
        out.extend_from_slice(&(p.bytes.len() as u32).to_be_bytes());
        out.extend_from_slice(&p.bytes);
    }
    out
}

fn read_pieces(mut bytes: &[u8]) -> Option<Vec<Piece>> {
    let mut out = Vec::new();
    while !bytes.is_empty() {
        if bytes.len() < 52 {
            return None;
        }
        let content: [u8; 32] = bytes[..32].try_into().ok()?;
        let size = u64::from_be_bytes(bytes[32..40].try_into().ok()?);
        let offset = u64::from_be_bytes(bytes[40..48].try_into().ok()?);
        let len = u32::from_be_bytes(bytes[48..52].try_into().ok()?) as usize;
        let piece = bytes.get(52..52 + len)?.to_vec();
        out.push(Piece { content, size, offset, bytes: piece });
        bytes = &bytes[52 + len..];
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// The mailbox
// ---------------------------------------------------------------------------

/// What one device keeps of its letters: what it posted and when, so as not
/// to post it again, and the last list it posted to each device.
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct Kept {
    /// Content, in hex, to when it was posted, in Unix seconds.
    posted: BTreeMap<String, u64>,
    /// A device, by its short name, to the fingerprint of the list last
    /// posted to it.
    lists: BTreeMap<String, String>,
    /// A device, by its short name, to the fingerprint of the contents last
    /// said to be wanted from it.
    #[serde(default)]
    wants: BTreeMap<String, String>,
    /// A device, by its short name, to when this one first read what it
    /// wants: packs to it from before were posted blind.
    #[serde(default)]
    informed: BTreeMap<String, u64>,
}

/// One device's side of the folder.
pub struct Mailbox {
    place: Place,
    key: [u8; 32],
    me: DeviceId,
    /// Beside the store: contents arriving, the lists taken, and [`Kept`].
    dir: PathBuf,
}

/// What a post did.
#[derive(Debug, Default, Clone)]
pub struct Posted {
    pub list: bool,
    pub packs: usize,
    pub bytes: u64,
    /// Receipts taken from the other device, and the contents they covered.
    pub receipts: usize,
    pub delivered: usize,
    /// Whether it stopped at [`CAP`].
    pub full: bool,
    /// Packs posted before the other device said what it wants, taken back.
    pub taken_back: usize,
}

/// What a collection did.
#[derive(Debug, Default, Clone)]
pub struct Collected {
    pub packs: usize,
    pub bytes: u64,
    /// Files taken in from the list, and those waiting for their contents.
    pub applied: usize,
    pub waiting: usize,
}

impl Mailbox {
    /// `key` is the person's `Purpose::Mailbox` key; `me` this device;
    /// `store_dir` the store's directory, beside which arrivals are kept.
    pub fn new(place: Place, key: [u8; 32], me: DeviceId, store_dir: &Path) -> Result<Self> {
        let dir = store_dir.join("mailbox");
        std::fs::create_dir_all(dir.join("in")).map_err(io(&dir))?;
        Ok(Self { place, key, me, dir })
    }

    pub fn place(&self) -> &Place {
        &self.place
    }

    fn kept(&self) -> Kept {
        std::fs::read(self.dir.join("kept.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    fn keep(&self, kept: &Kept) {
        let path = self.dir.join("kept.json");
        if let Ok(bytes) = serde_json::to_vec(kept) {
            let _ = std::fs::write(&path, bytes);
        }
    }

    /// The letters between this device and `peer`, one way.
    fn between(letters: &[Letter], to: &DeviceId, from: &DeviceId, kind: Kind) -> Vec<Letter> {
        let (to, from) = (short(to), short(from));
        letters
            .iter()
            .filter(|l| read_name(&l.name).is_some_and(|(t, f, k)| t == to && f == from && k == kind))
            .cloned()
            .collect()
    }

    /// Say to `to` that this device uses this folder, unless something from
    /// it to `to` is here already. A computer signed in to several accounts
    /// posts to a phone only in the one the phone's letters are in.
    pub async fn say_hello(&self, to: &DeviceId) -> Result<bool> {
        let letters = self.place.list().await?;
        let (to_short, me) = (short(to), short(&self.me));
        if letters.iter().any(|l| read_name(&l.name).is_some_and(|(t, f, _)| t == to_short && f == me)) {
            return Ok(false);
        }
        self.place.put(&letter_name(to, &self.me, Kind::Hello), seal(&self.key, b"hello")).await?;
        Ok(true)
    }

    /// Which devices have left anything here for this one: by the short
    /// names letters carry (see [`short_name`]).
    pub async fn heard_from(&self) -> Result<std::collections::HashSet<String>> {
        let me = short(&self.me);
        Ok(self
            .place
            .list()
            .await?
            .iter()
            .filter_map(|l| read_name(&l.name))
            .filter(|(to, _, _)| *to == me)
            .map(|(_, from, _)| from)
            .collect())
    }

    /// Post to `peer`: take its receipts, then its list if it changed, then
    /// packs of what it is not known to hold, until [`CAP`] is waiting or
    /// `budget` is spent. The engine is held only to read and record, never
    /// while bytes cross the network ([`Holds`]): on a phone, screens wait
    /// for it (2026-11-13).
    pub async fn post<H: Holds>(&self, holds: &H, peer: &TrustedPeer, budget: Duration) -> Result<Posted> {
        let started = Instant::now();
        let mut posted = Posted::default();
        let them = peer.device_id;
        posted.taken_back = self.take_back_blind(&them).await?;
        let mut kept = self.kept();
        let asker = qurb_peer::Fingerprint::from_bytes(peer.fingerprint);
        let letters = self.place.list().await?;

        // Its receipts first: what it took is not posted again.
        let mut receipts = Vec::new();
        for letter in Self::between(&letters, &self.me, &them, Kind::Got) {
            let bytes = self.place.get(&letter).await?;
            receipts.push((letter, bytes));
        }
        let (tree, encoded) = holds.store(|store| -> Result<_> {
            for (_, bytes) in &receipts {
                if let Some(plain) = open(&self.key, bytes) {
                    for content in plain.chunks_exact(32) {
                        let content: [u8; 32] = content.try_into().expect("32 bytes");
                        if qurb_peer::server::answer_offline(store, &qurb_peer::Request::Got { content }, asker).is_ok() {
                            posted.delivered += 1;
                        }
                        kept.posted.remove(&hex(&content));
                    }
                }
            }
            // Its list, as it would be shown it live.
            let tree = match qurb_peer::server::answer_offline(store, &qurb_peer::Request::Tree, asker)? {
                qurb_peer::Response::Tree(tree) => tree,
                _ => Vec::new(),
            };
            let encoded = qurb_peer::Response::Tree(tree.clone()).encode();
            Ok((tree, encoded))
        })?;
        for (letter, _) in &receipts {
            self.place.delete(letter).await?;
            posted.receipts += 1;
        }

        let digest = blake3::hash(&encoded).to_hex().to_string();
        let lists = Self::between(&letters, &them, &self.me, Kind::List);
        if kept.lists.get(&short(&them)) != Some(&digest) || lists.is_empty() {
            self.place.put(&letter_name(&them, &self.me, Kind::List), seal(&self.key, &encoded)).await?;
            for old in lists {
                self.place.delete(&old).await?;
            }
            kept.lists.insert(short(&them), digest);
            posted.list = true;
        }
        self.keep(&kept);

        // What it said it still wants, if it has said: only those are posted.
        // A device that never says (a build before 2026-11-14) is posted what
        // it is not known to hold, as before.
        let mut wants = Self::between(&letters, &self.me, &them, Kind::Want);
        wants.sort_by(|a, b| a.name.cmp(&b.name));
        let wanted: Option<std::collections::HashSet<[u8; 32]>> = match wants.last() {
            Some(latest) => {
                let plain = open(&self.key, &self.place.get(latest).await?).unwrap_or_default();
                Some(plain.chunks_exact(32).map(|c| c.try_into().expect("32 bytes")).collect())
            }
            None => None,
        };
        // Packs, a few at a time: read under the engine, sent without it.
        let mut waiting: u64 = Self::between(&letters, &them, &self.me, Kind::Pack).iter().map(|l| l.size).sum();
        let mut from = 0usize;
        while from < tree.len() && waiting < CAP && started.elapsed() < budget {
            let (packs, next, bytes) = holds.store(|store| self.gather(store, &tree, from, &them, wanted.as_ref(), &mut kept))?;
            from = next;
            for pack in packs {
                waiting += pack.len() as u64;
                self.place.put(&letter_name(&them, &self.me, Kind::Pack), pack).await?;
                posted.packs += 1;
            }
            posted.bytes += bytes;
            self.keep(&kept);
        }
        posted.full = waiting >= CAP;
        Ok(posted)
    }

    /// Packs to `them` posted before it first said what it wants, which were
    /// posted blind: taken back, and what of them it wants made ready to be
    /// posted again. Left, a phone drained them pass after pass and used none
    /// (the S23 on 2026-11-14, about 17 MB a pass, from a backlog of up to
    /// [`CAP`]). Run on every round, reached or not: a phone reached at home is
    /// never posted to, and went on collecting the backlog. Returns how many.
    pub async fn take_back_blind(&self, them: &DeviceId) -> Result<usize> {
        let letters = self.place.list().await?;
        let mut wants = Self::between(&letters, &self.me, them, Kind::Want);
        wants.sort_by(|a, b| a.name.cmp(&b.name));
        let Some(latest) = wants.last() else { return Ok(0) };
        let mut kept = self.kept();
        let said_at = *kept.informed.entry(short(them)).or_insert_with(now);
        let blind: Vec<Letter> =
            Self::between(&letters, them, &self.me, Kind::Pack).into_iter().filter(|pack| stamp(&pack.name) < said_at).collect();
        if !blind.is_empty() {
            let plain = open(&self.key, &self.place.get(latest).await?).unwrap_or_default();
            let wanted: std::collections::HashSet<[u8; 32]> = plain.chunks_exact(32).map(|c| c.try_into().expect("32 bytes")).collect();
            for pack in &blind {
                self.place.delete(pack).await?;
            }
            kept.posted.retain(|content, _| unhex(content).is_none_or(|c| !wanted.contains(&c)));
        }
        self.keep(&kept);
        Ok(blind.len())
    }

    /// From `tree[from..]`, the contents `them` is not known to hold, sealed
    /// into up to [`ROUND`] packs; where it stopped; and the bytes of content
    /// taken. A file over [`LARGEST`] is left for a live path.
    fn gather(
        &self,
        store: &Store,
        tree: &[FileVersion],
        from: usize,
        them: &DeviceId,
        wanted: Option<&std::collections::HashSet<[u8; 32]>>,
        kept: &mut Kept,
    ) -> Result<(Vec<Vec<u8>>, usize, u64)> {
        let now = now();
        let mut packs = Vec::new();
        let mut pieces: Vec<Piece> = Vec::new();
        let mut in_pack = 0usize;
        let mut taken = 0u64;
        let mut at = from;
        while at < tree.len() && packs.len() < ROUND {
            let file = &tree[at];
            at += 1;
            let Content::File { hash, size } = file.content else { continue };
            if size == 0 || size > LARGEST || wanted.is_some_and(|w| !w.contains(&hash)) {
                continue;
            }
            if kept.posted.get(&hex(&hash)).is_some_and(|t| now.saturating_sub(*t) < POST_AGAIN) {
                continue;
            }
            let content = blake3::Hash::from(hash);
            if store.db().device_holds(&content, them).unwrap_or(false) {
                continue;
            }
            let Some(bytes) = store.read_content(&content)? else { continue };
            let mut offset = 0usize;
            while offset < bytes.len() {
                let take = (PACK - in_pack).min(bytes.len() - offset);
                pieces.push(Piece { content: hash, size, offset: offset as u64, bytes: bytes[offset..offset + take].to_vec() });
                in_pack += take;
                offset += take;
                if in_pack >= PACK {
                    packs.push(seal(&self.key, &write_pieces(&pieces)));
                    pieces.clear();
                    in_pack = 0;
                }
            }
            taken += size;
            kept.posted.insert(hex(&hash), now);
        }
        if !pieces.is_empty() {
            packs.push(seal(&self.key, &write_pieces(&pieces)));
        }
        Ok((packs, at, taken))
    }

    /// Collect from `peer`: its packs, into place beside the store, then its
    /// latest list, planned from and applied as far as the contents allow,
    /// and a receipt for what was taken. The engine is held only to plan and
    /// apply.
    pub async fn collect<H: Holds>(&self, holds: &H, peer: &TrustedPeer, budget: Duration) -> Result<Collected> {
        let started = Instant::now();
        let mut collected = Collected::default();
        let them = peer.device_id;
        let letters = self.place.list().await?;

        for letter in Self::between(&letters, &self.me, &them, Kind::Pack) {
            if started.elapsed() >= budget {
                break;
            }
            let bytes = self.place.get(&letter).await?;
            if let Some(pieces) = open(&self.key, &bytes).and_then(|plain| read_pieces(&plain)) {
                for piece in pieces {
                    self.take_piece(&piece)?;
                }
            }
            collected.packs += 1;
            collected.bytes += letter.size;
            self.place.delete(&letter).await?;
        }

        // The latest list, kept here once read: packs may come after it.
        let list_path = self.dir.join(format!("list-{}", short(&them)));
        let mut lists = Self::between(&letters, &self.me, &them, Kind::List);
        lists.sort_by(|a, b| a.name.cmp(&b.name));
        if let Some(latest) = lists.last() {
            let bytes = self.place.get(latest).await?;
            if let Some(plain) = open(&self.key, &bytes) {
                std::fs::write(&list_path, plain).map_err(io(&list_path))?;
            }
            for list in &lists {
                self.place.delete(list).await?;
            }
        }
        let Ok(encoded) = std::fs::read(&list_path) else { return Ok(collected) };
        let tree = match qurb_peer::Response::decode(&encoded) {
            Ok(qurb_peer::Response::Tree(tree)) => tree,
            _ => return Ok(collected),
        };

        // Planned as from a list fetched live; applied only where the
        // contents are here, so a backup half-arrived is no failure.
        let inbox = self.dir.join("in");
        let mut wanted = std::collections::HashSet::new();
        let mut still: std::collections::BTreeSet<[u8; 32]> = std::collections::BTreeSet::new();
        let received = holds.engine(|engine| -> Result<Vec<[u8; 32]>> {
            let plan = engine.plan_with(&tree, Some(&them))?;
            wanted.extend(plan.iter().filter_map(needed));
            let (ready, waiting): (Vec<Action>, Vec<Action>) = plan.into_iter().partition(|action| {
                needed(action).is_none_or(|hash| {
                    inbox.join(hex(&hash)).exists() || engine.store().can_read_content(&blake3::Hash::from(hash)).unwrap_or(false)
                })
            });
            collected.waiting = waiting.len();
            still.extend(waiting.iter().filter_map(needed));
            if ready.is_empty() {
                return Ok(Vec::new());
            }
            let mut source = Arrived { dir: inbox.clone(), received: Vec::new() };
            let stats = engine.apply_plan(&ready, &mut source)?;
            collected.applied = stats.adopted + stats.conflicts + stats.resurrected + stats.held;
            Ok(source.received)
        })?;
        if !received.is_empty() {
            let mut receipt = Vec::with_capacity(received.len() * 32);
            for content in &received {
                receipt.extend_from_slice(content);
                let _ = std::fs::remove_file(inbox.join(hex(content)));
            }
            self.place.put(&letter_name(&them, &self.me, Kind::Got), seal(&self.key, &receipt)).await?;
        }
        self.tidy_inbox(&wanted);
        self.say_wanted(&letters, &them, &still).await?;
        Ok(collected)
    }

    /// Tell `them` what is still wanted from it, when that changed: one
    /// letter, the older ones taken away.
    async fn say_wanted(&self, letters: &[Letter], them: &DeviceId, still: &std::collections::BTreeSet<[u8; 32]>) -> Result<()> {
        let mut plain = Vec::with_capacity(still.len() * 32);
        for content in still {
            plain.extend_from_slice(content);
        }
        let digest = blake3::hash(&plain).to_hex().to_string();
        let mut kept = self.kept();
        let mine = Self::between(letters, them, &self.me, Kind::Want);
        if kept.wants.get(&short(them)) == Some(&digest) && !mine.is_empty() {
            return Ok(());
        }
        self.place.put(&letter_name(them, &self.me, Kind::Want), seal(&self.key, &plain)).await?;
        for letter in &mine {
            self.place.delete(letter).await?;
        }
        kept.wants.insert(short(them), digest);
        self.keep(&kept);
        Ok(())
    }

    /// What arrived and no plan wants any more -- a file deleted or changed
    /// again before its contents came, or already had another way -- let go
    /// of once a day old, whole or in part. Until 2026-11-14 it stayed: 346 MB
    /// of the S23's 916 (the owner: "i want my phone apps to take as little
    /// space as possible"). A day, so a file another device's list still
    /// wants is not dropped between its collections.
    fn tidy_inbox(&self, wanted: &std::collections::HashSet<[u8; 32]>) {
        let day = std::time::Duration::from_secs(24 * 60 * 60);
        let Ok(entries) = std::fs::read_dir(self.dir.join("in")) else { return };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let content = name.split('.').next().and_then(unhex);
            if content.is_some_and(|c| wanted.contains(&c)) {
                continue;
            }
            let old = entry.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age > day);
            if old {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }

    /// A piece written where its content is being put together, and the
    /// whole moved into place once every byte is there and its hash checks.
    fn take_piece(&self, piece: &Piece) -> Result<()> {
        use std::io::{Seek, SeekFrom, Write};
        let inbox = self.dir.join("in");
        let whole = inbox.join(hex(&piece.content));
        if whole.exists() {
            return Ok(());
        }
        let part = inbox.join(format!("{}.part", hex(&piece.content)));
        let mut file = std::fs::OpenOptions::new().create(true).write(true).truncate(false).open(&part).map_err(io(&part))?;
        file.seek(SeekFrom::Start(piece.offset)).map_err(io(&part))?;
        file.write_all(&piece.bytes).map_err(io(&part))?;
        drop(file);
        // Which ranges arrived, beside it: the file's length alone does not
        // say a gap was filled.
        let ranges = inbox.join(format!("{}.ranges", hex(&piece.content)));
        let mut have: Vec<(u64, u64)> = std::fs::read(&ranges).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        have.push((piece.offset, piece.offset + piece.bytes.len() as u64));
        have.sort();
        let mut merged: Vec<(u64, u64)> = Vec::new();
        for (start, end) in have {
            match merged.last_mut() {
                Some(last) if start <= last.1 => last.1 = last.1.max(end),
                _ => merged.push((start, end)),
            }
        }
        if merged.len() == 1 && merged[0] == (0, piece.size) {
            let bytes = std::fs::read(&part).map_err(io(&part))?;
            if *blake3::hash(&bytes).as_bytes() == piece.content {
                std::fs::rename(&part, &whole).map_err(io(&part))?;
            } else {
                tracing::warn!("a file put together from the mailbox did not match its hash; dropped");
                let _ = std::fs::remove_file(&part);
            }
            let _ = std::fs::remove_file(&ranges);
        } else if let Ok(bytes) = serde_json::to_vec(&merged) {
            let _ = std::fs::write(&ranges, bytes);
        }
        Ok(())
    }
}

/// The content an action needs, if any.
fn needed(action: &Action) -> Option<[u8; 32]> {
    let version: &FileVersion = match action {
        Action::Adopt { remote } | Action::Hold { remote, .. } | Action::Restored { remote } => remote,
        Action::Resurrect { resolved } => resolved,
        Action::Conflict { renamed, .. } => renamed,
        Action::Offer { .. } | Action::Merge { .. } => return None,
    };
    match version.content {
        Content::File { hash, .. } => Some(hash),
        Content::Deleted => None,
    }
}

/// Contents that arrived in packs, for applying a plan.
struct Arrived {
    dir: PathBuf,
    received: Vec<[u8; 32]>,
}

impl qurb_engine::ContentSource for Arrived {
    fn fetch(&mut self, hash: &[u8; 32], _size: u64) -> qurb_engine::Result<Vec<u8>> {
        let path = self.dir.join(hex(hash));
        std::fs::read(&path).map_err(|source| qurb_engine::Error::Io { path, source })
    }

    fn received(&mut self, content: &[u8; 32]) {
        self.received.push(*content);
    }
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// When a letter was left, from its name (`letter_name`): the Unix seconds
/// before its random tag. Zero for a name not read.
fn stamp(name: &str) -> u64 {
    let last = name.rsplit('.').next().unwrap_or("");
    let digits = last.len().saturating_sub(12);
    last.get(..digits).and_then(|d| d.parse().ok()).unwrap_or(0)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A content hash from its name in the inbox, as [`hex`] wrote it.
fn unhex(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_letter_opens_only_with_its_key_and_its_name_says_who() {
        let (key, other) = ([1u8; 32], [2u8; 32]);
        let sealed = seal(&key, b"a list");
        assert_eq!(open(&key, &sealed).as_deref(), Some(b"a list".as_slice()));
        assert!(open(&other, &sealed).is_none());
        let (to, from) = (DeviceId::from_bytes([3; 32]), DeviceId::from_bytes([4; 32]));
        let name = letter_name(&to, &from, Kind::Pack);
        assert_eq!(read_name(&name), Some((short(&to), short(&from), Kind::Pack)));
    }

    #[test]
    fn pieces_come_back_as_they_went() {
        let pieces = vec![
            Piece { content: [5; 32], size: 10, offset: 0, bytes: b"hello".to_vec() },
            Piece { content: [6; 32], size: 3, offset: 0, bytes: b"abc".to_vec() },
        ];
        let back = read_pieces(&write_pieces(&pieces)).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!((back[0].content, back[0].size, back[0].bytes.as_slice()), ([5; 32], 10, b"hello".as_slice()));
        assert_eq!(back[1].bytes, b"abc");
    }
}
