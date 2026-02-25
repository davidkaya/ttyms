use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use crate::config::Config;

const KEYRING_SERVICE: &str = "ttyms-teams-client";
const KEYRING_USER: &str = "default";
const SCOPES: &str = "User.Read User.ReadBasic.All Chat.ReadWrite ChatMessage.Read ChatMessage.Send Presence.Read Presence.ReadWrite Team.ReadBasic.All Channel.ReadBasic.All ChannelMessage.Read.All ChannelMessage.Send offline_access";

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
    pub token_type: String,
    #[serde(default)]
    pub obtained_at: u64,
}

impl Drop for TokenResponse {
    fn drop(&mut self) {
        self.access_token.zeroize();
        if let Some(ref mut rt) = self.refresh_token {
            rt.zeroize();
        }
    }
}

impl TokenResponse {
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now >= self.obtained_at + self.expires_in.saturating_sub(60)
    }

    pub fn with_timestamp(mut self) -> Self {
        self.obtained_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self
    }
}

#[derive(Debug, Deserialize)]
struct TokenError {
    error: String,
    error_description: Option<String>,
}

fn keyring_entry(name: &str) -> Option<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, name).ok()
}

fn token_file_path() -> Result<std::path::PathBuf> {
    Ok(crate::config::config_dir()?.join(".tokens"))
}

fn store_token(token: &TokenResponse) -> Result<()> {
    // Try keyring with split entries (each under Windows 2560 char limit)
    if store_token_keyring(token) {
        return Ok(());
    }
    // Fall back to file in config dir (protected by OS user permissions)
    store_token_file(token)
}

fn store_token_keyring(token: &TokenResponse) -> bool {
    let Some(at_entry) = keyring_entry("at") else { return false };
    let Some(rt_entry) = keyring_entry("rt") else { return false };
    let Some(meta_entry) = keyring_entry("meta") else { return false };

    if at_entry.set_password(&token.access_token).is_err() {
        return false;
    }
    if rt_entry.set_password(token.refresh_token.as_deref().unwrap_or("")).is_err() {
        return false;
    }
    let meta = format!("{},{}", token.expires_in, token.obtained_at);
    meta_entry.set_password(&meta).is_ok()
}

fn store_token_file(token: &TokenResponse) -> Result<()> {
    let mut json = serde_json::to_string(token)?;
    let path = token_file_path()?;
    std::fs::write(&path, &json)?;
    json.zeroize();
    Ok(())
}

fn load_cached_token() -> Result<Option<TokenResponse>> {
    // Try keyring first (split entries), silently fall through on any error
    if let Some(token) = load_token_keyring() {
        return Ok(Some(token));
    }
    // Fall back to file
    load_token_file()
}

fn load_token_keyring() -> Option<TokenResponse> {
    let at_entry = keyring_entry("at")?;
    let access_token = at_entry.get_password().ok().filter(|s| !s.is_empty())?;
    let refresh_token = keyring_entry("rt")
        .and_then(|e| e.get_password().ok())
        .filter(|s| !s.is_empty());
    let meta = keyring_entry("meta")
        .and_then(|e| e.get_password().ok())
        .unwrap_or_default();
    let mut parts = meta.split(',');
    let expires_in = parts.next().and_then(|s| s.parse().ok()).unwrap_or(3600);
    let obtained_at = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);

    Some(TokenResponse {
        access_token,
        refresh_token,
        expires_in,
        token_type: "Bearer".to_string(),
        obtained_at,
    })
}

fn load_token_file() -> Result<Option<TokenResponse>> {
    let path = token_file_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let mut json = std::fs::read_to_string(&path)?;
    let result = serde_json::from_str(&json);
    json.zeroize();
    match result {
        Ok(token) => Ok(Some(token)),
        Err(_) => Ok(None),
    }
}

pub fn clear_stored_tokens() -> Result<()> {
    // Clear keyring entries
    for key in ["at", "rt", "meta"] {
        if let Some(entry) = keyring_entry(key) {
            let _ = entry.delete_password();
        }
    }
    // Clear legacy single entry
    if let Some(entry) = keyring_entry(KEYRING_USER) {
        let _ = entry.delete_password();
    }
    // Clear file fallback
    if let Ok(path) = token_file_path() {
        if path.exists() {
            // Overwrite before delete
            let len = std::fs::metadata(&path).map(|m| m.len() as usize).unwrap_or(0);
            let _ = std::fs::write(&path, vec![0u8; len]);
            let _ = std::fs::remove_file(&path);
        }
    }
    Ok(())
}

pub async fn get_valid_token(
    client: &reqwest::Client,
    config: &Config,
) -> Result<Option<TokenResponse>> {
    if let Some(token) = load_cached_token()? {
        if !token.is_expired() {
            return Ok(Some(token));
        }
        if let Some(ref refresh_tok) = token.refresh_token {
            match refresh_access_token(client, config, refresh_tok).await {
                Ok(new_token) => return Ok(Some(new_token)),
                Err(_) => return Ok(None),
            }
        }
    }
    Ok(None)
}

pub async fn request_device_code(
    client: &reqwest::Client,
    config: &Config,
) -> Result<DeviceCodeResponse> {
    let url = format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/devicecode",
        config.tenant_id
    );
    client
        .post(&url)
        .form(&[
            ("client_id", config.client_id.as_str()),
            ("scope", SCOPES),
        ])
        .send()
        .await?
        .json::<DeviceCodeResponse>()
        .await
        .context("Failed to initiate device code flow")
}

