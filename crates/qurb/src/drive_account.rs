//! The Google account this computer leaves letters through, when the person
//! turned the mailbox on (`qurb_mailbox`, 2026-11-13): the desktop client the
//! owner made, and the refresh token Google gave when the person signed in.
//! Both in the person's config folder, readable by them alone; neither in the
//! synced folder or the repository.

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

fn account_file() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("google-drive.json"))
}

/// The signed-in account, if the mailbox is on.
pub fn signed_in() -> Option<qurb_mailbox::drive::Refresh> {
    let bytes = std::fs::read(account_file()?).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Keep the account the person signed in with: the mailbox is on.
pub fn keep(refresh: &qurb_mailbox::drive::Refresh) -> std::io::Result<()> {
    let file = account_file().ok_or_else(|| std::io::Error::other("no config folder"))?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&file, serde_json::to_vec(refresh).map_err(std::io::Error::other)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Forget it: the mailbox is off. Letters already in the folder stay until
/// the account is signed in again or the folder is cleared in Google's
/// settings.
pub fn forget() -> std::io::Result<()> {
    match account_file().map(std::fs::remove_file) {
        Some(Err(e)) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}
