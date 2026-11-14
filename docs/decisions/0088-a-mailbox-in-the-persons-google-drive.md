# 0088 — A mailbox in the person's Google Drive

**Status:** Accepted and built, 2026-11-13. Tested through a plain folder
(`crates/mailbox/tests/exchange.rs`), and once between the S23 and the laptop
through Google Drive: three screenshots sent with the laptop off, collected
when it started ([testing.md](../testing.md)). Amends
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

- One run between devices only; no measure yet of speed through Drive.
- Files over 64 MB; a resumable upload for larger packs.
- WebDAV as a second place, for people who would rather not use Google.
- Showing in the window and on the phone that something waits in the mailbox.

## Amended 2026-11-14: each phone's own account, handed to the computer

The owner, 2026-11-13: the new phone signs in to Google with another account
than the laptop's. Then, 2026-11-14: "include setting up google account
during the setup of the phone / and when that phone is connected to a laptop,
the laptop must also get that google account associated with that phone"; and
yes to Google's sign-in library (`play-services-auth` 21.3.0) for it.

- **A computer may be signed in to several accounts** (commit d6b286c,
  documented here late). `~/.config/qurb/google-drive.json` holds a list; the
  old single sign-in is read as a list of one. A phone leaves a `hello`
  letter for each of its computers in its own account's folder; the computer
  collects from every account, and posts to a phone only in the account its
  hello was found in. The window lists the accounts, adds one, removes one.
- **The phone signs in through Google's sign-in**, not Android's account
  list: Google's account sheet, the `drive.appdata` permission, and with it
  a one-time code made for QurbSpace's *web* client
  (`docs/google-drive-setup.md`, step 8). The access token is asked for
  silently before each sync, as before.
- **The phone hands the code to each computer it reaches**
  (`Request::DriveCode`, wire tag 39, own devices only), before the sync.
  The computer exchanges it with the web client's secret
  (`~/.config/qurb/google-web-client.json`, mode 600, not committed) for a
  lasting sign-in to the phone's account, and keeps it beside its own. So
  the person signs in once, on the phone, and the computer is never asked.
  Only a sign-in asks for a code, with the person's yes; a code asked
  for in the background brought no lasting sign-in, and took the place of
  the one that did (found 2026-11-14, below).
- **Asked during setup**: *Connected* has a third switch, Google Drive, on by
  default; Google's sheet follows in the app, and can be cancelled.

Tried 2026-11-14 on the OPPO CPH2363 (0.2.0-beta.10, home Wi-Fi, the laptop
on the commit that adds this): signed in from its Settings to an account other
than the laptop's; the phone handed the code at 17:42:30 IST, the laptop took
it on its next mailbox round 62 s later ("signed in to a phone's Google
account") and its window listed the account; nobody signed in on the laptop.
Since the same day the computer exchanges a code the moment it is handed
(`qurb_peer::drive_code_handed`), not on its next round; not yet timed.

Left undone: the setup switch not yet tried; nothing yet posted or collected
through the handed account (the phone was reachable directly). A code is single-use and short-lived, so
one handed to a computer that is off is wasted until the next day's.

## Amended 2026-11-14: only what was asked for

Found measuring the phones' space (testing.md, 2026-11-14): the computer
posted every file in a phone's list that the phone was not known to hold, so
the phone collected packs of files its plan never fetches -- shared files it
opens from the computer, for one -- and posted again each day. The OPPO had
137 MB of them waiting, none applied ("applied=0 waiting=0"); the S23 346 MB.

- After planning from a list, a device leaves a **`want`** letter: the
  contents its plan still waits for from the other, sealed, one letter, the
  older taken away, and written again only when it changed.
- The other posts **only those**. A device that has never left one -- a build
  before this -- is posted to as before.
- Arrivals no plan wants are let go of a day after they came
  (`Mailbox::tidy_inbox`).

So a new file reaches the other device one round later than before: its list
first, then the want, then the packs. Tested by `only_what_was_asked_for_is_posted`
(`crates/mailbox/tests/exchange.rs`), through a folder; not yet through Drive.

Found in the device pass, 2026-11-14 (docs/beta.md): the OPPO, set up
again, chose its account at setup, and the background token refresh asked
Google for a code of its own seconds later; that code replaced the setup's
before it was handed, and Google gives a lasting sign-in only with a code
asked for with the person's consent ("Google gave no refresh token for the
phone's code"). Now the refresh asks for the token alone.

