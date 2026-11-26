//! Fetching over the call the other device made (decision 0092, 2026-11-26).
//!
//! On a direct link the computer calls the phone, but the phone's call to the
//! computer is stopped by the computer's firewall, which lets in only its
//! home network. So the phone asks over the computer's own call: the computer
//! answers requests on the connections it makes over the link, and the phone
//! keeps the connection the computer made to it there.
//!
//! Real QUIC on loopback, with loopback standing in for the link's network.

use qurb_engine::Engine;
use qurb_peer::{DirectNet, Identity, NetworkSource, PeerClient, PeerServer};
use qurb_storage::{ChunkKey, Store};
use qurb_watcher::IgnoreRules;
use std::sync::{Arc, Mutex};

const KEY: [u8; 32] = [42; 32];

struct Device {
    _dir: tempfile::TempDir,
    root: std::path::PathBuf,
    engine: Engine,
    identity: Identity,
}

impl Device {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("sync");
        std::fs::create_dir_all(&root).unwrap();
        let store_dir = root.join(".qurb");
        let store = Store::open(&store_dir, ChunkKey::from_bytes(KEY)).unwrap();
        let identity = Identity::load_or_create(&store_dir).unwrap();
        let engine = Engine::new(root.clone(), store, IgnoreRules::new().with_store_dir(&store_dir));
        Self { _dir: dir, root, engine, identity }
    }

    fn reader(&self) -> Store {
        Store::open(&self.root.join(".qurb"), ChunkKey::from_bytes(KEY)).unwrap().in_tree(&self.root)
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_phone_fetches_over_the_computers_call() {
    let mut phone = Device::new();
    let mut laptop = Device::new();
    let contents = vec![7u8; 300_000];
    std::fs::write(laptop.root.join("for-the-phone.bin"), &contents).unwrap();
    laptop.engine.reconcile().unwrap();
    let (phone_fp, laptop_fp) = (phone.identity.fingerprint(), laptop.identity.fingerprint());

    // The link: loopback stands in for the phone's group network.
    qurb_peer::set_direct_link(Some(DirectNet {
        address: std::net::Ipv4Addr::new(127, 0, 0, 1),
        prefix: 8,
        gateway: std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
    }));

    // The phone listens, as its passes do.
    let server = PeerServer::bind(
        "127.0.0.1:0".parse().unwrap(),
        &phone.identity,
        &qurb_peer::tls::TrustList::new(vec![laptop_fp]),
    )
    .unwrap();
    let phone_at = server.local_addr().unwrap();
    let phone_store = Arc::new(Mutex::new(phone.reader()));
    tokio::spawn(async move { server.serve(phone_store).await });

    // The computer calls it, and answers on that call.
    let call = PeerClient::connect(phone_at, &laptop.identity, phone_fp).await.expect("the computer's call");
    let laptop_store = Arc::new(Mutex::new(laptop.reader()));
    let trust = qurb_peer::tls::TrustList::new(vec![phone_fp]);
    let answering = call.connection();
    tokio::spawn(async move {
        qurb_peer::server::serve_connection(answering, laptop_store, qurb_peer::Generation::new(), &trust).await
    });

    // The phone has the computer's call, and asks over it: no call of its own.
    let mut theirs = None;
    for _ in 0..50 {
        theirs = qurb_peer::connection_on_link(&laptop_fp);
        if theirs.is_some() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let theirs = theirs.expect("the phone kept the computer's call over the link");
    let elsewhere = quinn::Endpoint::client("127.0.0.1:0".parse().unwrap()).unwrap();
    let client = PeerClient::on_their_connection(elsewhere, theirs);

    let tree = client.tree().await.expect("the computer answers on its own call");
    let plan = phone.engine.plan_against(&tree).unwrap();
    let reader = phone.reader();
    let mut source = NetworkSource::new(&client, &reader);
    let stats = tokio::task::block_in_place(|| phone.engine.apply_plan(&plan, &mut source)).unwrap();
    assert!(stats.is_clean(), "{:?}", stats.failures);
    assert_eq!(std::fs::read(phone.root.join("for-the-phone.bin")).unwrap(), contents);

    // Closing it closes that call only: the computer's own endpoint is not
    // the phone's to close.
    client.close();
    qurb_peer::set_direct_link(None);
    assert!(qurb_peer::connection_on_link(&laptop_fp).is_none(), "forgotten with the link");
}
