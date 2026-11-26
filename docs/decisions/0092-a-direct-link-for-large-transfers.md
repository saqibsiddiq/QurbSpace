# 0092: A direct link for large transfers

**Date:** 2026-11-22
**Status:** built, and measured on the S23 and the laptop (amended below)

## What was asked

Measured on 2026-11-19 (testing.md): the S23 and the laptop, both on the
home router's Wi-Fi, carried 9 to 10 MB/s by plain TCP phone to laptop, 7 the
other way, and four connections at once no more; QurbSpace had measured 10.6
to 14. The phone's link said 351 Mbit/s. Every byte crosses the air twice
through the router, on one channel, and Wi-Fi's own overhead takes about 40%
of what is left. The owner asked for 30 to 40 MB/s, and chose two ways to try
for it: "i find 2 and 5 interesting":

- **2:** the computer on the phone's own hotspot: one hop.
- **5:** Wi-Fi Direct between them: one hop, and the computer stays on its
  usual network. The laptop's card (Intel Raptor Lake CNVi) runs a Wi-Fi
  Direct client beside its usual connection (`iw list`), and NetworkManager
  lists `p2p-dev-wlan0`.

## Decided

**The phone makes the network; the computer joins it; the transfer goes over
it; it ends with the transfer.**

1. **Phone.** For a pass with 32 MiB or more waiting for another device (a
   long pass, decision 0050) and *Fast link* not off (Advanced), the app
   makes the network before the pass: a Wi-Fi Direct group it owns, with a
   name and a 20-character password of its own (`WifiP2pConfig`, 5 GHz), or
   Android's local-only hotspot (no internet shared, no setting changed).
   Android 13 and later, with the Nearby Wi-Fi devices permission asked
   without location. `DirectLink.kt`.
