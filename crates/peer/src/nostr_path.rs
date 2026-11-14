//! QUIC over public Nostr relays: the path when there is no other
//! (2026-11-13). A phone on Jio had no IPv4 of its own, its laptop no IPv6,
//! and Tor did not get through; its announcements through the relays always
//! arrived. So the same QUIC session runs over them, a packet an event
//! (`qurb_signal::nostr::datagrams`): the same pinned identities and the same
//! end-to-end encryption as on any path, the relays seeing sealed bytes.
//!
//! As the relay socket does (`qurb_relay::RelaySocket`), each device spoken to
//! is given an address QUIC can use, from a range never routed (here
//! 198.51.100.0/24, RFC 5737's second, so that one in a log is plainly this
//! path) and translated to and from the device's name in the group.

use qurb_signal::MemberId;
use std::collections::HashMap;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::sync::mpsc;

/// This socket's own address, and the range its peers' come from.
const LOCAL: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 255)), 9);
const MOST: u8 = 254;

/// The datagrams of a group's devices, as a socket.
pub struct NostrSocket {
    send: mpsc::UnboundedSender<(MemberId, Vec<u8>)>,
    received: Mutex<mpsc::UnboundedReceiver<(MemberId, Vec<u8>)>>,
    peers: Mutex<HashMap<MemberId, u8>>,
    // Held: dropped, the relays are let go.
    _datagrams: Mutex<Option<qurb_signal::nostr::Datagrams>>,
}

impl std::fmt::Debug for NostrSocket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("NostrSocket")
    }
}

impl NostrSocket {
    pub fn over(mut datagrams: qurb_signal::nostr::Datagrams) -> Arc<Self> {
        let send = datagrams.send.clone();
        let (_, empty) = mpsc::unbounded_channel();
        let received = std::mem::replace(&mut datagrams.received, empty);
        Arc::new(Self {
            send,
            received: Mutex::new(received),
            peers: Mutex::new(HashMap::new()),
            _datagrams: Mutex::new(Some(datagrams)),
        })
    }

    /// The address `member` has on this path, given it now if it has none.
    pub fn address_of(&self, member: MemberId) -> Option<SocketAddr> {
        let mut peers = self.peers.lock().ok()?;
        let next = peers.len() as u8 + 1;
        let host = match peers.get(&member) {
            Some(host) => *host,
            None if next <= MOST => {
                peers.insert(member, next);
                next
            }
            None => return None,
        };
        Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(198, 51, 100, host)), 9))
    }

    fn member_at(&self, address: &SocketAddr) -> Option<MemberId> {
        let IpAddr::V4(v4) = address.ip() else { return None };
        let host = v4.octets()[3];
        self.peers.lock().ok()?.iter().find(|(_, h)| **h == host).map(|(m, _)| *m)
    }
}

#[derive(Debug)]
struct AlwaysWritable;

impl quinn::UdpPoller for AlwaysWritable {
    fn poll_writable(self: Pin<&mut Self>, _cx: &mut Context) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

impl quinn::AsyncUdpSocket for NostrSocket {
    fn create_io_poller(self: Arc<Self>) -> Pin<Box<dyn quinn::UdpPoller>> {
        Box::pin(AlwaysWritable)
    }

    fn try_send(&self, transmit: &quinn::udp::Transmit) -> io::Result<()> {
        let member = self
            .member_at(&transmit.destination)
            .ok_or_else(|| io::Error::new(io::ErrorKind::AddrNotAvailable, "no device at that address on the nostr path"))?;
        // One datagram an event, however QUIC batched them.
        let size = transmit.segment_size.unwrap_or(transmit.contents.len()).max(1);
        for packet in transmit.contents.chunks(size) {
            self.send
                .send((member, packet.to_vec()))
                .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "the nostr path closed"))?;
        }
        Ok(())
    }

    fn poll_recv(
        &self,
        cx: &mut Context,
        bufs: &mut [io::IoSliceMut<'_>],
        meta: &mut [quinn::udp::RecvMeta],
    ) -> Poll<io::Result<usize>> {
        if bufs.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let mut received = self.received.lock().expect("receiver");
        loop {
            match received.poll_recv(cx) {
                Poll::Ready(Some((from, payload))) => {
                    // A device not seen gets an address now: how one that
                    // was only listening learns to answer.
                    let Some(address) = self.address_of(from) else { continue };
                    let len = payload.len().min(bufs[0].len());
                    bufs[0][..len].copy_from_slice(&payload[..len]);
                    meta[0] = quinn::udp::RecvMeta { addr: address, len, stride: len, ecn: None, dst_ip: None };
                    return Poll::Ready(Ok(1));
                }
                Poll::Ready(None) => return Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, "the nostr path closed"))),
                Poll::Pending => return Poll::Pending,
            }
        }
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok(LOCAL)
    }

    // One packet at a time: an event is a datagram.
    fn max_transmit_segments(&self) -> usize {
        1
    }
}

/// A QUIC endpoint whose packets go over the relays.
pub fn endpoint_over(socket: Arc<NostrSocket>, server_config: Option<quinn::ServerConfig>) -> crate::Result<quinn::Endpoint> {
    let runtime = quinn::default_runtime().ok_or_else(|| crate::Error::Tls("no async runtime available".into()))?;
    quinn::Endpoint::new_with_abstract_socket(quinn::EndpointConfig::default(), server_config, socket, runtime)
        .map_err(|e| crate::Error::Io { path: "the nostr path".into(), source: e })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_device_keeps_its_address() {
        let (send, _r) = mpsc::unbounded_channel();
        let (_s, received) = mpsc::unbounded_channel();
        let socket = NostrSocket { send, received: Mutex::new(received), peers: Mutex::new(HashMap::new()), _datagrams: Mutex::new(None) };
        let (a, b) = (MemberId::from_bytes([1; 32]), MemberId::from_bytes([2; 32]));
        let at_a = socket.address_of(a).unwrap();
        assert_eq!(socket.address_of(a), Some(at_a));
        assert_ne!(socket.address_of(b), Some(at_a));
        assert_eq!(socket.member_at(&at_a), Some(a));
    }
}
