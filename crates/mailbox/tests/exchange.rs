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

    // Nothing left in the folder but the list.
    let left: Vec<_> = fs::read_dir(folder.path()).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    assert!(left.iter().all(|n| n.contains(".list.")), "{left:?}");
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