2. **Offered** at the pass's first reach of a computer
   (`Request::DirectLink`: how, name, password, protection, and for Wi-Fi
   Direct the phone's address), over QurbSpace's own encrypted connection,
   and heard only from this person's own phones.
3. **Computer.** Joined through NetworkManager's command line, which the
   person's own account may use without root: a Wi-Fi connection on the
   Wi-Fi device for the hotspot (the usual network left meanwhile), or a
   Wi-Fi Direct connection on the Wi-Fi Direct device beside it. Everything
   the phone sent is checked and passed as whole arguments, never through a
   shell. `qurb_cli::direct`.
4. **Over the link.** Joined, the computer drops the connections it keeps
   to its phones and makes them again, trying the phone first at the link's
   gateway (the phone) and in the link's network before any other path
   (`qurb_peer::direct_link`, `Connector::race`): raced together with the
   router's path, either could win.
5. **Ended** when the pass ends: the phone closes the network; the computer
   sees its connection go, deletes it, and NetworkManager returns to what it
   had. At most an hour, whatever happens.

Linux computers only: elsewhere an offer is declined and the transfer goes
the usual way. The fast link is off until the person turns it on.

## Not done, and why

- **Not tried on the devices.** Built on 2026-11-22; the phone and laptop
  were on different networks, and the owner chose to test everything
  together. Nothing here is measured yet.
- **Will the phone ask before letting the laptop join its Wi-Fi Direct
  group?** Android may show its own *invitation* dialog when a device joins
  a group; if it does, that is a tap each time. Not known until tried.
- **Only for what the phone sends.** The link is made when the phone has a
  large transfer *for* another device (its backups, a large share). A large
  file the laptop sends the phone does not make one yet: the phone does not
  know its size before the pass.
- **The phone does not prefer the link** when it dials the computer; the
  computer does when it dials the phone, which is the direction a backup
  goes.
- **Windows and macOS** decline: no NetworkManager.

## Amended 2026-11-22: tried, and changed by what was measured

Measured the same day (testing.md, 2026-11-22), 512 MB phone to laptop: the
router's path 9.1 MB/s; the phone's hotspot 8.3 (Android puts an app's
hotspot on 2.4 GHz); a Wi-Fi Direct group not found by the laptop's Wi-Fi
Direct search ("peer could not be found"), and, joined as a plain Wi-Fi
network, 8.0 on another channel than the phone's home network; **33.2, best
38.9**, on the same channel. Changed:

- **The group goes on the home network's channel** when that is 5 GHz
  (`WifiP2pConfig.setGroupOperatingFrequency`): one radio on two channels
  takes turns between them, which, not the router's hop, held every run to
  8 to 11 MB/s.
- **The laptop joins the group as a Wi-Fi network**, by its name and
  password, not as a Wi-Fi Direct client: so it **leaves its home network
  while the link lasts**, as with the hotspot. Staying on both is for later.
- **The phone hotspot is kept as a choice** but gains nothing (2.4 GHz).
  Wi-Fi Direct is the one to use.


## Amended 2026-11-22, evening: towards on by default

The owner: "how do we ensure maximum speed for everyone by default?". What
held it back was the computer leaving its usual network while the link
lasts. Built, and tried on the S23 and the laptop where it says so:

- **Taken only when nobody is at the computer**: its screen locked or idle
  (logind on Linux; Windows' lock screen running). Asked as the offer
  arrives, so a computer somebody is using says no at once and the phone
  goes the usual way without waiting: tried, 512 MB with the owner at the
  laptop, bytes moving 14 s after the tap as with no link at all. Joining
  beside the usual network as a Wi-Fi Direct client was tried first, and
  failed again with the group on the home network's channel; dropped.
- **On in setup**: *Fast transfers* among the choices of the step that
  follows pairing, on, Android's permission asked there.
- **Used only once the computer is on it**: the phone makes the link, offers
  it in a short pass, and waits for the computer to join (25 s at most);
  then the transfer. A computer that agreed and never came keeps links off
  for ten minutes; a no does not (it is free). Tried: locked, 512 MB phone to
  laptop at **29.8 MB/s** (best 36.1).
- **Both ways**: a large transfer to the phone (32 MiB) is put off by the
  pass that finds it and fetched over a link made for it. Tried, and the
  phone could not reach the computer on the link: it knew only the
  computer's home address, gone with it, and the new one went out over the
  internet the computer had just left. Fixed: a device is dialled first
  where it last connected from on the link (the computer reaches the phone
  as soon as it joins). **Not yet tried again.**
- **Windows**: joined through `netsh wlan`, with a profile written for the
  link and deleted after; taken only with the screen locked. Built and
  checked by GitHub's Windows machines; **not yet tried on a Windows computer
  with Wi-Fi.**

Still not done: staying on the usual network beside the link (the one change
that would let the link be taken while somebody is at the computer); macOS.

## Amended 2026-11-22, night: meeting over Wi-Fi Direct, tried and set aside

To keep the computer on its usual network beside the link (the one thing
between the fast link and *whoever is at the computer*), the two were made
to **meet**: the phone offers its Wi-Fi Direct name and address, the
computer answers with its own, and each connects to the other, the phone
asking to own the group (`DIRECT_MEET`; `DirectLink.meet`;
`qurb_cli::direct::wifi::join`). Found on the way, with the phone's group held
a minute by a long press on *Fast link*:

- NetworkManager does not **join** a group another device owns: it asks to
  **make** one ("GO-NEG-REQUEST" in the phone's log), which a phone already
  owning one ignores. Hence the meeting.
- Android gives an app the phone's **own** Wi-Fi Direct address
  (`d4:ab:cd:92:19:cf`) and uses a **random** one on the air
  (`ce:f9:f0:87:f0:49`): NetworkManager looked for the first and saw only the
  second, "peer not found". The first try of all, that morning, failed the
  same way. Now found by its name.

Three tries, the owner at the laptop each time; the laptop stayed on its home
network throughout, every time:

| try | what happened | the transfer |
|---|---|---|
| 1 | the laptop gave up looking after 10 s, the phone began after its 10 s pass | the usual way, bytes from 40 s, 10.2 MB/s |
| 2 | the phone looked from the start and found the laptop; the laptop, given the phone's own address, never did | the usual way, 4.9 MB/s, bytes from 50 s |
| 3 | found by name; **the laptop joined beside its home network in 13.7 s**; Android told the phone *busy* (both connecting at once), the group came up with no address for the laptop | the usual way, **3.7 MB/s** |

Slower than not trying, so **set aside** (`DirectLink.MEET = false`): the fast
link is the phone's own group, which the computer joins only when nobody is
at it. The code stays for the next try, which would have one side connect
and the other only accept, and give the computer an address on the group.

## Amended 2026-11-22, night: transfers to the phone, tried and turned off

Test 3, 256 MB laptop to phone with the laptop locked, after the dial-back
fix: the phone put the file off for a link, made it, the laptop joined
(3.8 s) and reached the phone over it; then the pass meant to fetch over the
link took nothing ("adopted=0"), the link ended, and the next pass put the
file off again: a new link every minute for eleven minutes, the file never
arriving. Worse than slow, so **turned off**: a large transfer to the phone
goes the usual way (`setDirectPossible(false)`), and the stuck file arrived
the usual way, intact, minutes after. Why the pass over the link took
nothing is not found yet. **Phone to computer is unchanged**: 29.8 to 33.2
MB/s with nobody at the computer.

## Amended 2026-11-23 to 11-26: transfers to the phone, made to work

Turned on again, and three causes found in four tries on the S23 and the
laptop (testing.md):

1. The phone fetched the moment the computer joined its Wi-Fi, before it had
   an address there: it now waits until the two have **met over the link**
   (`heard_on_direct_link`), running short passes meanwhile that listen and
   dial, the link offered once (`direct_link_offered`).
2. A dual-stack socket gave the computer's address as `::ffff:192.168.49.35`,
   which was not counted as on the link (`DirectNet::holds` now takes it).
3. The computer's **firewall** let UDP in only from its home network, so the
   phone's own call over the link timed out, while the computer's call to the
   phone passed; Windows' firewall does the same on a new network. So the
   phone **asks over the computer's call**: the computer answers requests on
   the connections it makes over the link (`serve_what_is_dialled_on_link`),
   and the phone keeps the computer's call (`connection_on_link`) and asks
   on it first (`PeerClient::on_their_connection`, which closes that
   connection only). Tried briefly first, so a computer from before this,
   which answers nothing on its own call, costs 3 s and the usual paths.

A link that was declined or carried nothing is not tried again for ten
minutes, and the file goes the usual way at once (`usualWayForNow`): no
link every minute any more.

Measured 2026-11-26, laptop locked: **256 MB laptop to phone at 35.9 MB/s**.
Not yet good: 33 s pass before the transfer starts, so the link pays only
from about 1 GB; the phone notices the computer between 4 s passes, which
is the most of it. Windows not yet tried on a PC.

