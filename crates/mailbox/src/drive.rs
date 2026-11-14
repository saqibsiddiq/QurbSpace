//! The hidden app folder in a person's Google Drive: where letters go when
//! the person turned the mailbox on with their Google account (2026-11-13).
//!
//! Allowed `drive.appdata` and nothing else: QurbSpace's own folder, which
//! the person's Drive does not show and no other app can read; never their
//! files. Set up once by the owner (docs/google-drive-setup.md).
//!
//! The phone signs in with the account already on it, and hands its access
//! tokens in ([`Drive::with_token`]); the computer signs in once in its
//! browser ([`sign_in`]) and keeps the refresh token that Google gives it,
//! from which it takes new access tokens as they run out.

use crate::{Error, Letter, Result};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const FILES: &str = "https://www.googleapis.com/drive/v3/files";
const UPLOAD: &str = "https://www.googleapis.com/upload/drive/v3/files";
const AUTH: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN: &str = "https://oauth2.googleapis.com/token";
/// QurbSpace's own hidden folder, and nothing else in the Drive.
pub const SCOPE: &str = "https://www.googleapis.com/auth/drive.appdata";

/// The desktop client the owner made (docs/google-drive-setup.md, step 5).
#[derive(Clone, serde::Deserialize)]
pub struct Client {
    pub client_id: String,
    pub client_secret: String,
}

impl Client {
    /// Read from the JSON Google gives for a desktop client, or for a web
    /// one (the one a phone's codes are made for, [`exchange`]).
    pub fn from_json(json: &str) -> Option<Self> {
        let value: serde_json::Value = serde_json::from_str(json).ok()?;
        let client = value.get("installed").or_else(|| value.get("web"))?;
        serde_json::from_value(client.clone()).ok()
    }
}

/// How the computer gets a new access token: its client and the refresh
/// token Google gave it when the person signed in.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Refresh {
    pub client_id: String,
    pub client_secret: String,
    pub refresh_token: String,
    /// The account, for showing which one is signed in.
    #[serde(default)]
    pub account: Option<String>,
}

/// One person's hidden app folder.
pub struct Drive {
    http: reqwest::Client,
    access: Mutex<Option<(String, Instant)>>,
    refresh: Option<Refresh>,
}

impl Drive {
    /// The computer's: tokens taken from `refresh` as needed.
    pub fn with_refresh(refresh: Refresh) -> Self {
        Self { http: reqwest::Client::new(), access: Mutex::new(None), refresh: Some(refresh) }
    }

    /// The phone's: an access token the phone's account gave, good for about
    /// an hour; [`set_token`](Self::set_token) when the phone has a new one.
    pub fn with_token(token: String) -> Self {
        let drive = Self { http: reqwest::Client::new(), access: Mutex::new(None), refresh: None };
        drive.set_token(token);
        drive
    }

    pub fn set_token(&self, token: String) {
        if let Ok(mut access) = self.access.lock() {
            *access = Some((token, Instant::now() + Duration::from_secs(50 * 60)));
        }
    }

    async fn token(&self) -> Result<String> {
        if let Some((token, until)) = self.access.lock().ok().and_then(|a| a.clone()) {
            if Instant::now() < until {
                return Ok(token);
            }
        }
        let Some(refresh) = &self.refresh else { return Err(Error::NotSignedIn) };
        let answer: serde_json::Value = self
            .http
            .post(TOKEN)
            .form(&[
                ("client_id", refresh.client_id.as_str()),
                ("client_secret", refresh.client_secret.as_str()),
                ("refresh_token", refresh.refresh_token.as_str()),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .map_err(place)?
            .json()
            .await
            .map_err(place)?;
        let token = answer.get("access_token").and_then(|t| t.as_str()).ok_or(Error::NotSignedIn)?.to_string();
        let lasts = answer.get("expires_in").and_then(|t| t.as_u64()).unwrap_or(3600);
        if let Ok(mut access) = self.access.lock() {
            *access = Some((token.clone(), Instant::now() + Duration::from_secs(lasts.saturating_sub(120))));
        }
        Ok(token)
    }

    /// Forget the access token, after Google refused it.
    fn forget_token(&self) {
        if let Ok(mut access) = self.access.lock() {
            *access = None;
        }
    }

    async fn checked(&self, response: reqwest::Response) -> Result<reqwest::Response> {
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            self.forget_token();
            return Err(Error::NotSignedIn);
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Place(format!("Google Drive answered {status}: {}", body.chars().take(300).collect::<String>())));
        }
        Ok(response)
    }

