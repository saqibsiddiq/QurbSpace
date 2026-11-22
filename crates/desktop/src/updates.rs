//! Updating the window wherever the computer is, during the beta (decision
//! 0078).
//!
//! The latest release on GitHub says what it offers (`latest.json`, written
//! by `.github/workflows/release.yml`), and each build it names is signed
//! with the updater key, whose public half is in `tauri.conf.json`: a build
//! not signed with it is refused, wherever it came from. Looked for a minute
//! after starting and every six hours. Where an update can be put in place
//! while the window runs (an AppImage on Linux) it is, and takes over at
//! the next start; on Windows, where installing closes the window, it is
//! fetched and waits to be installed from Settings, kept on disk so a
//! restart before then does not fetch it again. In place, it restarts
//! itself into the new one once nobody is using the window (N-2). Installed by a package
//! manager, the window cannot replace itself, and says a newer one exists.
//!
//! The check sends nothing about the person: one request for a public file.

use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

/// Where updating stands, for the window.
#[derive(Clone, Serialize, Default)]
pub struct UpdateState {
    /// This build's version.
    current: String,
    /// "idle", "checking", "downloading", "ready" (fetched, to install),
    /// "installed" (in place, taking over at the next start), "manual" (a
    /// newer one exists; this install cannot replace itself), "available" (a
    /// newer one exists, and the person asked to be asked first), or "failed".
    state: &'static str,
    /// The newer version, when there is one.
    version: Option<String>,
    /// How much of it has come, 0 to 1, while downloading.
    progress: Option<f64>,
    error: Option<String>,
}

struct Held {
    state: UpdateState,
    /// A fetched build waiting to be installed (Windows).
    bytes: Option<Vec<u8>>,
}

static HELD: Mutex<Option<Held>> = Mutex::new(None);

const FIRST_LOOK: Duration = Duration::from_secs(60);
const EVERY: Duration = Duration::from_secs(6 * 60 * 60);

fn set(change: impl FnOnce(&mut Held)) {
    let mut held = HELD.lock().expect("update state poisoned");
    let held = held.get_or_insert_with(|| Held { state: UpdateState::default(), bytes: None });
    change(held);
}

pub fn state(app: &AppHandle) -> UpdateState {
    let held = HELD.lock().expect("update state poisoned");
    let mut state = held.as_ref().map(|h| h.state.clone()).unwrap_or_default();
    state.current = app.package_info().version.to_string();
    if state.state.is_empty() {
        state.state = "idle";
    }
    state
}

/// Whether this install can replace itself: an AppImage on Linux, the
/// installed app on Windows and macOS. A binary placed by hand or by a
/// package manager cannot.
fn replaceable() -> bool {
    !cfg!(target_os = "linux") || std::env::var_os("APPIMAGE").is_some()
}

/// Whether the person asked for updates to wait for a yes (decision 0086).
fn asks_first(app: &AppHandle) -> bool {
    use tauri::Manager;
    app.try_state::<std::sync::Arc<crate::session::Hosted>>()
        .and_then(|hosted| qurb_cli::Config::load(&qurb_cli::store_dir(&hosted.root())).ok())
        .is_some_and(|c| c.ask_before_update)
}

