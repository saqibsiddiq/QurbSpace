# The platform tests

Asked for on 2026-11-16 ("cant we do a testing of our windows built and
other linux built as well? is there anyway?"): we have one Linux laptop and
no Windows machine. GitHub has both, free for a public repository, so
[`.github/workflows/platforms.yml`](../.github/workflows/platforms.yml) runs
the released programs there: once the release workflow has finished, by hand for any
release (`gh workflow run platforms.yml -f tag=v0.2.0-beta.12`), and on a push
to the `platform-tests` branch.

The released files are not downloaded from the release: GitHub counts every
download, and until 2026-11-22 these tests made about fourteen a run, which
the dashboard showed as people. The first job, `released`, takes them from
the release build's own artifacts ([`scripts/released-files.sh`](../scripts/released-files.sh))
and checks each against the published file's digest, so what runs is byte
for byte what people download. The artifacts are kept 90 days: an older
release can no longer be tested this way.
Tried on 2026-11-22 (run 45032421923, beta.13): every job passed, and
beta.13's counts were the same after as before (AppImage 6, .deb 7, Windows
installer 2), where the old way would have added fourteen.

## What each job proves, and what it does not

| Job | Runs | Proves | Does not prove |
|---|---|---|---|
| `windows-install` | the release's `QurbSpace-windows-setup.exe` on Windows Server 2025 (GitHub's `windows-latest`) | it installs without a question, starts as it does at login, and its engine sets up and listens; it uninstalls | the window: nobody looks at it |
| `windows-engine` | the engine's own tests, every crate but the phone's and the window's | 900 and more tests on Windows' file system and network | anything a person does |
| `two-devices` | two devices through the command line, on Windows and on Ubuntu 24.04 ([`scripts/two_devices.py`](../scripts/two_devices.py)) | paired by code (the numbers compared and approved); a file in a folder each way, nested folders included; a 3.2 MB send each way into Downloads; every byte checked | the window; two machines (both devices are on one) |
| `linux-start` | the AppImage on Ubuntu 22.04 and 24.04, Debian 12, Fedora, Arch and openSUSE Tumbleweed, the .deb where it installs (Ubuntu, Debian); each in its own container | it runs on each, with what a desktop of it has, sets up and listens | the window on any of them; a real desktop session (a virtual display, and WebKit's sandbox off, which a container cannot give) |
| `linux-window` | the window itself, every function that needs no phone ([`scripts/desktop-smoke.sh`](../scripts/desktop-smoke.sh), `SMOKE_MODE=full`), on Ubuntu 24.04 with the released .deb | setup, pairing by six digits, sending each way, pausing, freeing and fetching, deleting, the Vault, versions, and the rest, through the real window and engine | other distributions' window; Windows' window |

Not run anywhere: **the window on Windows** (it needs Microsoft's WebDriver
for WebView2; not done yet), **macOS** (no build), **a phone** (the phone is
tested on the owner's devices: [testing.md](testing.md)).

## What the first runs found, 2026-11-16

On Linux, nothing: every distribution started, and the full window test
passed once one of its steps stopped asking for Recently deleted, removed in
[decision 0089](decisions/0089-no-recently-deleted.md).

On Windows, the app installed and started; the engine did not hold up
(910 of 917 of its tests passing at first, all 917 after the fixes, run
43926713489):

| Found | What it meant | Fixed |
|---|---|---|
| A path with `..\` or `C:\` from a paired device was written where it named | a device could write outside the QurbSpace folder on Windows | refused on Windows: `\` and `:` (`qurb_sync::is_safe_path`) |
| Every file sent to a Windows computer: *Access is denied* | nothing could be sent to Windows | the file flushed to disk through a handle that may write, which Windows requires (`take_into_downloads`) |
| A folder made where the QurbSpace folder was just removed counted as the old one | its files could have been taken for deleted | a mark inside the store, not the creation time Windows hands on (`Here` in the daemon) |
| `PATH` split on `:` | a program on the path not found on Windows | split as the platform splits it |
| Tests assuming Linux (`/proc`, a battery's name with a colon, links, two names differing only in case, `/` in listed paths, a program's name without `.exe`) | the tests, not the app | each limited to where it applies, or written for both |

Measured on Windows (GitHub's runner, two devices on one machine): a file
each way in 0.5 s, a 3.2 MB send each way in 0.5 s.

## Known gaps

- **A task the daemon starts outlives it on Windows.** The replica tests
  waited minutes for their runtime to shut down after the daemon had stopped
  on time; what keeps it is not found. The app ends its process when quit, so
  a person does not wait on it; the tests now let the runtime go after 5 s.
- **Placeholders for other people's folders are not read-only on Windows**
  (Windows' read-only attribute does not stop a folder taking files), as
  `layout.rs` already said: a file put in one there is not synced.
- **The window on Windows has not been driven by a test**, nor looked at by
  a person.
- **`Save a report` on Windows has no log**: it reads `journalctl`.
