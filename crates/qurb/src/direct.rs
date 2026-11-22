//! A direct link to a phone for a large transfer (decision 0092): the phone
//! makes a Wi-Fi network of its own, a Wi-Fi Direct group or its hotspot, and
//! offers it ([`qurb_peer::wire::Request::DirectLink`]); this computer joins
//! it while the transfer runs. Through the home router the S23 and the laptop
//! carried 9 MB/s; on the phone's group, on the home network's channel, 33.
//!
//! Joined as any Wi-Fi network, which leaves the computer's usual network
//! while the link lasts: so taken only when nobody is at the computer, and
//! said no to at once otherwise (`away`). As a Wi-Fi Direct client, which
//! would keep the usual network, the group was never found
//! (2026-11-22: "the Wi-Fi P2P peer could not be found").
//!
//! Through what the person's own account may use without administrator
//! rights: NetworkManager's command line on Linux, `netsh wlan` on Windows.
//! Elsewhere, or with no Wi-Fi, offers are declined and the transfer goes the
//! usual way. What the phone sends is checked first, and passed as whole
//! arguments or written into a profile, never through a shell. Only this
//! person's own phones are heard (`qurb_peer::server`).

use qurb_peer::wire::{DIRECT_HOTSPOT, DIRECT_P2P};
use qurb_peer::DirectLinkOffer;
use std::time::{Duration, Instant};

/// The link's name where the system keeps networks, made for the link and
/// deleted after it.
const NAME: &str = "qurb-direct";
/// How long a link is kept at most, if the phone never ends it.
const LONGEST: Duration = Duration::from_secs(60 * 60);

/// Take offers from now on, where this computer can join them.
pub fn start() {
    if !wifi::available() {
        return;
    }
    // Taken only when nobody is at this computer, and said at once, so the
    // phone does not wait for a computer that will not come (2026-11-22: 70 s
    // lost so).
    let Some(mut offers) = qurb_peer::take_direct_links(wifi::away) else { return };
    tokio::spawn(async move {
        while let Some(offer) = offers.recv().await {
            // One at a time: a second offer while one is joined replaces it.
            let _ = tokio::task::spawn_blocking(move || join(&offer)).await;
        }
    });
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

fn join(offer: &DirectLinkOffer) {
    if let Err(why) = checked(offer) {
        tracing::warn!(phone = %offer.from.short(), why, "a direct link offered was not joined");
        return;
    }
    let started = Instant::now();
    if let Err(e) = wifi::join(offer) {
        tracing::warn!(phone = %offer.from.short(), error = %e, "a direct link could not be joined");
        wifi::leave();
        return;
    }
    let how = if offer.how == DIRECT_HOTSPOT { "the phone's hotspot" } else { "the phone's Wi-Fi Direct group" };
    let net = wifi::joined_net(offer);
    tracing::info!(phone = %offer.from.short(), how, ms = started.elapsed().as_millis() as u64, net = ?net, "joined a direct link");
    // The phone at the far end, tried first from now on, and every
    // connection kept to it made again.
    qurb_peer::set_direct_link(net);
    // Kept while it lasts: the phone ends it when the transfer is done; then
    // it is removed here, and the system returns to its usual network.
    loop {
        std::thread::sleep(Duration::from_secs(5));
        if !wifi::up(offer) || started.elapsed() > LONGEST {
            wifi::leave();
            qurb_peer::set_direct_link(None);
            tracing::info!(how, minutes = started.elapsed().as_secs() / 60, "a direct link ended");
            return;
        }
    }
}

fn run(program: &str, args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new(program).args(args).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        let said = String::from_utf8_lossy(&out.stderr);
        let said = if said.trim().is_empty() { String::from_utf8_lossy(&out.stdout) } else { said };
        Err(said.trim().to_string())
    }
}

#[cfg(target_os = "linux")]
mod wifi {
    //! NetworkManager, which lets the person's own account manage Wi-Fi.
    use super::{run, NAME};
    use qurb_peer::DirectLinkOffer;

    fn nmcli(args: &[&str]) -> Result<String, String> {
        run("nmcli", args)
    }

