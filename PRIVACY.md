# Privacy

QurbSpace moves your files between your own devices. It has no account, no
server of ours, and no analytics. This page says everything that does leave
your devices, and to whom. It describes the open beta, November 2026.

## Your files

- **Between your devices** they travel encrypted end to end (QUIC with TLS
  1.3), and each device accepts only the devices you paired it with. On your
  home Wi-Fi they go straight across it.
- **Through the internet**, they go directly between your devices when the
  networks allow it. When they do not, the encrypted connection is carried
  through **public Nostr relays** or the **Tor network**. Neither can read
  it.
- **Through Google Drive**, only if you turn it on: QurbSpace writes into its
  own hidden app folder in your Drive, never your other files. What it puts
  there is sealed on your device with a key Google does not have.

Nothing is ever stored on a server of ours. There is none.

## What others can see

| Who | What they learn | When |
|---|---|---|
| Public **Nostr relays** (relay.damus.io, relay.primal.net, nostr.oxtr.dev, offchain.pub, relay.nostr.net, nostr-pub.wellorder.net) | Your IP address, when your devices meet and roughly how much they send. Not who you are: the names used there are random and change daily. | Whenever your devices look for each other away from the same Wi-Fi |
| **STUN servers** (Cloudflare, Google) | Your IP address, as any video call reveals it | When a device looks for its own public address |
| The **Tor network** | As Tor is designed: no single relay learns both ends | Browsing your computer from the phone, when other paths fail |
| **Google**, if you turn on Drive | That your account uses QurbSpace, when, and how much; not the contents | Only with Drive on |
| **Google's backup** of your phone | Your QurbSpace key, so a new phone can be set up again — kept only when Android encrypts that backup end to end with your screen lock | Android phones with Google Play services |
| **GitHub** | Your IP address, when the apps check for an update | About once a day |

## What QurbSpace does not do

- No accounts, no sign-up, no email address asked for.
- No analytics, no tracking, no crash reporting. *Save a report* (Settings
  → Advanced) saves a file on your device -- how your devices connected,
  with their network addresses, and no file contents; it goes nowhere unless
  you send it yourself.
- No ads, and no selling of anything.

## On your devices

The phone app keeps its index, keys and caches in its own private storage.
The computer keeps your devices' backups in its QurbSpace folder, encrypted
for the phone's Private Vault. Removing a device, or resetting the phone app,
deletes what that device holds of QurbSpace's own data; files you saved out
stay.

## Questions

sddqsaqib@gmail.com
