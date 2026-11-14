# The open beta

The plan for QurbSpace's first open beta, asked for on 2026-11-14 ("lets plan a
beta release then"), and what has been done of it. Kept current as items pass;
an item is ticked only once it has been tried on real devices, and says on
which.

## What the owner chose (2026-11-14)

| | |
|---|---|
| who | open to anyone, up to about 100 people |
| phone app | the APK from the GitHub release, as now; it updates itself (decision 0078) |
| computers | Linux and Windows; no macOS build |
| when | this week |
| version | `v0.2.0-beta.10`, the number the builds since beta.9 already carry |

## What testers are told

Said plainly on the release page and in the guide, not hidden:

- **It is a beta.** Keep your own copy of anything you cannot lose.
- **A deletion is for good**, on every device, since Recently deleted was
  removed (decision 0089).
- **Restoring a backup to a new phone has not been proven end to end**
  (WhatsApp's included, decision 0062).
- **Windows has never been run** by us; Linux has, on the owner's laptop.
- **Google Drive** is optional and limited while Google has not verified the
  app (below).
- What leaves the devices: files go directly between them, or encrypted
  through the person's own Google Drive if they turn it on; meeting points
  (public Nostr relays) see IP addresses and timing, never contents.

## Before the tag

Built by Claude unless marked **owner**.

- [ ] **Google Drive's clients in the release.** The laptop reads them only
  from `~/.config/qurb/` (`qurb::drive_account`), which exists on the owner's
  laptop alone, so no tester could turn Drive on. Built into the program from
  two release secrets, the file still read first where it exists.
  **Owner:** add the secrets `GOOGLE_DESKTOP_CLIENT` and `GOOGLE_WEB_CLIENT`
  (the two JSON files' contents) to the repository.
  Note: the web client's secret then ships inside the program, as the desktop
  client's does; Google treats a desktop client's secret as public, not a web
  client's. Acceptable for a beta; to revisit before public release.
- [ ] **Who may sign in to Drive.** While the consent screen is in *Testing*,
  only listed test users can (100 at most), and they sign in again every
  seven days. **Owner:** check in the console whether `drive.appdata` counts
  as a non-sensitive scope; if so, publishing the consent screen needs only
  the brand details (home page, privacy policy link) and lifts both limits.
  If not, Drive stays for testers who send their address.
- [ ] **A privacy policy and a home page**, short, for the release page and
  Google's consent screen. Drafted by Claude; **owner** publishes them.
- [ ] **A tester's guide**: install on the phone and the computer, pair, what
  to try, how to report. Drafted by Claude.
- [ ] **Reporting**: GitHub Issues with a template asking for phone, Android
  version, carrier, computer and what happened; *Send a report* on both apps
  saves a file to attach. **Owner:** confirm the repository's issues are open.
- [ ] **The release page's words**: what is new since beta.9, the notice
  above, the downloads by device.

## The device pass

Everything below on the owner's devices before the tag: the Galaxy S23, the
OPPO CPH2363 (Jio), and the laptop. The OPPO goes through as a new user,
reset first (*Reset this device*), since the testers will all be new users.

| | item | S23 | OPPO | laptop |
|---|---|---|---|---|
| 1 | Set up from nothing: install, pair by QR, and by six digits | | ✅ six digits, after two fixes: a stale key from the Google backup offered with no way to join; Google's code replaced before it was handed | ✅ |
| 2 | Setup's *Connected*: Photos, WhatsApp, Google Drive switches | | ✅ | |
| 3 | Photos backed up; WhatsApp backed up | | ✅ set up again, recognised as itself: 1,309 files checked in 0.5 s, all already on the laptop | |
| 4 | A send each way, on Wi-Fi: how many seconds | ✅ phone to laptop, 3.2 MB in 7 s, identical; laptop to phone on opening | | ✅ |
| 4b | A send to a phone with the app in the background (Stay connected, decision 0090) | ❌ before 0090: not collected in 4 min; ✅ after: 11.2 s | | |
| 5 | A send and browsing on mobile data, away from home | | | |
| 6 | A preview of a large photo, remotely | ✅ the laptop's folder listed in 3.4 s, a photo's viewer open in under 4 s (the image itself not looked at) | | |
| 7 | Laptop off: a send through Drive, collected when it starts | | | |
| 8 | The Drive account handed from the phone to the laptop | | ✅ 17:42, 62 s (before it was made instant) | ✅ |
| 9 | Settings: every control once | | | |
| 10 | Space the app takes, after a day (testing.md, 2026-11-14) | | | |
| 11 | Updates: beta.9 updating itself to beta.10, phone and laptop | | | |
| 12 | A restore drill: the app's data cleared on the OPPO, its files back from the laptop | | | |
| 13 | Windows: the zip run on a Windows machine, if one can be borrowed | | | |

Also found and fixed in the pass: the laptop had posted the S23 271 packs
(about 1 GB) through Drive that it never used, collected at 17 MB a pass
(decision 0088, amended); Settings showed "Google account" rather than the
address.

Item 11 matters most for the owner's own laptop: its installed app is
beta.5, which cannot open the current folder (testing.md, *Found, and left*).

## Release day

1. **Owner** tags `v0.2.0-beta.10` and pushes it; the workflow builds and
   publishes (decision 0078).
2. The release's own files installed on both phones and the laptop, not the
   local builds, and items 1 and 4 run once more on them.
3. The release page's words and the guide's link put up; the beta opened.

## During the beta

- Issues read daily; a fix released as the next beta, which installed copies
  fetch on their own.
- What testers' phones and networks do, collected into testing.md with their
  conditions, as every measurement is.

## Leaving the beta

Not before: the restore drill passing; Windows run by someone; Google's
verification done; no open issue that loses data; two weeks without a new
kind of failure. The public release's own list (Play Store, a security
review, macOS) is planned then.