    pub async fn list(&self) -> Result<Vec<Letter>> {
        let mut out = Vec::new();
        let mut page: Option<String> = None;
        loop {
            let token = self.token().await?;
            let mut query = vec![
                ("spaces", "appDataFolder".to_string()),
                ("pageSize", "1000".to_string()),
                ("fields", "nextPageToken,files(id,name,size)".to_string()),
            ];
            if let Some(page) = &page {
                query.push(("pageToken", page.clone()));
            }
            let response = self.http.get(FILES).bearer_auth(token).query(&query).send().await.map_err(place)?;
            let answer: serde_json::Value = self.checked(response).await?.json().await.map_err(place)?;
            for file in answer.get("files").and_then(|f| f.as_array()).into_iter().flatten() {
                let (Some(id), Some(name)) = (file.get("id").and_then(|v| v.as_str()), file.get("name").and_then(|v| v.as_str())) else {
                    continue;
                };
                let size = file.get("size").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0);
                out.push(Letter { id: id.to_string(), name: name.to_string(), size });
            }
            page = answer.get("nextPageToken").and_then(|p| p.as_str()).map(String::from);
            if page.is_none() {
                return Ok(out);
            }
        }
    }

    /// One file, its name and its bytes in one request: the multipart
    /// upload, good to 5 MB, which a pack stays under.
    pub async fn put(&self, name: &str, bytes: Vec<u8>) -> Result<()> {
        let token = self.token().await?;
        let mut boundary = [0u8; 12];
        getrandom::fill(&mut boundary).expect("the system's random source");
        let boundary = format!("qurb{}", crate::hex(&boundary));
        let metadata = serde_json::json!({"name": name, "parents": ["appDataFolder"]}).to_string();
        let mut body = Vec::with_capacity(bytes.len() + 512);
        body.extend_from_slice(format!("--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{metadata}\r\n").as_bytes());
        body.extend_from_slice(format!("--{boundary}\r\nContent-Type: application/octet-stream\r\n\r\n").as_bytes());
        body.extend_from_slice(&bytes);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        let response = self
            .http
            .post(UPLOAD)
            .bearer_auth(token)
            .query(&[("uploadType", "multipart"), ("fields", "id")])
            .header("Content-Type", format!("multipart/related; boundary={boundary}"))
            .body(body)
            .send()
            .await
            .map_err(place)?;
        self.checked(response).await.map(|_| ())
    }

    pub async fn get(&self, id: &str) -> Result<Vec<u8>> {
        let token = self.token().await?;
        let response = self.http.get(format!("{FILES}/{id}")).bearer_auth(token).query(&[("alt", "media")]).send().await.map_err(place)?;
        Ok(self.checked(response).await?.bytes().await.map_err(place)?.to_vec())
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        let token = self.token().await?;
        let response = self.http.delete(format!("{FILES}/{id}")).bearer_auth(token).send().await.map_err(place)?;
        match response.status() {
            // Gone already: what was wanted.
            reqwest::StatusCode::NOT_FOUND => Ok(()),
            _ => self.checked(response).await.map(|_| ()),
        }
    }
}

fn place(e: reqwest::Error) -> Error {
    Error::Place(format!("Google Drive: {e}"))
}

