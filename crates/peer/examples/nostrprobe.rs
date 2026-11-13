//! QUIC across the live public Nostr relays, measured: two members of a
//! throwaway group in one process, the handshake timed, then 256 KB sent one
//! way and echoed back. `cargo run --release --example nostrprobe`. Posts a few
//! hundred ephemeral events, sealed, to the default relays.

use qurb_signal::{nostr, GroupId, MemberId};
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() {
    use rand::RngCore;
    let mut seeds = [[0u8; 32]; 3];
    for seed in &mut seeds {
        rand::rngs::OsRng.fill_bytes(seed);
    }
    let identity_a = qurb_peer::Identity::derived(&seeds[0]).expect("an identity");
    let identity_b = qurb_peer::Identity::derived(&seeds[1]).expect("an identity");
    let group = GroupId::from_bytes(seeds[2]);
    let (a, b) = (MemberId::from_bytes([0xa1; 32]), MemberId::from_bytes([0xb2; 32]));

    let server = qurb_peer::nostr_path::NostrSocket::over(nostr::datagrams("nostr", group, a));
    let server_end = qurb_peer::nostr_path::endpoint_over(
        server,
        Some(qurb_peer::tls::server_config(&identity_a, &qurb_peer::tls::TrustList::new(vec![identity_b.fingerprint()])).unwrap()),
    )
    .unwrap();
    tokio::spawn(async move {
        while let Some(incoming) = server_end.accept().await {
            tokio::spawn(async move {
                let Ok(connection) = incoming.await else { return };
                while let Ok((mut send, mut recv)) = connection.accept_bi().await {
                    let bytes = recv.read_to_end(1 << 20).await.unwrap_or_default();
                    let _ = send.write_all(&bytes).await;
                    let _ = send.finish();
                }
            });
        }
    });

    let client = qurb_peer::nostr_path::NostrSocket::over(nostr::datagrams("nostr", group, b));
    let to_a = client.address_of(a).unwrap();
    let client_end = qurb_peer::nostr_path::endpoint_over(client, None).unwrap();
    // The relays connect in the background: a moment for them first.
    tokio::time::sleep(Duration::from_secs(4)).await;

    let started = Instant::now();
    let config = qurb_peer::tls::client_config(&identity_b, identity_a.fingerprint()).unwrap();
    let connection = match tokio::time::timeout(Duration::from_secs(60), client_end.connect_with(config, to_a, "qurb-device").unwrap()).await {
        Ok(Ok(c)) => c,
        Ok(Err(e)) => return println!("handshake failed after {:.1} s: {e}", started.elapsed().as_secs_f64()),
        Err(_) => return println!("no handshake in 60 s"),
    };
    println!("handshake over the relays: {:.1} s, rtt {} ms", started.elapsed().as_secs_f64(), connection.rtt().as_millis());

    for size in [4 << 10, 256 << 10] {
        let payload = vec![7u8; size];
        let started = Instant::now();
        let (mut send, mut recv) = connection.open_bi().await.unwrap();
        send.write_all(&payload).await.unwrap();
        send.finish().unwrap();
        match tokio::time::timeout(Duration::from_secs(180), recv.read_to_end(1 << 20)).await {
            Ok(Ok(back)) if back == payload => {
                let secs = started.elapsed().as_secs_f64();
                println!("{} KB there and back: {secs:.1} s, {:.1} KB/s each way", size >> 10, (size as f64 / 1024.0) / secs * 2.0);
            }
            Ok(Ok(back)) => println!("{} KB: {} bytes came back", size >> 10, back.len()),
            Ok(Err(e)) => println!("{} KB: failed: {e}", size >> 10),
            Err(_) => println!("{} KB: not back in 180 s", size >> 10),
        }
    }
}
