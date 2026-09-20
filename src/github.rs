//! GitHub Device Flow and encrypted token storage for the optional diagnostics uploader.
//!
//! The OAuth client id is deliberately public: it identifies this installed application, not a
//! user.  User credentials are stored encrypted with a key held in Vita Safe Memory.

use anyhow::{Context, Result, bail};
use reqwest::Client;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey};
use ring::{
    aead,
    rand::{SecureRandom, SystemRandom},
};
use serde::{Deserialize, Serialize};

pub const CLIENT_ID: &str = "Ov23lilgeellp1IlFDeH";
const STORE: &str = "ux0:data/opennow-vita/github-auth.json";
const MAGIC: &[u8; 8] = b"ONVGHK01";
const KEY_SIZE: usize = 32;
const RECORD_SIZE: usize = MAGIC.len() + KEY_SIZE;
// Kept separate from the GFN record (0..40) so signing out of either service is independent.
const KEY_OFFSET: i64 = 64;
const NONCE_SIZE: usize = 12;
const AAD: &[u8] = b"opennow-vita/github-oauth/v1";

#[derive(Clone, Debug, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[serde(default)]
    pub verification_uri_complete: Option<String>,
    #[serde(default = "default_interval")]
    pub interval: u64,
    pub expires_in: u64,
}
fn default_interval() -> u64 {
    5
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tokens {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_at: u64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Store {
    version: u8,
    nonce: String,
    ciphertext: String,
}

pub async fn begin(client: &Client) -> Result<DeviceCode> {
    let response = client
        .post("https://github.com/login/device/code")
        .form(&[("client_id", CLIENT_ID), ("scope", "repo")])
        .send()
        .await?
        .error_for_status()?;
    response
        .json()
        .await
        .context("GitHub returned an invalid device code response")
}

/// `Ok(None)` means the user has not approved the code yet; it is not an error.
pub async fn poll(client: &Client, device_code: &str) -> Result<Option<Tokens>> {
    let response: TokenResponse = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", CLIENT_ID),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    if let Some(error) = response.error {
        return match error.as_str() {
            "authorization_pending" | "slow_down" => Ok(None),
            _ => bail!(
                "GitHub login {error}: {}",
                response.error_description.unwrap_or_default()
            ),
        };
    }
    let access_token = response
        .access_token
        .context("GitHub did not return an access token")?;
    Ok(Some(Tokens {
        access_token,
        refresh_token: response.refresh_token,
        expires_at: now().saturating_add(response.expires_in.unwrap_or(28_800)),
    }))
}

pub async fn refresh(client: &Client, tokens: &Tokens) -> Result<Tokens> {
    let refresh_token = tokens
        .refresh_token
        .as_deref()
        .context("GitHub login needs to be repeated")?;
    let response: TokenResponse = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", CLIENT_ID),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let access_token = response.access_token.context(
        response
            .error_description
            .unwrap_or_else(|| "GitHub refused token refresh".to_owned()),
    )?;
    Ok(Tokens {
        access_token,
        refresh_token: response
            .refresh_token
            .or_else(|| tokens.refresh_token.clone()),
        expires_at: now().saturating_add(response.expires_in.unwrap_or(28_800)),
    })
}

/// Refresh slightly early so an upload never starts with a token that expires mid-batch.
pub fn needs_refresh(tokens: &Tokens) -> bool {
    tokens.expires_at <= now().saturating_add(60)
}

pub fn load() -> Option<Tokens> {
    load_inner().ok()
}
pub fn clear() {
    let _ = std::fs::remove_file(STORE);
    let _ = crate::safe_memory::save(KEY_OFFSET, &[0; RECORD_SIZE]);
}

pub fn save(tokens: &Tokens) -> Result<()> {
    let mut data = serde_json::to_vec(tokens)?;
    let key = key_or_create()?;
    let mut nonce = [0; NONCE_SIZE];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| anyhow::anyhow!("GitHub nonce generation failed"))?;
    cipher(&key)?
        .seal_in_place_append_tag(
            Nonce::assume_unique_for_key(nonce),
            Aad::from(AAD),
            &mut data,
        )
        .map_err(|_| anyhow::anyhow!("GitHub token encryption failed"))?;
    std::fs::create_dir_all(crate::logger::data_root())?;
    std::fs::write(
        STORE,
        serde_json::to_vec_pretty(&Store {
            version: 1,
            nonce: hex(&nonce),
            ciphertext: hex(&data),
        })?,
    )?;
    Ok(())
}
fn load_inner() -> Result<Tokens> {
    let value: Store = serde_json::from_slice(&std::fs::read(STORE)?)?;
    if value.version != 1 {
        bail!("unsupported GitHub credential record")
    }
    let nonce: [u8; NONCE_SIZE] = unhex(&value.nonce)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid GitHub nonce"))?;
    let mut data = unhex(&value.ciphertext)?;
    let plain = cipher(&key()?)?
        .open_in_place(
            Nonce::assume_unique_for_key(nonce),
            Aad::from(AAD),
            &mut data,
        )
        .map_err(|_| anyhow::anyhow!("GitHub credential authentication failed"))?;
    Ok(serde_json::from_slice(plain)?)
}
fn cipher(key: &[u8; KEY_SIZE]) -> Result<LessSafeKey> {
    Ok(LessSafeKey::new(
        UnboundKey::new(&aead::CHACHA20_POLY1305, key)
            .map_err(|_| anyhow::anyhow!("GitHub cipher unavailable"))?,
    ))
}
fn key() -> Result<[u8; KEY_SIZE]> {
    let record = crate::safe_memory::load::<RECORD_SIZE>(KEY_OFFSET)?;
    if &record[..MAGIC.len()] != MAGIC {
        bail!("GitHub encryption key missing")
    };
    let mut out = [0; KEY_SIZE];
    out.copy_from_slice(&record[MAGIC.len()..]);
    Ok(out)
}
fn key_or_create() -> Result<[u8; KEY_SIZE]> {
    if let Ok(key) = key() {
        return Ok(key);
    };
    let mut out = [0; KEY_SIZE];
    SystemRandom::new()
        .fill(&mut out)
        .map_err(|_| anyhow::anyhow!("GitHub key generation failed"))?;
    let mut r = [0; RECORD_SIZE];
    r[..MAGIC.len()].copy_from_slice(MAGIC);
    r[MAGIC.len()..].copy_from_slice(&out);
    crate::safe_memory::save(KEY_OFFSET, &r)?;
    Ok(out)
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|x| x.as_secs())
        .unwrap_or(0)
}
fn hex(input: &[u8]) -> String {
    input.iter().map(|x| format!("{x:02x}")).collect()
}
fn unhex(input: &str) -> Result<Vec<u8>> {
    if !input.len().is_multiple_of(2) {
        bail!("odd hexadecimal credential")
    };
    input
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let s = std::str::from_utf8(pair)?;
            u8::from_str_radix(s, 16).context("invalid hexadecimal credential")
        })
        .collect()
}