/// Sign the computer in, once: Google's page opened in the browser by `open`,
/// the person signs in and allows QurbSpace its folder, and Google sends the
/// browser back to a port on this machine with a code, which is exchanged for
/// a refresh token. PKCE, as Google asks of an app that cannot keep a secret.
pub async fn sign_in(client: &Client, open: impl FnOnce(&str)) -> Result<Refresh> {
    use sha2::Digest;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.map_err(|e| Error::Place(format!("a port for signing in: {e}")))?;
    let port = listener.local_addr().map_err(|e| Error::Place(e.to_string()))?.port();
    let redirect = format!("http://127.0.0.1:{port}");
    let mut verifier = [0u8; 32];
    getrandom::fill(&mut verifier).expect("the system's random source");
    let verifier = base64url(&verifier);
    let challenge = base64url(&sha2::Sha256::digest(verifier.as_bytes()));
    let mut state = [0u8; 16];
    getrandom::fill(&mut state).expect("the system's random source");
    let state = crate::hex(&state);
    let url = reqwest::Url::parse_with_params(
        AUTH,
        &[
            ("client_id", client.client_id.as_str()),
            ("redirect_uri", redirect.as_str()),
            ("response_type", "code"),
            ("scope", &format!("{SCOPE} email")),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
            ("state", state.as_str()),
            ("access_type", "offline"),
            ("prompt", "select_account consent"),
        ],
    )
    .map_err(|e| Error::Place(e.to_string()))?;
    open(url.as_str());

    // The browser comes back here once: the code, or why not.
    let (mut socket, _) = tokio::time::timeout(Duration::from_secs(300), listener.accept())
        .await
        .map_err(|_| Error::Place("signing in took more than five minutes".into()))?
        .map_err(|e| Error::Place(e.to_string()))?;
    let mut request = vec![0u8; 8192];
    let n = socket.read(&mut request).await.map_err(|e| Error::Place(e.to_string()))?;
    let line = String::from_utf8_lossy(&request[..n]).lines().next().unwrap_or_default().to_string();
    let target = line.split_whitespace().nth(1).unwrap_or("/");
    let back = reqwest::Url::parse(&format!("http://127.0.0.1{target}")).map_err(|e| Error::Place(e.to_string()))?;
    let param = |name: &str| back.query_pairs().find(|(k, _)| k == name).map(|(_, v)| v.to_string());
    let page = |words: &str| {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n\
             <!doctype html><meta charset=utf-8><title>QurbSpace</title>\
             <body style=\"font:16px system-ui;margin:4em auto;max-width:28em;text-align:center\">{words}</body>"
        )
    };
    if param("state").as_deref() != Some(state.as_str()) {
        let _ = socket.write_all(page("This was not QurbSpace's sign-in. Close this tab and try again.").as_bytes()).await;
        return Err(Error::Place("the sign-in came back without this attempt's mark".into()));
    }
    let Some(code) = param("code") else {
        let _ = socket.write_all(page("Not signed in. You can close this tab.").as_bytes()).await;
        return Err(Error::Place(format!("not signed in: {}", param("error").unwrap_or_default())));
    };
    let _ = socket.write_all(page("Signed in. You can close this tab and go back to QurbSpace.").as_bytes()).await;

    let answer: serde_json::Value = reqwest::Client::new()
        .post(TOKEN)
        .form(&[
            ("client_id", client.client_id.as_str()),
            ("client_secret", client.client_secret.as_str()),
            ("code", code.as_str()),
            ("code_verifier", verifier.as_str()),
            ("redirect_uri", redirect.as_str()),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await
        .map_err(place)?
        .json()
        .await
        .map_err(place)?;
    // Google asks for each permission with a box of its own, and the one for
    // QurbSpace's folder can be left unticked: signed in, and allowed only an
    // email address (the owner's first sign-in, 2026-11-13).
    let granted = answer.get("scope").and_then(|s| s.as_str()).unwrap_or_default();
    if !granted.split_whitespace().any(|s| s == SCOPE) {
        return Err(Error::Place(
            "Google Drive was not allowed: sign in again and tick the box for QurbSpace's own data in your Drive".into(),
        ));
    }
    let refresh_token = answer
        .get("refresh_token")
        .and_then(|t| t.as_str())
        .ok_or_else(|| Error::Place(format!("Google gave no refresh token: {}", answer.get("error").and_then(|e| e.as_str()).unwrap_or("no reason"))))?
        .to_string();
    // Which account: the email in the ID token's middle part, not checked,
    // shown to the person, never trusted for anything.
    let account = answer
        .get("id_token")
        .and_then(|t| t.as_str())
        .and_then(|t| t.split('.').nth(1))
        .and_then(unbase64url)
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| v.get("email").and_then(|e| e.as_str()).map(String::from));
    Ok(Refresh { client_id: client.client_id.clone(), client_secret: client.client_secret.clone(), refresh_token, account })
}