    /// The first device of a kind NetworkManager manages.
    fn device(kind: &str) -> Option<String> {
        nmcli(&["-t", "-f", "DEVICE,TYPE", "device"])
            .ok()?
            .lines()
            .filter_map(|l| l.split_once(':'))
            .find(|(_, t)| *t == kind)
            .map(|(d, _)| d.to_string())
    }

    pub fn available() -> bool {
        device("wifi").is_some()
    }

    pub fn join(offer: &DirectLinkOffer) -> Result<(), String> {
        // A link left from before goes first.
        let _ = nmcli(&["connection", "delete", NAME]);
        let wifi = device("wifi").ok_or("no Wi-Fi device")?;
        nmcli(&[
            "connection", "add", "type", "wifi", "ifname", &wifi, "con-name", NAME, "ssid", &offer.name,
            "wifi-sec.key-mgmt", &offer.security, "wifi-sec.psk", &offer.password,
            "connection.autoconnect", "no",
        ])?;
        nmcli(&["--wait", "60", "connection", "up", NAME]).map(|_| ())
    }

    pub fn up(_offer: &DirectLinkOffer) -> bool {
        nmcli(&["-t", "-f", "GENERAL.STATE", "connection", "show", NAME]).is_ok_and(|s| s.contains("activated"))
    }

    /// Deleted: NetworkManager goes back to the usual network.
    pub fn leave() {
        let _ = nmcli(&["connection", "delete", NAME]);
    }

    /// This computer's address on the link and the phone's, as the phone's
    /// DHCP gave them.
    pub fn joined_net(_offer: &DirectLinkOffer) -> Option<qurb_peer::DirectNet> {
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

    /// Whether nobody is at this computer: a graphical session of this
    /// person's with its screen locked or idle (logind). Not known counts as
    /// somebody.
    pub fn away() -> bool {
        let user = std::env::var("USER").unwrap_or_default();
        let Ok(sessions) = run("loginctl", &["list-sessions", "--no-legend"]) else { return false };
        let mut seen = false;
        for id in sessions.lines().filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            (f.len() >= 4 && f[2] == user && f[3].starts_with("seat")).then(|| f[0].to_string())
        }) {
            let Ok(said) = run("loginctl", &["show-session", &id, "-p", "IdleHint", "-p", "LockedHint", "-p", "Active"]) else {
                continue;
            };
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
}

#[cfg(windows)]
mod wifi {
    //! `netsh wlan`, which an account without administrator rights may use
    //! for its own networks: a profile for the link, connected, and deleted
    //! after it, when Windows goes back to its usual network by itself. Not
    //! yet tried on a Windows computer with Wi-Fi (decision 0092).
    use super::{profile, run, NAME};
    use qurb_peer::DirectLinkOffer;

    fn netsh(args: &[&str]) -> Result<String, String> {
        run("netsh", args)
    }

    /// `netsh wlan show interfaces`, as name and value pairs.
    fn interface() -> Vec<(String, String)> {
        netsh(&["wlan", "show", "interfaces"])
            .unwrap_or_default()
            .lines()
            .filter_map(|l| l.split_once(':'))
            .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
            .collect()
    }

    fn value(of: &[(String, String)], key: &str) -> Option<String> {
        of.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }

    pub fn available() -> bool {
        value(&interface(), "Name").is_some()
    }

