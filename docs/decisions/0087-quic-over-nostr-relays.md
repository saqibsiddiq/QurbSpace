# 0087: QUIC over the public Nostr relays, when no other path reaches

**Status:** Accepted and built, 2026-11-13. Measured between two processes on
the owner's laptop across the live default relays; **not yet tried between
the phone on mobile data and the laptop.** Amends
[0082](0082-each-users-computer-is-their-server.md), which had no relay of
ours and Tor as the last resort.

## What was asked

The owner, 2026-11-13: "we have to find a way to make it work from anywhere"
(on Jio, Vi, Airtel, any network) "i do not want our users to download 2
applications and the setup must be simple as hell". No VPS. Asked whether the
laptop could be its own relay, then whether Syncthing's relays could carry
it, then: "go ahead, build the nostr path".

## What was found that day

The new phone, an OPPO on Jio, against the laptop at home on Excitel:

- Jio's mobile network gave the phone no public IPv4 address that a STUN
  server would answer for ("no STUN server answered; UDP may be blocked
  outbound"), only IPv6. The laptop has no IPv6 at home. So no direct path:
  not IPv6, not IPv4 punching, not the port sweep.
- Tor did not get through from Jio: "Failed to obtain hidden service circuit",
  twice, within half a minute each. From the laptop, a cold Tor reached the
  laptop's own onion service in 58 s and a warm one in under 1 s
  (`examples/torprobe.rs`), but the laptop's onion service was also failing to
  publish its descriptors much of the time.
- The phone's announcements through the Nostr relays reached the laptop in
  every test.

## Decided

- **The same QUIC session runs over the Nostr relays** when they are the
  meeting point (`qurb_signal::nostr::datagrams`, `qurb_peer::nostr_path`).
  Each QUIC packet is an ephemeral event (kind 25050, not stored), sealed with
  a key derived from the group, tagged with a topic that names, to the group
  alone, the device it is for and changes every day, and signed by a key made
  for the session. The devices' pinned identities and end-to-end encryption
  hold as on any path; the relays could drop packets, never read or change
  them. What a relay learns: that some key posted to a topic it cannot link to
  a person, from an IP address, when, and how much.
- **Raced, not tried last**: away from the device's network, the rendezvous
  introduction starts at once, and Tor to a kept onion address and the Nostr
  path start four seconds later; the first to connect is kept.
- **Both ends accept on it**, as on the relay path, and a phone answers on the
  connection it made (2026-11-13, same day), so the laptop can collect from a
  phone it cannot dial.
- **At most 60 packets a second**, each posted to every default relay: about
  60 KB/s of posts. Measured without a relay refusing; more would ask more of
  volunteers' relays.

## Measured

`crates/peer/examples/nostrprobe.rs`, two members of a throwaway group in one
process on the owner's laptop (Excitel, Wi-Fi), across the six default relays,
2026-11-13:

| cap | handshake | round trip | 256 KB there and back |
|---|---|---|---|
| 25 a second | 0.5 s | 454 ms | 30.3 s, 16.9 KB/s each way |
| 60 a second | 0.4 s | 404 ms | 9.9 s, 51.8 KB/s each way |
| 60 a second, again | 0.5 s | 513 ms | 14.8 s, 34.5 KB/s each way |

## Amended 2026-11-13: what each path is for

The owner: "if we are either sending or backing up, it should first try home
wifi then direct connection then drive; but if we are remote accessing, it
should try home wifi then direct connection then tor or nostr relay whichever
is better; ... the operations must be executed within seconds".

- **To move files** (`Need::Transfer`): home Wi-Fi, then directly
  (introduced, IPv6, punching, the port sweep), within 10 s; what does not go
  that way goes through the mailbox
  ([0088](0088-a-mailbox-in-the-persons-google-drive.md)). Tor and the Nostr
  relays are not used to move files: tens of kilobytes a second.
- **To look** (`Need::Access`: browsing a computer's folder, opening a file,
  an approval): home Wi-Fi, then directly, and Tor and the Nostr relays two
  seconds behind, within 15 s. If the Nostr relays connect first, Tor is
  waited for three seconds more and kept if it comes: it carries far more.
- Waits shortened to fit: the introduction 6 s (was 20), each address 4 s
  (was 8), the port sweep 6 s (was 12).

## Consequences

- Away from home on any network that lets a phone reach a public WebSocket
  server, the phone and its computer are connected: approvals, file lists,
  sends and small files work, at tens of kilobytes a second. A backup of
  gigabytes does not move this way in any useful time; backups stay on Wi-Fi
  by default (decision 0086).
- QurbSpace now leans on volunteers' relays for data, not only meetings. If a
  relay starts refusing, the others carry on; if they all did, the path is
  gone and the rest still works.
- Metadata: the relays see IP addresses and volumes, as any relay would. Tor
  would hide them, and stays as a path where it works.

## Left undone

- Not yet tried from the phone on Jio, Airtel or Vi.
- No figure for how the relays behave under a long transfer, or with many
  users: the cap is a guess at what is polite, not a measured limit.
- The relays a device uses are the defaults for everyone; nothing yet spreads
  users across more relays.
