use crate::monster::Stage;
use crate::save::{AccountSession, CloudVerificationStatus, SaveFile};
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use reqwest::blocking::{Client, Response};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

const DEFAULT_API_BASE_URL: &str = "https://devimon-api.julienigou33.workers.dev";

#[derive(Debug, Clone, Deserialize)]
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
    ranked_xp_delta: u32,
    snapshot: ProfileSnapshot,
}

#[derive(Debug, Serialize)]
struct ProfileSnapshot {
    name: String,
    hunger: f32,
    energy: f32,
    mood: f32,
    total_xp: u32,
    last_active_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct SyncResponse {
    pub monster_id: String,
    pub synced_at: DateTime<Utc>,
    #[serde(default)]
    pub official_rank: Option<u64>,
    #[serde(default)]
    pub leaderboard_rank: Option<u64>,
    #[serde(default)]
    pub verification_status: Option<CloudVerificationStatus>,
    #[serde(default)]
    pub cloud_total_xp: Option<u32>,
    #[serde(default)]
    pub cloud_level: Option<u32>,
    #[serde(default)]
    pub cloud_stage: Option<Stage>,
    #[serde(default)]
    pub trusted_total_xp: Option<u32>,
    #[serde(default)]
    pub trusted_level: Option<u32>,
    #[serde(default)]
    pub trusted_stage: Option<Stage>,
    #[serde(default)]
    pub accepted_xp_delta: Option<u32>,
    #[serde(default)]
    pub requested_xp_delta: Option<u32>,
    #[serde(default)]
    pub max_accepted_xp_delta: Option<u32>,
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

    let snapshot = {
        let m = state.leaderboard_monster();
        ProfileSnapshot {
            name: m.name.clone(),
            hunger: m.hunger,
            energy: m.energy,
            mood: m.mood,
            total_xp: m.total_xp,
            last_active_at: m.last_active,
        }
    };
    let monster_id = state.cloud.monster_id.as_deref();
    let client = http_client()?;
    let response = client
        .post(api_url("/api/sync"))
        .header("Authorization", auth_header(account))
        .json(&SyncRequest {
            device_id: &state.cloud.device_id,
            monster_id,
            ranked_xp_delta: state.cloud.pending_ranked_xp_delta,
            snapshot,
        })
        .send()
        .map_err(|e| format!("failed to sync monster: {}", e))?;

    let sync: SyncResponse = parse_json(response)?;
    state.cloud.monster_id = Some(sync.monster_id.clone());
    state.cloud.last_synced_at = Some(sync.synced_at);
    state.cloud.cloud_total_xp = sync.cloud_total_xp;
    state.cloud.cloud_level = sync.cloud_level;
    state.cloud.cloud_stage = sync.cloud_stage;
    state.cloud.verification_status = sync.verification_status;
    state.cloud.trusted_total_xp = sync.trusted_total_xp;
    state.cloud.trusted_level = sync.trusted_level;
    state.cloud.trusted_stage = sync.trusted_stage;
    state.cloud.leaderboard_rank = sync.official_rank.or(sync.leaderboard_rank);
    state.cloud.last_accepted_xp_delta = sync.accepted_xp_delta;
    state.cloud.last_requested_xp_delta = sync.requested_xp_delta;
    state.cloud.last_max_accepted_xp_delta = sync.max_accepted_xp_delta;
    if let Some(accepted) = sync.accepted_xp_delta {
        state.cloud.pending_ranked_xp_delta =
            state.cloud.pending_ranked_xp_delta.saturating_sub(accepted);
    }
    state.cloud.sync_dirty = state.cloud.pending_ranked_xp_delta > 0;
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
