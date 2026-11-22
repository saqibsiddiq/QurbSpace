//! A direct link to a phone for a large transfer (decision 0092): the phone
//! makes a Wi-Fi network of its own, its hotspot or a Wi-Fi Direct group, and
//! offers it ([`qurb_peer::wire::Request::DirectLink`]); this computer joins
//! it while the transfer runs, so the bytes cross the air once instead of
//! twice through the home router (2026-11-19: 10 MB/s through the router,
//! whatever carried them).
//!
//! Joined through NetworkManager's command line, which lets the person's own
//! account manage Wi-Fi without root. On Linux only, and only where there is
//! NetworkManager and a Wi-Fi device; elsewhere offers are declined, and the
//! transfer goes the usual way.
//!
//! What the phone sends is used as arguments, each one whole, never through a
//! shell; and checked first. Only this person's own phones are heard
//! (`qurb_peer::server`).

use qurb_peer::wire::{DIRECT_HOTSPOT, DIRECT_P2P};
use qurb_peer::DirectLinkOffer;
use std::process::Command;
use std::time::{Duration, Instant};

/// The connection's name in NetworkManager, made for the link and deleted
/// after it.
const NAME: &str = "qurb-direct";
/// How long a link is kept at most, if the phone never ends it.
const LONGEST: Duration = Duration::from_secs(60 * 60);

/// Take offers from now on, where this computer can join them.
pub fn start() {
    if !cfg!(target_os = "linux") || device("wifi").is_none() {
        return;
    }
    let Some(mut offers) = qurb_peer::take_direct_links() else { return };
    tokio::spawn(async move {
        while let Some(offer) = offers.recv().await {
            // One at a time: a second offer while one is joined replaces it.
            let _ = tokio::task::spawn_blocking(move || join(&offer)).await;
        }
    });
}

/// The first device of a kind (`wifi`, `wifi-p2p`) NetworkManager manages.
fn device(kind: &str) -> Option<String> {
    let out = Command::new("nmcli").args(["-t", "-f", "DEVICE,TYPE", "device"]).output().ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_once(':'))
        .find(|(_, t)| *t == kind)
        .map(|(d, _)| d.to_string())
}

/// Whether the offer is one this computer may act on: a name a Wi-Fi network
/// can have, a password WPA takes, a protection it knows, an address.
fn checked(offer: &DirectLinkOffer) -> Result<(), &'static str> {
    let printable = |s: &str| s.chars().all(|c| !c.is_control());
    if offer.name.is_empty() || offer.name.len() > 32 || !printable(&offer.name) {
        return Err("a network name no Wi-Fi network has");
    }
    if !(8..=63).contains(&offer.password.len()) || !printable(&offer.password) {
        return Err("a password WPA does not take");
    }
    if !matches!(offer.security.as_str(), "wpa-psk" | "sae") {
        return Err("a protection this computer does not join");
    }
    if offer.how == DIRECT_P2P {
        let hex = offer.device.split(':').collect::<Vec<_>>();
        if hex.len() != 6 || !hex.iter().all(|h| h.len() == 2 && h.chars().all(|c| c.is_ascii_hexdigit())) {
            return Err("no Wi-Fi Direct address");
        }
    } else if offer.how != DIRECT_HOTSPOT {
        return Err("a kind of link this computer does not know");
    }
    Ok(())
}

