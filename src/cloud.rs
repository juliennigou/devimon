use crate::monster::Stage;
use crate::save::{AccountSession, SaveFile};
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use reqwest::blocking::{Client, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

const DEFAULT_API_BASE_URL: &str = "http://127.0.0.1:8787";

#[derive(Debug, Deserialize)]
pub struct StartLoginResponse {
    pub login_id: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval_seconds: u64,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PollLoginStatus {
    Pending,
    Complete,
    Expired,
    Denied,
}

#[derive(Debug, Deserialize)]
pub struct PollLoginResponse {
    pub status: PollLoginStatus,
    pub message: Option<String>,
    pub interval_seconds: Option<u64>,
    pub account: Option<AccountEnvelope>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AccountEnvelope {
    pub account_id: String,
    pub username: String,
    pub session_token: String,
}

impl From<AccountEnvelope> for AccountSession {
    fn from(value: AccountEnvelope) -> Self {
        Self {
            account_id: value.account_id,
            username: value.username,
            session_token: value.session_token,
        }
    }
}

#[derive(Debug, Serialize)]
struct LoginPollRequest<'a> {
    login_id: &'a str,
}

#[derive(Debug, Serialize)]
struct SyncRequest<'a> {
    device_id: &'a str,
    monster_id: Option<&'a str>,
    snapshot: MonsterSnapshot,
}

#[derive(Debug, Serialize)]
struct MonsterSnapshot {
    name: String,
    level: u32,
    xp: u32,
    total_xp: u32,
    stage: Stage,
    hunger: f32,
    energy: f32,
    mood: f32,
    last_active_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SyncResponse {
    pub monster_id: String,
    pub synced_at: DateTime<Utc>,
    pub leaderboard_rank: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct MeResponse {
    pub account_id: String,
    pub username: String,
    pub monster_id: Option<String>,
}

fn api_base_url() -> String {
    env::var("DEVIMON_API_BASE_URL")
        .unwrap_or_else(|_| DEFAULT_API_BASE_URL.to_string())
        .trim_end_matches('/')
        .to_string()
}

fn api_url(path: &str) -> String {
    format!("{}{}", api_base_url(), path)
}

fn http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("failed to build HTTP client: {}", e))
}

fn parse_json<T: DeserializeOwned>(response: Response) -> Result<T, String> {
    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .unwrap_or_else(|_| "failed to read error response".to_string());
        let detail = if body.trim().is_empty() {
            status
                .canonical_reason()
                .unwrap_or("request failed")
                .to_string()
        } else {
            body
        };
        return Err(format!("API {}: {}", status.as_u16(), detail));
    }

    response
        .json()
        .map_err(|e| format!("failed to decode API response: {}", e))
}

fn auth_header(account: &AccountSession) -> String {
    format!("Bearer {}", account.session_token)
}

pub fn start_login() -> Result<StartLoginResponse, String> {
    let client = http_client()?;
    let response = client
        .post(api_url("/api/auth/github/device/start"))
        .send()
        .map_err(|e| format!("failed to start login: {}", e))?;
    parse_json(response)
}

pub fn poll_login(login_id: &str) -> Result<PollLoginResponse, String> {
    let client = http_client()?;
    let response = client
        .post(api_url("/api/auth/github/device/poll"))
        .json(&LoginPollRequest { login_id })
        .send()
        .map_err(|e| format!("failed to poll login: {}", e))?;
    parse_json(response)
}

pub fn fetch_me(account: &AccountSession) -> Result<MeResponse, String> {
    let client = http_client()?;
    let response = client
        .get(api_url("/api/me"))
        .header("Authorization", auth_header(account))
        .send()
        .map_err(|e| format!("failed to fetch account: {}", e))?;
    parse_json(response)
}

pub fn sync_state(state: &mut SaveFile) -> Result<SyncResponse, String> {
    let account = state
        .cloud
        .account
        .as_ref()
        .ok_or_else(|| "not logged in — run `devimon login` first.".to_string())?;

    let snapshot = MonsterSnapshot {
        name: state.monster.name.clone(),
        level: state.monster.level,
        xp: state.monster.xp,
        total_xp: state.monster.total_xp,
        stage: state.monster.stage,
        hunger: state.monster.hunger,
        energy: state.monster.energy,
        mood: state.monster.mood,
        last_active_at: state.monster.last_active,
    };
    let monster_id = state.cloud.monster_id.as_deref();
    let client = http_client()?;
    let response = client
        .post(api_url("/api/sync"))
        .header("Authorization", auth_header(account))
        .json(&SyncRequest {
            device_id: &state.cloud.device_id,
            monster_id,
            snapshot,
        })
        .send()
        .map_err(|e| format!("failed to sync monster: {}", e))?;

    let sync: SyncResponse = parse_json(response)?;
    state.cloud.monster_id = Some(sync.monster_id.clone());
    state.cloud.last_synced_at = Some(sync.synced_at);
    state.cloud.sync_dirty = false;
    Ok(sync)
}

pub fn validate_session(account: &AccountSession) -> Result<(), String> {
    let client = http_client()?;
    let response = client
        .get(api_url("/api/me"))
        .header("Authorization", auth_header(account))
        .send()
        .map_err(|e| format!("failed to validate session: {}", e))?;
    if response.status() == StatusCode::UNAUTHORIZED {
        return Err("stored session is no longer valid; run `devimon login` again.".to_string());
    }
    parse_json::<MeResponse>(response).map(|_| ())
}
