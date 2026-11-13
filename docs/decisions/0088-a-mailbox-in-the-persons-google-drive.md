# 0088 — A mailbox in the person's Google Drive

**Status:** Accepted and built, 2026-11-13. Tested through a plain folder
(`crates/mailbox/tests/exchange.rs`); **not yet run against Google Drive or
between the phone and the laptop.** Amends
[0082](0082-each-users-computer-is-their-server.md) (no relay of ours, no
cloud): this uses the person's own cloud, by their choice.

## What was asked

The owner, 2026-11-13: "can we use services like google drive, dropbox and
others to make the system always work", then "google drive first, include
backups too", and that while both devices are awake it should move "as much
data as possible". Sign-in on the phone through Android's own accounts, with
no library added; the waiting space capped at 2 GB.

## Decided

- **Store and forward, through a folder both devices see.** A device
  answers offline the requests the other would make live
  (`qurb_peer::server::answer_offline`): its list, as that device would be
  shown it; packs of the contents it is not known to hold; receipts, answered
  as `Got`. So every rule about what a device is shown and what a delivery
  means holds unchanged. The receiver plans as from a live list and applies
  only what has arrived.
- **Google Drive's hidden app folder** (`drive.appdata` only): QurbSpace's own
  folder in the person's Drive, which their Drive does not show and no other
  app reads, never their files. Set up once by the owner
  ([google-drive-setup.md](../google-drive-setup.md)).
- **Sealed** with a key from the person's master key (`Purpose::Mailbox`):
  Google holds bytes it cannot read, named by random device identifiers.
  Another person's devices never hold the key, so guests stay on live paths.
- **Opt-in**, one sign-in on each device: the computer through its browser
  (PKCE, a refresh token kept in `~/.config/qurb/google-drive.json`, mode
  600); the phone through Android's accounts, a token asked for before each
  sync and replaced when Google refuses it.
- **Collected from always, posted to only when the device was not reached
  live**: a minute apart at most on the computer; within what is left of a
  phone's pass.
- **2 GB** waiting from one device at most; packs of 4 MB, sixteen MB read
  under the engine at a time and sent without it; **files over 64 MB wait for
  a live path**, so a phone never holds a whole video in memory.
- Backups go this way as the tree a device would be shown says: WhatsApp and
  chosen folders only while the phone offers them (Wi-Fi, or mobile data set
  to *Everything*, decision 0086).

## Consequences

- With both devices awake and no live path, it runs as a pipe, seconds
  behind, at the speed of the slower of the phone's upload and the computer's
  download; with one asleep, up to 2 GB waits for it.
- Google learns that the account uses QurbSpace, and when and how much.
- Until Google verifies the project, only its test users can sign in, and
  must sign in again every seven days.

## Left undone

- Not run against Google Drive yet, nor between the devices.
- Files over 64 MB; a resumable upload for larger packs.
- WebDAV as a second place, for people who would rather not use Google.
- Showing in the window and on the phone that something waits in the mailbox.
