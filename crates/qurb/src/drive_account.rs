//! The Google accounts this computer leaves letters through, when the person
//! turned the mailbox on (`qurb_mailbox`, 2026-11-13): the desktop client the
//! owner made, and a refresh token for each account signed in. A phone may
//! use its own account, not the computer's (the owner's new phone did), so
//! the computer can be signed in to several, and posts to each phone in the
//! one its letters are in. The tokens are in the person's config folder,
//! readable by them alone; none in the synced folder or the repository. The
//! two clients are built into a release, or read from that folder.

use std::path::PathBuf;

fn config_dir() -> Option<PathBuf> {
    match std::env::var("XDG_CONFIG_HOME") {
        Ok(dir) if !dir.is_empty() => Some(PathBuf::from(dir).join("qurb")),
        _ => std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config").join("qurb")),
    }
}

/// The desktop client (docs/google-drive-setup.md, step 5), if it is there.
pub fn client() -> Option<qurb_mailbox::drive::Client> {
    from_file_or_built_in("google-desktop-client.json", option_env!("QURB_GOOGLE_DESKTOP_CLIENT"))
}

/// The web client the owner made, which phones ask their codes for
/// (docs/google-drive-setup.md, step 8).
pub fn web_client() -> Option<qurb_mailbox::drive::Client> {
    from_file_or_built_in("google-web-client.json", option_env!("QURB_GOOGLE_WEB_CLIENT"))
}

/// A client from the config folder where one is there, else the one the
/// release was built with (`QURB_GOOGLE_DESKTOP_CLIENT`, `QURB_GOOGLE_WEB_CLIENT`,
/// set from the repository's secrets by .github/workflows/release.yml): until
/// the beta of 2026-11 they were read from the owner's config folder alone,
/// so nobody else's computer could turn Google Drive on (docs/beta.md).
fn from_file_or_built_in(name: &str, built_in: Option<&str>) -> Option<qurb_mailbox::drive::Client> {
    let file = config_dir().and_then(|dir| std::fs::read_to_string(dir.join(name)).ok());
    file.as_deref().and_then(qurb_mailbox::drive::Client::from_json)
        .or_else(|| built_in.filter(|json| !json.trim().is_empty()).and_then(qurb_mailbox::drive::Client::from_json))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A computer with no client file uses the one the release was built
    /// with; one built empty offers none.
    #[test]
    fn a_built_in_client_is_used_where_no_file_is() {
        let json = r#"{"installed":{"client_id":"id.apps.googleusercontent.com","client_secret":"s"}}"#;
        assert!(from_file_or_built_in("no-such-client.json", Some(json)).is_some());
        assert!(from_file_or_built_in("no-such-client.json", Some("")).is_none());
        assert!(from_file_or_built_in("no-such-client.json", None).is_none());
    }
}