/// Look now, and fetch what is found, or, with `fetch` false, only say
/// that it is there. Never fails: how it went is the state.
pub async fn look(app: AppHandle, fetch: bool) {
    set(|h| {
        h.state.state = "checking";
        h.state.error = None;
    });
    let found = match app.updater() {
        Ok(updater) => updater.check().await,
        Err(e) => Err(e),
    };
    let update = match found {
        Ok(Some(update)) => update,
        Ok(None) => {
            // Installed, or none: a kept update is done with.
            if let Some(dir) = kept_dir(&app) {
                let _ = std::fs::remove_dir_all(dir);
            }
            return set(|h| h.state.state = "idle");
        }
        Err(e) => {
            tracing::warn!(error = %e, "could not look for an update");
            return set(|h| {
                h.state.state = "failed";
                h.state.error = Some(e.to_string());
            });
        }
    };
    let version = update.version.clone();
    tracing::info!(%version, "an update is available");
    if !replaceable() {
        return set(|h| {
            h.state.state = "manual";
            h.state.version = Some(version);
        });
    }
    if !fetch {
        return set(|h| {
            h.state.state = "available";
            h.state.version = Some(version);
        });
    }
    // Fetched before and kept, waiting to be installed (Windows): the same
    // file again, not another download.
    if !cfg!(target_os = "linux") {
        if let Some(bytes) = kept(&app, &update) {
            tracing::info!(%version, "the update fetched before is kept; not fetched again");
            return ready(update, version, bytes);
        }
    }
    set(|h| {
        h.state.state = "downloading";
        h.state.version = Some(version.clone());
        h.state.progress = Some(0.0);
    });
    let mut got = 0usize;
    let fetched = update
        .download(
            |chunk, total| {
                got += chunk;
                if let Some(total) = total.filter(|t| *t > 0) {
                    set(|h| h.state.progress = Some(got as f64 / total as f64));
                }
            },
            || {},
        )
        .await;
    let bytes = match fetched {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::warn!(error = %e, "could not fetch the update");
            return set(|h| {
                h.state.state = "failed";
                h.state.error = Some(e.to_string());
            });
        }
    };
    // In place now where the running window is not disturbed by it: the
    // AppImage is a file, and the one running keeps its own copy open.
    if cfg!(target_os = "linux") {
        match update.install(&bytes) {
            Ok(()) => {
                tracing::info!(%version, "update in place; it takes over at the next start");
                set(|h| {
                    h.state.state = "installed";
                    h.state.progress = None;
                });
            }
            Err(e) => set(|h| {
                h.state.state = "failed";
                h.state.error = Some(e.to_string());
            }),
        }
        return;
    }
    keep(&app, &version, &bytes);
    ready(update, version, bytes);
}

/// Fetched, signed, and waiting to be installed from Settings.
fn ready(update: tauri_plugin_updater::Update, version: String, bytes: Vec<u8>) {
    set(|h| {
        h.state.state = "ready";
        h.state.version = Some(version);
        h.state.progress = None;
        h.bytes = Some(bytes);
    });
    *PENDING.lock().expect("pending update poisoned") = Some(update);
}

/// Where a fetched update waits on disk to be installed. Kept in memory only,
/// it was fetched again at every start of a computer restarted before the
/// person installed it, about ten megabytes each time (found 2026-11-22).
fn kept_dir(app: &AppHandle) -> Option<PathBuf> {
    use tauri::Manager;
    app.path().app_cache_dir().ok().map(|dir| dir.join("update"))
}

fn kept_file(dir: &std::path::Path, version: &str) -> PathBuf {
    dir.join(format!("{version}.update"))
}

/// Keep a fetched update, in place of any kept before. Not kept is no harm:
/// it is fetched again, as before.
fn keep(app: &AppHandle, version: &str, bytes: &[u8]) {
    let Some(dir) = kept_dir(app) else { return };
    let _ = std::fs::remove_dir_all(&dir);
    let file = kept_file(&dir, version);
    let partial = file.with_extension("partial");
    let kept = std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::write(&partial, bytes))
        .and_then(|()| std::fs::rename(&partial, &file));
    if let Err(e) = kept {
        tracing::warn!(error = %e, "could not keep the update; it is fetched again after a restart");
    }
}

/// The update for this version, kept before, if it is there and the
/// signature announced now is the updater key's over it: checked as the
/// updater checks a download, so a file changed on disk is not installed.
fn kept(app: &AppHandle, update: &tauri_plugin_updater::Update) -> Option<Vec<u8>> {
    let file = kept_file(&kept_dir(app)?, &update.version);
    let bytes = std::fs::read(&file).ok()?;
    let key = app.config().plugins.0.get("updater")?.get("pubkey")?.as_str()?;
    if signed(key, &bytes, &update.signature) {
        Some(bytes)
    } else {
        tracing::warn!("the kept update is not the one signed; fetched again");
        let _ = std::fs::remove_file(&file);
        None
    }
}

/// Whether `signature` is the key's over `bytes`, both as the updater has
/// them: base64 of minisign's text (tauri.conf.json, latest.json).
fn signed(key: &str, bytes: &[u8], signature: &str) -> bool {
    use base64::Engine;
    let text = |b64: &str| {
        base64::engine::general_purpose::STANDARD
            .decode(b64)
            .ok()
            .and_then(|d| String::from_utf8(d).ok())
    };
    let (Some(key), Some(signature)) = (text(key), text(signature)) else { return false };
    match (minisign_verify::PublicKey::decode(&key), minisign_verify::Signature::decode(&signature)) {
        (Ok(key), Ok(signature)) => key.verify(bytes, &signature, true).is_ok(),
        _ => false,
    }
}

