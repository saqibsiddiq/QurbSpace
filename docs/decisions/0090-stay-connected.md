# 0090 — Stay connected: the phone listens, instead of push

**Status:** Accepted and built, 2026-11-14. Amends
[0082](0082-each-users-computer-is-their-server.md), which dropped push and
with it any way to wake a phone. **Not yet measured on a phone.**

## What was found

The device pass of 2026-11-14 (docs/beta.md): a file sent from the laptop to
the S23 with QurbSpace in the background was not collected in four minutes.
Android freezes a backgrounded app, the laptop's "something is waiting for
you" (sent on every round, `announce_deliveries`) reached nobody, and the
file waited for the app to be opened or a background pass, fifteen minutes
apart at best.

## What was asked

The owner: "is there no way to push notification, no work around?" Offered:
push through Google sent from a small cloud function (a server of ours), or
the phone kept listening under a foreground service (none). Chosen: "lets
build 2 for beta, but the notification must be minimal, pretty and silent and
also optional".

## Decided

- **Stay connected**, on unless turned off (Settings → Connection): a
  foreground service (`StayConnected.kt`) waits for one of the person's
  computers to say it has news (`Qurb::wait_for_news`: the home Wi-Fi's
  beacons, and the meeting point from anywhere), and runs a sync pass when
  one does. Ten minutes a wait, then afresh, so a phone that moved networks
  listens from where it is.
- **Its notification**: its own channel at the lowest importance -- silent,
  no vibration, no status-bar icon on most phones, folded at the foot of the
  shade -- the word *Connected* in the app's green, hidden on the lock
  screen. Turning the switch off removes it.
- Started when the app opens, after every background pass, when the phone
  starts, and when the app has updated itself (`StartAgain.kt`): an update
  stops the app, and on the S23 it was listening again seconds after one. Android 14's type for it is `specialUse`, with
  the reason given in the manifest; Google Play would review that.

## Consequences

- A file sent to a phone arrives in seconds while the phone is on any
  network, as long as Android lets the service run. The battery exemption
  the app already asks for makes that likely; some phones (OPPO, Xiaomi) may
  still stop it, and the next background pass starts it again.
- An idle connection to the meeting point's relays, all day: some battery,
  unmeasured.
- Push through a function of ours stays the way to drop the notification,
  before a public release.

## Measured

The S23 on home Wi-Fi, QurbSpace in the background, 2026-11-14: a 13-byte
file sent from the laptop at 21:43:53 was heard of within a second, the
laptop reached at 21:43:57, and the file collected 11.2 s after the send
(the laptop's send list checked every half second). Before this, the same
send was not collected in four minutes.

## Left undone

- Not measured: battery over a day, whether the OPPO
  keeps it running.
