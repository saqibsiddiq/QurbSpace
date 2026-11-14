//! A file of a phone's own vault deleted while its bytes are not on the phone
//! -- freed to the computer keeping them, or a backup read where it lay --
//! kept in Recently deleted all the same, and put back from the chunks the
//! deleted row still lists (decision 0073). Since 2026-11-14 nothing shows or
//! restores it (decision 0089), and its chunks go with the ordinary retention
//! window rather than thirty days; the store's restore path is still tested.

use qurb_storage::{ChunkKey, OutsideEntry, Store};
use std::time::Duration;

const DAY: i64 = 24 * 3600;

struct Phone {
    dir: tempfile::TempDir,
    root: std::path::PathBuf,
    store: Store,
}

impl Phone {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("folder");
        std::fs::create_dir_all(&root).unwrap();
        let mut store = Store::open(&root.join(".qurb"), ChunkKey::from_bytes([6; 32])).unwrap().in_tree(&root);
        store.set_new_files_private(true);
        let computer = qurb_sync::DeviceId::from_bytes([8; 32]);
        store.db().set_peer_kind(&computer, "computer").unwrap();
        Self { dir, root, store }
    }

    /// A file of the vault, kept by the computer and freed here.
    fn freed(&mut self, path: &str, bytes: &[u8]) {
        std::fs::create_dir_all(self.root.join(path).parent().unwrap()).unwrap();
        std::fs::write(self.root.join(path), bytes).unwrap();
        self.store.put_file(path, &self.root.join(path)).unwrap();
        let computer = qurb_sync::DeviceId::from_bytes([8; 32]);
        self.store.db().note_replica(&blake3::hash(bytes), &computer).unwrap();
        self.store.free_local(path).unwrap();
        assert!(!self.root.join(path).exists());
    }

    /// Move a deletion `days` into the past.
    fn age(&self, path: &str, days: i64) {
        self.store
            .db()
            .conn()
            .execute(
                "UPDATE files SET deleted_at = deleted_at - ?1 WHERE path = ?2",
                rusqlite::params![days * DAY, path],
            )
            .unwrap();
    }
}

#[test]
fn a_freed_file_deleted_costs_nothing_here_and_goes_with_the_week() {
    let mut phone = Phone::new();
    let bytes = vec![9u8; 3 << 20];
    phone.freed("Notes/diary.txt", &bytes);

    phone.store.delete_to_trash("Notes/diary.txt", None).unwrap().expect("nothing kept");
    let listed = phone.store.recently_deleted().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].size, 0, "counted as costing space here");
    assert_eq!(phone.store.db().trash_bytes().unwrap(), 0);
    assert_eq!(phone.store.empty_trash_by(1).unwrap(), 0);

    // Past the week, gone like everything else deleted (decision 0089): the
    // thirty days it was kept for Recently deleted are no more.
    phone.age("Notes/diary.txt", 10);
    phone.store.gc(Duration::from_secs(7 * DAY as u64)).unwrap();
    let me = phone.store.device_id().unwrap();
    assert!(phone.store.db().deleted_row_in("Notes/diary.txt", &me, &blake3::hash(&bytes)).unwrap().is_none());
}

#[test]
fn a_vault_deletion_goes_with_the_retention_window() {
    let mut phone = Phone::new();
    phone.freed("old.txt", b"deleted long ago");
    phone.freed("recent.txt", b"deleted three weeks ago");
    phone.freed("new.txt", b"deleted yesterday");
    phone.store.delete_to_trash("old.txt", None).unwrap();
    phone.store.delete_to_trash("recent.txt", None).unwrap();
    phone.store.delete_to_trash("new.txt", None).unwrap();
    phone.age("old.txt", 31);
    phone.age("recent.txt", 21);
    phone.age("new.txt", 1);

    let stats = phone.store.gc(Duration::from_secs(7 * DAY as u64)).unwrap();
    assert_eq!(stats.tombstones_expired, 2);
    let me = phone.store.device_id().unwrap();
    assert!(phone.store.db().deleted_row_in("recent.txt", &me, &blake3::hash(b"deleted three weeks ago")).unwrap().is_none());
    assert!(phone.store.db().deleted_row_in("new.txt", &me, &blake3::hash(b"deleted yesterday")).unwrap().is_some());
}

#[test]
fn a_backup_deleted_comes_back_into_the_vault_by_its_name() {
    let mut phone = Phone::new();
    let outside = phone.dir.path().join("whatsapp/Media/IMG-1.jpg");
    std::fs::create_dir_all(outside.parent().unwrap()).unwrap();
    std::fs::write(&outside, b"a photo, backed up").unwrap();
    let entry = OutsideEntry {
        path: "Media/IMG-1.jpg".into(),
        source: outside.to_string_lossy().into_owned(),
        size: 18,
        mtime_ns: 1,
    };
    phone.store.keep_outside_folder(".qurb-outside/whatsapp", &[entry], &|_| true, None, &|_| false).unwrap();

    let id = phone
        .store
        .delete_to_trash(".qurb-outside/whatsapp/Media/IMG-1.jpg", None)
        .unwrap()
        .expect("nothing kept");
    assert_eq!(phone.store.restore_from_trash(id).unwrap(), "IMG-1.jpg");
    let me = phone.store.device_id().unwrap();
    assert!(matches!(phone.store.db().folder_row("IMG-1.jpg").unwrap(), Some((_, Some(owner))) if owner == me));
}