fn nmcli(args: &[&str]) -> Result<String, String> {
    let out = Command::new("nmcli").args(args).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn join(offer: &DirectLinkOffer) {
    if let Err(why) = checked(offer) {
        tracing::warn!(phone = %offer.from.short(), why, "a direct link offered was not joined");
        return;
    }
    // A link left from before goes first.
    let _ = nmcli(&["connection", "delete", NAME]);
    let started = Instant::now();
    // Beside the usual network first: a Wi-Fi Direct client, so this
    // computer stays online while the link lasts. Only for a Wi-Fi Direct
    // group; tried briefly, since a search that finds nothing waits out its
    // whole time. On 2026-11-22 the group on another channel than the home
    // network was not found this way ("the Wi-Fi P2P peer could not be
    // found"); the group is now on the home network's channel.
    let mut beside = false;
    if offer.how == DIRECT_P2P {
        if let Some(p2p) = device("wifi-p2p") {
            let added = nmcli(&[
                "connection", "add", "type", "wifi-p2p", "ifname", &p2p, "con-name", NAME,
                "wifi-p2p.peer", &offer.device, "connection.autoconnect", "no",
            ]);
            beside = added.is_ok() && nmcli(&["--wait", "20", "connection", "up", NAME]).is_ok();
            if !beside {
                tracing::info!(phone = %offer.from.short(), "not joined as Wi-Fi Direct; as a Wi-Fi network, if nobody is using this computer");
                let _ = nmcli(&["connection", "delete", NAME]);
            }
        }
    }
    // Otherwise as any Wi-Fi network: the phone's hotspot, or its group,
    // which is a Wi-Fi network too. That leaves the usual network while the
    // link lasts, so this computer is offline meanwhile: done only when
    // nobody is at it (its screen locked or idle), never under somebody's
    // call or download (2026-11-22: "how do we ensure maximum speed for
    // everyone by default?").
    if !beside {
        if !away() {
            tracing::info!(phone = %offer.from.short(), "somebody is using this computer: the transfer goes the usual way");
            return;
        }
        let Some(wifi) = device("wifi") else { return };
        let added = nmcli(&[
            "connection", "add", "type", "wifi", "ifname", &wifi, "con-name", NAME, "ssid", &offer.name,
            "wifi-sec.key-mgmt", &offer.security, "wifi-sec.psk", &offer.password,
            "connection.autoconnect", "no",
        ]);
        if let Err(e) = added {
            tracing::warn!(phone = %offer.from.short(), error = %e, "a direct link could not be set up");
            return;
        }
        if let Err(e) = nmcli(&["--wait", "60", "connection", "up", NAME]) {
            tracing::warn!(phone = %offer.from.short(), error = %e, "a direct link could not be joined");
            let _ = nmcli(&["connection", "delete", NAME]);
            return;
        }
    }
    let how = match (beside, offer.how) {
        (true, _) => "Wi-Fi Direct, beside the usual network",
        (false, DIRECT_HOTSPOT) => "the phone's hotspot",
        _ => "the phone's Wi-Fi Direct group, as a Wi-Fi network",
    };
    let net = joined_net();
    tracing::info!(phone = %offer.from.short(), how, ms = started.elapsed().as_millis() as u64, net = ?net, "joined a direct link");
    // The phone at the far end, tried first from now on, and every
    // connection kept to it made again.
    qurb_peer::set_direct_link(net);
    // Kept while it lasts: the phone ends it when the transfer is done, and
    // then it is deleted here, and NetworkManager returns to what it had.
    loop {
        std::thread::sleep(Duration::from_secs(5));
        let state = nmcli(&["-t", "-f", "GENERAL.STATE", "connection", "show", NAME]).unwrap_or_default();
        let up = state.contains("activated");
        if !up || started.elapsed() > LONGEST {
            let _ = nmcli(&["connection", "delete", NAME]);
            qurb_peer::set_direct_link(None);
            tracing::info!(how, minutes = started.elapsed().as_secs() / 60, "a direct link ended");
            return;
        }
    }
}

/// Whether nobody is at this computer: a graphical session of this person's
/// with its screen locked or idle (logind). Not known counts as somebody.
fn away() -> bool {
    let user = std::env::var("USER").unwrap_or_default();
    let Ok(out) = Command::new("loginctl").args(["list-sessions", "--no-legend"]).output() else { return false };
    let sessions = String::from_utf8_lossy(&out.stdout);
    let mut seen = false;
    for id in sessions.lines().filter_map(|l| {
        let f: Vec<&str> = l.split_whitespace().collect();
        (f.len() >= 4 && f[2] == user && f[3].starts_with("seat")).then(|| f[0].to_string())
    }) {
        let Ok(out) = Command::new("loginctl").args(["show-session", &id, "-p", "IdleHint", "-p", "LockedHint", "-p", "Active"]).output() else { continue };
        let said = String::from_utf8_lossy(&out.stdout);
        if !said.contains("Active=yes") {
            continue;
        }
        seen = true;
        if !(said.contains("IdleHint=yes") || said.contains("LockedHint=yes")) {
            return false;
        }
    }
    seen
}

/// This computer's address on the link and the phone's, as NetworkManager
/// was given them (the phone runs the link's DHCP).
fn joined_net() -> Option<qurb_peer::DirectNet> {
    let out = nmcli(&["-g", "IP4.ADDRESS,IP4.GATEWAY", "connection", "show", NAME]).ok()?;
    let mut lines = out.lines();
    let (address, prefix) = lines.next()?.split('|').next()?.trim().split_once('/')?;
    let gateway = lines.next()?.trim();
    Some(qurb_peer::DirectNet {
        address: address.parse().ok()?,
        prefix: prefix.parse().ok()?,
        gateway: gateway.parse().ok()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer(how: u8, name: &str, password: &str, security: &str, device: &str) -> DirectLinkOffer {
        DirectLinkOffer {
            from: qurb_sync::DeviceId::from_bytes([1; 32]),
            how,
            name: name.into(),
            password: password.into(),
            security: security.into(),
            device: device.into(),
        }
    }

    #[test]
    fn only_what_a_wifi_network_can_be_is_joined() {
        assert!(checked(&offer(DIRECT_HOTSPOT, "AndroidShare_1234", "abcdefgh", "wpa-psk", "")).is_ok());
        assert!(checked(&offer(DIRECT_P2P, "DIRECT-qb-1", "abcdefgh", "sae", "aa:bb:cc:dd:ee:ff")).is_ok());
        assert!(checked(&offer(DIRECT_HOTSPOT, "", "abcdefgh", "wpa-psk", "")).is_err());
        assert!(checked(&offer(DIRECT_HOTSPOT, "x", "short", "wpa-psk", "")).is_err());
        assert!(checked(&offer(DIRECT_HOTSPOT, "x", "abcdefgh", "none", "")).is_err());
        assert!(checked(&offer(DIRECT_HOTSPOT, "x\n", "abcdefgh", "wpa-psk", "")).is_err());
        assert!(checked(&offer(DIRECT_P2P, "x", "abcdefgh", "wpa-psk", "not an address")).is_err());
        assert!(checked(&offer(9, "x", "abcdefgh", "wpa-psk", "")).is_err());
    }
}