    pub fn join(offer: &DirectLinkOffer) -> Result<(), String> {
        let _ = netsh(&["wlan", "delete", "profile", &format!("name={NAME}")]);
        // The profile holds the password, so it is written where only this
        // account reads, and gone once Windows has it.
        let file = std::env::temp_dir().join(format!("{NAME}-{}.xml", std::process::id()));
        std::fs::write(&file, profile(offer)).map_err(|e| e.to_string())?;
        let added = netsh(&["wlan", "add", "profile", &format!("filename={}", file.display()), "user=current"]);
        let _ = std::fs::remove_file(&file);
        added?;
        netsh(&["wlan", "connect", &format!("name={NAME}"), &format!("ssid={}", offer.name)])?;
        for _ in 0..60 {
            if up(offer) {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        Err("not connected within a minute".into())
    }

    pub fn up(offer: &DirectLinkOffer) -> bool {
        let now = interface();
        value(&now, "SSID").as_deref() == Some(offer.name.as_str())
            && value(&now, "State").is_some_and(|s| s.eq_ignore_ascii_case("connected"))
    }

    /// Deleted: Windows goes back to the usual network.
    pub fn leave() {
        let _ = netsh(&["wlan", "disconnect"]);
        let _ = netsh(&["wlan", "delete", "profile", &format!("name={NAME}")]);
    }

    /// This computer's address on the link and the phone's, from the Wi-Fi
    /// interface's IPv4 settings.
    pub fn joined_net(_offer: &DirectLinkOffer) -> Option<qurb_peer::DirectNet> {
        let name = value(&interface(), "Name")?;
        let said = netsh(&["interface", "ipv4", "show", "config", &format!("name={name}")]).ok()?;
        let field = |key: &str| {
            said.lines().find_map(|l| l.trim().strip_prefix(key).map(|v| v.trim_start_matches(':').trim().to_string()))
        };
        let address = field("IP Address")?;
        let prefix = field("Subnet Prefix")?;
        let prefix = prefix.split_whitespace().next()?.split_once('/')?.1.to_string();
        let gateway = field("Default Gateway")?;
        Some(qurb_peer::DirectNet {
            address: address.parse().ok()?,
            prefix: prefix.parse().ok()?,
            gateway: gateway.split_whitespace().next()?.parse().ok()?,
        })
    }

    /// Whether nobody is at this computer: its screen locked, when Windows
    /// runs its lock screen. Not known counts as somebody.
    pub fn away() -> bool {
        run("tasklist", &["/FI", "IMAGENAME eq LogonUI.exe", "/NH"]).is_ok_and(|s| s.contains("LogonUI.exe"))
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod wifi {
    use qurb_peer::DirectLinkOffer;
    pub fn available() -> bool {
        false
    }
    pub fn join(_: &DirectLinkOffer) -> Result<(), String> {
        Err("no direct links here".into())
    }
    pub fn up(_: &DirectLinkOffer) -> bool {
        false
    }
    pub fn leave() {}
    pub fn joined_net(_: &DirectLinkOffer) -> Option<qurb_peer::DirectNet> {
        None
    }
    pub fn away() -> bool {
        false
    }
}

/// A Windows Wi-Fi profile for the link: its name, the network's, and its
/// password, each escaped for XML; connected by hand, never on its own.
#[cfg_attr(not(windows), allow(dead_code))]
fn profile(offer: &DirectLinkOffer) -> String {
    let xml = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;");
    let authentication = if offer.security == "sae" { "WPA3SAE" } else { "WPA2PSK" };
    format!(
        r#"<?xml version="1.0"?>
<WLANProfile xmlns="http://www.microsoft.com/networking/WLAN/profile/v1">
  <name>{NAME}</name>
  <SSIDConfig><SSID><name>{ssid}</name></SSID></SSIDConfig>
  <connectionType>ESS</connectionType>
  <connectionMode>manual</connectionMode>
  <MSM><security>
    <authEncryption><authentication>{authentication}</authentication><encryption>AES</encryption><useOneX>false</useOneX></authEncryption>
    <sharedKey><keyType>passPhrase</keyType><protected>false</protected><keyMaterial>{password}</keyMaterial></sharedKey>
  </security></MSM>
</WLANProfile>
"#,
        ssid = xml(&offer.name),
        password = xml(&offer.password),
    )
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

    #[test]
    fn a_windows_profile_carries_the_network_escaped() {
        let p = profile(&offer(DIRECT_P2P, "DIRECT-qb-<a&b>", "pass\"word'1", "wpa-psk", "aa:bb:cc:dd:ee:ff"));
        assert!(p.contains("<name>DIRECT-qb-&lt;a&amp;b&gt;</name>"));
        assert!(p.contains("<keyMaterial>pass&quot;word&apos;1</keyMaterial>"));
        assert!(p.contains("<authentication>WPA2PSK</authentication>"));
        assert!(p.contains("<connectionMode>manual</connectionMode>"));
        assert!(profile(&offer(DIRECT_HOTSPOT, "x", "abcdefgh", "sae", "")).contains("WPA3SAE"));
    }
}