/// The update fetched and waiting to be installed (Windows), kept with what
/// it needs to install itself.
static PENDING: Mutex<Option<tauri_plugin_updater::Update>> = Mutex::new(None);

/// Put a fetched update in place and start the new one. On Linux it is in
/// place already: a restart is all.
pub fn apply(app: &AppHandle) -> Result<(), String> {
    let state = state(app);
    match state.state {
        "installed" => app.restart(),
        "ready" => {
            let update = PENDING.lock().expect("pending update poisoned").take().ok_or("nothing is waiting to be installed")?;
            let bytes = {
                let mut held = HELD.lock().expect("update state poisoned");
                held.as_mut().and_then(|h| h.bytes.take()).ok_or("nothing is waiting to be installed")?
            };
            update.install(bytes).map_err(|e| e.to_string())?;
            app.restart()
        }
        _ => Err("there is no update to install".to_string()),
    }
}

/// How often a waiting update looks for a quiet moment to restart in.
const QUIET_LOOK: Duration = Duration::from_secs(120);

/// An update in place restarts the program by itself once nobody is using
/// it (the window closed, nothing moving, no person's folder open) as
/// the phone installs while the app is away (the owner's review of
/// 2026-11-03, N-2): a computer that is everyone's server would otherwise
/// keep the old one for weeks, waiting for a button.
fn restart_when_quiet(app: AppHandle) {
    use tauri::Manager;
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(QUIET_LOOK).await;
            // Asked first: put in place only with the yes, and then restarted
            // by it (decision 0086).
            if state(&app).state != "installed" || asks_first(&app) {
                continue;
            }
            let shown = app.get_webview_window("main").and_then(|w| w.is_visible().ok()).unwrap_or(true);
            let moving = app
                .try_state::<std::sync::Arc<crate::session::Hosted>>()
                .and_then(|hosted| hosted.status())
                .map(|s| !s.incoming.is_empty() || !s.outgoing.is_empty())
                .unwrap_or(true);
            if shown || moving || crate::mount::any() {
                continue;
            }
            tracing::info!("restarting into the update, nobody using the window");
            app.restart();
        }
    });
}

/// Look a minute after starting, and every six hours after.
pub fn watch(app: AppHandle) {
    restart_when_quiet(app.clone());
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_LOOK).await;
        loop {
            let busy = matches!(state(&app).state, "downloading" | "installed" | "ready");
            if !busy {
                look(app.clone(), !asks_first(&app)).await;
            }
            tokio::time::sleep(EVERY).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::signed;
    use base64::Engine;

    // minisign-verify's own example: a key, and its signature over "test".
    const KEY: &str = "untrusted comment: minisign public key\nRWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3\n";
    const SIGNATURE: &str = "untrusted comment: signature from minisign secret key
RWQf6LRCGA9i59SLOFxz6NxvASXDJeRtuZykwQepbDEGt87ig1BNpWaVWuNrm73YiIiJbq71Wi+dP9eKL8OC351vwIasSSbXxwA=
trusted comment: timestamp:1555779966\tfile:test
QtKMXWyYcwdpZAlPF7tE2ENJkRd1ujvKjlj1m9RtHTBnZPa5WKU5uWRs5GoP5M/VqE81QFuMKI5k/SfNQUaOAA==";

    fn b64(s: &str) -> String {
        base64::engine::general_purpose::STANDARD.encode(s)
    }

    #[test]
    fn a_kept_update_is_used_only_as_signed() {
        assert!(signed(&b64(KEY), b"test", &b64(SIGNATURE)));
        // Changed on disk.
        assert!(!signed(&b64(KEY), b"Test", &b64(SIGNATURE)));
        assert!(!signed(&b64(KEY), b"", &b64(SIGNATURE)));
        // Not the updater's text at all.
        assert!(!signed("not base64", b"test", &b64(SIGNATURE)));
        assert!(!signed(&b64(KEY), b"test", "not base64"));
    }
}
