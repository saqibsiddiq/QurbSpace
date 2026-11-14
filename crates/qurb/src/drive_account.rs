//! The Google accounts this computer leaves letters through, when the person
//! turned the mailbox on (`qurb_mailbox`, 2026-11-13): the desktop client the
//! owner made, and a refresh token for each account signed in. A phone may
//! use its own account, not the computer's (the owner's new phone did), so
//! the computer can be signed in to several, and posts to each phone in the
//! one its letters are in. All in the person's config folder, readable by
//! them alone; none in the synced folder or the repository.

use std::path::PathBuf;

fn config_dir() -> Option<PathBuf> {
    match std::env::var("XDG_CONFIG_HOME") {
        Ok(dir) if !dir.is_empty() => Some(PathBuf::from(dir).join("qurb")),
        _ => std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config").join("qurb")),
    }
}

/// The desktop client (docs/google-drive-setup.md, step 5), if it is there.
pub fn client() -> Option<qurb_mailbox::drive::Client> {
    let json = std::fs::read_to_string(config_dir()?.join("google-desktop-client.json")).ok()?;
    qurb_mailbox::drive::Client::from_json(&json)
}

fn accounts_file() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("google-drive.json"))
}

/// Every account signed in: none when the mailbox is off. The file held one
/// account before several were allowed, and is read either way.
pub fn accounts() -> Vec<qurb_mailbox::drive::Refresh> {
    let Some(bytes) = accounts_file().and_then(|f| std::fs::read(f).ok()) else { return Vec::new() };
    serde_json::from_slice::<Vec<qurb_mailbox::drive::Refresh>>(&bytes)
        .or_else(|_| serde_json::from_slice::<qurb_mailbox::drive::Refresh>(&bytes).map(|one| vec![one]))
        .unwrap_or_default()
}

fn save(accounts: &[qurb_mailbox::drive::Refresh]) -> std::io::Result<()> {
    let file = accounts_file().ok_or_else(|| std::io::Error::other("no config folder"))?;
    if accounts.is_empty() {
        return match std::fs::remove_file(&file) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&file, serde_json::to_vec(accounts).map_err(std::io::Error::other)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Keep an account the person signed in with, replacing one of the same
/// address.
pub fn keep(refresh: &qurb_mailbox::drive::Refresh) -> std::io::Result<()> {
    let mut all = accounts();
    all.retain(|a| a.account.is_none() || a.account != refresh.account);
    all.push(refresh.clone());
    save(&all)
}

/// Forget one account, by its address; or every one.
pub fn forget(account: Option<&str>) -> std::io::Result<()> {
    let mut all = accounts();
    match account {
        Some(account) => all.retain(|a| a.account.as_deref() != Some(account)),
        None => all.clear(),
    }
    save(&all)
}
