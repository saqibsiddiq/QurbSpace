//! Two devices of one person and a folder both see, with no connection
//! between them: what one posts the other collects, files whole, and the
//! receipt it leaves marks them delivered on the first.

use qurb_engine::Engine;
use qurb_mailbox::{Mailbox, Place};
use qurb_storage::{ChunkKey, Store};
use qurb_watcher::IgnoreRules;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

struct Device {
    _dir: tempfile::TempDir,
    root: PathBuf,
    engine: Engine,
    identity: qurb_peer::Identity,
}

impl Device {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("sync");
        fs::create_dir_all(&root).unwrap();
        let store_dir = root.join(".qurb");
        let store = Store::open(&store_dir, ChunkKey::from_bytes([42; 32])).unwrap();
        let identity = qurb_peer::Identity::load_or_create(&store_dir).unwrap();
        let ignore = IgnoreRules::new().with_store_dir(&store_dir);
        Self { _dir: dir, root: root.clone(), engine: Engine::new(root, store, ignore), identity }
    }

    fn write(&mut self, rel: &str, contents: &[u8]) {
        let path = self.root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
        self.engine.reconcile().unwrap();
    }

    fn id(&self) -> qurb_sync::DeviceId {
        self.engine.store().device_id().unwrap()
    }

    fn trust(&self, other: &Device, name: &str) -> qurb_storage::db::TrustedPeer {
        let db = self.engine.store().db();
        db.trust_peer(&other.id(), other.identity.fingerprint().as_bytes(), name).unwrap();
        db.peer_by_fingerprint(other.identity.fingerprint().as_bytes()).unwrap().unwrap()
    }

    fn mailbox(&self, folder: &std::path::Path) -> Mailbox {
        Mailbox::new(Place::Folder(folder.to_path_buf()), [9; 32], self.id(), &self.root.join(".qurb")).unwrap()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn what_one_device_posts_the_other_collects_and_the_receipt_comes_back() {
    let folder = tempfile::tempdir().unwrap();
    let mut phone = Device::new();
    let mut laptop = Device::new();
    let laptop_as_seen = phone.trust(&laptop, "Laptop");
    let phone_as_seen = laptop.trust(&phone, "Phone");

    phone.write("DCIM/small.txt", b"a note from the phone");
    // Larger than one pack, so it comes in pieces across two.
    let big: Vec<u8> = (0..(qurb_mailbox::PACK + 300_000)).map(|i| (i * 31 % 251) as u8).collect();
    phone.write("DCIM/big.bin", &big);

    let at_phone = phone.mailbox(folder.path());
    let posted = at_phone.post(&qurb_mailbox::Owned(std::sync::Mutex::new(&mut phone.engine)), &laptop_as_seen, Duration::from_secs(60)).await.unwrap();
    assert!(posted.list && posted.packs >= 2, "{posted:?}");

    let at_laptop = laptop.mailbox(folder.path());
    let collected = at_laptop.collect(&qurb_mailbox::Owned(std::sync::Mutex::new(&mut laptop.engine)), &phone_as_seen, Duration::from_secs(60)).await.unwrap();
    assert_eq!(collected.waiting, 0, "{collected:?}");
    assert_eq!(fs::read(laptop.root.join("DCIM/small.txt")).unwrap(), b"a note from the phone");
    assert_eq!(fs::read(laptop.root.join("DCIM/big.bin")).unwrap(), big, "the large file did not arrive whole");

    // The receipt: the phone takes it, and knows the laptop holds both.
    let again = at_phone.post(&qurb_mailbox::Owned(std::sync::Mutex::new(&mut phone.engine)), &laptop_as_seen, Duration::from_secs(60)).await.unwrap();
    assert_eq!(again.receipts, 1, "{again:?}");
    assert_eq!(again.packs, 0, "contents the laptop took were posted again: {again:?}");
    let small = blake3::hash(b"a note from the phone");
    assert!(phone.engine.store().db().device_holds(&small, &laptop.id()).unwrap(), "not recorded as delivered");

    // Nothing left in the folder but the list, and what the laptop wants.
    let left: Vec<_> = fs::read_dir(folder.path()).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    assert!(left.iter().all(|n| n.contains(".list.") || n.contains(".want.")), "{left:?}");
}

async fn post(at: &Mailbox, device: &mut Device, to: &qurb_storage::db::TrustedPeer) -> qurb_mailbox::Posted {
    at.post(&qurb_mailbox::Owned(std::sync::Mutex::new(&mut device.engine)), to, Duration::from_secs(60)).await.unwrap()
}

/// Once a device has said what it wants, the other posts that and nothing
/// else (2026-11-14): a new file waits for the list that names it to be
/// planned from and asked for, rather than being posted on the chance.
#[tokio::test(flavor = "multi_thread")]
async fn only_what_was_asked_for_is_posted() {
    let folder = tempfile::tempdir().unwrap();
    let mut phone = Device::new();
    let mut laptop = Device::new();
    let laptop_as_seen = phone.trust(&laptop, "Laptop");
    let phone_as_seen = laptop.trust(&phone, "Phone");
    let at_phone = phone.mailbox(folder.path());
    let at_laptop = laptop.mailbox(folder.path());

    phone.write("a.txt", b"first");
    assert!(post(&at_phone, &mut phone, &laptop_as_seen).await.packs > 0, "a device that has said nothing is posted to as before");
    at_laptop.collect(&qurb_mailbox::Owned(std::sync::Mutex::new(&mut laptop.engine)), &phone_as_seen, Duration::from_secs(60)).await.unwrap();

    // The laptop has all it wanted, and said so: a new file is not posted
    // until it asks.
    phone.write("b.txt", b"second");
    assert_eq!(post(&at_phone, &mut phone, &laptop_as_seen).await.packs, 0);
    let collected = at_laptop.collect(&qurb_mailbox::Owned(std::sync::Mutex::new(&mut laptop.engine)), &phone_as_seen, Duration::from_secs(60)).await.unwrap();
    assert_eq!(collected.waiting, 1, "{collected:?}");
    assert!(post(&at_phone, &mut phone, &laptop_as_seen).await.packs > 0, "asked for, and not posted");
    at_laptop.collect(&qurb_mailbox::Owned(std::sync::Mutex::new(&mut laptop.engine)), &phone_as_seen, Duration::from_secs(60)).await.unwrap();
    assert_eq!(fs::read(laptop.root.join("b.txt")).unwrap(), b"second");
}

#[tokio::test(flavor = "multi_thread")]
async fn half_arrived_contents_wait_rather_than_fail() {
    let folder = tempfile::tempdir().unwrap();
    let mut phone = Device::new();
    let mut laptop = Device::new();
    let laptop_as_seen = phone.trust(&laptop, "Laptop");
    let phone_as_seen = laptop.trust(&phone, "Phone");
    phone.write("a.txt", b"first");

    // Posted, then the packs lost before the laptop looks: only the list.
    let at_phone = phone.mailbox(folder.path());
    at_phone.post(&qurb_mailbox::Owned(std::sync::Mutex::new(&mut phone.engine)), &laptop_as_seen, Duration::from_secs(60)).await.unwrap();
    for entry in fs::read_dir(folder.path()).unwrap().flatten() {
        if entry.file_name().to_string_lossy().contains(".pack.") {
            fs::remove_file(entry.path()).unwrap();
        }
    }
    let collected = laptop.mailbox(folder.path()).collect(&qurb_mailbox::Owned(std::sync::Mutex::new(&mut laptop.engine)), &phone_as_seen, Duration::from_secs(60)).await.unwrap();
    assert_eq!((collected.applied, collected.waiting), (0, 1), "{collected:?}");
    assert!(!laptop.root.join("a.txt").exists());
}

/// A phone on its own Google account says hello there, and the hello stays
/// through collecting: how a computer signed in to several accounts knows
/// which is the phone's (2026-11-13).
#[tokio::test(flavor = "multi_thread")]
async fn a_phones_hello_says_which_folder_it_uses_and_stays() {
    let folder = tempfile::tempdir().unwrap();
    let mut phone = Device::new();
    let mut laptop = Device::new();
    let _ = phone.trust(&laptop, "Laptop");
    let phone_as_seen = laptop.trust(&phone, "Phone");

    let at_phone = phone.mailbox(folder.path());
    assert!(at_phone.say_hello(&laptop.id()).await.unwrap());
    assert!(!at_phone.say_hello(&laptop.id()).await.unwrap(), "a second hello was left");

    let at_laptop = laptop.mailbox(folder.path());
    let heard = at_laptop.heard_from().await.unwrap();
    assert!(heard.contains(&qurb_mailbox::short_name(&phone.id())), "{heard:?}");
    at_laptop.collect(&qurb_mailbox::Owned(std::sync::Mutex::new(&mut laptop.engine)), &phone_as_seen, Duration::from_secs(10)).await.unwrap();
    assert!(at_laptop.heard_from().await.unwrap().contains(&qurb_mailbox::short_name(&phone.id())), "collecting took the hello");
}

/// Packs posted before the other device said what it wants were posted
/// blind: once it has said, they are taken back, and what of them it wants is
/// posted again (2026-11-14: the S23 drained such a backlog, ~17 MB a pass).
#[tokio::test(flavor = "multi_thread")]
async fn packs_posted_blind_are_taken_back_once_it_says() {
    let folder = tempfile::tempdir().unwrap();
    let mut phone = Device::new();
    let mut laptop = Device::new();
    let laptop_as_seen = phone.trust(&laptop, "Laptop");
    let phone_as_seen = laptop.trust(&phone, "Phone");
    let at_phone = phone.mailbox(folder.path());
    let at_laptop = laptop.mailbox(folder.path());

    phone.write("a.txt", b"first");
    assert!(post(&at_phone, &mut phone, &laptop_as_seen).await.packs > 0);
    // Letters are stamped to the second.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    // The laptop reads the list and says what it wants, with no time to
    // take the packs.
    at_laptop.collect(&qurb_mailbox::Owned(std::sync::Mutex::new(&mut laptop.engine)), &phone_as_seen, Duration::ZERO).await.unwrap();

    let again = post(&at_phone, &mut phone, &laptop_as_seen).await;
    assert!(again.taken_back > 0, "{again:?}");
    assert!(again.packs > 0, "what it wants was not posted again: {again:?}");
    let collected = at_laptop.collect(&qurb_mailbox::Owned(std::sync::Mutex::new(&mut laptop.engine)), &phone_as_seen, Duration::from_secs(60)).await.unwrap();
    assert_eq!(collected.waiting, 0, "{collected:?}");
    assert_eq!(fs::read(laptop.root.join("a.txt")).unwrap(), b"first");
}

