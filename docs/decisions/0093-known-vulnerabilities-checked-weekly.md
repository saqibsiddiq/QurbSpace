# 0093: Known vulnerabilities checked weekly, and what is accepted

**Date:** 2026-11-23
**Status:** built; the first runs found what is below

## What was asked

The owner, on the risk of someone exploiting a vulnerability to reach a
device or file they were never allowed: of three steps offered (a check for
libraries with published flaws, fuzzing what runs before a device is known,
an independent review), "do this" for the first, and "yes go ahead" to fix
what it found.

## Decided

- **A check, weekly and on every change to a lock file**
  ([`.github/workflows/audit.yml`](../../.github/workflows/audit.yml)): the
  Rust crates against the RustSec database (`cargo audit`), the website's
  packages against npm's (`npm audit`, what runs on the site, high and
  above). A failed run is GitHub's email to the owner.
- **Fixed what could be fixed**, the same day:
  - the website from Next.js 16.2.10 to 16.4.6: 25 advisories, among them
    unauthenticated code execution through image optimisation and `use
    cache`; with it postcss, nanoid, sharp, source-map-js and
    baseline-browser-mapping. The site built, typed and linted after.
  - in Rust, within the versions already chosen: quinn-udp 0.5.16 (a
    truncated datagram from anyone could make the receive loop spin: the
    computer listens on the internet), zstd and zstd-safe (memory safety),
    rustls-webpki and webpki-roots (distrusted certificate intermediates),
    tokio, url, rustix, ctutils.
  - the updater's zip support turned off (`default-features = false` on
    `tauri-plugin-updater`): it unpacks only a Windows update packed as a
    zip, and QurbSpace's are a signed installer. Six zip advisories went
    with the library.
- **Accepted, where no fixed version can be taken**
  ([`.cargo/audit.toml`](../../.cargo/audit.toml), each with its reason):
  rsa (Marvin timing; through Tor), crypto-bigint 0.5 (timing; through the
  Nostr signatures' k256), libsqlite3-sys 0.38 (SQLite's own flaw: Tor,
  arti 0.47 and the latest 0.48, requires rusqlite below 0.41, and only one
  SQLite can be linked), glib 0.18 (an iterator QurbSpace does not use;
  through GTK). Still listed by each run, without failing it. Looked at on
  each release and by 2027-02-01; one comes off the day its fix can be taken.
- Crates only *unmaintained* (bincode, chrono, paste, the unic crates and
  others) are warnings and stay so.

## Not done, and why

- **The Android app's Gradle libraries** are not checked: no check as plain
  as these exists for them without a heavier tool. Next.
- **Fuzzing and an independent review**, the other two steps, are not
  started.
- **The fixes reach people only with a release**: the website on its next
  deployment; the apps with beta.14.