/// Exchange a one-time code a phone was given for its Google account
/// (`Request::DriveCode`) for a lasting sign-in to that account: so the
/// computer reaches the phone's own Drive mailbox with nobody signing in on
/// it (the owner, 2026-11-14). `client` is the web client the phone asked
/// the code for.
pub async fn exchange(client: &Client, code: &str) -> Result<Refresh> {
    let answer: serde_json::Value = reqwest::Client::new()
        .post(TOKEN)
        .form(&[
            ("client_id", client.client_id.as_str()),
            ("client_secret", client.client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            // A code given to an Android app has no address to return to.
            ("redirect_uri", ""),
        ])
        .send()
        .await
        .map_err(place)?
        .json()
        .await
        .map_err(place)?;
    let granted = answer.get("scope").and_then(|s| s.as_str()).unwrap_or_default();
    if !granted.split_whitespace().any(|s| s == SCOPE) {
        return Err(Error::Place(format!(
            "the phone's code did not allow QurbSpace's folder: {}",
            answer.get("error_description").or_else(|| answer.get("error")).and_then(|e| e.as_str()).unwrap_or("no reason given")
        )));
    }
    let refresh_token = answer
        .get("refresh_token")
        .and_then(|t| t.as_str())
        .ok_or_else(|| Error::Place("Google gave no refresh token for the phone's code".into()))?
        .to_string();
    let account = answer
        .get("id_token")
        .and_then(|t| t.as_str())
        .and_then(|t| t.split('.').nth(1))
        .and_then(unbase64url)
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| v.get("email").and_then(|e| e.as_str()).map(String::from));
    Ok(Refresh { client_id: client.client_id.clone(), client_secret: client.client_secret.clone(), refresh_token, account })
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn base64url(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        for i in 0..=chunk.len() {
            out.push(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    out
}

fn unbase64url(text: &str) -> Option<Vec<u8>> {
    let mut bits = 0u32;
    let mut count = 0;
    let mut out = Vec::new();
    for c in text.bytes().filter(|c| *c != b'=') {
        let v = ALPHABET.iter().position(|a| *a == c)? as u32;
        bits = (bits << 6) | v;
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64url_round_trips_and_matches_the_rfc() {
        // RFC 7636's example: the verifier's SHA-256, base64url.
        use sha2::Digest;
        let challenge = base64url(&sha2::Sha256::digest(b"dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"));
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
        for len in 0..20 {
            let bytes: Vec<u8> = (0..len as u8).map(|i| i.wrapping_mul(37)).collect();
            assert_eq!(unbase64url(&base64url(&bytes)).unwrap(), bytes);
        }
    }

    #[test]
    fn the_desktop_client_file_is_read() {
        let json = r#"{"installed":{"client_id":"abc.apps.googleusercontent.com","client_secret":"s","redirect_uris":["http://localhost"]}}"#;
        let client = Client::from_json(json).unwrap();
        assert_eq!(client.client_id, "abc.apps.googleusercontent.com");
    }
}