pub async fn poll_for_token(
    client: &reqwest::Client,
    config: &Config,
    device_code: &str,
    interval: u64,
) -> Result<TokenResponse> {
    let url = format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
        config.tenant_id
    );

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(interval)).await;

        let resp = client
            .post(&url)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", config.client_id.as_str()),
                ("device_code", device_code),
            ])
            .send()
            .await?;

        let status = resp.status();
        let mut body = resp.text().await?;

        if status.is_success() {
            let result = serde_json::from_str::<TokenResponse>(&body);
            body.zeroize();
            let token = result.context("Failed to parse token response")?.with_timestamp();
            store_token(&token)?;
            return Ok(token);
        }

        if let Ok(error) = serde_json::from_str::<TokenError>(&body) {
            body.zeroize();
            match error.error.as_str() {
                "authorization_pending" => continue,
                "slow_down" => {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
                "expired_token" => anyhow::bail!("Device code expired. Please restart and try again."),
                _ => anyhow::bail!(
                    "Authentication error: {} - {}",
                    error.error,
                    error.error_description.unwrap_or_default()
                ),
            }
        }

        body.zeroize();
        anyhow::bail!("Unexpected response during authentication");
    }
}

async fn refresh_access_token(
    client: &reqwest::Client,
    config: &Config,
    refresh_tok: &str,
) -> Result<TokenResponse> {
    let url = format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
        config.tenant_id
    );

    let resp = client
        .post(&url)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", config.client_id.as_str()),
            ("refresh_token", refresh_tok),
            ("scope", SCOPES),
        ])
        .send()
        .await?;

    let mut body = resp.text().await?;
    let result = serde_json::from_str::<TokenResponse>(&body);
    body.zeroize();
    let token = result.context("Failed to refresh token")?.with_timestamp();
    store_token(&token)?;
    Ok(token)
}

// ---- PKCE Browser Auth Flow ----
// More compatible with Conditional Access policies than device code flow.

pub async fn authenticate_browser(
    client: &reqwest::Client,
    config: &Config,
) -> Result<TokenResponse> {
    let code_verifier = generate_code_verifier();
    let code_challenge = compute_code_challenge(&code_verifier);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .context("Failed to start local authentication server")?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://localhost:{}", port);

    let auth_url = format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/authorize?\
         client_id={}&response_type=code&redirect_uri={}&scope={}&\
         code_challenge={}&code_challenge_method=S256&prompt=select_account",
        config.tenant_id,
        percent_encode(&config.client_id),
        percent_encode(&redirect_uri),
        percent_encode(SCOPES),
        &code_challenge,
    );

    if open::that(&auth_url).is_err() {
        println!("  Open this URL in your browser:\n  {}", auth_url);
    }

    let code = capture_auth_code(listener).await?;
    exchange_code_for_token(client, config, &code, &code_verifier, &redirect_uri).await
}

pub fn generate_code_verifier() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn compute_code_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

pub fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

async fn capture_auth_code(listener: tokio::net::TcpListener) -> Result<String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let (mut stream, _) = tokio::time::timeout(
        std::time::Duration::from_secs(300),
        listener.accept(),
    )
    .await
    .context("Authentication timed out after 5 minutes")??;

    let mut buf = vec![0u8; 8192];
    let n = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);

    let code = parse_auth_callback(&request)?;

    let html = "<html><body style='font-family:system-ui;text-align:center;padding:60px'>\
        <h2 style='color:#0078d4'>Authentication Successful</h2>\
        <p style='color:#666'>You can close this window and return to the terminal.</p>\
        </body></html>";
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{}",
        html.len(),
        html
    );
    let _ = stream.write_all(resp.as_bytes()).await;
    let _ = stream.shutdown().await;

    Ok(code)
}

pub fn parse_auth_callback(request: &str) -> Result<String> {
    let path = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("");

    let query = path.split('?').nth(1).unwrap_or("");
    let mut code = None;
    let mut error = None;
    let mut error_desc = None;

    for param in query.split('&') {
        let mut kv = param.splitn(2, '=');
        match kv.next() {
            Some("code") => code = kv.next().map(simple_url_decode),
            Some("error") => error = kv.next().map(simple_url_decode),
            Some("error_description") => error_desc = kv.next().map(simple_url_decode),
            _ => {}
        }
    }

    if let Some(code) = code {
        return Ok(code);
    }

    let err = error.unwrap_or_else(|| "unknown".to_string());
    let desc = error_desc.unwrap_or_default();
    anyhow::bail!("Authentication denied: {} - {}", err, desc)
}

pub fn simple_url_decode(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '%' => {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                }
            }
            '+' => result.push(' '),
            _ => result.push(ch),
        }
    }
    result
}

async fn exchange_code_for_token(
    client: &reqwest::Client,
    config: &Config,
    code: &str,
    code_verifier: &str,
    redirect_uri: &str,
) -> Result<TokenResponse> {
    let url = format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
        config.tenant_id
    );

    let resp = client
        .post(&url)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", config.client_id.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", code_verifier),
            ("scope", SCOPES),
        ])
        .send()
        .await?;

    let status = resp.status();
    let mut body = resp.text().await?;

    if !status.is_success() {
        let err_msg = body.clone();
        body.zeroize();
        anyhow::bail!("Token exchange failed ({}): {}", status, err_msg);
    }

    let result = serde_json::from_str::<TokenResponse>(&body);
    body.zeroize();
    let token = result
        .context("Failed to parse token response")?
        .with_timestamp();
    store_token(&token)?;
    Ok(token)
}
